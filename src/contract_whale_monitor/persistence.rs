use crate::{
    contract_whale_monitor::{
        config::contract_whale_runtime_config,
        impact_baseline::{
            build_robust_impact_baseline, score_event_impact, ImpactBaselineKey,
            RobustImpactBaseline,
        },
        impact_grade::{
            assess_contract_impact_episode, ContractEventImpactAssessment, ContractImpactEpisode,
        },
        log_events,
        types::{
            ContractFlowBucket, ContractFundingSnapshot, ContractLiquidationBucket,
            ContractOiSnapshot, ContractWhaleSignal,
        },
        LOG_PREFIX, LOG_TARGET,
    },
    normalizers::trade::now_ms,
    storage::{
        contract_event_grade_repo::ContractEventGradeRepo,
        contract_whale_repo::{
            ContractWhaleRepo, ContractWhaleRetentionPruneResult, ContractWhaleSignalQuery,
        },
        storage_health::{RetentionRunHealth, RetentionTableStatus, StorageHealthTracker},
        SqliteStore,
    },
};

/// Materialize the V3 assessment beside the legacy signal row. This is kept
/// separate from the legacy payload so shadow rollout and replay remain
/// backward compatible.
pub async fn materialize_contract_whale_impact_grades_nonblocking(
    store: Option<SqliteStore>,
    signals: Vec<ContractWhaleSignal>,
    now_ms: i64,
) -> anyhow::Result<Vec<ContractEventImpactAssessment>> {
    let Some(store) = store else {
        return Ok(Vec::new());
    };
    if signals.is_empty() {
        return Ok(Vec::new());
    }
    tokio::task::spawn_blocking(move || {
        materialize_contract_whale_impact_grades(&store, &signals, now_ms)
    })
    .await
    .map_err(|error| anyhow::anyhow!("impact grade task failed: {error}"))?
}

fn materialize_contract_whale_impact_grades(
    store: &SqliteStore,
    signals: &[ContractWhaleSignal],
    now_ms: i64,
) -> anyhow::Result<Vec<ContractEventImpactAssessment>> {
    let config = contract_whale_runtime_config();
    let grade_repo = ContractEventGradeRepo::new(store.clone());
    let mut baselines = std::collections::BTreeMap::<String, Option<RobustImpactBaseline>>::new();
    let mut assessments = Vec::with_capacity(signals.len());
    for signal in signals {
        let profile = if signal.threshold_profile.trim().is_empty() {
            "default".to_string()
        } else {
            signal.threshold_profile.clone()
        };
        let key_string = format!(
            "{}:{}:{}",
            signal.symbol.to_ascii_uppercase(),
            signal.window_sec,
            profile
        );
        if !baselines.contains_key(&key_string) {
            let samples = store
                .query_contract_whale_signals(&ContractWhaleSignalQuery {
                    symbol: Some(signal.symbol.clone()),
                    window_sec: Some(signal.window_sec),
                    limit: config.impact_grade_v3.baseline_min_samples,
                    ..ContractWhaleSignalQuery::default()
                })?
                .into_iter()
                .map(|row| row.total_volume_btc)
                .collect::<Vec<_>>();
            baselines.insert(
                key_string.clone(),
                build_robust_impact_baseline(
                    ImpactBaselineKey {
                        symbol: signal.symbol.to_ascii_uppercase(),
                        window_sec: signal.window_sec,
                        threshold_profile: profile.clone(),
                    },
                    samples,
                    config.impact_grade_v3.baseline_min_samples,
                ),
            );
        }
        let baseline = baselines.get(&key_string).and_then(Option::as_ref);
        let robust_score =
            baseline.and_then(|baseline| score_event_impact(signal.total_volume_btc, baseline));
        let lifecycle_event_id = if signal.event_lifecycle.event_id.trim().is_empty() {
            signal.id.clone()
        } else {
            signal.event_lifecycle.event_id.clone()
        };
        let sources = signal
            .active_sources
            .contract
            .iter()
            .map(|source| source.exchange.clone())
            .collect::<Vec<_>>();
        let price_move = signal.price_move_pct.map(f64::abs);
        let liquidation_btc = (signal.liquidation_long_btc.max(0.0)
            + signal.liquidation_short_btc.max(0.0))
        .is_sign_positive()
        .then(|| signal.liquidation_long_btc.max(0.0) + signal.liquidation_short_btc.max(0.0));
        let liquidation_usd =
            (signal.liquidation_notional_usd > 0.0).then_some(signal.liquidation_notional_usd);
        let unique_turnover_btc = signal.event_lifecycle.unique_turnover_btc;
        let unique_turnover_usd = unique_turnover_btc.and_then(|volume| {
            signal
                .current_market_price_usd
                .or(signal.order_price_usd)
                .filter(|price| price.is_finite() && *price > 0.0)
                .map(|price| volume * price)
        });
        let episode = ContractImpactEpisode {
            episode_id: lifecycle_event_id,
            symbol: signal.symbol.clone(),
            start_time_ms: signal.event_lifecycle.start_time.max(signal.ts),
            end_time_ms: signal.event_lifecycle.last_update_time.max(signal.ts),
            source_event_ids: std::iter::once(signal.id.clone())
                .chain(signal.merged_from.clone())
                .collect(),
            total_volume_btc: signal
                .event_lifecycle
                .peak_window_volume_btc
                .max(signal.total_volume_btc),
            total_notional_usd: signal.total_notional_usd.max(0.0),
            net_volume_btc: signal.net_volume_btc,
            unique_turnover_btc,
            unique_turnover_notional_usd: unique_turnover_usd,
            live_liquidation_btc: liquidation_btc,
            live_liquidation_notional_usd: liquidation_usd,
            peak_abs_price_move_pct: price_move,
            peak_abs_oi_change_pct: signal.oi_change_pct.map(f64::abs),
            confirmed_sources: sources,
            data_quality: signal.data_quality,
            robust_percentile: robust_score.map(|score| score.percentile),
            robust_z: robust_score.map(|score| score.robust_z),
            baseline_sample_count: robust_score.map(|score| score.sample_count).unwrap_or(0),
        };
        let assessment = assess_contract_impact_episode(&episode, &config, now_ms);
        grade_repo.upsert_assessment(&assessment, now_ms)?;
        assessments.push(assessment);
    }
    Ok(assessments)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractWhalePersistenceOutcome {
    pub attempted: bool,
    pub succeeded: bool,
    pub written: usize,
}

impl ContractWhalePersistenceOutcome {
    fn skipped() -> Self {
        Self {
            attempted: false,
            succeeded: false,
            written: 0,
        }
    }

    fn success(written: usize) -> Self {
        Self {
            attempted: true,
            succeeded: true,
            written,
        }
    }

    fn failed() -> Self {
        Self {
            attempted: true,
            succeeded: false,
            written: 0,
        }
    }
}

pub async fn flush_contract_flow_buckets_nonblocking(
    store: Option<SqliteStore>,
    buckets: Vec<ContractFlowBucket>,
) -> ContractWhalePersistenceOutcome {
    if buckets.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::BUCKET_FLUSHED,
            "{} bucket flush skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    let count = buckets.len();
    let mut per_symbol = std::collections::BTreeMap::<String, usize>::new();
    for bucket in &buckets {
        *per_symbol.entry(bucket.symbol.clone()).or_default() += 1;
    }
    let symbol_breakdown = per_symbol
        .into_iter()
        .map(|(symbol, rows)| format!("{symbol}:{rows}"))
        .collect::<Vec<_>>()
        .join(",");
    match tokio::task::spawn_blocking(move || store.upsert_contract_flow_buckets(&buckets)).await {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::BUCKET_FLUSHED,
                bucket_count = count,
                written = written,
                symbols = symbol_breakdown.as_str(),
                "{} bucket flushed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                bucket_count = count,
                error = %error,
                "{} bucket flush failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                bucket_count = count,
                error = %error,
                "{} bucket flush task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn flush_contract_liquidation_buckets_nonblocking(
    store: Option<SqliteStore>,
    buckets: Vec<ContractLiquidationBucket>,
) -> ContractWhalePersistenceOutcome {
    if buckets.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::BUCKET_FLUSHED,
            "{} liquidation bucket flush skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    let count = buckets.len();
    match tokio::task::spawn_blocking(move || store.upsert_contract_liquidation_buckets(&buckets))
        .await
    {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::BUCKET_FLUSHED,
                bucket_count = count,
                written = written,
                "{} liquidation bucket flushed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                bucket_count = count,
                error = %error,
                "{} liquidation bucket flush failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                bucket_count = count,
                error = %error,
                "{} liquidation bucket flush task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn persist_contract_oi_snapshots_nonblocking(
    store: Option<SqliteStore>,
    snapshots: Vec<ContractOiSnapshot>,
) -> ContractWhalePersistenceOutcome {
    if snapshots.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::BUCKET_FLUSHED,
            "{} oi snapshot flush skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    let count = snapshots.len();
    match tokio::task::spawn_blocking(move || store.upsert_contract_oi_snapshots(&snapshots)).await
    {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::BUCKET_FLUSHED,
                snapshot_count = count,
                written,
                "{} oi snapshot flush success",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                snapshot_count = count,
                error = %error,
                "{} oi snapshot flush failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                snapshot_count = count,
                error = %error,
                "{} oi snapshot flush task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn persist_contract_funding_snapshots_nonblocking(
    store: Option<SqliteStore>,
    snapshots: Vec<ContractFundingSnapshot>,
) -> ContractWhalePersistenceOutcome {
    if snapshots.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::BUCKET_FLUSHED,
            "{} funding snapshot flush skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    let count = snapshots.len();
    match tokio::task::spawn_blocking(move || store.upsert_contract_funding_snapshots(&snapshots))
        .await
    {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::BUCKET_FLUSHED,
                snapshot_count = count,
                written,
                "{} funding snapshot flush success",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                snapshot_count = count,
                error = %error,
                "{} funding snapshot flush failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                snapshot_count = count,
                error = %error,
                "{} funding snapshot flush task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn persist_contract_whale_signal_nonblocking(
    store: Option<SqliteStore>,
    signal: ContractWhaleSignal,
) -> ContractWhalePersistenceOutcome {
    tracing::info!(
        target: LOG_TARGET,
        event = log_events::SIGNAL_GENERATED,
        signal_id = signal.id.as_str(),
        persist_attempt = true,
        "{} signal persistence attempt",
        LOG_PREFIX
    );
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::SIGNAL_GENERATED,
            signal_id = signal.id.as_str(),
            persist_attempt = false,
            persist_skip_reason = "sqlite_store_unavailable",
            "{} signal persistence skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    let signal_id = signal.id.clone();
    match tokio::task::spawn_blocking(move || store.upsert_contract_whale_signal(&signal)).await {
        Ok(Ok(())) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::SIGNAL_GENERATED,
                signal_id = signal_id.as_str(),
                persist_success = true,
                "{} signal persistence success",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(1)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                signal_id = signal_id.as_str(),
                persist_success = false,
                error = %error,
                "{} signal persistence failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                signal_id = signal_id.as_str(),
                persist_success = false,
                error = %error,
                "{} signal persistence task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub async fn persist_contract_whale_signals_nonblocking(
    store: Option<SqliteStore>,
    signals: Vec<ContractWhaleSignal>,
) -> ContractWhalePersistenceOutcome {
    if signals.is_empty() {
        return ContractWhalePersistenceOutcome::success(0);
    }
    let signal_count = signals.len();
    let symbols = signals
        .iter()
        .map(|signal| signal.symbol.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(",");
    tracing::info!(
        target: LOG_TARGET,
        event = log_events::SIGNAL_GENERATED,
        signal_count,
        symbols = symbols.as_str(),
        persist_attempt = true,
        "{} signal batch persistence attempt",
        LOG_PREFIX
    );
    let Some(store) = store else {
        tracing::warn!(
            target: LOG_TARGET,
            event = log_events::SIGNAL_GENERATED,
            signal_count,
            symbols = symbols.as_str(),
            persist_attempt = false,
            persist_skip_reason = "sqlite_store_unavailable",
            "{} signal batch persistence skipped: sqlite store unavailable",
            LOG_PREFIX
        );
        return ContractWhalePersistenceOutcome::skipped();
    };

    match tokio::task::spawn_blocking(move || store.upsert_contract_whale_signals(&signals)).await {
        Ok(Ok(written)) => {
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::SIGNAL_GENERATED,
                signal_count,
                symbols = symbols.as_str(),
                persist_success = true,
                written,
                "{} signal batch persistence success",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::success(written)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                signal_count,
                symbols = symbols.as_str(),
                persist_success = false,
                error = %error,
                "{} signal batch persistence failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                signal_count,
                symbols = symbols.as_str(),
                persist_success = false,
                error = %error,
                "{} signal batch persistence task failed",
                LOG_PREFIX
            );
            ContractWhalePersistenceOutcome::failed()
        }
    }
}

pub fn spawn_contract_whale_retention_task(
    store: Option<SqliteStore>,
    flow_1s_days: i64,
    signals_days: i64,
    impact_b_days: i64,
    storage_health: StorageHealthTracker,
) {
    const INITIAL_RETENTION_DELAY_SECS: u64 = 30;
    let Some(store) = store else {
        return;
    };
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    handle.spawn(async move {
        tracing::info!(
            target: LOG_TARGET,
            event = log_events::RETENTION_PRUNED,
            flow_1s_days,
            signals_days,
            impact_b_days,
            initial_delay_seconds = INITIAL_RETENTION_DELAY_SECS,
            "{} retention task scheduled",
            LOG_PREFIX
        );
        tokio::time::sleep(std::time::Duration::from_secs(INITIAL_RETENTION_DELAY_SECS)).await;
        prune_contract_whale_retention_nonblocking(
            store.clone(),
            flow_1s_days,
            signals_days,
            impact_b_days,
            now_ms(),
            storage_health.clone(),
        )
        .await;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
        interval.tick().await;
        loop {
            interval.tick().await;
            prune_contract_whale_retention_nonblocking(
                store.clone(),
                flow_1s_days,
                signals_days,
                impact_b_days,
                now_ms(),
                storage_health.clone(),
            )
            .await;
        }
    });
}

pub async fn prune_contract_whale_retention_nonblocking(
    store: SqliteStore,
    flow_1s_days: i64,
    signals_days: i64,
    impact_b_days: i64,
    now_ms: i64,
    storage_health: StorageHealthTracker,
) -> Option<ContractWhaleRetentionPruneResult> {
    let flow_cutoff = retention_cutoff_ms(now_ms, flow_1s_days);
    let signal_cutoff = retention_cutoff_ms(now_ms, signals_days);
    let impact_b_cutoff = retention_cutoff_ms(now_ms, impact_b_days);
    let started_at = std::time::Instant::now();
    match tokio::task::spawn_blocking(move || {
        store.prune_contract_whale_retention(flow_cutoff, signal_cutoff, impact_b_cutoff)
    })
    .await
    {
        Ok(Ok(result)) => {
            for table_result in &result.table_results {
                match table_result.status {
                    RetentionTableStatus::Ok => tracing::info!(
                        target: LOG_TARGET,
                        table = table_result.table.as_str(),
                        time_column = table_result.time_column.as_str(),
                        status = table_result.status.as_str(),
                        deleted = table_result.deleted_rows,
                        duration_ms = table_result.duration_ms,
                        "{} retention table result",
                        LOG_PREFIX
                    ),
                    RetentionTableStatus::Skipped => tracing::warn!(
                        target: LOG_TARGET,
                        table = table_result.table.as_str(),
                        time_column = table_result.time_column.as_str(),
                        status = table_result.status.as_str(),
                        reason = table_result.reason.as_deref().unwrap_or("unknown"),
                        duration_ms = table_result.duration_ms,
                        "{} retention table skipped",
                        LOG_PREFIX
                    ),
                    RetentionTableStatus::Error => tracing::warn!(
                        target: LOG_TARGET,
                        table = table_result.table.as_str(),
                        time_column = table_result.time_column.as_str(),
                        status = table_result.status.as_str(),
                        error_kind = table_result.error_kind.as_deref().unwrap_or("unknown"),
                        error = table_result.error.as_deref().unwrap_or("unknown"),
                        duration_ms = table_result.duration_ms,
                        "{} retention table failed",
                        LOG_PREFIX
                    ),
                }
            }
            let total_deleted_rows = result.flow_1s_deleted
                + result.liquidation_deleted
                + result.oi_deleted
                + result.funding_deleted
                + result.percentile_deleted
                + result.signal_deleted;
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::RETENTION_PRUNED,
                flow_cutoff_ts = result.flow_cutoff_ts,
                signal_cutoff_ts = result.signal_cutoff_ts,
                flow_1s_deleted = result.flow_1s_deleted,
                liquidation_deleted = result.liquidation_deleted,
                oi_deleted = result.oi_deleted,
                funding_deleted = result.funding_deleted,
                percentile_deleted = result.percentile_deleted,
                signal_deleted = result.signal_deleted,
                total_deleted_rows = total_deleted_rows,
                failed_tables = result
                    .table_results
                    .iter()
                    .filter(|entry| entry.status == RetentionTableStatus::Error)
                    .count(),
                skipped_tables = result
                    .table_results
                    .iter()
                    .filter(|entry| entry.status == RetentionTableStatus::Skipped)
                    .count(),
                protected_s_count = result.protected_s_count,
                protected_net_volume_count = result.protected_net_volume_count,
                duration_ms = started_at.elapsed().as_millis() as u64,
                "{} retention pruned",
                LOG_PREFIX
            );
            storage_health.record_contract_whale_retention(
                RetentionRunHealth {
                    ok: result
                        .table_results
                        .iter()
                        .all(|entry| entry.status != RetentionTableStatus::Error),
                    total_deleted_rows,
                    failed_tables: result
                        .table_results
                        .iter()
                        .filter(|entry| entry.status == RetentionTableStatus::Error)
                        .map(|entry| entry.table.clone())
                        .collect(),
                    skipped_tables: result
                        .table_results
                        .iter()
                        .filter(|entry| entry.status == RetentionTableStatus::Skipped)
                        .map(|entry| entry.table.clone())
                        .collect(),
                    error: None,
                    duration_ms: Some(started_at.elapsed().as_millis() as u64),
                    finished_at_ms: Some(now_ms),
                },
                result.wal_checkpoint.clone(),
            );
            storage_health.refresh_now();
            Some(result)
        }
        Ok(Err(error)) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                error = %error,
                "{} retention prune failed",
                LOG_PREFIX
            );
            storage_health.record_contract_whale_retention(
                RetentionRunHealth {
                    ok: false,
                    total_deleted_rows: 0,
                    failed_tables: Vec::new(),
                    skipped_tables: Vec::new(),
                    error: Some(error.to_string()),
                    duration_ms: Some(started_at.elapsed().as_millis() as u64),
                    finished_at_ms: Some(now_ms),
                },
                None,
            );
            storage_health.refresh_now();
            None
        }
        Err(error) => {
            tracing::warn!(
                target: LOG_TARGET,
                event = log_events::ERROR,
                error = %error,
                "{} retention prune task failed",
                LOG_PREFIX
            );
            storage_health.record_contract_whale_retention(
                RetentionRunHealth {
                    ok: false,
                    total_deleted_rows: 0,
                    failed_tables: Vec::new(),
                    skipped_tables: Vec::new(),
                    error: Some(error.to_string()),
                    duration_ms: Some(started_at.elapsed().as_millis() as u64),
                    finished_at_ms: Some(now_ms),
                },
                None,
            );
            storage_health.refresh_now();
            None
        }
    }
}

fn retention_cutoff_ms(now_ms: i64, retention_days: i64) -> i64 {
    let safe_days = retention_days.max(1);
    now_ms.saturating_sub(safe_days.saturating_mul(24 * 60 * 60 * 1000))
}
