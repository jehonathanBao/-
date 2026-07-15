use std::{
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use tokio::sync::Mutex;

use super::contract_event_routes::ContractRetentionTables;

const FRESH_TTL: Duration = Duration::from_secs(15 * 60);
const STALE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const FAILURE_RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ContractRetentionRuntimeStats {
    pub started: usize,
    pub running: usize,
}

#[derive(Debug, Clone)]
struct CachedRetentionTables {
    value: ContractRetentionTables,
    completed_at: Instant,
    completed_at_ms: i64,
}

#[derive(Debug, Default)]
struct ContractRetentionRuntimeState {
    cached: Option<CachedRetentionTables>,
    in_flight: bool,
    retry_not_before: Option<Instant>,
}

struct ContractRetentionRuntimeInner {
    state: Mutex<ContractRetentionRuntimeState>,
    forced_delay_ms: AtomicU64,
    started: AtomicUsize,
    running: AtomicUsize,
}

#[derive(Clone)]
pub(crate) struct ContractRetentionRuntime {
    inner: Arc<ContractRetentionRuntimeInner>,
}

#[derive(Debug, Clone)]
pub(crate) enum ContractRetentionSnapshotOutcome {
    Fresh {
        value: ContractRetentionTables,
        cache_age: Duration,
        generated_at_ms: i64,
    },
    Stale {
        value: ContractRetentionTables,
        cache_age: Duration,
        generated_at_ms: i64,
    },
    Refreshing,
    RefreshFailed,
}

impl ContractRetentionRuntime {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(ContractRetentionRuntimeInner {
                state: Mutex::new(ContractRetentionRuntimeState::default()),
                forced_delay_ms: AtomicU64::new(0),
                started: AtomicUsize::new(0),
                running: AtomicUsize::new(0),
            }),
        }
    }

    pub(crate) async fn get_or_spawn<F>(&self, compute: F) -> ContractRetentionSnapshotOutcome
    where
        F: FnOnce() -> anyhow::Result<ContractRetentionTables> + Send + 'static,
    {
        let now = Instant::now();
        let mut state = self.inner.state.lock().await;
        if let Some(cached) = state.cached.as_ref() {
            let cache_age = now.saturating_duration_since(cached.completed_at);
            if cache_age <= FRESH_TTL {
                return ContractRetentionSnapshotOutcome::Fresh {
                    value: cached.value.clone(),
                    cache_age,
                    generated_at_ms: cached.completed_at_ms,
                };
            }
        }

        let stale = state.cached.as_ref().and_then(|cached| {
            let cache_age = now.saturating_duration_since(cached.completed_at);
            (cache_age <= STALE_TTL).then(|| ContractRetentionSnapshotOutcome::Stale {
                value: cached.value.clone(),
                cache_age,
                generated_at_ms: cached.completed_at_ms,
            })
        });

        if state.in_flight {
            return stale.unwrap_or(ContractRetentionSnapshotOutcome::Refreshing);
        }
        if state
            .retry_not_before
            .is_some_and(|retry_not_before| now < retry_not_before)
        {
            return stale.unwrap_or(ContractRetentionSnapshotOutcome::RefreshFailed);
        }

        state.in_flight = true;
        drop(state);
        self.spawn_refresh(compute);
        stale.unwrap_or(ContractRetentionSnapshotOutcome::Refreshing)
    }

    fn spawn_refresh<F>(&self, compute: F)
    where
        F: FnOnce() -> anyhow::Result<ContractRetentionTables> + Send + 'static,
    {
        self.inner.started.fetch_add(1, Ordering::SeqCst);
        let inner = self.inner.clone();
        tokio::spawn(async move {
            inner.running.fetch_add(1, Ordering::SeqCst);
            let forced_delay = Duration::from_millis(inner.forced_delay_ms.load(Ordering::SeqCst));
            let result = tokio::task::spawn_blocking(move || {
                if !forced_delay.is_zero() {
                    std::thread::sleep(forced_delay);
                }
                compute()
            })
            .await
            .unwrap_or_else(|error| Err(anyhow::anyhow!("retention refresh join failed: {error}")));
            inner.running.fetch_sub(1, Ordering::SeqCst);

            let mut state = inner.state.lock().await;
            state.in_flight = false;
            match result {
                Ok(value) => {
                    state.cached = Some(CachedRetentionTables {
                        value,
                        completed_at: Instant::now(),
                        completed_at_ms: crate::normalizers::trade::now_ms(),
                    });
                    state.retry_not_before = None;
                }
                Err(error) => {
                    state.retry_not_before = Instant::now().checked_add(FAILURE_RETRY_DELAY);
                    tracing::warn!(
                        target: "btc_toxic_flow_monitor_rs::contract_whale",
                        error = %error,
                        "[CWM] retention snapshot refresh failed"
                    );
                }
            }
        });
    }

    pub(crate) fn set_forced_delay(&self, delay: Duration) {
        self.inner.forced_delay_ms.store(
            delay.as_millis().min(u64::MAX as u128) as u64,
            Ordering::SeqCst,
        );
    }

    pub(crate) fn stats(&self) -> ContractRetentionRuntimeStats {
        ContractRetentionRuntimeStats {
            started: self.inner.started.load(Ordering::SeqCst),
            running: self.inner.running.load(Ordering::SeqCst),
        }
    }
}
