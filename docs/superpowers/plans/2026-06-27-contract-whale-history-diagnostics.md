# Contract Whale History Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Explain why `/contract-whale` shows only one 24h BTC historical event by exposing raw DB counts, API-stage filter counts, hidden-event reasons, latest-vs-history differences, and final-events-v2 projection counts without relaxing existing filters.

**Architecture:** Keep `contract_whale_signals` as the canonical raw source, add a read-only diagnostics layer in `contract_event_routes`, and let the frontend render the diagnostic summary plus optional hidden rows. `latest` remains a realtime snapshot, `contract-events` remains the historical stream, and `final-events-v2` remains a lifecycle projection.

**Tech Stack:** Rust + Axum + rusqlite backend, React + axios frontend, Vitest + Testing Library, Rust unit tests.

## Global Constraints

- Do not widen or disable existing price-deviation / quality / lifecycle filters.
- Do not inject `latest` rows into history.
- Keep debug endpoints lightweight and range-bounded; default to 24h and cap at 500 rows.
- Return diagnostic failures as structured JSON, not 500/502 where avoidable.
- Preserve existing API compatibility for normal consumers.

---

### Task 1: Add backend diagnostics and hidden-event metadata

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/api/contract_event_routes.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/api/contract_whale_routes.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/api/server.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/storage/contract_whale_repo.rs`
- Test: `D:/DevWorkspaces/Documents/有毒订单监控-rs/tests/contract_event_routes_tests.rs` or `D:/DevWorkspaces/Documents/有毒订单监控-rs/tests/contract_whale_routes_tests.rs`

**Interfaces:**
- Add `GET /api/contract-events/debug-counts`
- Extend `GET /api/contract-events` with `include_hidden=true|false`
- Add per-item `isVisible`, `hiddenReason`, `hiddenDetail`

**Implementation notes:**
- Compute DB raw counts separately from post-processing counts.
- Report filter-stage counts and reasons rather than silently dropping.
- Include `latest_vs_history` and `final_events_projection` evidence in debug output.

### Task 2: Expose frontend diagnostics and hidden-event expansion

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/api/contractWhale.js`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Test: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/ContractWhaleApi.test.js`
- Test: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consume `/api/contract-events/debug-counts`
- Render “后端返回 / 可见 / 隐藏 / latest 快照” summary
- Render expandable hidden-event list with reasons
- Explain latest/history source mismatch when counts differ

### Task 3: Add operator verification script and run validation

**Files:**
- Add: `D:/DevWorkspaces/Documents/有毒订单监控-rs/scripts/check_contract_event_counts.sh`

**Interfaces:**
- Read:
  - `/api/contract-events`
  - `/api/contract-events?include_hidden=true`
  - `/api/contract-events/debug-counts`
  - `/api/final-events-v2`
  - `/api/contract-whale/latest`
- Produce operator-readable evidence for server verification

### Task 4: Validate and summarize root cause

**Files:**
- No new product files beyond above tasks

**Interfaces:**
- Consumes `cargo test`, frontend Vitest, frontend build, and live debug output
- Produces exact numeric explanation:
  - DB 24h raw count
  - API matched count
  - visible count
  - hidden count with reason breakdown
  - latest count
  - final active/closed counts
