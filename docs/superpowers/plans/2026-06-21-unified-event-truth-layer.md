# Unified Event Truth Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a v1 FinalEvent projection so contract market events have one canonical read model for API, frontend, and Discord payload mapping.

**Architecture:** Keep existing CWM raw signal detection unchanged. Add a focused `core_event` read model that converts already-merged, lifecycle-scored CWM signals into immutable-style `FinalEvent` objects. Expose `/api/final-events` and `/api/final-events/:id`, then make the frontend event feed render those final events instead of doing any event interpretation.

**Tech Stack:** Rust/axum/serde for backend, existing CWM repo query path, React/Vitest for frontend.

## Global Constraints

- Read-only projection only; no trading, execution, or destructive action.
- No frontend merge or recompute for event rows.
- Existing raw CWM signal APIs remain for diagnostics and detail modal compatibility.
- Use TDD: write failing tests before production code.

---

### Task 1: Backend FinalEvent Projection

**Files:**
- Create: `src/core_event/mod.rs`
- Create: `src/core_event/final_store/mod.rs`
- Create: `src/core_event/final_store/final_event_store.rs`
- Modify: `src/lib.rs`
- Test: `tests/contract_whale_routes_tests.rs`

**Interfaces:**
- Consumes: `ContractWhaleSignal` after `build_contract_whale_history_response` has applied merge, lifecycle, quality, cluster, and trajectory.
- Produces: `FinalEvent`, `FinalEventStoreResponse`, `build_final_events_from_contract_whale_signals(&[ContractWhaleSignal])`.

- [ ] Write failing tests asserting one merged CWM event becomes one `FinalEvent` with canonical id, quality, volume, notional, and source signal id.
- [ ] Run targeted cargo test and confirm failure.
- [ ] Implement final event structs and conversion.
- [ ] Run targeted cargo test and confirm pass.

### Task 2: Backend FinalEvents API

**Files:**
- Create: `src/api/final_event_routes.rs`
- Modify: `src/api/mod.rs`
- Modify: `src/api/server.rs`
- Test: `tests/contract_whale_routes_tests.rs`

**Interfaces:**
- Consumes: existing `ContractWhaleSignalQuery` and `build_contract_whale_history_response` path.
- Produces: `GET /api/final-events` and `GET /api/final-events/:id`.

- [ ] Write failing route-level tests using the projection builder and JSON shape.
- [ ] Run targeted cargo test and confirm failure.
- [ ] Implement routes by querying existing CWM store, running existing event engine once, then projecting final events.
- [ ] Run targeted cargo test and confirm pass.

### Task 3: Frontend FinalEvents Read Path

**Files:**
- Modify: `toxic-order-monitor/src/api/contractWhale.js`
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: `/api/final-events?symbol=BTC&limit=12`.
- Produces: event feed rows rendered from final events only.

- [ ] Write failing frontend test that mocks final events and expects the CWM event feed to render canonical volume/quality/status.
- [ ] Run targeted Vitest and confirm failure.
- [ ] Implement `fetchFinalEvents` and update `ContractWhaleMonitor` event feed to use final events.
- [ ] Run targeted Vitest and confirm pass.

### Task 4: Verification

**Files:** none

- [ ] Run `cargo fmt`.
- [ ] Run targeted cargo tests.
- [ ] Run targeted frontend tests.
- [ ] Run `npm run build` if frontend changed.
- [ ] Run `git diff --check`.
