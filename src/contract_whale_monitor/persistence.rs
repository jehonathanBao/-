use crate::{
    contract_whale_monitor::{
        log_events,
        types::{ContractFlowBucket, ContractLiquidationBucket, ContractWhaleSignal},
        LOG_PREFIX, LOG_TARGET,
    },
    normalizers::trade::now_ms,
    storage::{
        contract_whale_repo::{ContractWhaleRepo, ContractWhaleRetentionPruneResult},
        storage_health::{RetentionRunHealth, RetentionTableStatus, StorageHealthTracker},
        SqliteStore,
    },
};

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
            initial_delay_seconds = INITIAL_RETENTION_DELAY_SECS,
            "{} retention task scheduled",
            LOG_PREFIX
        );
        tokio::time::sleep(std::time::Duration::from_secs(INITIAL_RETENTION_DELAY_SECS)).await;
        prune_contract_whale_retention_nonblocking(
            store.clone(),
            flow_1s_days,
            signals_days,
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
    now_ms: i64,
    storage_health: StorageHealthTracker,
) -> Option<ContractWhaleRetentionPruneResult> {
    let flow_cutoff = retention_cutoff_ms(now_ms, flow_1s_days);
    let signal_cutoff = retention_cutoff_ms(now_ms, signals_days);
    let started_at = std::time::Instant::now();
    match tokio::task::spawn_blocking(move || {
        store.prune_contract_whale_retention(flow_cutoff, signal_cutoff)
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
