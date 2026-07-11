# Contract Whale Pipeline Debug + Stale Latest Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Diagnose why BTC has no persisted `contract_whale_signals` in the last 24 hours while `/api/contract-whale/latest` still shows old BTC snapshots, and make that state explicit in both backend diagnostics and frontend messaging.

**Architecture:** Treat the contract whale pipeline as one staged flow: raw `contract_flow_1s` buckets -> rolling windows -> detector -> price deviation / lifecycle / event quality -> persistence -> history/latest projections. Add a read-only pipeline diagnostics route plus stale metadata for `latest`, without changing thresholds or inventing history rows.

**Tech Stack:** Rust + Axum + rusqlite backend, React frontend, Vitest + Testing Library, Rust unit tests.

## Global Constraints

- Do not change detector thresholds or relax current signal-generation rules.
- Do not synthesize BTC history rows when persistence produced none.
- Keep `latest` backward compatible: stale metadata is additive, and `hide_stale` defaults to `false`.
- Keep diagnostics read-only and range-bounded.
- Prefer counters and explicit reject reasons over inferred prose.

---

### Task 1: Add backend pipeline-debug response and counters

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/api/contract_whale_routes.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/storage/contract_whale_repo.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/api/server.rs`

**Interfaces:**
- Add `GET /api/contract-whale/pipeline-debug?symbol=BTC&range=24h`
- Return staged counters for:
  - `raw_flow`
  - `rolling_windows`
  - `detector`
  - `persistence`
  - `history`
  - `latest`

**Implementation notes:**
- Reuse `list_contract_flow_buckets_between` for raw 1s flow evidence.
- Recompute 5s/15s/60s windows from persisted buckets for diagnostics only.
- Add detector explain/reject metadata so we can count why windows did not become signals.
- History count must come from real `contract_whale_signals` rows in range.

### Task 2: Add stale latest metadata and logging

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/types.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/api/contract_whale_routes.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/detector.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/persistence.rs`

**Interfaces:**
- Extend latest item payload with:
  - `ageSec`
  - `isStale`
  - `staleReason`
- Support `hide_stale=true` on `/api/contract-whale/latest`

**Implementation notes:**
- Mark latest items stale when they fall outside the requested/latest default history horizon.
- Add structured detector summary logs per symbol/window with reject-reason counts.
- Add structured persistence logs for attempt/success/skip/error plus skip reasons when store is unavailable.
- Add latest stale summary logs (`stale_count`, `max_age_sec`) at response build time.

### Task 3: Add frontend stale-warning handling

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/api/contractWhale.js`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`

**Interfaces:**
- Consume latest stale fields
- Show:
  - `BTC latest 为旧快照，最近 24h 没有新的 BTC 主力历史信号。`
  when latest exists but all items are stale and history/debug indicates no recent persisted BTC history

### Task 4: TDD, validation, and self-check

**Files:**
- Modify/add tests under:
  - `D:/DevWorkspaces/Documents/有毒订单监控-rs/tests/contract_whale_routes_tests.rs`
  - `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/ContractWhaleApi.test.js`
  - `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Backend tests must verify:
  - pipeline-debug JSON shape
  - stale latest metadata
  - `hide_stale=true` behavior
- Frontend tests must verify:
  - normalized stale latest fields
  - stale-warning rendering

**Validation commands:**
- `cargo test --test contract_whale_routes_tests`
- `cargo test --test contract_whale_monitor_tests` if touched
- `npm test -- --run toxic-order-monitor/src/tests/ContractWhaleApi.test.js toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`
- `npm run build`
