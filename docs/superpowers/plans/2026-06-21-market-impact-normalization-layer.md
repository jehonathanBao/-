# Market Impact Normalization Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a read-only market impact normalization layer to canonical FinalEvent output so BTC, ETH, and altcoin events expose comparable impact_score, z_score, percentile, and normalized_strength.

**Architecture:** Compute normalization in the backend FinalEvent projection stage, after event lifecycle/quality filtering and before API/frontend projection. Frontend renders backend-provided canonical values only and does not recompute strength.

**Tech Stack:** Rust backend modules/tests, Axum routes, React/Vite frontend API normalization/tests.

## Global Constraints

- Read-only analytics only; do not enable execution, trading, Discord sends, or irreversible actions.
- TDD required: write failing tests, verify red, implement minimal code, verify green.
- Preserve existing FinalEventStore single source of truth semantics.
- Do not replace existing CWM signal/event lifecycle behavior.

---

### Task 1: Backend Impact Normalization Model

**Files:**
- Create: `src/normalization/mod.rs`
- Create: `src/normalization/market_impact.rs`
- Modify: `src/lib.rs`
- Test: `tests/contract_whale_routes_tests.rs`

**Interfaces:**
- Consumes: event volume distribution from `ContractWhaleSignal.total_volume_btc`.
- Produces: `MarketImpactNormalization`, `MarketImpactBaseline`, `normalize_market_impact(volume, baseline)`.

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn final_event_store_computes_cross_event_impact_normalization() {
    let mut low = persisted_signal(1_700_000_000_000, ContractWhaleSeverity::Medium);
    low.total_volume_btc = 100.0;
    let mut mid = persisted_signal(1_700_000_010_000, ContractWhaleSeverity::Medium);
    mid.total_volume_btc = 200.0;
    let mut high = persisted_signal(1_700_000_020_000, ContractWhaleSeverity::High);
    high.total_volume_btc = 700.0;

    let response = build_contract_whale_history_response(vec![high, mid, low], "BTC", 50, None, true, true, None);
    let final_response = build_final_event_store_response_from_contract_whale_response(&response);
    let strongest = final_response.items.iter().find(|event| event.volume == 700.0).unwrap();

    assert!(strongest.impact_score > 2.0);
    assert!(strongest.z_score > 1.0);
    assert!(strongest.percentile >= 90.0);
    assert_eq!(strongest.normalized_strength, "EXTREME");
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --test contract_whale_routes_tests final_event_store_computes_cross_event_impact_normalization`
Expected: fail because `impact_score`, `z_score`, `percentile`, and `normalized_strength` do not exist.

- [ ] **Step 3: Implement minimal code**

Add a small backend normalization module and enrich `FinalEvent` while building a `FinalEventStoreResponse`.

- [ ] **Step 4: Verify green**

Run: `cargo test --test contract_whale_routes_tests final_event_store_computes_cross_event_impact_normalization`
Expected: PASS.

### Task 2: Frontend Projection Contract

**Files:**
- Modify: `toxic-order-monitor/src/api/contractWhale.js`
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Test: `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: `FinalEvent.impactScore`, `FinalEvent.zScore`, `FinalEvent.percentile`, `FinalEvent.normalizedStrength`.
- Produces: table-visible impact metrics from FinalEventStore projection.

- [ ] **Step 1: Write failing API test**

Assert `fetchFinalEvents` maps canonical impact fields without client recompute.

- [ ] **Step 2: Verify red**

Run: `npm test -- --run src/tests/ContractWhaleApi.test.js -t "impact normalization"`
Expected: FAIL before normalization fields are mapped.

- [ ] **Step 3: Implement minimal frontend normalization passthrough**

Map backend fields into `item.finalEvent` and show a compact impact column in the contract event feed.

- [ ] **Step 4: Verify green**

Run: `npm test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx`
Expected: PASS.

### Task 3: Regression Checks

**Files:**
- No new code unless checks fail.

**Interfaces:**
- Confirms backend/frontend build still works.

- [ ] **Step 1: Run backend checks**

Run:
```powershell
cargo fmt --check
cargo test --test contract_whale_routes_tests
cargo check
```

- [ ] **Step 2: Run frontend checks**

Run:
```powershell
npm test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx
npm run build
```

- [ ] **Step 3: Self-review**

Confirm no Discord send/trading/execution paths were enabled and frontend does not recompute canonical strength.
