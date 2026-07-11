# Contract Whale Latency Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose and reduce `/contract-whale` latest-to-history-to-final-events delay, then surface clear per-layer sync state in the UI without faking history.

**Architecture:** Add lightweight latency metadata and a dedicated `latency-debug` backend route on top of existing latest/history/final-events pipelines, then update the React screen to poll each layer independently and trigger downstream refreshes when upstream timestamps advance. Keep detector thresholds and persistence semantics unchanged; only add observability, cache metadata, and better client refresh behavior.

**Tech Stack:** Rust + Axum + rusqlite backend, React + Vite + Vitest frontend, existing contract whale repositories/routes/tests.

## Global Constraints

- Do not modify detector thresholds.
- Do not synthesize historical events.
- Do not present latest snapshots as history.
- Do not stuff stale latest rows into `contract-events`.
- Do not add blind aggressive polling or block one feed on another.
- Keep latency/debug queries lightweight and avoid full-table scans.
- Preserve existing operator-token requirement for sensitive diagnostics.

---

### Task 1: Add backend latency metadata contracts

**Files:**
- Modify: `src/api/contract_whale_routes.rs`
- Modify: `src/api/contract_event_routes.rs`
- Modify: `src/api/final_event_routes.rs`
- Modify: `src/api/server.rs`
- Test: `tests/contract_event_routes_tests.rs`

**Interfaces:**
- Consumes: existing `ContractWhaleLatestResponse`, `ContractEventPage`, `FinalEventsV2Response`, route registration in `src/api/server.rs`
- Produces:
  - `GET /api/contract-whale/latest` returns `serverTime`, `maxTs`, `maxAgeSec`, `staleCount`
  - `GET /api/contract-events` returns `serverTime`, `maxEventTs`, `maxPersistedAt`, `historyLagSec`, `latestLagSec`, `cacheAgeSec`, `cacheTtlSec`
  - `GET /api/final-events-v2` returns `serverTime`, `maxEventTs`, `generatedAt`, `cacheAgeSec`, `cacheTtlSec`, `projectionLagSec`
  - `GET /api/contract-whale/latency-debug?symbol=BTC&range=1h`

- [ ] **Step 1: Write the failing backend tests**

```rust
#[tokio::test]
async fn contract_events_exposes_latency_metadata() {
    let state = seeded_contract_event_state();
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let client = test_http_client();
    let response = client
        .get(format!("http://{addr}/api/contract-events?symbol=BTC&range=24h&limit=20"))
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("json");
    assert!(payload["serverTime"].as_i64().is_some());
    assert!(payload["maxEventTs"].as_i64().is_some());
    assert!(payload["cacheAgeSec"].as_i64().is_some());
    assert!(payload["cacheTtlSec"].as_i64().is_some());
    assert!(payload["historyLagSec"].as_i64().is_some());
    assert!(payload["latestLagSec"].as_i64().is_some());

    server.abort();
}

#[tokio::test]
async fn latest_exposes_staleness_summary_metadata() {
    let state = seeded_pipeline_debug_state();
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let client = test_http_client();
    let response = client
        .get(format!("http://{addr}/api/contract-whale/latest?symbol=BTC&range=24h"))
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("json");
    assert!(payload["serverTime"].as_i64().is_some());
    assert!(payload["maxTs"].as_i64().is_some());
    assert!(payload["maxAgeSec"].as_i64().is_some());
    assert!(payload["staleCount"].as_u64().is_some());

    server.abort();
}

#[tokio::test]
async fn final_events_v2_exposes_projection_latency_metadata() {
    let state = seeded_contract_event_state();
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let client = test_http_client();
    let response = client
        .get(format!("http://{addr}/api/final-events-v2?symbol=BTC&range=24h&limit=20"))
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("json");
    assert!(payload["serverTime"].as_i64().is_some());
    assert!(payload["maxEventTs"].as_i64().is_some());
    assert!(payload["generatedAt"].as_i64().is_some() || payload["generatedAt"].is_string());
    assert!(payload["cacheAgeSec"].as_i64().is_some());
    assert!(payload["cacheTtlSec"].as_i64().is_some());
    assert!(payload["projectionLagSec"].as_i64().is_some());

    server.abort();
}

#[tokio::test]
async fn latency_debug_reports_layer_diagnosis_and_lag_fields() {
    let state = seeded_contract_event_state();
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let client = test_http_client();
    let response = client
        .get(format!("http://{addr}/api/contract-whale/latency-debug?symbol=BTC&range=1h"))
        .header("Authorization", "Bearer test-operator-token")
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("json");
    assert_eq!(payload["symbol"], "BTC");
    assert_eq!(payload["range"], "1h");
    assert!(payload["latest"]["ageSec"].as_i64().is_some());
    assert!(payload["contractEvents"]["lagVsLatestSec"].as_i64().is_some());
    assert!(payload["finalEventsV2"]["projectionLagSec"].as_i64().is_some());
    assert!(payload["diagnosis"]["layer"].as_str().is_some());

    server.abort();
}
```

- [ ] **Step 2: Run backend test file to verify RED**

Run: `cargo test --test contract_event_routes_tests -- --nocapture`
Expected: FAIL because the new latency fields/route do not exist yet.

- [ ] **Step 3: Implement minimal backend metadata + route**

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractEventPage {
    pub items: Vec<ContractEventItem>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub limit: usize,
    pub range: String,
    pub server_time: i64,
    pub last_event_ts: Option<i64>,
    pub max_event_ts: Option<i64>,
    pub max_persisted_at: Option<i64>,
    pub history_lag_sec: i64,
    pub latest_lag_sec: i64,
    pub cache_age_sec: i64,
    pub cache_ttl_sec: i64,
}
```

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ContractWhaleLatencyDebugResponse {
    symbol: String,
    range: String,
    server_time: i64,
    latest: LatencyLatestLayer,
    contract_events: LatencyHistoryLayer,
    final_events_v2: LatencyProjectionLayer,
    flow: LatencyFlowLayer,
    diagnosis: LatencyDiagnosis,
}
```

```rust
.route(
    "/api/contract-whale/latency-debug",
    get(contract_whale_routes::contract_whale_latency_debug_route),
)
```

- [ ] **Step 4: Run backend test file to verify GREEN**

Run: `cargo test --test contract_event_routes_tests -- --nocapture`
Expected: PASS for the new latency metadata and debug-route assertions.

- [ ] **Step 5: Commit backend metadata contract**

```bash
git add src/api/contract_whale_routes.rs src/api/contract_event_routes.rs src/api/final_event_routes.rs src/api/server.rs tests/contract_event_routes_tests.rs
git commit -m "feat: add contract whale latency metadata endpoints"
```

### Task 2: Instrument persistence and derive diagnosis safely

**Files:**
- Modify: `src/contract_whale_monitor/persistence.rs`
- Modify: `src/storage/contract_whale_repo.rs`
- Modify: `src/app.rs`
- Test: `tests/contract_event_routes_tests.rs`

**Interfaces:**
- Consumes: contract whale nonblocking persistence functions, SQLite repo reads, app state access
- Produces:
  - lightweight timestamps for `persisted_at` / `created_at` based lag calculations
  - persistence log lines for queued/success/error timing
  - latency diagnosis fields that distinguish `signal_persistence`, `contract_events_query`, `final_events_projection`, `frontend_polling`, `no_recent_signal`

- [ ] **Step 1: Write the failing persistence/diagnosis test**

```rust
#[tokio::test]
async fn latency_debug_reports_no_recent_signal_when_latest_and_history_are_both_empty() {
    let config = test_config(temp_sqlite_path("contract-whale-latency-empty"));
    let state = AppState::new(config);
    let app = router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("server");
    });

    let client = test_http_client();
    let response = client
        .get(format!("http://{addr}/api/contract-whale/latency-debug?symbol=BTC&range=1h"))
        .header("Authorization", "Bearer test-operator-token")
        .send()
        .await
        .expect("response");

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = response.json().await.expect("json");
    assert_eq!(payload["diagnosis"]["layer"], "ok");
    assert_eq!(payload["diagnosis"]["reason"], "no_recent_signal");

    server.abort();
}
```

- [ ] **Step 2: Run targeted backend test to verify RED**

Run: `cargo test --test contract_event_routes_tests latency_debug_reports_no_recent_signal_when_latest_and_history_are_both_empty -- --nocapture`
Expected: FAIL because diagnosis currently lacks this branch/shape.

- [ ] **Step 3: Implement persistence timing/logging and diagnosis helper**

```rust
pub async fn persist_contract_whale_signal_nonblocking(
    store: Arc<SqliteStore>,
    signal: ContractWhaleSignal,
) {
    let queued_at = now_ms();
    tracing::info!(
        target: CWM_LOG_TARGET,
        signal_id = %signal.id,
        symbol = %signal.symbol,
        signal_ts = signal.ts,
        queued_at,
        "contract_signal_persist_queued"
    );

    match tokio::task::spawn_blocking(move || store.upsert_contract_whale_signal(&signal)).await {
        Ok(Ok(())) => tracing::info!(
            target: CWM_LOG_TARGET,
            signal_id = %signal.id,
            symbol = %signal.symbol,
            delay_ms = now_ms().saturating_sub(queued_at),
            "contract_signal_persist_success"
        ),
        Ok(Err(error)) => tracing::warn!(target: CWM_LOG_TARGET, signal_id = %signal.id, error = %error, "contract_signal_persist_error"),
        Err(error) => tracing::warn!(target: CWM_LOG_TARGET, signal_id = %signal.id, error = %error, "contract_signal_persist_join_error"),
    }
}
```

```rust
fn diagnose_latency(
    latest_max_ts: Option<i64>,
    history_max_ts: Option<i64>,
    projection_max_ts: Option<i64>,
    flow_max_ts: Option<i64>,
    history_lag_sec: i64,
    projection_lag_sec: i64,
) -> LatencyDiagnosis {
    if latest_max_ts.is_none() && history_max_ts.is_none() {
        return LatencyDiagnosis::new("ok", "no_recent_signal");
    }
    if history_lag_sec > 15 {
        return LatencyDiagnosis::new("signal_persistence", "history_lag_exceeds_budget");
    }
    if projection_lag_sec > 30 {
        return LatencyDiagnosis::new("final_events_projection", "projection_lag_exceeds_budget");
    }
    if flow_max_ts.is_some() && latest_max_ts.is_none() {
        return LatencyDiagnosis::new("frontend_polling", "latest_snapshot_missing_while_flow_present");
    }
    LatencyDiagnosis::new("ok", "within_budget")
}
```

- [ ] **Step 4: Run targeted backend test to verify GREEN**

Run: `cargo test --test contract_event_routes_tests latency_debug_reports_no_recent_signal_when_latest_and_history_are_both_empty -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit persistence/diagnosis layer**

```bash
git add src/contract_whale_monitor/persistence.rs src/storage/contract_whale_repo.rs src/app.rs tests/contract_event_routes_tests.rs
git commit -m "feat: instrument contract whale latency diagnosis"
```

### Task 3: Add frontend latency API adapters and tests

**Files:**
- Modify: `toxic-order-monitor/src/api/contractWhale.js`
- Modify: `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`

**Interfaces:**
- Consumes: backend responses from latest / contract-events / final-events-v2 / latency-debug
- Produces:
  - normalized `fetchContractWhaleLatest()` metadata (`serverTime`, `maxTs`, `maxAgeSec`, `staleCount`)
  - normalized `fetchContractEvents()` metadata (`maxEventTs`, `maxPersistedAt`, `historyLagSec`, `latestLagSec`, `cacheAgeSec`, `cacheTtlSec`)
  - normalized `fetchFinalEventsV2()` metadata (`generatedAt`, `projectionLagSec`, `cacheAgeSec`, `cacheTtlSec`)
  - new `fetchContractWhaleLatencyDebug()`

- [ ] **Step 1: Write the failing frontend API tests**

```javascript
it("normalizes latency metadata for latest, history, projection, and latency debug", async () => {
  axios.get
    .mockResolvedValueOnce({
      data: {
        items: [],
        serverTime: 1700000200000,
        maxTs: 1700000195000,
        maxAgeSec: 5,
        staleCount: 0,
      },
    })
    .mockResolvedValueOnce({
      data: {
        items: [],
        serverTime: 1700000200000,
        maxEventTs: 1700000194000,
        maxPersistedAt: 1700000194500,
        historyLagSec: 1,
        latestLagSec: 0,
        cacheAgeSec: 2,
        cacheTtlSec: 5,
      },
    })
    .mockResolvedValueOnce({
      data: {
        active: [],
        closed: [],
        serverTime: 1700000200000,
        maxEventTs: 1700000193000,
        generatedAt: 1700000198000,
        cacheAgeSec: 4,
        cacheTtlSec: 10,
        projectionLagSec: 6,
      },
    })
    .mockResolvedValueOnce({
      data: {
        symbol: "BTC",
        range: "1h",
        diagnosis: { layer: "ok", reason: "within_budget" },
      },
    });

  const latest = await fetchContractWhaleLatest(20, "BTC");
  const history = await fetchContractEvents({ symbol: "BTC", range: "24h", limit: 20 });
  const finalEvents = await fetchFinalEventsV2({ symbol: "BTC", range: "24h", limit: 20 });
  const latency = await fetchContractWhaleLatencyDebug({ symbol: "BTC", range: "1h" });

  expect(latest.meta.serverTime).toBe(1700000200000);
  expect(latest.meta.maxTs).toBe(1700000195000);
  expect(latest.meta.maxAgeSec).toBe(5);
  expect(latest.meta.staleCount).toBe(0);
  expect(history.maxEventTs).toBe(1700000194000);
  expect(history.historyLagSec).toBe(1);
  expect(finalEvents.generatedAt).toBe(1700000198000);
  expect(finalEvents.projectionLagSec).toBe(6);
  expect(latency.diagnosis.layer).toBe("ok");
});
```

- [ ] **Step 2: Run frontend API test file to verify RED**

Run: `npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleApi.test.js`
Expected: FAIL because the new normalized fields/function are missing.

- [ ] **Step 3: Implement API normalization and latency-debug fetcher**

```javascript
export async function fetchContractWhaleLatencyDebug(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({
      symbol: filters.symbol || "BTC",
      range: filters.range ?? "1h",
    });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-whale/latency-debug?${query}`, {
      timeoutMs: 4_000,
      axiosConfig: {
        headers: filters.operatorToken
          ? { Authorization: `Bearer ${filters.operatorToken}` }
          : undefined,
      },
    });
    return {
      symbol: String(response.data?.symbol || filters.symbol || "BTC"),
      range: String(response.data?.range || filters.range || "1h"),
      serverTime: numberOrNull(response.data?.serverTime ?? response.data?.server_time),
      latest: response.data?.latest || null,
      contractEvents: response.data?.contractEvents ?? response.data?.contract_events ?? null,
      finalEventsV2: response.data?.finalEventsV2 ?? response.data?.final_events_v2 ?? null,
      flow: response.data?.flow || null,
      diagnosis: response.data?.diagnosis || null,
      error: response.data?.error || null,
    };
  } catch {
    return {
      symbol: String(filters.symbol || "BTC"),
      range: String(filters.range || "1h"),
      serverTime: null,
      latest: null,
      contractEvents: null,
      finalEventsV2: null,
      flow: null,
      diagnosis: null,
      error: "latency_debug_unavailable",
    };
  }
}
```

- [ ] **Step 4: Run frontend API test file to verify GREEN**

Run: `npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleApi.test.js`
Expected: PASS

- [ ] **Step 5: Commit frontend API normalization**

```bash
git add toxic-order-monitor/src/api/contractWhale.js toxic-order-monitor/src/tests/ContractWhaleApi.test.js
git commit -m "feat: normalize contract whale latency metadata"
```

### Task 4: Implement layered polling and sync messaging in the UI

**Files:**
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: normalized latest/history/final-events metadata from API helpers
- Produces:
  - latest polling every 3s, history every 5s, final-events every 10s
  - immediate history refresh when latest `maxTs` advances
  - immediate projection refresh when history `maxEventTs` advances
  - layered delay text instead of a single ambiguous warning

- [ ] **Step 1: Write the failing UI tests**

```javascript
it("shows layered sync status when latest, history, and lifecycle timestamps diverge", async () => {
  fetchContractWhaleLatest.mockResolvedValueOnce({
    summary: seededSummary(),
    items: [],
    meta: { serverTime: 1700000200000, maxTs: 1700000195000, maxAgeSec: 5, staleCount: 0 },
    error: null,
  });
  fetchContractEvents.mockResolvedValueOnce({
    items: [],
    nextCursor: null,
    hasMore: false,
    limit: 50,
    range: "24h",
    serverTime: 1700000200000,
    lastEventTs: 1700000180000,
    maxEventTs: 1700000180000,
    maxPersistedAt: 1700000181000,
    historyLagSec: 15,
    latestLagSec: 15,
    cacheAgeSec: 3,
    cacheTtlSec: 5,
    error: null,
  });
  fetchFinalEventsV2.mockResolvedValueOnce({
    active: [],
    closed: [],
    nextCursor: null,
    hasMore: false,
    limit: 30,
    range: "24h",
    serverTime: 1700000200000,
    lastEventTs: 1700000170000,
    maxEventTs: 1700000170000,
    generatedAt: 1700000190000,
    cacheAgeSec: 4,
    cacheTtlSec: 10,
    projectionLagSec: 10,
    error: null,
  });

  render(<ContractWhaleMonitor />);

  expect(await screen.findByText(/实时快照：/)).toBeInTheDocument();
  expect(screen.getByText(/历史事件流同步中：/)).toBeInTheDocument();
  expect(screen.getByText(/生命周期视图同步中：/)).toBeInTheDocument();
});
```

```javascript
it("triggers history refresh when latest maxTs advances and lifecycle refresh when history maxEventTs advances", async () => {
  vi.useFakeTimers();
  fetchContractWhaleLatest
    .mockResolvedValueOnce({ summary: seededSummary(), items: [], meta: { maxTs: 1700000100000, serverTime: 1700000102000, maxAgeSec: 2, staleCount: 0 }, error: null })
    .mockResolvedValueOnce({ summary: seededSummary(), items: [], meta: { maxTs: 1700000200000, serverTime: 1700000202000, maxAgeSec: 2, staleCount: 0 }, error: null });
  fetchContractEvents
    .mockResolvedValueOnce({ items: [], maxEventTs: 1700000100000, lastEventTs: 1700000100000, serverTime: 1700000103000, historyLagSec: 0, latestLagSec: 0, cacheAgeSec: 1, cacheTtlSec: 5, error: null })
    .mockResolvedValueOnce({ items: [], maxEventTs: 1700000200000, lastEventTs: 1700000200000, serverTime: 1700000203000, historyLagSec: 0, latestLagSec: 0, cacheAgeSec: 1, cacheTtlSec: 5, error: null });
  fetchFinalEventsV2
    .mockResolvedValueOnce({ active: [], closed: [], maxEventTs: 1700000100000, lastEventTs: 1700000100000, generatedAt: 1700000105000, projectionLagSec: 0, cacheAgeSec: 1, cacheTtlSec: 10, error: null })
    .mockResolvedValueOnce({ active: [], closed: [], maxEventTs: 1700000200000, lastEventTs: 1700000200000, generatedAt: 1700000205000, projectionLagSec: 0, cacheAgeSec: 1, cacheTtlSec: 10, error: null });

  render(<ContractWhaleMonitor />);
  await screen.findByText(/BTC \/ ETH 合约监控/);

  await vi.advanceTimersByTimeAsync(3000);

  await waitFor(() => expect(fetchContractEvents).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(fetchFinalEventsV2).toHaveBeenCalledTimes(2));

  vi.useRealTimers();
});
```

- [ ] **Step 2: Run frontend monitor test file to verify RED**

Run: `npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleMonitor.test.jsx`
Expected: FAIL because the UI still shows the old single delay message and does not trigger downstream refreshes.

- [ ] **Step 3: Implement layered polling + sync status**

```javascript
const LATEST_REFRESH_MS = 3_000;
const HISTORY_REFRESH_MS = 5_000;
const FINAL_EVENTS_REFRESH_MS = 10_000;
const REFRESH_DEBOUNCE_MS = 2_000;
```

```javascript
const latestAdvanced = latestMaxTs > previousLatestMaxTs;
if (latestAdvanced) {
  void refreshContractEvents(50, { reason: "latest_advanced" });
}

const historyAdvanced = historyMaxTs > previousHistoryMaxTs;
if (historyAdvanced) {
  void refreshFinalEvents(30, { reason: "history_advanced" });
}
```

```jsx
<div className="text-xs text-cyan-100/80">
  <div>实时快照：{formatDateTime(latestMeta.maxTs)}</div>
  <div>
    历史事件流：{formatDateTime(state.contractEventsMaxEventTs)}，延迟 {formatLagSec(state.contractEventsHistoryLagSec)}
  </div>
  <div>
    生命周期视图：{formatDateTime(state.finalEventsMaxEventTs)}，延迟 {formatLagSec(state.finalEventsProjectionLagSec)}
  </div>
  {historyBehindLatest ? (
    <div className="text-yellow-300">
      历史事件流同步中：落后 latest {formatLagSec(historyBehindLatestSec)}，已自动触发刷新。
    </div>
  ) : null}
  {projectionBehindHistory ? (
    <div className="text-yellow-300">
      生命周期视图同步中：落后历史事件流 {formatLagSec(projectionBehindHistorySec)}，不代表数据丢失。
    </div>
  ) : null}
</div>
```

- [ ] **Step 4: Run frontend monitor test file to verify GREEN**

Run: `npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleMonitor.test.jsx`
Expected: PASS

- [ ] **Step 5: Commit UI sync behavior**

```bash
git add toxic-order-monitor/src/components/ContractWhaleMonitor.jsx toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx
git commit -m "feat: show layered contract whale sync latency"
```

### Task 5: Full verification and deployment sync

**Files:**
- Modify: `scripts/check_contract_event_counts.sh` (only if needed for latency-debug invocation)
- Verify: `src/...`, `toxic-order-monitor/...`

**Interfaces:**
- Consumes: completed backend/frontend changes
- Produces: passing local checks, pushed branch, synced server, captured online latency metrics

- [ ] **Step 1: Run backend verification**

Run:
```bash
cargo test --test contract_event_routes_tests
cargo check
cargo fmt --all --check
```

Expected: PASS

- [ ] **Step 2: Run frontend verification**

Run:
```bash
npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx
npm --prefix toxic-order-monitor run build
```

Expected: PASS

- [ ] **Step 3: Commit final integrated change**

```bash
git add src/api/contract_whale_routes.rs src/api/contract_event_routes.rs src/api/final_event_routes.rs src/api/server.rs src/contract_whale_monitor/persistence.rs src/storage/contract_whale_repo.rs src/app.rs toxic-order-monitor/src/api/contractWhale.js toxic-order-monitor/src/components/ContractWhaleMonitor.jsx toxic-order-monitor/src/tests/ContractWhaleApi.test.js toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx tests/contract_event_routes_tests.rs scripts/check_contract_event_counts.sh docs/superpowers/plans/2026-06-28-contract-whale-latency-sync.md
git commit -m "fix: improve contract whale latest and history sync visibility"
git push origin main
```

- [ ] **Step 4: Sync server and rebuild**

Run:
```bash
ssh -i C:\Users\byhdo\.ssh\codex_contabo_tokyo_root -o BatchMode=yes -o StrictHostKeyChecking=accept-new -b 192.168.1.229 root@5.104.80.120 "cd /opt/toxic-order-monitor-rs && git pull --ff-only && npm --prefix toxic-order-monitor ci && npm --prefix toxic-order-monitor run build && docker compose up -d --build backend frontend && docker compose ps"
```

Expected: backend and frontend containers healthy on the new commit.

- [ ] **Step 5: Capture online latency metrics**

Run:
```bash
ssh -i C:\Users\byhdo\.ssh\codex_contabo_tokyo_root -o BatchMode=yes -o StrictHostKeyChecking=accept-new -b 192.168.1.229 root@5.104.80.120 "curl -sS http://127.0.0.1:5173/api/contract-whale/latest?symbol=BTC | python3 -m json.tool | head -120 && echo --- && curl -sS http://127.0.0.1:5173/api/contract-events?symbol=BTC&range=24h&limit=5 | python3 -m json.tool | head -160 && echo --- && curl -sS http://127.0.0.1:5173/api/final-events-v2?symbol=BTC&range=4h&limit=5 | python3 -m json.tool | head -160"
```

Expected: enough fields to report `latest maxTs/maxAgeSec`, `history maxEventTs/historyLagSec`, `final-events maxEventTs/projectionLagSec`.
