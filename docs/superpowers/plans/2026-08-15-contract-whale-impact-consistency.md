# Contract Whale Impact Consistency Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the contract-event tape expose the detector-persisted impact grade used by Discord, while retaining page-cohort impact metrics under explicitly named fields.

**Architecture:** The detector remains the source of truth for operator-facing impact grade (`impactLevel`, `signalLevel`, `signalLabel`, and detector metrics). The final-event projection will calculate page-cohort metrics once, publish them as `cohort*` fields, and no longer overwrite the canonical detector grade. The frontend will render the canonical grade and show the cohort grade/metrics as secondary context in the event detail.

**Tech Stack:** Rust/Axum/Serde backend, SQLite-backed contract-whale projections, React/Vite frontend, Cargo integration tests, Vitest.

## Global Constraints

- Keep the monitor read-only; do not add execution, trading, signing, deletion, or admin controls.
- Preserve backward-compatible API fields; add optional/defaulted cohort fields.
- Preserve detector and Discord push semantics; only change final-event projection labeling.
- Do not commit secrets, `.env` files, runtime data, or generated caches.
- Commit, push, and deploy only after local tests and build pass.

---

### Task 1: Lock canonical-vs-cohort behavior with a failing backend regression test

**Files:**
- Modify: `src/core_event/final_store/final_event_store.rs:11-77,177-230`
- Test: `tests/contract_event_routes_tests.rs` or the existing `#[cfg(test)]` module in `src/core_event/final_store/final_event_store.rs`

**Interfaces:**
- Consumes: `ContractWhaleSignal` detector-persisted impact fields and `MarketImpactNormalization` page-cohort result.
- Produces: a regression assertion that a persisted detector `A/L3` remains canonical even when the cohort normalization is `C/L1`.

- [ ] **Step 1: Write the failing test**

Create a focused test using the existing sample signal helper. Set persisted detector fields to `A/L3`, pass a cohort normalization with `C/L1`, and assert the projected event currently returns `A/L3`. Also assert new `cohortImpactLevel`/`cohortSignalLevel` values are present after the implementation.

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `cargo test --lib final_store -- --test-threads=1`

Expected: FAIL because the current projection returns the passed cohort `C/L1` and has no cohort fields.

### Task 2: Implement the canonical detector grade plus cohort diagnostics

**Files:**
- Modify: `src/core_event/final_store/final_event_store.rs:11-77,177-230`
- Modify: `src/api/contract_event_routes.rs` only if response typing or serialization requires it

**Interfaces:**
- Consumes: the failing regression from Task 1.
- Produces: `FinalEvent` fields `cohortImpactScore`, `cohortZScore`, `cohortPercentile`, `cohortNormalizedScore`, `cohortNormalizedStrength`, `cohortImpactLevel`, `cohortSignalLevel`, and `cohortSignalLabel`, all defaulted for rolling compatibility.

- [ ] **Step 1: Add defaulted cohort fields**

Add numeric cohort fields with `#[serde(default)]` and string cohort fields with `#[serde(default)]` to `FinalEvent`.

- [ ] **Step 2: Select detector fields as canonical with safe fallbacks**

In `from_contract_signal_with_impact`, use persisted detector `impact_score`, `impact_z_score`, `percentile_level`, `normalized_strength`, `impact_level`, `signal_level`, and `signal_label` when present; fall back field-by-field to the cohort normalization for legacy rows. Store the complete cohort normalization in the new `cohort*` fields.

- [ ] **Step 3: Run the focused test to verify it passes**

Run: `cargo test --lib final_store -- --test-threads=1`

Expected: PASS, including the detector `A/L3` preservation and cohort `C/L1` diagnostics.

### Task 3: Normalize frontend payloads and expose the two meanings clearly

**Files:**
- Modify: `toxic-order-monitor/src/api/contractWhale.js:1102-1245`
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx:5287-5335,3580-3595`
- Test: `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: canonical and `cohort*` JSON fields from `FinalEvent`.
- Produces: UI rows/details where `Impact Level` is detector-canonical and `页面相对等级`/cohort metrics are clearly secondary.

- [ ] **Step 1: Write failing frontend tests**

Add one API normalization test and one detail-render test proving an item with canonical `A/L3` and cohort `C/L1` renders both without replacing the canonical grade.

- [ ] **Step 2: Run the frontend tests to verify they fail**

Run: `npm test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx`

Expected: FAIL because the normalizer and detail grid do not expose cohort fields yet.

- [ ] **Step 3: Implement the minimal normalizer/UI changes**

Map camelCase/snake_case cohort fields in `resolveImpactNormalization`/`normalizeFinalEvent`, retain canonical `impactLevel`, and add a read-only detail row showing `页面相对等级` plus the cohort score/z/percentile.

- [ ] **Step 4: Run the frontend tests to verify they pass**

Run: `npm test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx`

Expected: PASS with no new warnings.

### Task 4: Full verification, commit, push, deploy, and live consistency check

**Files:**
- No additional source files; inspect `git diff` and deployment runbook.

**Interfaces:**
- Consumes: passing backend/frontend tests and build artifacts.
- Produces: pushed commit, rebuilt backend/frontend on `/opt/toxic-order-monitor-rs`, healthy services, and matching live API grades.

- [ ] **Step 1: Run local verification**

Run sequentially:

```powershell
cargo test --test contract_event_routes_tests -- --test-threads=1
cargo test --test contract_whale_monitor_tests -- --test-threads=1
npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx
npm --prefix toxic-order-monitor run build
```

- [ ] **Step 2: Review and commit only intended files**

Run `git diff --check`, inspect `git status --short`, then commit with:

```powershell
git add src/core_event/final_store/final_event_store.rs src/api/contract_event_routes.rs toxic-order-monitor/src/api/contractWhale.js toxic-order-monitor/src/components/ContractWhaleMonitor.jsx toxic-order-monitor/src/tests/ContractWhaleApi.test.js toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx docs/superpowers/plans/2026-08-15-contract-whale-impact-consistency.md
git commit -m "fix: align contract impact grades across tape and Discord"
git push origin HEAD
```

- [ ] **Step 3: Sync the authorized server deployment**

Use the established physical-NIC SSH path with the configured key, run `git pull --ff-only`, rebuild/recreate only `backend` and `frontend`, then verify `docker compose ps`, `/healthz`, and `/readyz`.

- [ ] **Step 4: Verify live API consistency**

Fetch the same BTC event from `/api/contract-whale/history` and `/api/final-events-v2`; assert canonical `impactLevel`/`signalLevel` match while `cohortImpactLevel` remains available as secondary context.
