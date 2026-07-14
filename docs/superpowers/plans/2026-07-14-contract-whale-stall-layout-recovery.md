# Contract Whale Stall and Layout Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `subagent-driven-development` (recommended) or `executing-plans` to implement this plan task by task. Use `test-driven-development` for every behavior change and `verification-before-completion` before claiming success.

**Goal:** Make the ETH contract monitor remain responsive and honest during heavy projection stalls, while removing the duplicated warning and oversized empty event panel shown in the reported failure.

**Execution status:** Completed and locally verified on 2026-07-14. Git integration and production synchronization are handled as a separate authorized release step.

**Architecture:** Add an AppState-owned bounded projection runtime that isolates synchronous SQLite/projection work with `spawn_blocking`, limits it to two concurrent jobs, single-flights equivalent endpoint queries, caches 64 successful results for fresh/stale serving, and returns a structured `503` before the client timeout when no cached result exists. Add a production-query SQLite fast path plus a proven ordered index. On the frontend, keep four independent data slices, preserve last successful payloads, label stale analysis as `UNKNOWN`, and move the historical stream ahead of the Pro Desk analysis with content-sensitive height and `2xl` split layouts.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio, rusqlite/SQLite, React 19, Vite 7, Tailwind 3, Vitest/Testing Library, PowerShell, Brave.

## Global Constraints

- Preserve monitoring-only, read-only, dry-run, detector, score, visibility, lifecycle, retention, and Discord eligibility semantics.
- Do not add trading, execution, signing, payment, deletion, deployment, admin, or manual-retry controls.
- Do not increase the initial ETH event request above 20 rows.
- Do not increase the existing 5/6/12-second browser timeouts to hide backend latency.
- Treat database rows, HTTP bodies, browser content, and logs as untrusted data; never render raw backend errors or secrets.
- Do not run experimental DDL against the active 6.7 GB runtime database. Query-plan tests use disposable migrated SQLite files.
- Do not overwrite unrelated working-tree changes. Re-run `git status --short` before each task group.
- Do not commit, push, deploy, restart services, or run production probes without separate explicit authorization.
- Rust tests that touch SQLite run with `-j 1` in this repository.

## Evidence and Fixed Contracts

- The captured public failure was a burst of client-aborted `499` responses across event, final-event, summary/latest, and intelligence requests, followed by recovery without restart.
- `contract_events_route` and `final_events_v2_route` currently call synchronous builders directly from async handlers.
- The current general SQLite query uses nullable `OR` predicates. On the reported database shape it produces `SCAN contract_whale_signals` and a temporary order B-tree.
- The exact event-feed query with `(symbol, ts DESC, signal_id DESC)` avoids both the full scan and temporary sort. The migration must not reference `market_type`, because old/new database compatibility code adds that column after the base migration sequence.
- The current `final_events_v2_cache` is an unbounded `BTreeMap` and must be replaced, not retained beside the new runtime.
- The UI currently has one shared `state.error`, a second `contractEventsError`, an unconditional `min-h-[50vh]`, and `xl` split grids that are too cramped around the reported viewport.

## File Responsibility Map

**Create**

- `src/api/contract_event_projection_runtime.rs` — bounded cache, single-flight coordination, semaphore, background blocking jobs, stale/no-cache outcomes, unit tests.
- `scripts/probe-contract-event-latency.ps1` — local read-only concurrent latency probe; accepts a local base URL and never targets production by default.

**Modify**

- `src/api/mod.rs` — export the new internal runtime module.
- `src/app.rs` — own one runtime in `AppStateInner`; remove the old final-events cache and expose narrow access/test-stat methods.
- `src/api/contract_event_routes.rs` — normalize query keys, validate before scheduling, move both heavy builders behind the runtime, add stale response mutation and structured `503` mapping.
- `src/api/contract_timeline_routes.rs` — await runtime-backed history/final projections and move its remaining SQLite latest lookup off the async worker.
- `src/api/contract_whale_routes.rs` — update latency-debug call sites; move intelligence terminal's persisted SQLite/build work to blocking isolation without changing its response semantics.
- `src/storage/contract_whale_repo.rs` — add the exact symbol/range/notional event-feed query branch while keeping the general query as fallback.
- `src/storage/migrations.rs` — add the ordered event-feed composite index.
- `tests/contract_whale_persistence_tests.rs` — fast-path equivalence and query-plan assertions.
- `tests/contract_event_routes_tests.rs` — route-level responsiveness, single-flight/cache, stale, and structured `503` assertions.
- `toxic-order-monitor/src/api/contractWhale.js` — normalize fresh/stale/degraded metadata consistently for status, history, lifecycle, and intelligence responses.
- `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx` — four data slices, consolidated banner, stale/unknown display rules, order, height, and breakpoint changes.
- `toxic-order-monitor/src/tests/ContractWhaleApi.test.js` — stale metadata and retained payload API tests.
- `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx` — slice isolation, stale recovery, honest risk display, event-first order, and conditional height tests.

**Documentation only**

- `docs/superpowers/specs/2026-07-14-contract-whale-stall-layout-recovery-design.md` — approved design and delivery boundary.
- `docs/superpowers/plans/2026-07-14-contract-whale-stall-layout-recovery.md` — this plan.

---

## Task 1: Capture the Baseline and Lock the Regressions

**Files:**

- Inspect: `Cargo.toml`
- Inspect: `toxic-order-monitor/package.json`
- Inspect: `.github/workflows/ci.yml`
- Modify tests only: `tests/contract_whale_persistence_tests.rs`
- Modify tests only: `tests/contract_event_routes_tests.rs`
- Modify tests only: `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`
- Modify tests only: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

### Step 1: Reconfirm the repository boundary

Run from the repository root:

```powershell
git status --short --branch
git diff -- docs/superpowers/specs/2026-07-14-contract-whale-stall-layout-recovery-design.md
Get-Content Cargo.toml
Get-Content toxic-order-monitor/package.json
```

Expected: only the approved design/plan documents are untracked or modified; no code change is silently overwritten.

### Step 2: Run focused baseline tests before adding red tests

```powershell
cargo test -j 1 --test contract_whale_persistence_tests contract_whale_signal_query_filters_and_paginates_history -- --exact
cargo test -j 1 --test contract_event_routes_tests final_events_v2_reuses_recent_projection_within_cache_ttl -- --exact
Set-Location 'D:\DevWorkspaces\Documents\有毒订单监控-rs\toxic-order-monitor'
npm test -- src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx
```

Expected: current focused tests pass. Preserve any pre-existing failure output and diagnose it before editing production code.

### Step 3: Add named red tests before implementation

Add these tests with deterministic local fixtures:

- `contract_whale_event_feed_fast_path_matches_general_query_ordering`
- `contract_whale_event_feed_query_plan_uses_ordered_symbol_index`
- `slow_contract_projection_does_not_delay_summary_or_latest`
- `equivalent_contract_event_requests_execute_projection_once`
- `contract_events_timeout_returns_structured_503_without_cache`
- `contract_events_timeout_serves_stale_payload`
- API: `normalizes stale contract-event projection metadata without discarding items`
- API: `normalizes stale final-event projection metadata without discarding active and closed items`
- UI: `retains historical events and marks only history stale after a failed refresh`
- UI: `renders stale intelligence as UNKNOWN with the previous risk shown secondarily`
- UI: `clears stale state after the next successful automatic poll`
- UI: update `promotes historical events into the pro desk primary view` for order, height, and `2xl` layout.

The route tests must assert stable JSON fields on the no-cache response:

```json
{
  "dataState": "degraded",
  "degraded": true,
  "errorCode": "contract_projection_timeout",
  "lastKnownDataAvailable": false,
  "retryAfterMs": 2000
}
```

Run each newly added test by exact name and confirm it fails for the intended missing behavior, not because of a fixture or syntax error.

### Step 4: Review the test-only diff

```powershell
Set-Location 'D:\DevWorkspaces\Documents\有毒订单监控-rs'
git diff --check
git diff -- tests/contract_whale_persistence_tests.rs tests/contract_event_routes_tests.rs toxic-order-monitor/src/tests/ContractWhaleApi.test.js toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx
```

Do not commit.

---

## Task 2: Add the Verified SQLite Event-Feed Fast Path

**Files:**

- Modify: `src/storage/migrations.rs`
- Modify: `src/storage/contract_whale_repo.rs`
- Test: `tests/contract_whale_persistence_tests.rs`

### Step 1: Make the query-plan test prove the old behavior

The disposable store must insert all of the following: two eligible perp rows with the same timestamp and different IDs, a row below the notional floor, a row older than `from_ts`, an ETH row, and a spot row. The equivalence test compares the new repository result to the current nullable-OR SQL result by ordered signal ID.

The query-plan assertion collects the `detail` column from `EXPLAIN QUERY PLAN` and requires:

```rust
assert!(details.iter().any(|line| {
    line.contains("idx_contract_whale_signals_event_feed")
}));
assert!(!details.iter().any(|line| line.contains("SCAN contract_whale_signals")));
assert!(!details.iter().any(|line| line.contains("USE TEMP B-TREE")));
```

Run and confirm red:

```powershell
cargo test -j 1 --test contract_whale_persistence_tests contract_whale_event_feed_query_plan_uses_ordered_symbol_index -- --exact --nocapture
```

### Step 2: Add one migration-backed index

Append a migration after the current symbol/timestamp index migration:

```sql
CREATE INDEX IF NOT EXISTS idx_contract_whale_signals_event_feed
  ON contract_whale_signals(symbol, ts DESC, signal_id DESC);
```

Do not include `market_type` in this index. Do not run the migration against the active large database during local development.

### Step 3: Add the exact production-shape branch

In `query_contract_whale_signals`, branch only when `symbol`, `from_ts`, and a positive finite `min_notional_usd` exist, and severity/type/direction/Discord/window/exchange/net-volume/cursor filters are absent. `to_ts` may be optional. Preserve offset pagination; positioned cursor pagination stays on the general path.

The specialized SQL is:

```sql
SELECT payload_json, discord_eligible, discord_sent, discord_sent_at,
       active_sources_json, threshold_profile
FROM contract_whale_signals
WHERE market_type = 'perp'
  AND symbol = ?1
  AND ts >= ?2
  AND (?3 IS NULL OR ts <= ?3)
  AND total_notional_usd >= ?4
ORDER BY ts DESC, signal_id DESC
LIMIT ?5 OFFSET ?6
```

Use the existing `decode_signal_row`; do not duplicate JSON repair or visibility semantics.

The branch guard must be explicit:

```rust
let fast_path_min_notional = query
    .min_notional_usd
    .filter(|value| value.is_finite() && *value > 0.0);
let event_feed_fast_path = query.symbol.is_some()
    && query.from_ts.is_some()
    && fast_path_min_notional.is_some()
    && query.severity.is_none()
    && query.signal_type.is_none()
    && query.direction.is_none()
    && query.discord_sent.is_none()
    && query.window_sec.is_none()
    && query.exchange.is_none()
    && query.min_abs_net_volume_btc.is_none()
    && query.cursor_ts.is_none()
    && query.cursor_signal_id.is_none();
```

### Step 4: Turn both persistence tests green

```powershell
cargo test -j 1 --test contract_whale_persistence_tests contract_whale_event_feed_fast_path_matches_general_query_ordering -- --exact --nocapture
cargo test -j 1 --test contract_whale_persistence_tests contract_whale_event_feed_query_plan_uses_ordered_symbol_index -- --exact --nocapture
cargo test -j 1 --test contract_whale_persistence_tests
```

Expected: exact row/order equivalence; selected plan uses `idx_contract_whale_signals_event_feed`; no full table scan or temporary B-tree.

---

## Task 3: Build the Bounded Projection Runtime in Isolation

**Files:**

- Create: `src/api/contract_event_projection_runtime.rs`
- Modify: `src/api/mod.rs`

### Step 1: Add runtime unit tests first

Place unit tests in the new module so private coordination state stays private. Add:

- `equivalent_keys_execute_projection_once`
- `projection_concurrency_never_exceeds_two`
- `fresh_cache_skips_projection`
- `timed_out_waiter_receives_stale_cache`
- `timeout_without_cache_returns_unavailable`
- `failed_refresh_preserves_stale_cache`
- `cache_evicts_oldest_success_at_capacity`
- `dropped_waiter_does_not_cancel_refresh`
- `blocking_projection_does_not_block_current_thread_runtime`

Use atomics for starts/current/max concurrency and `std::thread::sleep` only inside the closure passed to `spawn_blocking`. For the current-thread test, start a 200 ms blocking job and require an independent 20 ms Tokio timer to complete within 100 ms.

Run and confirm red before the implementation exists:

```powershell
cargo test -j 1 api::contract_event_projection_runtime::tests -- --nocapture
```

### Step 2: Add exact runtime constants and key/value types

```rust
const MAX_RUNNING: usize = 2;
const MAX_ENTRIES: usize = 64;
const FRESH_TTL: Duration = Duration::from_secs(10);
const STALE_TTL: Duration = Duration::from_secs(300);
const WAIT_BUDGET: Duration = Duration::from_secs(4);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ProjectionView {
    ContractEvents,
    FinalEventsV2,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ProjectionKey {
    pub view: ProjectionView,
    pub canonical_query: String,
}

#[derive(Debug, Clone)]
pub(crate) enum ProjectionValue {
    ContractEvents(ContractEventPage),
    FinalEventsV2(FinalEventsV2Response),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectionFailure {
    pub error_code: &'static str,
    pub safe_message: String,
}
```

Store failures as stable code plus a sanitized message; do not place `anyhow::Error`, SQL text, paths, query data, or secrets in cached/watched values.

### Step 3: Implement bounded cache and single-flight state

`ContractEventProjectionRuntime` owns:

```rust
pub(crate) struct ContractEventProjectionRuntime {
    semaphore: Arc<Semaphore>,
    state: Arc<Mutex<ProjectionRuntimeState>>,
    wait_budget: Duration,
    fresh_ttl: Duration,
    stale_ttl: Duration,
}
```

The state contains two `BTreeMap<ProjectionKey, _>` maps: successful cache entries and in-flight watch receivers. Every map lookup/insert/removal happens under the async mutex, but permit acquisition, waiting, SQLite, projection, cache-response cloning, and eviction scanning happen after releasing it.

`get_or_spawn` must follow this order:

1. Clone and return a fresh cached value when age is at most 10 seconds.
2. Remember, but do not mutate, a stale candidate no older than 300 seconds.
3. Subscribe to an existing in-flight job or insert one new watch channel for the normalized key.
4. Spawn a detached Tokio task. Inside it, acquire one owned semaphore permit, then call `tokio::task::spawn_blocking(compute)`.
5. On success, remove in-flight state, insert the successful value, evict by oldest `completed_at_ms` until at most 64 cache entries remain, and publish the result.
6. On failure, remove in-flight state and publish only the sanitized failure. Preserve the old successful cache.
7. Wait at most four seconds. A dropped request only drops its receiver; the detached task remains alive.
8. On timeout/failure, return the remembered stale candidate when valid; otherwise return `ProjectionUnavailable` with `contract_projection_timeout` or `contract_projection_failed` and `retry_after_ms=2000`.

Cap unique in-flight keys at 64. If the cap is full, serve a valid stale candidate immediately or return `contract_projection_busy` without creating another queued job.

### Step 4: Make response freshness mutation typed and non-destructive

Do not mutate cached values. Add typed clone helpers that update only serving metadata:

```rust
fn mark_contract_events_stale(
    mut page: ContractEventPage,
    now_ms: i64,
    completed_at_ms: i64,
    error_code: &str,
) -> ContractEventPage {
    page.data_state = "stale".to_string();
    page.degraded = true;
    page.error_code = Some(error_code.to_string());
    page.last_known_data_available = !page.items.is_empty();
    page.server_time = now_ms;
    page.cache_age_sec = now_ms.saturating_sub(completed_at_ms).max(0) / 1000;
    page.cache_ttl_sec = 10;
    page.timeline.served_ts = now_ms;
    page
}
```

Add the equivalent helper for `FinalEventsV2Response`, preserving `generated_at`, active/closed rows, and canonical event timestamps.

### Step 5: Turn runtime tests green

```powershell
cargo test -j 1 api::contract_event_projection_runtime::tests -- --nocapture
cargo fmt --check
```

Expected: one compute for equivalent keys, max two active jobs, deterministic fresh/stale/eviction behavior, and no blocked Tokio timer.

---

## Task 4: Wire the Runtime into AppState and Every Heavy Projection Call Site

**Files:**

- Modify: `src/app.rs`
- Modify: `src/api/contract_event_routes.rs`
- Modify: `src/api/contract_timeline_routes.rs`
- Modify: `src/api/contract_whale_routes.rs`
- Test: `tests/contract_event_routes_tests.rs`

### Step 1: Replace the old AppState cache

Delete `CachedFinalEventsV2Entry`, `final_events_v2_cache`, `cached_final_events_v2`, and `store_final_events_v2_cache`. Add exactly one cloned runtime:

```rust
contract_event_projection_runtime: ContractEventProjectionRuntime,
```

Initialize it in `AppState::new` with production defaults and expose:

```rust
pub(crate) fn contract_event_projection_runtime(&self) -> ContractEventProjectionRuntime {
    self.inner.contract_event_projection_runtime.clone()
}
```

Add narrow deterministic `pub` test controls following existing `*_for_tests` conventions: configurable projection delay and a public read-only `ProjectionRuntimeStats` value for started/running/max-running/cache/in-flight. The integration tests under `tests/` must be able to call them, but no API route exposes these controls.

### Step 2: Normalize keys from stable raw query semantics

The key must include endpoint view plus every response-affecting field: normalized symbol, raw `range`, raw `from/to`, limit, offset/cursor, status, severity, signal type, direction, net direction, Discord status, window, exchange, include-hidden, net-volume floor, and notional floor.

Use raw `range=24h` in the key. Do not use the dynamic millisecond value produced by `parse_range_start_ms`, or equivalent requests will miss single-flight. Normalize numeric values before joining; for notional use parsed `f64::to_bits()` so `10000000` and `10000000.0` share one key.

### Step 3: Validate cheap request errors before background scheduling

Extract preparation functions that parse limit/include-hidden/status/history query on the async request path without touching SQLite. Invalid limit/cursor/filter requests must remain `400`, not become a projection `503`.

Move the existing bodies to synchronous blocking builders that accept prepared values:

```rust
fn build_contract_event_page_blocking(
    state: AppState,
    prepared: PreparedContractEventRequest,
) -> Result<ContractEventPage, ProjectionFailure>;

fn build_final_events_v2_blocking(
    state: AppState,
    prepared: PreparedFinalEventsRequest,
) -> Result<FinalEventsV2Response, ProjectionFailure>;
```

All SQLite reads, `max(created_at)`, lifecycle raw-flow reads, OI decoration, clustering, quality decoration, merging, and trajectory work remain inside these blocking closures.

### Step 4: Make the public routes runtime-backed

The route flow is prepare → key → `get_or_spawn` → typed outcome → JSON. Do not acquire a permit in the handler itself. The detached runtime job owns the permit and outlives a canceled waiter.

Map no-cache unavailable outcomes to HTTP `503` with both `Retry-After: 2` and the stable JSON body from Task 1. Stale cached outcomes remain HTTP `200` and include retained rows with `dataState="stale"` and `degraded=true`.

Keep endpoint-specific projection values separate. `ContractEventStream` and `FinalLifecycleEvent` have different display/visibility semantics, so do not reuse one endpoint's final response for the other. The normalized key includes `ProjectionView`.

### Step 5: Remove hidden synchronous call sites

- Make `build_contract_whale_timeline_response` async. Use `spawn_blocking` for its remaining latest-row lookup and await the two runtime-backed projections.
- Update `contract_whale_timeline_route` and `contract_whale_latency_debug_route` to await it.
- Update latency debug to await runtime-backed event/final views.
- Move the persisted SQLite/build branch of `contract_whale_intelligence_terminal_route` into `spawn_blocking`. It must not hold the projection semaphore while awaiting event/final projections, preventing nested permit deadlock.
- Move `contract_events_debug_counts_route` to `spawn_blocking`; it remains operator diagnostics and does not enter the user-facing projection cache.

Summary, latest, and health routes must never call `contract_event_projection_runtime`, wait on its semaphore, or join its in-flight map.

### Step 6: Turn route tests green

The slow-route test starts a deliberately delayed contract-events request, waits until runtime stats show one running projection, then requests summary and latest. Require both light responses to complete within two seconds while the heavy response is still pending.

The single-flight test sends at least eight equivalent requests and requires one started projection. The concurrency test sends distinct keys and requires `max_running <= 2`.

Run:

```powershell
cargo test -j 1 --test contract_event_routes_tests slow_contract_projection_does_not_delay_summary_or_latest -- --exact --nocapture
cargo test -j 1 --test contract_event_routes_tests equivalent_contract_event_requests_execute_projection_once -- --exact --nocapture
cargo test -j 1 --test contract_event_routes_tests contract_events_timeout_returns_structured_503_without_cache -- --exact --nocapture
cargo test -j 1 --test contract_event_routes_tests contract_events_timeout_serves_stale_payload -- --exact --nocapture
cargo test -j 1 --test contract_event_routes_tests final_events_v2_reuses_recent_projection_within_cache_ttl -- --exact
cargo test -j 1 --test contract_event_routes_tests
```

Expected: light routes stay responsive; equivalent work runs once; stale rows survive; no-cache returns before the browser timeout.

---

## Task 5: Normalize Backend Data-State Metadata in the Frontend API Layer

**Files:**

- Modify: `toxic-order-monitor/src/api/contractWhale.js`
- Test: `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`

### Step 1: Add red API normalization tests

Mock stale responses using mixed camel/snake case:

```js
{
  data_state: "stale",
  degraded: true,
  error_code: "contract_projection_busy",
  last_known_data_available: true,
  cache_age_sec: 42,
  cache_ttl_sec: 10,
  retry_after_ms: 2000
}
```

Contract-events must retain `items`; final-events must retain both `active` and `closed`. Intelligence catch/success responses must expose the same state contract.

Run and confirm red:

```powershell
Set-Location 'D:\DevWorkspaces\Documents\有毒订单监控-rs\toxic-order-monitor'
npm test -- src/tests/ContractWhaleApi.test.js
```

### Step 2: Add one shared normalization helper

```js
function normalizeDataState(payload, fallbackErrorCode = null) {
  return {
    dataState: String(payload?.dataState || payload?.data_state || (fallbackErrorCode ? "degraded" : "fresh")),
    degraded: Boolean(payload?.degraded || fallbackErrorCode),
    errorCode: payload?.errorCode ?? payload?.error_code ?? fallbackErrorCode,
    lastKnownDataAvailable: Boolean(
      payload?.lastKnownDataAvailable ?? payload?.last_known_data_available,
    ),
    cacheAgeSec: numberOrNull(payload?.cacheAgeSec ?? payload?.cache_age_sec),
    cacheTtlSec: numberOrNull(payload?.cacheTtlSec ?? payload?.cache_ttl_sec),
    retryAfterMs: numberOrNull(payload?.retryAfterMs ?? payload?.retry_after_ms),
  };
}
```

Spread this metadata into latest, contract-events, final-events-v2, and intelligence results. A successful `stale` payload has `error:null`; transport/no-cache failure has an error code and no usable payload. Do not change `fetchJsonWithTimeout` or endpoint timeout values.

### Step 3: Turn API tests green

```powershell
npm test -- src/tests/ContractWhaleApi.test.js
```

---

## Task 6: Introduce Independent Frontend Data Slices and Honest Stale Analysis

**Files:**

- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

### Step 1: Add red slice-isolation and recovery tests

Use fake timers for success → failure → success sequences. Required assertions:

- A failed 15-second history refresh retains the previous event row.
- Only history is marked stale when lifecycle and intelligence succeed.
- A stale intelligence payload shows current `UNKNOWN`, a visible `STALE` tag, and secondary `上次 HIGH RISK（时间）` / `上次 RANGING（时间）` text.
- The next successful automatic poll clears stale state and restores current values.
- Exactly one consolidated banner exists and contains no button.
- Existing 5-second status and 15-second heavy polling counts remain unchanged.

Run and confirm red:

```powershell
npm test -- src/tests/ContractWhaleMonitor.test.jsx
```

### Step 2: Add four compact data slices

Keep existing payload fields to minimize churn. Add a separate state object:

```js
function createDataSlice() {
  return {
    state: "loading",
    errorCode: null,
    lastSuccessAt: null,
    nextRetryAt: null,
  };
}

const [dataSlices, setDataSlices] = useState(() => ({
  status: createDataSlice(),
  historical: createDataSlice(),
  lifecycle: createDataSlice(),
  intelligence: createDataSlice(),
}));
```

`payloadIsUsable` returns true for `fresh`, `empty`, and `stale` payloads with no transport error. `nextDataSlice` records `lastSuccessAt` on usable data, maps a stale payload or a failed refresh after prior success to `stale`, maps first-load failure to `unavailable`, and computes `nextRetryAt` from the existing 5/15-second cadence or normalized retry metadata.

### Step 3: Update each refresh group only within its slice

- `refreshStatusViews` determines one status slice result after `Promise.allSettled`; summary and latest callbacks no longer race over one shared error.
- `refreshContractEvents` retains old events/cursor/timestamps on failure and updates only `historical`.
- `refreshFinalEvents` retains active/closed payloads on failure and updates only `lifecycle`.
- `refreshIntelligenceTerminal` retains old analysis on failure and updates only `intelligence`.
- A usable stale backend payload replaces payload fields with the explicitly retained server response and marks the slice stale.
- Delete shared `state.error`. Keep a panel-local error only where it controls the compact first-load empty state.

### Step 4: Render one consolidated banner

Add `ConsolidatedDataStateBanner` with one `role="status"` and `data-testid="contract-whale-data-state"`. It summarizes only non-fresh slices, shows `STALE` plus last-success time when retained data exists, and shows the next automatic retry time for first-load unavailability. It contains no button.

Delete the top shared yellow warning and do not duplicate an error warning inside the history explanation block.

### Step 5: Make stale intelligence impossible to read as current

Add `deriveIntelligenceDisplay(intelligence, slice)` and pass both current-display and previous-display metadata to Pro Desk/structure/risk panels. When the intelligence slice is not fresh:

- current regime is `UNKNOWN`;
- current risk is `UNKNOWN` with neutral/amber styling;
- retained regime/risk appears only as secondary `上次 …（时间）` text;
- retained ranked events may remain readable under a visible `STALE` label;
- missing values never fall back to green `LOW RISK` or current `RANGING`.

### Step 6: Turn component state tests green

```powershell
npm test -- src/tests/ContractWhaleMonitor.test.jsx
```

---

## Task 7: Fix First-Screen Order, Empty Height, and Responsive Splits

**Files:**

- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

### Step 1: Tighten the red layout assertions

Update `promotes historical events into the pro desk primary view` to require:

- filters/explanation precede the historical panel;
- historical panel precedes the Pro Desk overview in document order;
- populated history contains `min-h-[50vh]`;
- first-load error, empty, and filtered-empty history do not contain `min-h-[50vh]`;
- the main history/structure grid uses the exact `2xl:grid-cols-[minmax(0,1.55fr)_minmax(320px,0.95fr)]` class and no `xl:grid-cols-` split class.

### Step 2: Reorder the primary content

Render in this order:

1. compact heading/status pills;
2. consolidated data-state banner;
3. navigation, filters, and visibility explanation;
4. historical event stream;
5. Pro Desk overview;
6. structure/liquidity/setup analysis;
7. lifecycle/risk;
8. diagnostics.

The historical panel remains the first item in its grid at all widths.

### Step 3: Make half-viewport height content-sensitive

Inside `HistoricalEventStreamPanel`:

```jsx
const hasVisibleContent = visibleContractEvents.length > 0;
const panelClassName = [
  "mt-4 rounded-2xl border border-cyan-500/20 bg-slate-950/35",
  hasVisibleContent ? "min-h-[50vh]" : "",
].filter(Boolean).join(" ");
```

Use `panelClassName` on the section. Retained stale rows count as visible content and keep the workspace height; loading/error/empty/filtered-empty states stay content-driven.

### Step 4: Move fixed-column splits to `2xl`

Change the three monitoring split grids around history/structure, liquidity/setups, and lifecycle/risk from `xl:` to `2xl:`. Keep header flex/grid behavior unless an actual 1223/1280 overflow assertion proves it needs adjustment.

### Step 5: Turn focused layout tests green

```powershell
npm test -- src/tests/ContractWhaleMonitor.test.jsx
```

---

## Task 8: Add a Local Read-Only Latency Probe

**Files:**

- Create: `scripts/probe-contract-event-latency.ps1`

### Step 1: Implement a local-only default

The script accepts `-BaseUrl`, defaults to `http://127.0.0.1:3000`, rejects non-loopback hosts unless `-AllowRemote` is explicitly supplied, and only sends GET requests. It samples summary, latest, and contract-events concurrently, records status and elapsed milliseconds, and calculates p50/p95/p99/max.

Required invocation:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\probe-contract-event-latency.ps1 -BaseUrl 'http://127.0.0.1:3000' -Symbol ETH -Samples 30 -Concurrency 8
```

Exit non-zero when summary/latest p95 is at least 2000 ms, event p95 is at least 3000 ms, any request exceeds 5000 ms, or a response status is outside the expected `200/503` set. Print only URL path, status, and timing; never headers, response bodies, tokens, or environment values.

### Step 2: Syntax-check without contacting production

```powershell
$errors = $null
[System.Management.Automation.Language.Parser]::ParseFile(
  (Resolve-Path '.\scripts\probe-contract-event-latency.ps1'),
  [ref]$null,
  [ref]$errors
) | Out-Null
if ($errors.Count -gt 0) { $errors | Format-List; exit 1 }
```

---

## Task 9: Run the Full Local Verification Gate

**Files:**

- Verify all modified files.
- Do not modify deployment files or service state.

### Step 1: Rust formatting, static checks, and tests

```powershell
Set-Location 'D:\DevWorkspaces\Documents\有毒订单监控-rs'
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets --all-features -- -D warnings
cargo test -j 1 --test contract_whale_persistence_tests
cargo test -j 1 --test contract_event_routes_tests
cargo test -j 1 --all-targets --all-features --no-fail-fast
```

If a command fails, preserve its exact output, diagnose, fix only the in-scope cause, and rerun the failing command before continuing.

### Step 2: Frontend focused/full tests and production build

```powershell
Set-Location 'D:\DevWorkspaces\Documents\有毒订单监控-rs\toxic-order-monitor'
npm test -- src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx
npm test
npm run build
```

The project has no lint script and no Playwright dependency; do not add either solely for this task.

### Step 3: Start local services and run the latency probe

Use the repository's existing local backend startup path with a disposable/local database configuration. Run the read-only probe against `127.0.0.1` only. Required local gate:

- summary/latest p95 below 2 seconds;
- contract-events p95 below 3 seconds;
- no request above 5 seconds;
- statuses limited to expected `200` and deliberate structured `503` under forced no-cache saturation.

Stop only processes started by this verification session.

### Step 4: Verify 1223, 1280, and 1536-pixel desktop layouts

Start Vite locally:

```powershell
Set-Location 'D:\DevWorkspaces\Documents\有毒订单监控-rs\toxic-order-monitor'
npm run dev -- --host 127.0.0.1 --port 5173
& 'C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe' --new-window 'http://127.0.0.1:5173/contract-whale/eth'
```

Use browser automation for deterministic viewport checks. At each width, assert:

```js
document.documentElement.scrollWidth === document.documentElement.clientWidth
```

Also verify:

- 1223 and 1280 pixels: history and structure are stacked; no clipped secondary column.
- 1536 pixels: two columns fit without document overflow.
- historical event panel precedes Pro Desk by DOM order and bounding-box top.
- empty/error panel is content-height; populated panel retains at least half-viewport minimum height.
- only one consolidated warning exists.
- stale risk/regime is visibly `UNKNOWN`, with prior values secondary.

Capture local screenshots for evidence only; do not include secrets or production data.

### Step 5: Final safety and diff review

```powershell
Set-Location 'D:\DevWorkspaces\Documents\有毒订单监控-rs'
git diff --check
git status --short --branch
git diff --stat
git diff -- src/api src/app.rs src/storage tests toxic-order-monitor/src scripts/probe-contract-event-latency.ps1 docs/superpowers
rg -n "TODO|FIXME|placeholder|manual retry|手动重试" src toxic-order-monitor/src scripts/probe-contract-event-latency.ps1 docs/superpowers
```

Confirm:

- no `.env`, database, runtime data, tokens, secrets, generated build output, or screenshots are staged;
- no live/trading/admin/deploy path changed;
- ETH initial request still uses 20 rows;
- all in-flight and cache maps are bounded and clean themselves up;
- async locks are not held while awaiting permits, jobs, or receivers;
- no user-visible raw backend error is rendered;
- no commit, push, deployment, or restart occurred.

## Completion Evidence to Report

Report only after the full local gate passes:

- exact files changed;
- focused and full Rust/frontend commands with pass/fail results;
- query-plan evidence naming `idx_contract_whale_signals_event_feed` and confirming no full scan/temp sort;
- single-flight count and max heavy concurrency evidence;
- local p95/p99/max probe results;
- 1223/1280/1536 overflow/order/height results and local screenshot paths;
- explicit statement that work remains local, read-only/dry-run, uncommitted, undeployed, and services were not restarted.
