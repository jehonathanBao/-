use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use tokio::sync::{watch, Mutex, Semaphore};

use super::contract_event_routes::{ContractEventPage, FinalEventsV2Response};

const MAX_RUNNING: usize = 2;
const MAX_ENTRIES: usize = 64;
const FRESH_TTL: Duration = Duration::from_secs(10);
const STALE_TTL: Duration = Duration::from_secs(300);
const WAIT_BUDGET: Duration = Duration::from_secs(4);
const RETRY_AFTER_MS: u64 = 2_000;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ProjectionKey(String);

impl ProjectionKey {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectionFailure {
    pub(crate) error_code: &'static str,
}

impl ProjectionFailure {
    pub(crate) fn new(error_code: &'static str) -> Self {
        Self { error_code }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionUnavailableReason {
    Busy,
    Timeout,
    Failed,
    RefreshInProgress,
}

impl ProjectionUnavailableReason {
    pub(crate) fn error_code(self) -> &'static str {
        match self {
            Self::Busy => "contract_projection_busy",
            Self::Timeout => "contract_projection_timeout",
            Self::Failed => "contract_projection_failed",
            Self::RefreshInProgress => "contract_projection_refresh_in_progress",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectionUnavailable {
    pub(crate) reason: ProjectionUnavailableReason,
    pub(crate) retry_after_ms: u64,
}

impl ProjectionUnavailable {
    pub(crate) fn error_code(&self) -> &'static str {
        self.reason.error_code()
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ProjectionOutcome<T> {
    Fresh {
        value: T,
        cache_age: Duration,
        completed_at_ms: i64,
    },
    Stale {
        value: T,
        cache_age: Duration,
        completed_at_ms: i64,
        reason: ProjectionUnavailableReason,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum ContractEventProjectionValue {
    ContractEvents(ContractEventPage),
    FinalEventsV2(FinalEventsV2Response),
}

pub(crate) type ContractEventProjectionRuntime = ProjectionRuntime<ContractEventProjectionValue>;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectionRuntimeStats {
    pub started: usize,
    pub running: usize,
    pub max_running: usize,
    pub cache_entries: usize,
    pub in_flight: usize,
}

#[derive(Debug, Clone)]
struct CachedProjection<T> {
    completed_at: Instant,
    completed_at_ms: i64,
    value: T,
}

type SharedProjectionResult<T> = Arc<Result<CachedProjection<T>, ProjectionFailure>>;
type ProjectionReceiver<T> = watch::Receiver<Option<SharedProjectionResult<T>>>;

struct ProjectionRuntimeState<T> {
    cache: BTreeMap<ProjectionKey, CachedProjection<T>>,
    in_flight: BTreeMap<ProjectionKey, ProjectionReceiver<T>>,
}

impl<T> Default for ProjectionRuntimeState<T> {
    fn default() -> Self {
        Self {
            cache: BTreeMap::new(),
            in_flight: BTreeMap::new(),
        }
    }
}

struct ProjectionRuntimeInner<T> {
    semaphore: Arc<Semaphore>,
    state: Mutex<ProjectionRuntimeState<T>>,
    max_entries: usize,
    fresh_ttl: Duration,
    stale_ttl: Duration,
    wait_budget_ms: AtomicU64,
    forced_delay_ms: AtomicU64,
    started: AtomicUsize,
    running: AtomicUsize,
    max_running: AtomicUsize,
    cache_entries: AtomicUsize,
    in_flight: AtomicUsize,
}

pub(crate) struct ProjectionRuntime<T> {
    inner: Arc<ProjectionRuntimeInner<T>>,
}

impl<T> Clone for ProjectionRuntime<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> ProjectionRuntime<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub(crate) fn new() -> Self {
        Self::with_config(MAX_RUNNING, MAX_ENTRIES, FRESH_TTL, STALE_TTL, WAIT_BUDGET)
    }

    fn with_config(
        max_running: usize,
        max_entries: usize,
        fresh_ttl: Duration,
        stale_ttl: Duration,
        wait_budget: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(ProjectionRuntimeInner {
                semaphore: Arc::new(Semaphore::new(max_running.max(1))),
                state: Mutex::new(ProjectionRuntimeState::default()),
                max_entries: max_entries.max(1),
                fresh_ttl,
                stale_ttl: stale_ttl.max(fresh_ttl),
                wait_budget_ms: AtomicU64::new(wait_budget.as_millis().min(u64::MAX as u128) as u64),
                forced_delay_ms: AtomicU64::new(0),
                started: AtomicUsize::new(0),
                running: AtomicUsize::new(0),
                max_running: AtomicUsize::new(0),
                cache_entries: AtomicUsize::new(0),
                in_flight: AtomicUsize::new(0),
            }),
        }
    }

    pub(crate) fn set_forced_delay(&self, delay: Duration) {
        self.inner.forced_delay_ms.store(
            delay.as_millis().min(u64::MAX as u128) as u64,
            Ordering::SeqCst,
        );
    }

    pub(crate) fn set_wait_budget(&self, wait_budget: Duration) {
        self.inner.wait_budget_ms.store(
            wait_budget.as_millis().min(u64::MAX as u128) as u64,
            Ordering::SeqCst,
        );
    }

    pub(crate) async fn expire_cache_by(&self, age: Duration) {
        let expired_at = Instant::now().checked_sub(age).unwrap_or_else(Instant::now);
        let mut state = self.inner.state.lock().await;
        for cached in state.cache.values_mut() {
            cached.completed_at = expired_at;
        }
    }

    pub(crate) fn stats(&self) -> ProjectionRuntimeStats {
        ProjectionRuntimeStats {
            started: self.inner.started.load(Ordering::SeqCst),
            running: self.inner.running.load(Ordering::SeqCst),
            max_running: self.inner.max_running.load(Ordering::SeqCst),
            cache_entries: self.inner.cache_entries.load(Ordering::SeqCst),
            in_flight: self.inner.in_flight.load(Ordering::SeqCst),
        }
    }

    pub(crate) async fn get_or_spawn<F>(
        &self,
        key: ProjectionKey,
        compute: F,
    ) -> Result<ProjectionOutcome<T>, ProjectionUnavailable>
    where
        F: FnOnce() -> Result<T, ProjectionFailure> + Send + 'static,
    {
        let now = Instant::now();
        let mut stale_candidate = None;
        let mut receiver = None;
        let mut sender = None;
        let mut at_capacity = false;

        {
            let mut state = self.inner.state.lock().await;
            if let Some(cached) = state.cache.get(&key) {
                let cache_age = now.saturating_duration_since(cached.completed_at);
                if cache_age <= self.inner.fresh_ttl {
                    return Ok(ProjectionOutcome::Fresh {
                        value: cached.value.clone(),
                        cache_age,
                        completed_at_ms: cached.completed_at_ms,
                    });
                }
                if cache_age <= self.inner.stale_ttl {
                    stale_candidate = Some((cached.clone(), cache_age));
                }
            }

            if let Some(in_flight) = state.in_flight.get(&key) {
                receiver = Some(in_flight.clone());
            } else if state.in_flight.len() >= self.inner.max_entries {
                at_capacity = true;
            } else {
                let (new_sender, new_receiver) = watch::channel(None);
                state.in_flight.insert(key.clone(), new_receiver.clone());
                self.inner
                    .in_flight
                    .store(state.in_flight.len(), Ordering::SeqCst);
                receiver = Some(new_receiver);
                sender = Some(new_sender);
            }
        }

        if at_capacity {
            if let Some((cached, cache_age)) = stale_candidate {
                return Ok(stale_outcome(
                    cached,
                    cache_age,
                    ProjectionUnavailableReason::Busy,
                ));
            }
            return Err(unavailable(ProjectionUnavailableReason::Busy));
        }

        if let Some(sender) = sender {
            self.spawn_job(key, sender, compute);
        }

        if let Some((cached, cache_age)) = stale_candidate {
            return Ok(stale_outcome(
                cached,
                cache_age,
                ProjectionUnavailableReason::RefreshInProgress,
            ));
        }

        let receiver = receiver.ok_or_else(|| unavailable(ProjectionUnavailableReason::Failed))?;
        let wait_budget = Duration::from_millis(self.inner.wait_budget_ms.load(Ordering::SeqCst));
        match tokio::time::timeout(wait_budget, receive_projection(receiver)).await {
            Ok(Ok(result)) => match result.as_ref() {
                Ok(cached) => Ok(ProjectionOutcome::Fresh {
                    value: cached.value.clone(),
                    cache_age: cached.completed_at.elapsed(),
                    completed_at_ms: cached.completed_at_ms,
                }),
                Err(_) => Err(unavailable(ProjectionUnavailableReason::Failed)),
            },
            Ok(Err(_)) => Err(unavailable(ProjectionUnavailableReason::Failed)),
            Err(_) => Err(unavailable(ProjectionUnavailableReason::Timeout)),
        }
    }

    fn spawn_job<F>(
        &self,
        key: ProjectionKey,
        sender: watch::Sender<Option<SharedProjectionResult<T>>>,
        compute: F,
    ) where
        F: FnOnce() -> Result<T, ProjectionFailure> + Send + 'static,
    {
        self.inner.started.fetch_add(1, Ordering::SeqCst);
        let inner = self.inner.clone();
        tokio::spawn(async move {
            let permit = match inner.semaphore.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => {
                    finish_projection(
                        &inner,
                        &key,
                        &sender,
                        Err(ProjectionFailure::new("projection_runtime_closed")),
                    )
                    .await;
                    return;
                }
            };
            let running = inner.running.fetch_add(1, Ordering::SeqCst) + 1;
            inner.max_running.fetch_max(running, Ordering::SeqCst);
            let forced_delay = Duration::from_millis(inner.forced_delay_ms.load(Ordering::SeqCst));
            let result = tokio::task::spawn_blocking(move || {
                if !forced_delay.is_zero() {
                    std::thread::sleep(forced_delay);
                }
                compute()
            })
            .await
            .unwrap_or_else(|_| Err(ProjectionFailure::new("projection_join_failed")));
            inner.running.fetch_sub(1, Ordering::SeqCst);
            drop(permit);
            finish_projection(&inner, &key, &sender, result).await;
        });
    }
}

fn unavailable(reason: ProjectionUnavailableReason) -> ProjectionUnavailable {
    ProjectionUnavailable {
        reason,
        retry_after_ms: RETRY_AFTER_MS,
    }
}

fn stale_outcome<T>(
    cached: CachedProjection<T>,
    cache_age: Duration,
    reason: ProjectionUnavailableReason,
) -> ProjectionOutcome<T> {
    ProjectionOutcome::Stale {
        value: cached.value,
        cache_age,
        completed_at_ms: cached.completed_at_ms,
        reason,
    }
}

async fn receive_projection<T>(
    mut receiver: ProjectionReceiver<T>,
) -> Result<SharedProjectionResult<T>, ()>
where
    T: Clone,
{
    if let Some(result) = receiver.borrow().clone() {
        return Ok(result);
    }
    receiver.changed().await.map_err(|_| ())?;
    let result = receiver.borrow().clone().ok_or(())?;
    Ok(result)
}

async fn finish_projection<T>(
    inner: &Arc<ProjectionRuntimeInner<T>>,
    key: &ProjectionKey,
    sender: &watch::Sender<Option<SharedProjectionResult<T>>>,
    result: Result<T, ProjectionFailure>,
) where
    T: Clone,
{
    let completed = result.map(|value| CachedProjection {
        completed_at: Instant::now(),
        completed_at_ms: crate::normalizers::trade::now_ms(),
        value,
    });
    let shared = Arc::new(completed);
    let mut state = inner.state.lock().await;
    state.in_flight.remove(key);
    inner
        .in_flight
        .store(state.in_flight.len(), Ordering::SeqCst);
    if let Ok(cached) = shared.as_ref() {
        state.cache.insert(key.clone(), cached.clone());
        while state.cache.len() > inner.max_entries {
            let oldest_key = state
                .cache
                .iter()
                .min_by_key(|(_, entry)| entry.completed_at)
                .map(|(cache_key, _)| cache_key.clone());
            let Some(oldest_key) = oldest_key else {
                break;
            };
            state.cache.remove(&oldest_key);
        }
    }
    inner
        .cache_entries
        .store(state.cache.len(), Ordering::SeqCst);
    drop(state);
    let _ = sender.send(Some(shared));
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use super::{
        ProjectionFailure, ProjectionKey, ProjectionOutcome, ProjectionRuntime,
        ProjectionUnavailableReason,
    };

    fn test_runtime(
        max_running: usize,
        max_entries: usize,
        fresh_ttl: Duration,
        stale_ttl: Duration,
        wait_budget: Duration,
    ) -> ProjectionRuntime<usize> {
        ProjectionRuntime::with_config(max_running, max_entries, fresh_ttl, stale_ttl, wait_budget)
    }

    fn key(value: &str) -> ProjectionKey {
        ProjectionKey::new(value)
    }

    fn outcome_value(outcome: ProjectionOutcome<usize>) -> usize {
        match outcome {
            ProjectionOutcome::Fresh { value, .. } | ProjectionOutcome::Stale { value, .. } => {
                value
            }
        }
    }

    #[tokio::test]
    async fn equivalent_keys_execute_projection_once() {
        let runtime = test_runtime(
            2,
            64,
            Duration::from_secs(1),
            Duration::from_secs(5),
            Duration::from_secs(1),
        );
        let starts = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();

        for _ in 0..8 {
            let runtime = runtime.clone();
            let starts = starts.clone();
            tasks.push(tokio::spawn(async move {
                runtime
                    .get_or_spawn(key("events:ETH:24h:20"), move || {
                        starts.fetch_add(1, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(50));
                        Ok(7)
                    })
                    .await
                    .map(outcome_value)
            }));
        }

        for task in tasks {
            assert_eq!(task.await.unwrap().unwrap(), 7);
        }
        assert_eq!(starts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn projection_concurrency_never_exceeds_two() {
        let runtime = test_runtime(
            2,
            64,
            Duration::from_secs(1),
            Duration::from_secs(5),
            Duration::from_secs(2),
        );
        let current = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();

        for index in 0..6 {
            let runtime = runtime.clone();
            let current = current.clone();
            let maximum = maximum.clone();
            tasks.push(tokio::spawn(async move {
                runtime
                    .get_or_spawn(key(&format!("events:ETH:{index}")), move || {
                        let running = current.fetch_add(1, Ordering::SeqCst) + 1;
                        maximum.fetch_max(running, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(60));
                        current.fetch_sub(1, Ordering::SeqCst);
                        Ok(index)
                    })
                    .await
            }));
        }

        for task in tasks {
            assert!(task.await.unwrap().is_ok());
        }
        assert_eq!(maximum.load(Ordering::SeqCst), 2);
        assert_eq!(runtime.stats().max_running, 2);
    }

    #[tokio::test]
    async fn fresh_cache_skips_projection() {
        let runtime = test_runtime(
            2,
            64,
            Duration::from_secs(1),
            Duration::from_secs(5),
            Duration::from_secs(1),
        );
        let starts = Arc::new(AtomicUsize::new(0));
        let first_starts = starts.clone();
        let first = runtime
            .get_or_spawn(key("events:ETH:fresh"), move || {
                first_starts.fetch_add(1, Ordering::SeqCst);
                Ok(11)
            })
            .await
            .unwrap();
        let second_starts = starts.clone();
        let second = runtime
            .get_or_spawn(key("events:ETH:fresh"), move || {
                second_starts.fetch_add(1, Ordering::SeqCst);
                Ok(12)
            })
            .await
            .unwrap();

        assert_eq!(outcome_value(first), 11);
        assert_eq!(outcome_value(second), 11);
        assert_eq!(starts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn timed_out_waiter_receives_stale_cache() {
        let runtime = test_runtime(
            1,
            64,
            Duration::from_millis(10),
            Duration::from_secs(1),
            Duration::from_millis(20),
        );
        runtime
            .get_or_spawn(key("events:ETH:stale"), || Ok(21))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;

        let outcome = runtime
            .get_or_spawn(key("events:ETH:stale"), || {
                std::thread::sleep(Duration::from_millis(80));
                Ok(22)
            })
            .await
            .unwrap();

        match outcome {
            ProjectionOutcome::Stale {
                value,
                reason,
                cache_age,
                ..
            } => {
                assert_eq!(value, 21);
                assert_eq!(reason, ProjectionUnavailableReason::RefreshInProgress);
                assert!(cache_age >= Duration::from_millis(10));
            }
            ProjectionOutcome::Fresh { .. } => panic!("expected stale cache"),
        }
    }

    #[tokio::test]
    async fn timeout_without_cache_returns_unavailable() {
        let runtime = test_runtime(
            1,
            64,
            Duration::from_millis(10),
            Duration::from_secs(1),
            Duration::from_millis(20),
        );

        let error = runtime
            .get_or_spawn(key("events:ETH:no-cache"), || {
                std::thread::sleep(Duration::from_millis(80));
                Ok(31)
            })
            .await
            .unwrap_err();

        assert_eq!(error.reason, ProjectionUnavailableReason::Timeout);
        assert_eq!(error.error_code(), "contract_projection_timeout");
        assert_eq!(error.retry_after_ms, 2_000);
    }

    #[tokio::test]
    async fn failed_refresh_preserves_stale_cache() {
        let runtime = test_runtime(
            1,
            64,
            Duration::from_millis(10),
            Duration::from_secs(1),
            Duration::from_millis(50),
        );
        runtime
            .get_or_spawn(key("events:ETH:failed-refresh"), || Ok(41))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(20)).await;

        let stale = runtime
            .get_or_spawn(key("events:ETH:failed-refresh"), || {
                Err(ProjectionFailure::new("projection_failed"))
            })
            .await
            .unwrap();
        assert_eq!(outcome_value(stale), 41);

        tokio::time::timeout(Duration::from_secs(1), async {
            while runtime.stats().in_flight > 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(runtime.stats().cache_entries, 1);
    }

    #[tokio::test]
    async fn cache_evicts_oldest_success_at_capacity() {
        let runtime = test_runtime(
            2,
            2,
            Duration::from_secs(5),
            Duration::from_secs(5),
            Duration::from_secs(1),
        );
        runtime
            .get_or_spawn(key("events:1"), || Ok(1))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(2)).await;
        runtime
            .get_or_spawn(key("events:2"), || Ok(2))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(2)).await;
        runtime
            .get_or_spawn(key("events:3"), || Ok(3))
            .await
            .unwrap();

        let starts = Arc::new(AtomicUsize::new(0));
        let starts_for_job = starts.clone();
        let outcome = runtime
            .get_or_spawn(key("events:1"), move || {
                starts_for_job.fetch_add(1, Ordering::SeqCst);
                Ok(10)
            })
            .await
            .unwrap();

        assert_eq!(outcome_value(outcome), 10);
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert_eq!(runtime.stats().cache_entries, 2);
    }

    #[tokio::test]
    async fn dropped_waiter_does_not_cancel_refresh() {
        let runtime = test_runtime(
            1,
            64,
            Duration::from_secs(1),
            Duration::from_secs(5),
            Duration::from_secs(1),
        );
        let starts = Arc::new(AtomicUsize::new(0));
        let task_runtime = runtime.clone();
        let task_starts = starts.clone();
        let waiter = tokio::spawn(async move {
            task_runtime
                .get_or_spawn(key("events:ETH:dropped"), move || {
                    task_starts.fetch_add(1, Ordering::SeqCst);
                    std::thread::sleep(Duration::from_millis(80));
                    Ok(51)
                })
                .await
        });

        tokio::time::timeout(Duration::from_secs(1), async {
            while runtime.stats().started == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        waiter.abort();
        tokio::time::timeout(Duration::from_secs(1), async {
            while runtime.stats().in_flight > 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        let outcome = runtime
            .get_or_spawn(key("events:ETH:dropped"), || Ok(52))
            .await
            .unwrap();
        assert_eq!(outcome_value(outcome), 51);
        assert_eq!(starts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn blocking_projection_does_not_block_current_thread_runtime() {
        let runtime = test_runtime(
            1,
            64,
            Duration::from_secs(1),
            Duration::from_secs(5),
            Duration::from_secs(1),
        );
        let projection = tokio::spawn(async move {
            runtime
                .get_or_spawn(key("events:ETH:blocking"), || {
                    std::thread::sleep(Duration::from_millis(200));
                    Ok(61)
                })
                .await
        });

        tokio::time::timeout(Duration::from_millis(100), async {
            tokio::time::sleep(Duration::from_millis(20)).await;
        })
        .await
        .expect("Tokio timer was blocked by projection work");

        assert_eq!(outcome_value(projection.await.unwrap().unwrap()), 61);
    }
}
