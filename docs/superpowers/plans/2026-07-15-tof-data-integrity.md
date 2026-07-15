# TOF Data Integrity And Alert Isolation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Every behavior change follows RED -> GREEN -> focused regression before the next task.

**Goal:** Replace synthetic toxic-order measurements with source-traceable, symbol-scoped observations; preserve detector risk semantics; fail closed at every external-alert boundary; and make unavailable runtime data explicit in the operator UI.

**Architecture:** Add a compatibility provenance layer to the existing TOF, perpetual, and advanced payloads. The persisted detector score remains authoritative. Runtime enrichment reads symbol-matched flow, VPIN, sweep, and CWM evidence and emits nullable values plus lineage. HTTP, WebSocket, and Discord consume the same server-owned snapshot. The frontend keeps the previous field names but renders null/stale/inferred states honestly.

**Tech Stack:** Rust 2021, Axum, Tokio, Serde, SQLite, React 19, Zustand, Axios, Vitest, Testing Library, Vite, Docker Compose.

## Global constraints

- Keep the product read-only and analysis-only. Add no order, wallet, signing, payment, withdrawal, deployment, deletion, or live-trading control.
- Do not read, print, overwrite, or commit `.env`, webhook URLs, tokens, keys, runtime databases, or captured market data.
- Missing provenance defaults to `unavailable`; missing numeric evidence serializes as `null` and cannot be replaced by zero or a severity-derived estimate.
- Inferred or stale evidence may be displayed with a warning but cannot increase detector risk, data quality, confidence, direction certainty, or external-alert eligibility.
- Keep the existing High/Critical, score `>= 80`, data-quality `>= 70`, dedupe, and cooldown requirements. This work may make the gate stricter, never looser.
- Normalize and compare symbols before accepting runtime evidence. A mismatched symbol fails closed.
- Future outcome data is calibration-only and must never enter detection-time scoring.

---

### Task 1: Lock the provenance and authoritative-risk contract

**Files:**
- Create: `src/runtime/metric_provenance.rs`
- Modify: `src/runtime/mod.rs`
- Modify: `src/types/toxic_signal_inbox.rs`
- Modify: `src/toxicity/toxic_signal_inbox.rs`
- Modify: `tests/toxic_signal_inbox_tests.rs`
- Create: `tests/metric_provenance_tests.rs`

**Interfaces:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricProvenance {
    Observed,
    CalculatedFromObserved,
    Inferred,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricLineage {
    pub provenance: MetricProvenance,
    pub available: bool,
    pub fresh: bool,
    pub source: String,
    pub observed_at_ms: Option<i64>,
    pub unavailable_reason: Option<String>,
    pub alert_eligible: bool,
}
```

- [ ] Add tests proving serde uses the four stable snake-case values and that the default/missing constructor is unavailable and not alert-eligible.
- [ ] Add inbox behavior tests proving an input detector score of `83` and its numeric data-quality value survive mapping unchanged.
- [ ] Add a regression test proving runtime safety fields in the inbox copy the source fusion response rather than being synthesized by `build_toxic_signal_inbox_recent`.
- [ ] Run RED:

```powershell
cargo test -j 1 --test metric_provenance_tests --test toxic_signal_inbox_tests
```

Expected: compile/test failure because provenance and authoritative numeric fields do not yet exist.

- [ ] Implement the provenance types and safe constructors (`observed`, `calculated`, `inferred`, `unavailable`). Enforce `alert_eligible == available && fresh && provenance in {observed, calculated_from_observed}` in constructors.
- [ ] Add nullable `risk_score` and `data_quality_score` to `ToxicSignalInboxItem`, populated directly from `ToxicSignal.toxicity_score` and `ToxicSignal.data_quality`; do not map severity or quality buckets to numbers.
- [ ] Copy all runtime safety flags from `ToxicSignalRecentResponse` into inbox recent/status/detail responses.
- [ ] Run GREEN with the focused command, then run `cargo test -j 1 --test api_security_guard_tests`.

---

### Task 2: Replace synthetic enrichment with observed, nullable runtime snapshots

**Files:**
- Modify: `src/runtime/tof_metrics.rs`
- Modify: `src/runtime/perp_tof_metrics.rs`
- Modify: `src/runtime/advanced_tof_metrics.rs`
- Modify: `src/runtime/cwm_risk_fusion.rs`
- Modify: `src/api/toxic_signal_inbox_routes.rs`
- Modify: `src/api/toxic_signal_ws_routes.rs`
- Modify: `src/api/discord_notification_routes.rs`
- Modify: `src/app.rs`
- Modify: `tests/tof_metrics_tests.rs`
- Modify: `tests/oi_metrics_tests.rs`
- Modify: `tests/advanced_tof_metrics_tests.rs`
- Modify: `tests/toxic_signal_inbox_api_tests.rs`
- Modify: `tests/toxic_signal_ws_routes_tests.rs`
- Modify: `tests/api_security_guard_tests.rs`

**Interfaces:**

```rust
pub struct TofObservedInput<'a> {
    pub symbol: &'a str,
    pub candidate_at_ms: i64,
    pub flow: Option<&'a FlowState>,
    pub vpin: Option<&'a VpinState>,
    pub sweep: Option<&'a SweepState>,
}

pub struct PerpObservedInput<'a> {
    pub symbol: &'a str,
    pub candidate_at_ms: i64,
    pub cwm_signal: Option<&'a ContractWhaleSignal>,
}
```

Each metric family keeps its existing JSON object but makes source-dependent numeric fields `Option<f64>` and adds `lineage: MetricLineage`. TOF also exposes `vpinZscore`, `vpinPercentile`, and `perVenueVpin`. `riskScore` remains the persisted detector score; `toxicityHazardScore` is nullable and direction-free; `directionContext` remains separate.

- [ ] Change focused tests first to prove: no L2 means null spread/depth; no CWM OI/funding means null; inferred liquidation is display-only and alert-ineligible; advanced values are null unless all required inputs are fresh and alert-eligible; opposite direction with the same hazard evidence yields the same hazard score.
- [ ] Add HTTP/WS parity tests proving the same `83` detector score, data quality, lineage, and nullable metrics appear on both transports.
- [ ] Add alert tests proving a synthetic/inferred score of `92` cannot replace detector score `83`, inferred-only evidence cannot pass the gate, and a client cannot forge observed provenance.
- [ ] Run RED:

```powershell
cargo test -j 1 --test tof_metrics_tests --test oi_metrics_tests --test advanced_tof_metrics_tests --test toxic_signal_inbox_api_tests --test toxic_signal_ws_routes_tests --test api_security_guard_tests
```

- [ ] Remove production calls to `synthetic_perp_scenario`, severity-to-risk, quality-bucket-to-score, and summary-text measurement inference. Pure classifier helpers may remain for observed fixtures.
- [ ] Build one server-owned metric snapshot per inbox request from `AppState.flow_state_for_symbol`, symbol-scoped VPIN, sweep, and same-symbol CWM evidence. Reuse the same builder for REST, WebSocket, manual Discord preview, and automatic Discord evaluation.
- [ ] Calculate aggressive buy/sell from observed total/net CWM flow. Accept OI/funding only when their evidence state is available and fresh. Expose inferred liquidation as `squeezeRiskProxy` with inferred lineage; leave observed liquidation notional null until a real source exists.
- [ ] Make advanced metrics unavailable unless their prerequisites are alert-eligible. Do not fuse them into detector score or data quality.
- [ ] Make the backend Discord request/gate consume persisted detector score, persisted data quality, detector confidence, and server-owned provenance. Keep medium/low candidates inbox-only and keep dedupe/cooldown unchanged.
- [ ] Run GREEN with the focused command, then `cargo test -j 1 --test discord_notification_routes_tests --test alert_service_tests` when those test targets exist; otherwise run the equivalent in-module Discord test filter plus `--test alert_service_tests`.

---

### Task 3: Isolate VPIN and markout by normalized symbol and venue

**Files:**
- Modify: `src/toxicity/vpin_bucket_engine.rs`
- Modify: `src/toxicity/vpin_service.rs`
- Modify: `src/types/vpin.rs`
- Modify: `src/market_data/flow_window_service.rs`
- Modify: `src/toxicity/markout_engine.rs`
- Modify: `src/toxicity/markout_service.rs`
- Modify: `src/app.rs`
- Modify: `tests/vpin_bucket_engine_tests.rs`
- Modify: `tests/markout_tests.rs`
- Modify: `tests/flow_state_api_tests.rs`

**Interfaces:**

```rust
impl VpinBucketEngine {
    pub fn new_for_symbol(params: VpinParams, symbol: impl Into<String>) -> Self;
}

impl MarkoutEngine {
    pub fn get_state_for_symbol(
        &self,
        symbol: &str,
        now_ts: i64,
        has_price_index: bool,
    ) -> MarkoutState;
}
```

Preserve `VpinBucketEngine::new` as a fixture-compatible default only. Production `VpinService` must use `new_for_symbol(config.symbol)`. Markout sample IDs include normalized symbol; `MarkoutService` filters configured-symbol trades and resolves through `get_mid_at_or_before_for_symbol`.

- [ ] Add interleaved BTC/ETH and Binance/Bybit tests before implementation. Prove mismatched trades never enter a configured-symbol VPIN bucket, same trade ID on different symbols does not collide, and ETH future mids cannot resolve BTC samples.
- [ ] Add per-venue VPIN assertions computed only from venue contributions inside the symbol-scoped lookback. Keep z-score and percentile as the primary context, with fixed thresholds only as secondary reason codes.
- [ ] Run RED:

```powershell
cargo test -j 1 --test vpin_bucket_engine_tests --test vpin_service_tests --test markout_tests --test flow_state_api_tests
```

- [ ] Store the normalized configured symbol in the VPIN engine/service, reject mismatches before bucket mutation, preserve the bucket symbol, and expose per-venue relative VPIN.
- [ ] Include symbol in markout sample IDs and filter summaries by symbol. Update replay call sites to request their explicit symbol without changing replay semantics.
- [ ] Run GREEN with the focused command, then run `cargo test -j 1 --test replay_runner_tests --test active_trade_toxicity_tests` when available.

---

### Task 4: Calibrate volatility outcomes without future leakage

**Files:**
- Modify: `src/contract_whale_monitor/outcome_calibration.rs`
- Modify: `src/storage/contract_whale_repo.rs`
- Modify: `src/storage/migrations.rs`
- Modify: `tests/contract_whale_monitor_tests.rs`
- Modify: `tests/contract_whale_persistence_tests.rs`

**Interfaces:**

Add nullable fields to `ContractWhaleSignalOutcome`:

```rust
pub absolute_return_30s_bps: Option<f64>,
pub absolute_return_2m_bps: Option<f64>,
pub absolute_return_5m_bps: Option<f64>,
pub realized_volatility_5m_bps: Option<f64>,
pub max_absolute_excursion_5m_bps: Option<f64>,
pub liquidity_recovery_5m_bps: Option<f64>,
pub setup_outcome: Option<String>, // continuation | reversal | unclear
```

- [ ] Add tests first proving a large volatility expansion is a successful measured outcome even when directional follow-through is false, a flat path has low realized volatility, mixed/unknown direction can still produce direction-free volatility outcomes, and no future value is available before its horizon.
- [ ] Add repository round-trip tests for old rows and v2 rows. The migration must be additive/idempotent and preserve v1 data.
- [ ] Run RED:

```powershell
cargo test -j 1 --test contract_whale_monitor_tests --test contract_whale_persistence_tests
```

- [ ] Evaluate direction-free returns before requiring a directional classification. Calculate realized volatility from consecutive log returns, maximum absolute excursion from entry, and recovery as the decline from peak absolute excursion to the 5-minute absolute return. Keep signed markout and continuation flags secondary.
- [ ] Change the version to `v2_volatility_shadow`, add nullable SQLite columns through the existing migration helpers, and update upsert/query mappings without deleting or rewriting historical rows.
- [ ] Prove with a scoring regression that none of the new outcome fields is referenced by detection-time builders.
- [ ] Run GREEN with the focused command.

---

### Task 5: Make the operator UI preserve failures, lineage, and runtime truth

**Files:**
- Modify: `toxic-order-monitor/src/api/signals.js`
- Modify: `toxic-order-monitor/src/api/liquidationCascade.js`
- Modify: `toxic-order-monitor/src/api/alertGate.js`
- Modify: `toxic-order-monitor/src/store/signalsStore.js`
- Modify: `toxic-order-monitor/src/components/Dashboard.jsx`
- Modify: `toxic-order-monitor/src/components/Header.jsx`
- Modify: `toxic-order-monitor/src/components/Sidebar.jsx`
- Modify: `toxic-order-monitor/src/components/TofMetricsPanel.jsx`
- Modify: `toxic-order-monitor/src/components/PerpTofPanel.jsx`
- Modify: `toxic-order-monitor/src/components/AdvancedTofPanel.jsx`
- Modify: the existing liquidation-cascade page component found by `rg "fetchLiquidationCascade" toxic-order-monitor/src`
- Modify: `toxic-order-monitor/src/tests/SignalsApi.test.js`
- Modify: `toxic-order-monitor/src/tests/Store.test.js`
- Modify: `toxic-order-monitor/src/tests/WorkspaceShell.test.jsx`
- Modify: `toxic-order-monitor/src/tests/TofMetricsPanel.test.jsx`
- Modify: `toxic-order-monitor/src/tests/PerpTofPanel.test.jsx`
- Modify: `toxic-order-monitor/src/tests/AdvancedTofPanel.test.jsx`
- Modify: `toxic-order-monitor/src/tests/SignalInboxCardDisplay.test.js`
- Create: `toxic-order-monitor/src/tests/LiquidationCascadeApi.test.js`

**Interfaces:**

```js
export async function fetchSignalsSnapshot() {
  return {
    signals: [],
    request: { phase: "ready" | "error", source: "backend" | "cache" | null, errorCode: null, fetchedAtMs: 0 },
    runtime: { phase: "confirmed" | "unavailable", readOnly: null, monitoringStarted: null, executionEnabled: null, checkedAtMs: 0 },
  };
}
```

Keep `fetchSignals()` as a compatibility wrapper. The store merges/stales candidates only when `request.phase === "ready"`; an error preserves the previous live/stale state and exposes a request error.

- [ ] Update tests first to prove: exact `riskScore` survives; missing score has no severity fallback; 401/403/404/500/network/malformed responses are errors distinct from a successful empty list; a failed refresh preserves existing candidates; runtime failure shows `RUNTIME UNKNOWN`; null metrics render `不可用`; stale/inferred badges are visible; liquidation failure never renders CALM/NEUTRAL/ACCUMULATION or fake zeroes; acknowledged/false-positive items are excluded from the unhandled count.
- [ ] Run RED:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/SignalsApi.test.js src/tests/Store.test.js src/tests/WorkspaceShell.test.jsx src/tests/TofMetricsPanel.test.jsx src/tests/PerpTofPanel.test.jsx src/tests/AdvancedTofPanel.test.jsx src/tests/SignalInboxCardDisplay.test.js src/tests/LiquidationCascadeApi.test.js
```

- [ ] Implement `fetchSignalsSnapshot`, runtime-state storage, and error-preserving merge behavior. Do not persist transient request/runtime errors.
- [ ] Remove severity and quality-bucket numeric fallbacks from alert inputs. Require backend-confirmed read-only runtime, execution disabled, authoritative score/data quality, and alert-eligible lineage before any push request.
- [ ] Preserve nullable metrics in all normalizers; never call `Number(null)`. Render a common provenance/freshness badge and `不可用` for missing values. Label inferred squeeze/liquidation context as a proxy that does not participate in Discord.
- [ ] Make the liquidation API return `data: null` plus an explicit unavailable state on transport/parse/server failure; retain last-success age when available. Only a fresh successful zero-risk response may display calm/neutral.
- [ ] Render backend runtime fields in Header/Sidebar and show unknown/conflict states visibly. Keep the liquidation route off the sidebar as before.
- [ ] Run GREEN with the focused command, then the complete frontend suite.

---

### Task 6: Integration review, release, server synchronization, and rollback evidence

**Files:**
- Verify: all files above
- Update only if behavior changed: `README.md` or tracked deployment documentation

- [ ] Run format and focused regression after integrating every task:

```powershell
cargo fmt --check
cargo test -j 1 --test metric_provenance_tests --test tof_metrics_tests --test oi_metrics_tests --test advanced_tof_metrics_tests --test vpin_bucket_engine_tests --test markout_tests --test contract_whale_monitor_tests --test contract_whale_persistence_tests
npm --prefix toxic-order-monitor test -- --run src/tests/SignalsApi.test.js src/tests/Store.test.js src/tests/WorkspaceShell.test.jsx src/tests/LiquidationCascadeApi.test.js
```

- [ ] Run the complete local gate with fresh output:

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -j 1 --all-targets
npm --prefix toxic-order-monitor test
npm --prefix toxic-order-monitor run build
npm --prefix toxic-order-monitor audit --audit-level=high
docker compose config
git diff --check
git status --short
```

- [ ] Perform an independent code review of the complete diff. Fix every confirmed correctness/security finding through a new failing regression test, then rerun the affected focused gate and the full gate.
- [ ] Commit the verified implementation on `codex/tof-data-integrity`, fast-forward local `main`, and push `main` only after all checks pass. Never include `.env`, runtime databases, captures, logs, or build output.
- [ ] Before deployment, record the server's current commit plus `toxic-bot`/`toxic-frontend` image, StartedAt, RestartCount, and health status without printing environment variables.
- [ ] On `/opt/toxic-order-monitor-rs`, run `git pull --ff-only`, `docker compose build backend frontend`, and `docker compose up -d --no-deps backend frontend`. Do not prune Docker or restart unrelated services.
- [ ] Verify the deployed commit, `docker compose ps`, `http://127.0.0.1:8000/healthz`, `readyz`, the runtime safety/status payload, signal inbox payload, public frontend/API route, WebSocket handshake or snapshot, and absence of new startup error bursts. Confirm runtime remains monitoring-only and execution-disabled.
- [ ] If any health or contract check fails, stop rollout, capture the non-secret evidence, revert with a normal commit to the recorded pre-deploy revision, rebuild only backend/frontend, and repeat health verification.

## Definition of done

- Observed/calculated/inferred/unavailable values are distinguishable on every relevant transport and UI surface.
- Synthetic measurements no longer affect risk, confidence, quality, direction, or Discord eligibility.
- BTC/ETH and venue inputs cannot contaminate VPIN or markout state.
- Volatility-centric outcome calibration is stored independently from detection-time scoring.
- All local gates and independent review are green.
- The verified commit is running on the server with read-only/execution-disabled evidence and a documented rollback point.
