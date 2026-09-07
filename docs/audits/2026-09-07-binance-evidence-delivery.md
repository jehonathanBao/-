# Binance-only main-force evidence upgrade

## Scope and safety

Implements the approved main-force perspective audit without adding exchanges. Production remains Binance-only and read-only. No trade execution, new webhook, historical alert replay, or activation of sustained-flow notifications is introduced. Existing runtime credentials and enabled/dry-run settings are preserved.

## Changes

1. Configured Binance-only deployments can classify directional behavior and absorption candidates. An intentionally absent second venue is not treated as a failed venue. Missing configured venues still cannot silently shrink the evidence requirement. Public flow does not establish account identity; `isStrongMainForceIntent` remains false for these single-venue candidates.
2. Canonical importance moves to `cwm_impact_v3_4`. Exceptional directional flow can establish materiality without the old 0.10% price discontinuity; absolute amount, causal baseline, quality and S hard-evidence requirements remain. Flow-only importance is explicitly not proof of passive absorption.
3. Confirmed canonical A/S replaces legacy detector score/severity only in the contract V3 notification path. Discord also displays that canonical grade. Quality, warm-up, enabled/dry-run, deduplication and cooldown remain; sustained candidates remain blocked.
4. The existing Binance trade/top-20 book bus supplies bounded passive-execution evidence. Same visible prices, actual executions and replenishment must align; pure cancellation, balanced flow, duplicate fills, unknown levels, invalid books and feed gaps do not establish support. Disconnect/receiver loss resets evidence. No extra connector or dependency was added.
5. Sustained flow adds a low-participation route for 15/60-minute windows: at least 15 complete independent minutes, 95% bin coverage, 90% direction consistency, 3% net participation, at least 2 percentage points above prior normalized P95, and raw-net anomaly percentile at least 97.5. At least 60 prior bins are required. The existing higher-participation route is retained. These are initial heuristic thresholds, not calibrated detection probabilities.
6. Compatible sustained candidates share a bounded process identity across horizon/clock changes. Direction/hypothesis changes, over three minutes of inactivity, or four hours of process age split the identity. Overlapping volume is never added. Restart requires fresh windows; a process is anonymous flow continuity, not a recovered parent order.
7. Short liquidation contributes buy pressure, long liquidation sell pressure, and unknown/mixed flow is not labeled manipulation or stop hunting. OI/position language explicitly describes candidates. The UI shows passive/continuous-flow evidence separately from the sole canonical grade.
8. Retention preserves the previously deployed V3.3 confirmed A/S/B tiers and original archival version/evidence during the version upgrade. Display and notification queries still use the current canonical version; historical retention compatibility cannot authorize an old alert.

## Passive support boundaries

- At most BTC and ETH, five minutes of paired-book history, bounded pending fills and deduplication state.
- Adjacent valid snapshots must be no more than 250 ms apart. Each contributing level must be visible in both snapshots; a disappeared top-20 level is unknown.
- Support requires at least 90% snapshot interval coverage, three independent 10-second bins spanning at least 20 seconds, 65% directional flow, $1M matched executions and at least 25% matched replenishment.
- Missing, late or ambiguous trade/book matches fail closed. Short windows and old/restarted histories may therefore lack support even when a human suspects absorption. Evidence is event-time aligned and persisted; today's book cannot prove yesterday's event.
- Binance aggregate trades and public L2 remain anonymous and incomplete. No claimed real-account recall rate, trading win rate, hidden-order size or proof of coordinated actors is provided.

## Verification and release gate

The regression suite covers single/missing venue modes; grade boundaries; balanced churn; canonical delivery with low legacy rank; passive bid/ask symmetry, duplicates, invalidation and feed loss; low-participation baseline controls; process identity and non-additive volume; liquidation side; persistence and historical retention. Frontend full-suite and production-build results, backend suite totals and deployed revision are recorded in the release handoff after execution.

Pre-release verification on 2026-09-07: 371 backend tests passed (166 library tests plus nine affected integration suites, including risk fusion); 348 frontend tests across 41 files passed; frontend production build passed; touched Rust files passed formatting checks and the diff passed whitespace checks. No live Discord message was sent by these tests. Vite reports the existing large-chunk advisory; this does not prevent the build.

Deployment uses the tested commit, consistent SQLite backup, retained previous images, unchanged environment configuration and explicit image source-revision labels. No backfill executable is run. Health/readiness and the public UI build revision must be checked after replacement.

During local validation, a concurrent Windows Rust build exhausted virtual memory (`os error 1455`). Validation was rerun with `-j 1`; no machine-wide settings were changed. This infrastructure failure must not be counted as a passed test run.
