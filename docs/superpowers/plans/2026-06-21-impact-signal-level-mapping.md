# Impact Signal Level Mapping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Map canonical market impact metrics into impact levels C/B/A/S and trading signal levels L1/L2/L3/S.

**Architecture:** Extend the existing backend `MarketImpactNormalization` output with `impact_level`, `signal_level`, and `signal_label`, then pass those fields through FinalEventStore and the read-only frontend projection. Preserve existing `normalized_strength` as a compatibility field while making the new L-level visible in the contract event table and detail modal.

**Tech Stack:** Rust, serde camelCase API projection, React/Vitest, existing CWM FinalEventStore.

## Global Constraints

- Do not add execution buttons, live trading toggles, or irreversible actions.
- L1/L2/L3/S are read-only candidate signal levels in this task.
- Frontend must render backend-provided canonical fields and must not recompute signal levels.
- Existing FinalEventStore single-source-of-truth semantics remain intact.

---

### Task 1: Backend Impact Level Mapping

**Files:**
- Modify: `src/normalization/market_impact.rs`
- Modify: `src/core_event/final_store/final_event_store.rs`
- Test: `src/normalization/market_impact.rs`
- Test: `tests/contract_whale_routes_tests.rs`

**Interfaces:**
- Consumes: `MarketImpactBaseline::normalize(raw_volume: f64) -> MarketImpactNormalization`
- Produces: `impact_level: String`, `signal_level: String`, `signal_label: String`

- [x] **Step 1: Write the failing normalization test**

Add assertions for C->L1, B->L2, A->L3, and S->S using controlled baselines.

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test market_impact_maps_impact_levels_to_signal_levels`

- [x] **Step 3: Write minimal implementation**

Add `classify_impact_level`, `map_impact_to_signal_level`, and `impact_signal_label` in `market_impact.rs`.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test market_impact_maps_impact_levels_to_signal_levels`

### Task 2: API and Frontend Projection

**Files:**
- Modify: `toxic-order-monitor/src/api/contractWhale.js`
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Test: `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: `/api/final-events` fields `impactLevel`, `signalLevel`, `signalLabel`
- Produces: table and detail display for `signalLevel / impactLevel`

- [x] **Step 1: Write failing frontend tests**

Assert API mapping preserves canonical `impactLevel`, `signalLevel`, and `signalLabel`, and the event table renders `L3 / A`.

- [x] **Step 2: Run tests to verify they fail**

Run: `npm test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx -t "FinalEventStore|final events"`

- [x] **Step 3: Pass through and render the fields**

Update `normalizeFinalEvent` and `impactNormalizationBadge`.

- [x] **Step 4: Run tests to verify they pass**

Run: `npm test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx`

### Task 3: Full Verification

**Files:**
- No additional files.

**Interfaces:**
- Consumes: project validation commands.
- Produces: clean verification result.

- [x] **Step 1: Run Rust checks**

Run: `cargo fmt --check && cargo test --test contract_whale_routes_tests && cargo check`

- [x] **Step 2: Run frontend checks**

Run: `npm run build` from `toxic-order-monitor`.

- [x] **Step 3: Run diff safety checks**

Run: `git diff --check`.
