# BTC/ETH Contract Monitor Reliability v1

## Goal

Upgrade contract-whale monitoring so lifecycle volume and status are truthful, OI evidence is exchange-aligned and batched, evidence gaps fail closed, notification delivery cannot block signal production, and API failure is distinguishable from an empty result.

## Source Specification

The complete user-provided specification is stored at:

`C:\Users\byhdo\.codex\attachments\ecd8c189-b2ce-490e-ad13-216ccdf0eba7\pasted-text-1.txt`

It is the authoritative source for phases, safety limits, compatibility requirements, feature flags, tests, rollout, and final reporting.

## Requirements

1. Complete Milestone 1 first: live/replay lifecycle clocks, non-overlapping lifecycle volume semantics, latest/peak separation, OI per-exchange as-of resolution, and batched enrichment.
2. Complete evidence-safe severity and semantic shadow comparison before changing exposure behavior.
3. Decouple producer execution from Discord with an SQLite outbox and add non-overlap/missed-tick handling plus emission watermarks.
4. Add concurrent market-context polling, OKX metadata/fallback evidence, API degraded-vs-empty semantics, indexes, and performance diagnostics.
5. Add shadow-only outcome calibration after correctness and production-chain milestones are stable.
6. Provide compatibility fields and feature flags for every new behavior.

## Safety Boundaries

* Do not relax Medium, High, Critical, or S numeric thresholds.
* Do not change B/A/S Discord impact-gate product rules.
* Do not remove historical API fields, retention behavior, or introduce trading/execution infrastructure.
* Never treat missing dynamic, percentile, OI, funding, ctVal, or multi-exchange evidence as a pass.

## Acceptance Criteria

* [ ] All 15 mandatory criteria in the source specification Phase 12 are proven by targeted tests or direct runtime evidence.
* [ ] Required backend and frontend validation commands pass, or unrelated pre-existing failures are documented with evidence.
* [ ] All new production behavior is guarded by compatible configuration switches.
* [ ] Shadow rollout observations and remaining risk are recorded in the final delivery report.

## Technical Notes

* Existing changes in spot-whale modules, Compose, deployment, tests, and `.trellis` are user-owned and must be preserved.
* The contract-whale pipeline includes modules outside the initial code bundle, notably `scoring.rs`, `merge.rs`, `event_quality.rs`, `normalizer.rs`, API registration, migrations, and front-end tests.
