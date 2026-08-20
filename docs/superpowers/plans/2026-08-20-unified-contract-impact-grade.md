# Unified Contract Impact Grade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace competing contract-market letter grades with one versioned, evidence-first C/B/A/S assessment that includes observed liquidation, real robust historical normalization, deduplicated lifecycle evidence, and identical API/UI/Discord/retention semantics.

**Architecture:** Extend the existing contract-whale lifecycle into a maximum 60-minute canonical impact episode, enrich it from deduplicated raw flow and observed liquidation buckets, then call one `assess_contract_impact` function from every producer/projection path. A bounded liquidation runtime starts configured Binance and OKX collectors and records health without blocking trade-flow monitoring. Cohort calculations remain numerical diagnostics and stop producing letter grades.

**Tech Stack:** Rust 2021, Tokio, Axum, Rusqlite/SQLite, Serde, TOML configuration, React 19, Vitest 4, Docker Compose, PowerShell and SSH deployment.

## Global Constraints

- All enabled contract-market monitoring exposes exactly one authoritative letter grade: `C`, `B`, `A`, or `S`.
- Preserve `readOnly=true`, `analysisOnly=true`, and `executionEnabled=false`.
- Do not add order placement, signing, wallet access, trading execution, payment, or money-moving behavior.
- Missing, stale, inferred, non-finite, or degraded evidence never promotes a grade.
- S requires data quality at least 85, price movement at least 2.0%, at least two fresh sources, and either observed liquidation at least 250M USD or deduplicated turnover at least 1B USD with percentile at least 99.9 and robust z at least 6.0.
- A requires data quality at least 80, percentile at least 99.5, robust z at least 4.0, price movement at least 0.5%, turnover at least 150M USD, and one independent confirmation path.
- B requires data quality at least 70, percentile at least 99.0, robust z at least 2.5, price movement at least 0.15%, and turnover at least 50M USD.
- One-source evidence is capped at A; inferred liquidation cannot satisfy any S path.
- Overlapping 5/15/60-second windows must never multiply turnover or liquidation totals.
- API, frontend, Discord, and retention must consume the same versioned assessment.
- Never log or commit webhook URLs, tokens, environment contents, raw credentials, or complete untrusted payloads.
- Preserve the existing untracked `docs/superpowers/plans/2026-08-04-contract-whale-impact-grade-v3.md` file and unrelated user changes.

## File Structure

**Create:**

- `src/contract_whale_monitor/impact_grade.rs` — grade types, evidence contract, hierarchical gate, legacy adapter, and canonical field synchronization.
- `src/contract_whale_monitor/impact_baseline.rs` — log-notional median/MAD robust z-score and empirical percentile.
- `src/contract_whale_monitor/liquidation_runtime.rs` — bounded collector orchestration, persistence drain, and source diagnostics.
- `tests/contract_whale_impact_grade_tests.rs` — grade boundaries, missing evidence, episode dedupe, and legacy behavior.
- `tests/contract_whale_liquidation_runtime_tests.rs` — synthetic Binance/OKX normalization-to-persistence and runtime health.

**Modify:**

- `src/contract_whale_monitor/mod.rs`
- `src/contract_whale_monitor/types.rs`
- `src/contract_whale_monitor/config.rs`
- `src/contract_whale_monitor/aggregator.rs`
- `src/contract_whale_monitor/event_lifecycle.rs`
- `src/contract_whale_monitor/collector_binance.rs`
- `src/contract_whale_monitor/collector_okx.rs`
- `src/contract_whale_monitor/discord.rs`
- `src/contract_whale_monitor/discord_notifier.rs`
- `src/contract_whale_monitor/emission.rs`
- `src/api/contract_whale_routes.rs`
- `src/api/contract_event_routes.rs`
- `src/core_event/final_store/final_event_store.rs`
- `src/storage/contract_whale_repo.rs`
- `src/storage/sqlite.rs`
- `src/app.rs`
- `config/default.toml`
- `tests/contract_whale_monitor_tests.rs`
- `tests/contract_whale_persistence_tests.rs`
- `tests/contract_event_routes_tests.rs`
- `tests/contract_whale_discord_notifier_tests.rs`
- `toxic-order-monitor/src/api/contractWhale.js`
- `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`
- `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

---

### Task 1: Define the Versioned Grade Contract and Configuration

**Files:**
- Create: `src/contract_whale_monitor/impact_grade.rs`
- Create: `tests/contract_whale_impact_grade_tests.rs`
- Modify: `src/contract_whale_monitor/mod.rs`
- Modify: `src/contract_whale_monitor/types.rs`
- Modify: `src/contract_whale_monitor/config.rs`
- Modify: `config/default.toml`

**Interfaces:**
- Produces: `ContractImpactGrade`, `ContractImpactGradeState`, `ContractImpactEvidence`, `ContractImpactAssessment`, `UnifiedImpactGradeConfig`, `assess_contract_impact`, and `sync_contract_impact_fields`.
- Consumes: finite evidence supplied by later baseline, lifecycle, and liquidation tasks.

- [ ] **Step 1: Add failing business-boundary tests**

Create `tests/contract_whale_impact_grade_tests.rs` with a concrete evidence fixture and tests for ordinary C, exact B/A/S boundaries, one-source S capping, and unavailable evidence:

```rust
use btc_toxic_flow_monitor_rs::contract_whale_monitor::{
    config::UnifiedImpactGradeConfig,
    impact_grade::{
        assess_contract_impact, ContractImpactEvidence, ContractImpactGrade,
        ContractImpactGradeState, LiquidationEvidenceState,
    },
};

fn evidence() -> ContractImpactEvidence {
    ContractImpactEvidence {
        data_quality: 85,
        unique_turnover_notional_usd: Some(1_000_000_000.0),
        peak_abs_price_move_pct: Some(2.0),
        robust_percentile: Some(99.9),
        robust_z: Some(6.0),
        observed_liquidation_notional_usd: Some(250_000_000.0),
        liquidation_evidence_state: LiquidationEvidenceState::Observed,
        liquidation_source_count: 2,
        trade_confirmation_source_count: 2,
        oi_behavior_confirmed: false,
        critical_degradation_reasons: Vec::new(),
    }
}

#[test]
fn exact_two_source_liquidation_boundary_is_s() {
    let result = assess_contract_impact(&evidence(), &UnifiedImpactGradeConfig::default());
    assert_eq!(result.grade, ContractImpactGrade::S);
    assert_eq!(result.state, ContractImpactGradeState::Confirmed);
    assert!(result.reason_codes.iter().any(|code| code == "s_observed_liquidation"));
}

#[test]
fn one_source_extreme_is_capped_at_a() {
    let mut input = evidence();
    input.liquidation_source_count = 1;
    input.trade_confirmation_source_count = 1;
    let result = assess_contract_impact(&input, &UnifiedImpactGradeConfig::default());
    assert_eq!(result.grade, ContractImpactGrade::A);
    assert!(result.reason_codes.iter().any(|code| code == "s_two_sources_required"));
}

#[test]
fn inferred_liquidation_never_produces_s() {
    let mut input = evidence();
    input.liquidation_evidence_state = LiquidationEvidenceState::Inferred;
    input.unique_turnover_notional_usd = Some(149_999_999.0);
    let result = assess_contract_impact(&input, &UnifiedImpactGradeConfig::default());
    assert_ne!(result.grade, ContractImpactGrade::S);
}
```

- [ ] **Step 2: Run RED and record the expected missing-module failure**

Run:

```powershell
cargo test -j 1 --test contract_whale_impact_grade_tests -- --nocapture
```

Expected: compilation fails because `impact_grade` and `UnifiedImpactGradeConfig` do not exist.

- [ ] **Step 3: Add the typed assessment model and exact hierarchical gate**

Implement these public types in `impact_grade.rs` and add `pub mod impact_grade;` in `mod.rs`:

```rust
pub const UNIFIED_IMPACT_GRADE_VERSION: &str = "cwm_unified_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ContractImpactGrade { C, B, A, S }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractImpactGradeState { EvidenceInsufficient, Provisional, Confirmed }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiquidationEvidenceState { Unavailable, Inferred, ObservedSnapshot, Observed }

impl Default for LiquidationEvidenceState {
    fn default() -> Self { Self::Unavailable }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractImpactEvidence {
    pub data_quality: u8,
    pub unique_turnover_notional_usd: Option<f64>,
    pub peak_abs_price_move_pct: Option<f64>,
    pub robust_percentile: Option<f64>,
    pub robust_z: Option<f64>,
    pub observed_liquidation_notional_usd: Option<f64>,
    pub liquidation_evidence_state: LiquidationEvidenceState,
    pub liquidation_source_count: usize,
    pub trade_confirmation_source_count: usize,
    pub oi_behavior_confirmed: bool,
    pub critical_degradation_reasons: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractImpactAssessment {
    pub grade_version: String,
    pub grade: ContractImpactGrade,
    pub state: ContractImpactGradeState,
    pub reason_codes: Vec<String>,
    pub assessed_at_ms: i64,
    pub evidence: ContractImpactEvidence,
}
```

Implement `assess_contract_impact` in S/A/B/C order with finite-value helpers. The liquidation S path accepts both `ObservedSnapshot` and `Observed`, but records `liquidation_observed_lower_bound` for snapshots. The turnover S path requires percentile and robust z. If A gates fail after one-source S capping, evaluate B and C normally; the test fixture intentionally satisfies A.

Add `impact_assessment: Option<ContractImpactAssessment>` to `ContractWhaleSignal`, and make `sync_contract_impact_fields` map `S -> S/SHOCK IMPACT EVENT`, `A -> L3/HIGH IMPACT EVENT`, `B -> L2/MEDIUM IMPACT EVENT`, and `C -> L1/LOW IMPACT EVENT`.

- [ ] **Step 4: Add and load exact configuration values**

Add `UnifiedImpactGradeConfig` under `ContractWhaleRuntimeConfig` with nested S/A/B structs. Defaults and `config/default.toml` values must exactly match Global Constraints. Add lifecycle defaults `update_window_seconds = 900`, `close_after_seconds = 900`, and `max_duration_seconds = 3600`.

Config loading must reject or fall back from non-finite/negative thresholds and must preserve the ordering B < A < S. Tests must cover equality, one unit below, NaN, infinity, and inverted percentile values.

- [ ] **Step 5: Run GREEN and the existing monitor characterization tests**

Run:

```powershell
cargo test -j 1 --test contract_whale_impact_grade_tests -- --nocapture
cargo test -j 1 --test contract_whale_monitor_tests raw_a -- --nocapture
```

Expected: all new grade tests pass; existing raw-A tests may still pass through the legacy adapter until Task 5 removes the old sanitizer path.

- [ ] **Step 6: Commit the contract**

```powershell
git add src/contract_whale_monitor/impact_grade.rs src/contract_whale_monitor/mod.rs src/contract_whale_monitor/types.rs src/contract_whale_monitor/config.rs config/default.toml tests/contract_whale_impact_grade_tests.rs
git commit -m "feat: define unified contract impact grade"
```

### Task 2: Calculate Real Robust Historical Normalization

**Files:**
- Create: `src/contract_whale_monitor/impact_baseline.rs`
- Modify: `src/contract_whale_monitor/mod.rs`
- Modify: `src/api/contract_whale_routes.rs`
- Modify: `src/contract_whale_monitor/detector.rs`
- Modify: `src/normalization/market_impact.rs`
- Test: `tests/contract_whale_impact_grade_tests.rs`

**Interfaces:**
- Produces: `RobustImpactScore` and `score_robust_notional(current_notional_usd, samples, min_samples)`.
- Consumes: finite positive 60-second USD-notional samples loaded from the existing `contract_flow_1s` store for up to 90 days.

- [ ] **Step 1: Add failing median/MAD tests**

Add tests proving exact sample-count failure, outlier resistance, and no dynamic-multiple fallback:

```rust
#[test]
fn robust_notional_uses_log_median_and_mad() {
    let samples = (1..=10_000).map(|n| 1_000_000.0 + n as f64 * 10.0).collect::<Vec<_>>();
    let score = score_robust_notional(200_000_000.0, &samples, 10_000).expect("score");
    assert!(score.percentile >= 99.9);
    assert!(score.robust_z.is_finite());
    assert!(score.robust_z > 6.0);
}

#[test]
fn robust_notional_fails_closed_below_minimum_samples() {
    assert!(score_robust_notional(200_000_000.0, &[1_000_000.0; 9_999], 10_000).is_none());
}

#[test]
fn one_extreme_outlier_does_not_move_the_robust_center() {
    let mut samples = (1..=10_000).map(|n| 1_000_000.0 + n as f64).collect::<Vec<_>>();
    let before = score_robust_notional(2_000_000.0, &samples, 10_000).expect("before");
    samples[9_999] = 100_000_000_000.0;
    let after = score_robust_notional(2_000_000.0, &samples, 10_000).expect("after");
    assert!((before.robust_z - after.robust_z).abs() < 0.05);
}
```

- [ ] **Step 2: Run RED**

```powershell
cargo test -j 1 --test contract_whale_impact_grade_tests robust_notional -- --nocapture
```

Expected: compilation fails because `score_robust_notional` does not exist.

- [ ] **Step 3: Implement the robust scorer**

Implement `score_robust_notional` with finite-positive filtering, natural logarithms, sorted median, MAD, scale factor `1.4826`, and empirical nearest-rank percentile. Return `None` for insufficient samples, invalid current notional, zero/non-finite MAD, or non-finite output.

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RobustImpactScore {
    pub percentile: f64,
    pub robust_z: f64,
    pub sample_count: usize,
}
```

- [ ] **Step 4: Wire baseline loading without per-request double queries**

Extend `ContractWhaleQualityBaseline`, `RollingWindowStatsOptions`, and `ContractWhaleWindowStats` with `robust_percentile`, `robust_z`, and `robust_sample_count`. In `load_quality_baselines`, use the already loaded raw flow buckets, group deduplicated exchange/symbol buckets into non-overlapping 60-second USD-notional samples, keep only the configured 90-day lookback, and call `score_robust_notional` for the current episode/window notional. Pass those three values through `RollingWindowStatsOptions` into the detector without deriving them from dynamic multiple.

Change `market_impact_normalization` so `impact_z_score` receives the real robust z. Keep `dynamic_multiple` in its existing diagnostic field but never pass it as z-score. The raw `normalize_market_impact_from_metrics` letter result becomes diagnostic only; Task 5 replaces all canonical letter consumers.

- [ ] **Step 5: Run GREEN and detector regressions**

```powershell
cargo test -j 1 --test contract_whale_impact_grade_tests robust_notional -- --nocapture
cargo test -j 1 --test contract_whale_monitor_tests detector_populates_market_impact_fields -- --nocapture
```

Expected: real z-score assertions pass and no test asserts equality between dynamic multiple and z-score.

- [ ] **Step 6: Commit the baseline**

```powershell
git add src/contract_whale_monitor/impact_baseline.rs src/contract_whale_monitor/mod.rs src/api/contract_whale_routes.rs src/contract_whale_monitor/detector.rs src/normalization/market_impact.rs tests/contract_whale_impact_grade_tests.rs tests/contract_whale_monitor_tests.rs
git commit -m "feat: add robust contract impact baseline"
```

### Task 3: Turn Lifecycle Events into Deduplicated Impact Episodes

**Files:**
- Modify: `src/contract_whale_monitor/types.rs`
- Modify: `src/contract_whale_monitor/config.rs`
- Modify: `src/contract_whale_monitor/event_lifecycle.rs`
- Modify: `src/api/contract_whale_routes.rs`
- Modify: `config/default.toml`
- Test: `tests/contract_whale_impact_grade_tests.rs`
- Test: `tests/contract_whale_persistence_tests.rs`

**Interfaces:**
- Produces: `enrich_lifecycle_unique_evidence(events, flow_buckets, liquidation_buckets, failed_symbols)` and deterministic 60-minute episode evidence.
- Consumes: existing `ContractFlowBucket`, `ContractLiquidationBucket`, and lifecycle event IDs.

- [ ] **Step 1: Add failing lifecycle and overlap tests**

Add tests that create the same 5/15/60-second flow from identical one-second buckets and assert one episode, one turnover total, one liquidation total, and stable identity after replay:

```rust
#[test]
fn overlapping_windows_contribute_unique_evidence_once() {
    let mut events = lifecycle_candidates_same_direction();
    events = apply_contract_whale_event_lifecycle(events, ContractWhaleLifecycleClock::Replay {
        replay_now_ms: 1_700_000_060_000,
    });
    enrich_lifecycle_unique_evidence(&mut events, &flow_buckets(), &liquidation_buckets(), &Default::default());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_lifecycle.unique_turnover_notional_usd, Some(210_000_000.0));
    assert_eq!(events[0].event_lifecycle.observed_liquidation_notional_usd, Some(75_000_000.0));
    assert_eq!(events[0].event_lifecycle.liquidation_source_count, 2);
}
```

- [ ] **Step 2: Run RED**

```powershell
cargo test -j 1 --test contract_whale_impact_grade_tests overlapping_windows -- --nocapture
```

Expected: compilation fails because the USD lifecycle evidence fields and enrichment function do not exist.

- [ ] **Step 3: Extend lifecycle evidence and merge rules**

Add these fields to `ContractWhaleEventLifecycle` with serde defaults:

```rust
pub unique_turnover_notional_usd: Option<f64>,
pub observed_liquidation_notional_usd: Option<f64>,
pub liquidation_source_count: usize,
pub liquidation_evidence_state: LiquidationEvidenceState,
pub peak_abs_price_move_pct: Option<f64>,
pub evidence_degradation_reasons: Vec<String>,
```

Change lifecycle matching to require same symbol and direction, a gap no larger than `update_window_seconds`, and elapsed duration no larger than `max_duration_seconds`; do not require identical `signal_type`. The deterministic event ID must include `cwm_unified_v1`, symbol, direction, and first source event ID.

Implement one enrichment pass that deduplicates flow and liquidation buckets by `(symbol, exchange, ts_bucket)`, sums USD notional, counts distinct fresh liquidation exchanges, records snapshot provenance, and calculates peak absolute price movement from lifecycle snapshots. Query failures add reason codes and leave values unavailable.

- [ ] **Step 4: Reassess the episode after enrichment**

At the end of lifecycle enrichment, build `ContractImpactEvidence`, call `assess_contract_impact`, enforce monotonic active-episode promotion with `max(previous_grade, next_grade)`, store the assessment, and call `sync_contract_impact_fields`.

- [ ] **Step 5: Run GREEN and persistence regressions**

```powershell
cargo test -j 1 --test contract_whale_impact_grade_tests overlapping_windows -- --nocapture
cargo test -j 1 --test contract_whale_persistence_tests contract_liquidation_1s_upsert_and_window_context_are_available -- --nocapture
```

Expected: evidence is counted once and lifecycle persistence remains backward compatible.

- [ ] **Step 6: Commit lifecycle episodes**

```powershell
git add src/contract_whale_monitor/types.rs src/contract_whale_monitor/config.rs src/contract_whale_monitor/event_lifecycle.rs src/api/contract_whale_routes.rs config/default.toml tests/contract_whale_impact_grade_tests.rs tests/contract_whale_persistence_tests.rs
git commit -m "feat: grade deduplicated contract episodes"
```

### Task 4: Start and Observe Binance and OKX Liquidation Collection

**Files:**
- Create: `src/contract_whale_monitor/liquidation_runtime.rs`
- Create: `tests/contract_whale_liquidation_runtime_tests.rs`
- Modify: `src/contract_whale_monitor/mod.rs`
- Modify: `src/contract_whale_monitor/collector_binance.rs`
- Modify: `src/contract_whale_monitor/collector_okx.rs`
- Modify: `src/contract_whale_monitor/persistence.rs`
- Modify: `src/app.rs`
- Modify: `config/default.toml`

**Interfaces:**
- Produces: `ContractWhaleLiquidationRuntime`, `LiquidationCollectorHealthRegistry`, `LiquidationCollectorDiagnostics`, `start`, `stop`, and `diagnostics`.
- Consumes: configured enabled symbols, exchange liquidation settings, `SqliteStore`, normalizers, aggregator, and nonblocking persistence.

- [ ] **Step 1: Add failing synthetic end-to-end runtime tests**

Create tests that feed one Binance force-order JSON and one OKX liquidation-order JSON through normalizers into a bounded channel, drain one batch to a temporary SQLite store, and assert two exchanges, nonzero short liquidation, health timestamps, and no raw payload in diagnostics.

```rust
#[tokio::test]
async fn synthetic_liquidations_reach_persistence_and_health() {
    let store = temp_store("unified-liquidation-runtime");
    let registry = LiquidationCollectorHealthRegistry::default();
    let (sender, receiver) = tokio::sync::mpsc::channel(16);
    sender.send(binance_short_liquidation()).await.expect("binance order");
    sender.send(okx_short_liquidation()).await.expect("okx order");
    drop(sender);
    drain_liquidation_orders(receiver, Some(store.clone()), registry.clone()).await;
    let rows = store.list_contract_liquidation_buckets_between("BTC", 0, i64::MAX).expect("rows");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().map(|row| row.short_liq_btc).sum::<f64>() > 0.0);
    assert_eq!(registry.diagnostics().len(), 2);
}
```

- [ ] **Step 2: Run RED**

```powershell
cargo test -j 1 --test contract_whale_liquidation_runtime_tests -- --nocapture
```

Expected: compilation fails because the runtime and health registry do not exist.

- [ ] **Step 3: Implement bounded runtime and health registry**

Use a channel capacity of 2,048. The receiver batches until a one-second interval or channel close, calls `aggregate_liquidation_1s_buckets`, persists with `flush_contract_liquidation_buckets_nonblocking`, and records last persisted time. Collector health stores only exchange, symbol, configured/enabled state, connecting/connected/degraded/stopped state, last message time, last persisted time, reconnect count, safe error code, data age, and freshness.

Generalize collectors with these entrypoints while keeping BTC wrappers for existing tests:

```rust
pub async fn run_binance_force_order_collector_for_symbol(
    symbol: String,
    sender: tokio::sync::mpsc::Sender<ContractLiquidationOrder>,
    health: LiquidationCollectorHealthRegistry,
);

pub async fn run_okx_liquidation_collector_for_symbol(
    symbol: String,
    sender: tokio::sync::mpsc::Sender<ContractLiquidationOrder>,
    ct_val_base: f64,
    health: LiquidationCollectorHealthRegistry,
);
```

Record Binance provenance as `ObservedSnapshot`. OKX instrument metadata fallback marks evidence degraded and cannot count as a fresh S source until live metadata succeeds.

- [ ] **Step 4: Wire start, stop, and diagnostics into AppState**

Add `contract_whale_liquidation_runtime` to `AppStateInner`. Start it after market services when contract-whale monitoring and at least one liquidation source are enabled. Stop it before the store-dependent monitoring loops stop. Include its diagnostics in `ContractWhaleRuntimeDiagnostics` returned by `/api/contract-whale/latest`.

Keep Binance enabled with `exchanges.binance.liquidation.enabled = true`. Set `exchanges.okx.enabled = true` and `exchanges.okx.liquidation.enabled = true`, while leaving OKX spot, perp-trade, level2, funding, and OI sources unchanged unless they were already enabled by deployment configuration. Keep all execution flags false. A collector failure must update diagnostics and reconnect without stopping the producer loop.

- [ ] **Step 5: Run GREEN and Tokio safety tests**

```powershell
cargo test -j 1 --test contract_whale_liquidation_runtime_tests -- --nocapture
cargo test -j 1 --test contract_whale_monitor_tests binance_force_order -- --nocapture
cargo test -j 1 --test contract_whale_monitor_tests okx_liquidation -- --nocapture
```

Expected: synthetic data persists, health is bounded/safe, and existing normalizer tests pass.

- [ ] **Step 6: Commit liquidation runtime**

```powershell
git add src/contract_whale_monitor/liquidation_runtime.rs src/contract_whale_monitor/mod.rs src/contract_whale_monitor/collector_binance.rs src/contract_whale_monitor/collector_okx.rs src/contract_whale_monitor/persistence.rs src/app.rs config/default.toml tests/contract_whale_liquidation_runtime_tests.rs tests/contract_whale_monitor_tests.rs
git commit -m "feat: start contract liquidation evidence runtime"
```

### Task 5: Make the Unified Assessment the Only Backend Grade

**Files:**
- Modify: `src/contract_whale_monitor/impact_grade.rs`
- Modify: `src/contract_whale_monitor/detector.rs`
- Modify: `src/contract_whale_monitor/discord.rs`
- Modify: `src/contract_whale_monitor/discord_notifier.rs`
- Modify: `src/contract_whale_monitor/emission.rs`
- Modify: `src/storage/contract_whale_repo.rs`
- Modify: `src/storage/sqlite.rs`
- Modify: `src/api/contract_whale_routes.rs`
- Modify: `src/api/contract_event_routes.rs`
- Modify: `src/core_event/final_store/final_event_store.rs`
- Test: `tests/contract_whale_impact_grade_tests.rs`
- Test: `tests/contract_event_routes_tests.rs`
- Test: `tests/contract_whale_discord_notifier_tests.rs`

**Interfaces:**
- Consumes: current-version `ContractImpactAssessment` from Tasks 1–4.
- Produces: one canonical grade across persistence, routes, final-event projection, Discord, and retention.

- [ ] **Step 1: Add failing cross-surface consistency and S-promotion tests**

Add a raw A event with current-version two-source 250M USD observed liquidation evidence and assert it becomes S. Add an API/Discord/retention projection test that asserts all surfaces expose the same grade and version.

```rust
#[test]
fn hard_episode_evidence_promotes_raw_a_to_s() {
    let mut signal = raw_a_signal();
    signal.impact_assessment = Some(assess_contract_impact(&s_evidence(), &UnifiedImpactGradeConfig::default()));
    sync_contract_impact_fields(&mut signal);
    assert_eq!(signal.impact_level.as_deref(), Some("S"));
    assert_eq!(signal.signal_level.as_deref(), Some("S"));
}
```

- [ ] **Step 2: Run RED against current sanitizer behavior**

```powershell
cargo test -j 1 --test contract_whale_impact_grade_tests hard_episode_evidence -- --nocapture
cargo test -j 1 --test contract_event_routes_tests unified_contract_grade -- --nocapture
```

Expected: tests fail because old sanitization cannot promote raw A and projections do not carry the versioned assessment.

- [ ] **Step 3: Replace canonical sanitizer decisions with assessment synchronization**

Keep `sanitize_contract_whale_impact` as a compatibility entrypoint, but make it call only functions from `impact_grade.rs`:

- current-version assessment: trust the typed assessment and synchronize string fields;
- legacy persisted S: create a `legacy_v2` assessment capped at A unless existing replayable hard evidence passes;
- legacy A/B/C: create a legacy assessment without promoting it;
- new signals: call `assess_contract_impact` from lifecycle evidence, then synchronize.

Remove every independent mutation of `impact_level`, `signal_level`, and `signal_label` outside `impact_grade.rs`. Raw detector severity remains an operational diagnostic and does not write a canonical letter.

- [ ] **Step 4: Unify confirmation, Discord dedupe, and retention**

Replace the Bitfinex 20 BTC special case with the 2% notional/net-share and 0.55 dominance rule. Make S Discord eligibility require `grade=S`, current grade version, and `state=confirmed`. Change the stable Discord episode key to include `event_lifecycle.event_id` and grade version. Retention reads the typed assessment; confirmed S receives permanent/maximum retention, A receives 365 days, B receives 90 days, and C receives 7 days.

- [ ] **Step 5: Synchronize API and final-event projections**

Ensure nested source signals and flattened final-event fields copy from the same assessment. Add `impactGradeVersion`, `impactGradeState`, and `impactGradeReasonCodes`. Legacy rows expose `legacy_v2` and cannot appear as newly confirmed S.

- [ ] **Step 6: Run GREEN across backend surfaces**

```powershell
cargo test -j 1 --test contract_whale_impact_grade_tests -- --nocapture
cargo test -j 1 --test contract_event_routes_tests unified_contract_grade -- --nocapture
cargo test -j 1 --test contract_whale_discord_notifier_tests -- --nocapture
cargo test -j 1 --test contract_whale_persistence_tests -- --nocapture
```

Expected: S promotion works, legacy records fail closed, Discord dedupes by episode/version, and every backend surface agrees.

- [ ] **Step 7: Commit backend authority**

```powershell
git add src/contract_whale_monitor/impact_grade.rs src/contract_whale_monitor/detector.rs src/contract_whale_monitor/discord.rs src/contract_whale_monitor/discord_notifier.rs src/contract_whale_monitor/emission.rs src/storage/contract_whale_repo.rs src/storage/sqlite.rs src/api/contract_whale_routes.rs src/api/contract_event_routes.rs src/core_event/final_store/final_event_store.rs tests/contract_whale_impact_grade_tests.rs tests/contract_event_routes_tests.rs tests/contract_whale_discord_notifier_tests.rs tests/contract_whale_persistence_tests.rs
git commit -m "fix: make unified impact grade canonical"
```

### Task 6: Remove the Second Letter Grade from API Normalization and UI

**Files:**
- Modify: `toxic-order-monitor/src/api/contractWhale.js`
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`
- Modify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`
- Test: `tests/contract_event_routes_tests.rs`

**Interfaces:**
- Consumes: canonical `impactLevel`, `signalLevel`, `signalLabel`, grade version/state/reasons, plus numerical cohort percentile/z diagnostics.
- Produces: frontend models and views with exactly one C/B/A/S letter grade.

- [ ] **Step 1: Change tests first to reject cohort letter fields**

Update API tests so normalized events keep `cohortPercentile` and `cohortZScore` but do not contain `cohortImpactLevel`, `cohortSignalLevel`, or `cohortSignalLabel`. Update the component test to assert `页面相对等级` is absent while `页面相对指标` and the canonical impact grade remain visible.

```javascript
expect(payload.active[0]).toMatchObject({
  impactLevel: "A",
  signalLevel: "L3",
  cohortPercentile: 90.476,
});
expect(payload.active[0]).not.toHaveProperty("cohortImpactLevel");
expect(screen.queryByText("页面相对等级")).not.toBeInTheDocument();
```

- [ ] **Step 2: Run RED**

```powershell
npm --prefix toxic-order-monitor test -- ContractWhaleApi.test.js ContractWhaleMonitor.test.jsx
```

Expected: tests fail because the normalizer and component still expose cohort letter grades.

- [ ] **Step 3: Remove cohort letter derivation and display**

Delete cohort letter parsing/fallback fields from `normalizeImpactFields`, `normalizeFinalEvent`, and component `resolveImpactDisplay`. Keep only numerical cohort impact score, robust z, percentile, normalized score, and normalized strength. Render canonical grade/version/reason codes and retain the numerical `页面相对指标` row.

Remove public backend cohort letter fields from the final-event serializer while preserving numerical diagnostics.

- [ ] **Step 4: Run GREEN and build frontend**

```powershell
npm --prefix toxic-order-monitor test -- ContractWhaleApi.test.js ContractWhaleMonitor.test.jsx
npm --prefix toxic-order-monitor run build
cargo test -j 1 --test contract_event_routes_tests -- --nocapture
```

Expected: tests and build pass; no rendered or normalized contract event contains a second letter grade.

- [ ] **Step 5: Commit the single-grade UI**

```powershell
git add toxic-order-monitor/src/api/contractWhale.js toxic-order-monitor/src/components/ContractWhaleMonitor.jsx toxic-order-monitor/src/tests/ContractWhaleApi.test.js toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx src/api/contract_event_routes.rs src/core_event/final_store/final_event_store.rs tests/contract_event_routes_tests.rs
git commit -m "refactor: expose one contract impact grade"
```

### Task 7: Full Verification, Git Push, and Server Deployment

**Files:**
- Modify only if verification finds a task-related defect: files already listed in Tasks 1–6.
- Preserve: `docs/superpowers/plans/2026-08-04-contract-whale-impact-grade-v3.md`.

**Interfaces:**
- Consumes: completed local implementation and exact branch commit.
- Produces: pushed Git history and a healthy read-only server running the same commit.

- [ ] **Step 1: Run the focused backend matrix**

```powershell
cargo test -j 1 --test contract_whale_impact_grade_tests -- --nocapture
cargo test -j 1 --test contract_whale_liquidation_runtime_tests -- --nocapture
cargo test -j 1 --test contract_whale_monitor_tests -- --nocapture
cargo test -j 1 --test contract_whale_persistence_tests -- --nocapture
cargo test -j 1 --test contract_event_routes_tests -- --nocapture
cargo test -j 1 --test contract_whale_discord_notifier_tests -- --nocapture
```

Expected: every suite exits 0 with zero failed tests.

- [ ] **Step 2: Run the full repository gate sequentially**

```powershell
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -j 1 --all-targets --all-features --no-fail-fast
npm --prefix toxic-order-monitor test
npm --prefix toxic-order-monitor run build
docker compose config --quiet
```

Expected: all commands exit 0. If an unrelated pre-existing failure occurs, preserve the output and distinguish it from task-related failures before proceeding.

- [ ] **Step 3: Audit scope and secrets before push**

```powershell
git status --short --branch
git diff origin/codex/main-force-behavior-v4...HEAD --stat
git diff origin/codex/main-force-behavior-v4...HEAD --check
git diff origin/codex/main-force-behavior-v4...HEAD --name-only
```

Confirm the old untracked V3 plan is still untracked, no `.env` or runtime data is staged, and no webhook/token string appears in the diff.

- [ ] **Step 4: Push the exact branch**

```powershell
git push origin codex/main-force-behavior-v4
git rev-parse HEAD
git ls-remote origin refs/heads/codex/main-force-behavior-v4
```

Expected: local HEAD and remote branch hash are identical.

- [ ] **Step 5: Fast-forward and rebuild affected server services**

Use the established physical-NIC SSH path. On `/opt/toxic-order-monitor-rs` run:

```bash
git status --short --branch
git fetch origin codex/main-force-behavior-v4
git merge --ff-only origin/codex/main-force-behavior-v4
git rev-parse HEAD
docker compose build --no-cache backend frontend
docker compose up -d --no-deps backend frontend
docker compose ps backend frontend
```

Do not prune Docker, delete data, restart unrelated services, or overwrite a dirty server worktree. Stop and report if fast-forward is impossible.

- [ ] **Step 6: Verify health, safety, collectors, and canonical output**

On the server, verify:

```bash
curl -fsS http://127.0.0.1:8000/healthz
curl -fsS http://127.0.0.1:8000/readyz
operator_token="$(docker inspect toxic-bot --format '{{range .Config.Env}}{{println .}}{{end}}' | sed -n 's/^OPERATOR_TOKEN=//p' | head -n1)"
curl -fsS -H "X-Operator-Token: ${operator_token}" 'http://127.0.0.1:8000/api/contract-whale/latest?symbol=BTC&limit=10'
curl -fsS -H "X-Operator-Token: ${operator_token}" 'http://127.0.0.1:8000/api/final-events-v2?symbol=BTC&range=24h&limit=100'
```

Parse responses without printing `operator_token`. Confirm:

- deployed hash equals pushed hash;
- backend and frontend containers are healthy;
- `readOnly=true`, `analysisOnly=true`, and `executionEnabled=false`;
- Binance and enabled OKX liquidation diagnostics are present, running, and do not expose URLs or credentials;
- newly persisted liquidation buckets become nonzero when real liquidation messages arrive;
- each event has one canonical `impactLevel` matching nested `sourceSignal`;
- no public event contains cohort letter-grade fields;
- no new startup error burst appears in recent backend logs.

- [ ] **Step 7: Run a clearly marked synthetic grade smoke without fake production history**

Execute the deterministic synthetic episode through the backend test binary or an enabled internal acceptance path. The payload must contain `SIMULATED / TEST`, must not insert a real-market event into production history, and must not invoke any trading action. Verify it resolves to S under two-source hard evidence and that repeated execution retains the same episode/version dedupe key.

- [ ] **Step 8: Record deployment evidence and final repository state**

```powershell
git status --short --branch
git log -7 --oneline --decorate
```

Report exact commit, pushed hash, server hash, container health, test counts, collector states, canonical grade consistency, read-only flags, and the unchanged untracked V3 plan.

## Plan Self-Review Checklist

- Spec coverage: Tasks 1–7 cover the grade contract, real robust z, episode dedupe, live liquidation collection, runtime diagnostics, canonical backend projection, single-grade frontend, tests, Git push, and deployment.
- Placeholder scan: the plan contains no deferred requirement, undefined implementation placeholder, or unspecified error-handling step.
- Type consistency: `ContractImpactEvidence`, `ContractImpactAssessment`, `UnifiedImpactGradeConfig`, `LiquidationEvidenceState`, and `ContractWhaleLiquidationRuntime` keep the same names and roles across all tasks.
- Safety: every external notification remains alert-only; no task adds execution or exposes a secret.
- Scope: unrelated frontend redesign, database replacement, historical reclassification mutation, and trading features are excluded.
