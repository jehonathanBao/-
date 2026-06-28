use crate::{
    contract_whale_monitor::{
        log_events,
        types::{ContractFlowBucket, ContractLiquidationBucket, ContractWhaleSignal},
        LOG_PREFIX, LOG_TARGET,
    },
    normalizers::trade::now_ms,
    storage::{
        contract_whale_repo::{ContractWhaleRepo, ContractWhaleRetentionPruneResult},
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
) {
    let Some(store) = store else {
        return;
    };
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    handle.spawn(async move {
        prune_contract_whale_retention_nonblocking(
            store.clone(),
            flow_1s_days,
            signals_days,
            now_ms(),
        )
        .await;
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
        loop {
            interval.tick().await;
            prune_contract_whale_retention_nonblocking(
                store.clone(),
                flow_1s_days,
                signals_days,
                now_ms(),
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
            tracing::info!(
                target: LOG_TARGET,
                event = log_events::RETENTION_PRUNED,
                flow_cutoff_ts = result.flow_cutoff_ts,
                signal_cutoff_ts = result.signal_cutoff_ts,
                flow_1s_deleted = result.flow_1s_deleted,
                signal_deleted = result.signal_deleted,
                protected_s_count = result.protected_s_count,
                protected_net_volume_count = result.protected_net_volume_count,
                duration_ms = started_at.elapsed().as_millis() as u64,
                "{} retention pruned",
                LOG_PREFIX
            );
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
            None
        }
    }
}

fn retention_cutoff_ms(now_ms: i64, retention_days: i64) -> i64 {
    let safe_days = retention_days.max(1);
    now_ms.saturating_sub(safe_days.saturating_mul(24 * 60 * 60 * 1000))
}
