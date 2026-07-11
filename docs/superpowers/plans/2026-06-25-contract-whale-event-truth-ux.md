# Contract Whale Event Truth UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `/contract-whale` default to a traceable historical event stream, keep `latest` as a realtime snapshot only, add paginated ACTIVE/CLOSED projections, and surface retention diagnostics so “disappeared” no longer looks like silent deletion.

**Architecture:** Reuse `contract_whale_signals` as the single historical source, project it into two read models: `contract-events` for the main feed and `final-events-v2` for lifecycle splits. Add a lightweight retention status read model plus richer prune logging without changing the underlying delete rules.

**Tech Stack:** Rust + Axum + rusqlite backend, React + axios frontend, Vitest + Testing Library, Rust unit tests.

## Global Constraints

- Keep existing retention rules unchanged: flow 14d, signals 365d, `severity == S` protected, `ABS(net_volume_btc) >= 500` protected.
- Keep existing APIs compatible: `/api/contract-whale/latest`, `/api/contract-whale/history`, `/api/final-events`.
- No frontend fake history data.
- Use parameter binding for SQL; no string-concatenated filters.

---

### Task 1: Add backend read models and diagnostics

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/storage/contract_whale_repo.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/api/contract_whale_routes.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/api/final_event_routes.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/api/server.rs`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/contract_whale_monitor/persistence.rs`
- Test: `D:/DevWorkspaces/Documents/有毒订单监控-rs/tests/contract_whale_routes_tests.rs`
- Test: `D:/DevWorkspaces/Documents/有毒订单监控-rs/tests/contract_whale_persistence_tests.rs`

**Interfaces:**
- Consumes: `ContractWhaleSignalQuery`, `build_contract_whale_history_response(...)`, `build_final_events_from_contract_whale_signals(...)`
- Produces:
  - `GET /api/contract-events`
  - `GET /api/final-events-v2`
  - `GET /api/contract-retention-status`

### Task 2: Switch frontend default feed to event stream

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/api/contractWhale.js`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Test: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/ContractWhaleApi.test.js`
- Test: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes:
  - `/api/contract-events`
  - `/api/final-events-v2`
  - `/api/contract-retention-status`
- Produces:
  - Main contract list backed by historical event stream
  - ACTIVE/CLOSED sections with load-more
  - Retention diagnostic panel and explanatory copy

### Task 3: Verify regressions and delivery surface

**Files:**
- No code-only task; validate touched files above

**Interfaces:**
- Consumes: existing `cargo test`, frontend Vitest suite, frontend build
- Produces: verified working behavior and exact changed-file summary
