# Binance Main Force Evidence Implementation Plan

> **For agentic workers:** Use executing-plans inline, task by task. User approved the preceding audit and requested implementation and server synchronization; do not reopen design approval or delegate recursively.

**Goal:** Improve public BTC/ETH contract behavior detection while preserving Binance-only production sources, a single canonical S/A/B/C grade, and read-only monitoring.

**Architecture:** Reuse the existing trade/book bus, event enrichment, persistence, canonical grade and delivery pipeline. Add bounded causal passive-execution evidence and sustained-flow process tracking; never claim account identity. Existing raw score remains diagnostic, not a second gate for canonically confirmed A/S events. Newly detected sustained candidates remain shadow/display-only until a separately accepted real-data evaluation supports activation.

**Tech Stack:** Existing Rust/Tokio/Axum/SQLite and React/Vitest; no new dependencies.

## Global Constraints

- Continue on the existing `codex/main-force-behavior-v4` implementation branch; preserve unrelated untracked files. Do not create new tasks or worktrees for this continuation.
- Do not enable OKX, Bybit, Bitfinex or other production sources. Source checks use configured eligible sources, never silently shrink to whichever source happens to be healthy.
- No exchange execution, credential changes, new webhook, historical notification replay or live simulated signals.
- Keep existing operational enabled/dry-run, fresh-evidence, data quality, deduplication and cooldown safeguards. Canonical confirmed A/S replaces legacy score/severity only in the contract V3 alert path, as approved in audit P1-3; other toxic alert gates remain unchanged.
- No destructive database migration; new signal evidence is optional/serde-defaulted and historical missing evidence stays unknown.
- No changes to `.env` or tracked runtime data. Build/release only after tests; retain rollback images and check actual image source revision.

## Task 1 — Regression-first classification, grading and delivery

Files: `classification.rs`, `impact_grade.rs`, `discord_notifier.rs`, corresponding contract monitor / grade / notifier tests, `config/default.toml`.

- [ ] Add failing tests for configured Binance-only absorption candidates, missing configured venue, price missing, 0.099/0.100/0.101% grade discontinuity, and canonical A/S with low legacy score/severity.
- [ ] Preserve distinct meanings: event materiality may come from exceptional directional flow even without a price move; this is not proof of passive absorption. Keep absolute materiality, baseline and S hard-evidence floors. Version the revised grade.
- [ ] Implement source-mode-aware candidate classification and canonical-only importance gating while preserving operational rejection checks and sustained candidate suppression.
- [ ] Run narrow Rust tests and inspect failures before broadening verification.

Expected boundary assertions:

```rust
assert_eq!(result.classification_v2.structure_interpretation,
    ContractWhaleStructureInterpretation::DownsideAbsorption);
assert_eq!(assess_contract_impact_episode(&episode, &config, now).grade,
    ContractEventImpactGrade::A);
assert!(evaluate_contract_whale_discord_v3_gate(
    &settings, &signal, &assessment, &cooldown, now).allowed);
```

## Task 2 — Bounded Binance passive execution evidence

Files: new `src/contract_whale_monitor/passive_execution.rs`; existing `mod.rs`, `types.rs`, `app.rs`, `behavior_assessment.rs`, `src/api/contract_whale_routes.rs`; focused tests.

- [ ] Add synthetic tests before implementation: matched sell fills plus repeated bid replenishment, mirrored ask case, pure cancellation, missing/truncated levels, duplicate trades, late/future data, disconnected/lagged bus and normal balanced market making.
- [ ] Consume the existing Binance trade and top-20 book stream, retaining only a bounded recent history. Match executions to same visible price levels; missing prices/levels/continuity produce no replenishment evidence. Bound memory and per-event work.
- [ ] Attach event-time evidence during live production enrichment and persist with the signal. Never attach current books to historical signals. Keep supporting evidence separate from account identity and distinguish active-flow direction from passive-side hypothesis.
- [ ] Require repeated independent observations and adequate directional execution/coverage for support; no new external notifications from this evidence alone.

Interface: `PassiveExecutionService::new(bus)`, `start()`, `stop()`, `evidence(symbol, at, window_sec) -> PassiveExecutionEvidence`; optional `ContractWhaleSignal.passive_execution` carries version/status/side/coverage/reasons, not a new grade.

## Task 3 — Sustained low-participation evidence and process identity

Files: `sustained_flow.rs`, `app.rs`; unit and replay fixtures.

- [ ] Add failing low-participation persistent-flow, balanced-flow, future, missing-bin, overlap and horizon-transition tests.
- [ ] Preserve current high-anomaly path and add a stricter-duration/consistency path for lower participation. Compare cumulative directional pressure against causal recent baseline; record admission reason and independent minutes. Do not pretend that a parent account/order was identified.
- [ ] Track compatible consecutive candidates as one bounded behavior process across horizon/clock boundaries, split on direction/hypothesis changes or inactivity, retain unique raw-volume accounting. No start-up cache alerts.
- [ ] Keep all new sustained candidates display-only and verify dedupe/retention behavior.

## Task 4 — Safe behavior language and integration validation

Files: `trajectory.rs`, `types.rs`, `behavior_assessment.rs`, `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`, API normalization/tests as needed.

- [ ] Test short squeezes and long liquidation separately; ordinary mixed direction must not imply manipulation.
- [ ] Replace new trajectory intent values with directional liquidation / mixed-flow descriptions; legacy historical labels render conservatively. OI context uses candidate language.
- [ ] Expose concise passive/slow-flow evidence without creating new trading or admin controls; preserve canonical grade display.
- [ ] Run backend affected suites, lint/format checks proportionate to baseline, frontend full tests/build, and bounded synthetic scenario evaluation. Document existing failures separately rather than broad unrelated rewrites.

## Task 5 — Git and server synchronization

- [ ] Review diff and secret safety; commit only task-owned files and push the existing branch.
- [ ] Verify server working tree/base revision, preserve configuration/data and rollback images, fetch and fast-forward to the exact tested revision.
- [ ] Build backend/frontend with explicit source revision; keep old healthy services running during build, then replace only scoped services.
- [ ] Verify readiness/health, running image revisions, public UI build revision, Binance-only active/configured sources and maintained notification safety. Record delivery result and any shadow-only capability limits.
