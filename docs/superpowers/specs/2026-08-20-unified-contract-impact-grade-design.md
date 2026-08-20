# Unified Contract Impact Grade Design

## Objective

All enabled contract-market monitoring must expose one authoritative impact grade: `C`, `B`, `A`, or `S`. Short-window flow, lifecycle accumulation, observed liquidation, price response, open interest, source confirmation, Discord, API projection, UI, and retention must use that same grade assessment. No second shock grade or cohort letter grade may compete with it.

The monitor remains read-only and analysis-only. This work must not add order placement, signing, wallet access, trading execution, or any other money-moving path.

## Current Problems

The current implementation has five defects that prevent the grade from representing major market events correctly:

1. Binance and OKX liquidation collectors are defined but are not started by the application, so production contract-whale events contain zero observed liquidation.
2. The detector passes the same dynamic flow multiple as both `impact_score` and `impact_z_score`; the displayed z-score is not a statistical z-score.
3. The sanitizer only preserves a raw S that already has hard evidence. It cannot promote an A or B event to S when later lifecycle evidence proves an extreme liquidation shock.
4. Cross-exchange confirmation is tied to a Binance-plus-Bitfinex profile whose fixed Bitfinex contribution floors are practically unreachable in current BTC traffic.
5. The public model exposes both a canonical letter grade and a cohort-relative letter grade, which violates the requirement for one standard.

## Considered Approaches

### 1. One hierarchical hard-gate engine — selected

Aggregate all admissible evidence into one assessment, then evaluate the grade in the order S, A, B, C. Each higher grade has explicit mandatory evidence. A large value in one metric cannot compensate for missing hard evidence.

This approach is selected because it is deterministic, auditable, fail-closed, and directly addresses the production defects.

### 2. Weighted score mapped to letters — rejected

This would combine flow, price, liquidation, OI, and source quality into a 0–100 score. It is compact but permits a very large flow multiple to offset unavailable liquidation or confirmation data. That failure mode already exists in the current system.

### 3. Grade each evidence lane and take the maximum — rejected

This is simple to implement but retains several competing standards internally. A noisy lane can promote the final result without satisfying the complete evidence contract.

## Canonical Grade Unit

The canonical grade unit is a contract-impact episode, not an individual overlapping 5-second, 15-second, or 60-second window.

- An episode starts when the existing contract-whale detector emits a qualifying event.
- Evidence is merged only when the symbol and direction match.
- A same-direction event extends the episode when it arrives no more than 15 minutes after the previous evidence.
- One episode can contain at most 60 minutes of evidence. Evidence after that boundary starts a new episode.
- An episode finalizes after 15 minutes without matching evidence or when the 60-minute maximum is reached.
- Raw one-second trade and liquidation buckets are deduplicated by their existing exchange, symbol, and timestamp identity. Overlapping 5/15/60-second window totals must never be summed.
- The episode identifier is deterministic from grade version, symbol, direction, and first source-event identifier so replay and restart produce the same identity.

While an episode is active, its canonical grade may only move upward as additional observed evidence arrives. Finalized episodes remain stable for their recorded grade version. A later grade implementation uses a new version rather than silently rewriting the historical assessment.

## Unified Evidence Contract

Every assessment must carry these typed fields:

- grade version and assessment state;
- symbol, direction, episode start/end, and source event identifiers;
- data quality and explicit degradation reasons;
- unique observed contract turnover in USD;
- peak same-direction price movement;
- robust historical percentile and robust z-score;
- observed long- and short-liquidation USD totals by exchange;
- liquidation source count and source freshness;
- trade-flow confirmation source count;
- OI direction, OI freshness, and OI confirmation state;
- ordered reason codes explaining the final grade.

Base-asset quantity remains a diagnostic field. The grade uses USD notional for absolute scale so BTC, ETH, and future enabled contract symbols share the same business standard. Historical normalization is always keyed by symbol and compatible source/window profile.

Missing, stale, inferred, non-finite, or degraded data never becomes zero-valued positive evidence. Inferred liquidation can be displayed as context but cannot raise the grade.

## Real Historical Normalization

The grade engine must stop using `dynamic_multiple` as a z-score.

- Build the baseline from finite, positive observed USD-notional samples for the same symbol and compatible source profile.
- Use a 90-day lookback.
- Require at least 10,000 samples.
- Apply natural-log transformation to notional, then calculate median and median absolute deviation.
- Calculate `robust_z = (ln(current) - median) / (1.4826 * MAD)`.
- Calculate empirical percentile from the sorted sample distribution.
- A zero/non-finite MAD or insufficient sample count produces `baseline_insufficient`; it does not silently fall back to dynamic multiple.
- The existing dynamic multiple remains diagnostic only and cannot satisfy a grade gate.

## One Grade Standard

The engine evaluates the following rules from highest to lowest. Equality at a threshold passes. Every numeric input must be finite.

### S — systemic contract-market shock

Common gates:

- data quality at least 85;
- peak same-direction price movement at least 2.0%;
- at least two fresh observed contract-market sources;
- no critical evidence-degradation reason.

In addition to the common gates, one of these hard-evidence paths must pass:

1. Observed liquidation path: cumulative observed liquidation notional within the episode is at least 250,000,000 USD.
2. Extraordinary turnover path: deduplicated observed turnover is at least 1,000,000,000 USD, robust percentile is at least 99.9, and robust z-score is at least 6.0.

S is confirmed only when the required evidence is observed. One-source evidence is capped at A. Price movement, OI, percentile, dynamic multiple, or inferred liquidation alone can never produce S.

### A — rare major impact

All gates must pass:

- data quality at least 80;
- robust percentile at least 99.5;
- robust z-score at least 4.0;
- peak absolute price movement at least 0.5%;
- deduplicated observed turnover at least 150,000,000 USD;
- at least one independent confirmation path.

Independent confirmation means one of:

- fresh same-direction trade flow from at least two contract exchanges;
- fresh observed liquidation aligned with the price direction;
- confirmed OI/price behavior using fresh OI evidence.

### B — material local impact

All gates must pass:

- data quality at least 70;
- robust percentile at least 99.0;
- robust z-score at least 2.5;
- peak absolute price movement at least 0.15%;
- deduplicated observed turnover at least 50,000,000 USD.

B does not require independent confirmation, but its reason codes must state when confirmation is absent.

### C — ordinary or evidence-insufficient event

Every detected event that does not satisfy B is C. C must distinguish ordinary evidence from unavailable or degraded evidence in its reason codes.

## Source Confirmation

The confirmation algorithm must not depend on the old Bitfinex-specific 20 BTC floor.

A fresh same-direction trade source counts when:

- its data is no more than five seconds old at the assessed window;
- its observed notional is positive;
- its direction matches the episode;
- its notional share is at least 2%;
- its net contribution share is at least 2%;
- its own directional dominance is at least 0.55.

Binance and OKX liquidation streams must be started only when their configured liquidation source is enabled. Each collector uses a bounded channel, reconnects with bounded exponential backoff, aggregates one-second buckets, and persists through the existing nonblocking SQLite path.

Binance `forceOrder` is a latest-order snapshot stream and therefore represents an observed lower bound, not complete exchange liquidation. Its provenance must be recorded as `observed_snapshot`; the application must not claim that this value equals total exchange or total-market liquidation.

## Runtime Health and Failure Handling

Runtime status must expose, per liquidation source:

- configured/enabled state;
- connecting, connected, degraded, or stopped state;
- last message timestamp;
- last persisted bucket timestamp;
- reconnect count;
- last non-sensitive error code;
- data age and freshness classification.

Collector failures must not stop trade-flow monitoring. A disconnected or stale source marks liquidation evidence unavailable and prevents affected promotions. Logs must never contain webhook URLs, tokens, raw environment values, or complete untrusted payloads.

The runtime remains `readOnly=true`, `analysisOnly=true`, and `executionEnabled=false`.

## Canonical Projection and User-Facing Surfaces

The versioned unified assessment is the only source of letter grades.

- Detector/lifecycle persistence stores the canonical assessment and reason codes.
- History and final-event API projections copy `impactLevel`, `signalLevel`, and `signalLabel` from the same assessment.
- Discord eligibility and payloads use the same assessment.
- Retention uses the same assessment.
- The frontend renders only the canonical C/B/A/S grade.
- Cohort diagnostics may expose numerical percentile and robust z-score, but `cohortImpactLevel`, `cohortSignalLevel`, and `cohortSignalLabel` are removed from the public response and frontend.
- Legacy persisted rows remain readable. They are explicitly marked with their legacy grade version and are not silently represented as newly confirmed S events.

S Discord delivery is deduplicated by episode identifier plus grade version. Reprocessing, refresh, restart, filtering, and pagination cannot send the same S episode twice. Existing alert-volume and read-only notification boundaries remain in effect.

## Testing Requirements

Implementation follows test-first red/green cycles. Automated tests must not call real Discord endpoints or depend on external network availability.

Required unit and integration cases:

1. Ordinary flow with a high dynamic multiple but insufficient absolute evidence remains C.
2. Exact B, A, and S boundaries pass; one unit below every boundary fails that grade.
3. A one-source 300M USD liquidation shock is capped at A.
4. A two-source 250M USD liquidation episode with 2% price movement and quality 85 becomes S.
5. A two-source 1B USD unique-turnover episode at percentile 99.9 and robust z 6 becomes S.
6. Missing, stale, inferred, NaN, infinite, or degraded liquidation never promotes a grade.
7. Overlapping 5/15/60-second windows do not multiply turnover or liquidation totals.
8. The same episode identity and grade survive restart and replay.
9. Binance and OKX synthetic WebSocket payloads reach normalized buckets, SQLite persistence, episode evidence, and the final grade.
10. Collector channel pressure remains bounded and does not block the Tokio runtime.
11. API, Discord projection, retention, and frontend all expose the same canonical grade.
12. No public contract response contains a second cohort letter grade.
13. Existing read-only and execution-disabled assertions remain true.
14. Logs and test snapshots contain no tokens, webhook URLs, environment values, or raw payloads.

## Production Rollout

Deployment is a monitored backend-only rollout unless frontend removal of cohort letter fields requires a frontend rebuild.

1. Run focused grade, collector, persistence, API projection, Discord, and retention tests.
2. Run formatting, Clippy with warnings denied, and the full Rust test gate sequentially.
3. Run the affected frontend tests and production build when frontend/API fields change.
4. Commit only task-related files; preserve unrelated user changes.
5. Push the current `codex/` branch.
6. On `/opt/toxic-order-monitor-rs`, fetch and fast-forward to the exact pushed commit.
7. Rebuild and recreate only affected services without pruning Docker or restarting unrelated services.
8. Verify container health, `/healthz`, `/readyz`, read-only flags, exact deployed commit, collector connection states, source freshness, and absence of new startup error bursts.
9. Verify a clearly marked synthetic episode through the internal test path without inserting a fake production market event or triggering trading behavior.
10. Verify subsequent real API output has exactly one canonical grade field set and no contradictory nested/top-level grade.

## Acceptance Criteria

The work is complete only when all of the following are evidenced:

- one versioned function is the sole producer of contract C/B/A/S grades;
- observed liquidation flows from enabled Binance and OKX sources into persisted episode evidence;
- `impact_z_score` is a real robust historical z-score, not dynamic multiple;
- hard S evidence can promote an episode directly to S;
- no unavailable or inferred evidence can promote a grade;
- one-source extreme evidence cannot exceed A;
- overlapping windows cannot double-count evidence;
- API, UI, Discord, and retention agree on the same grade;
- the public UI/API no longer exposes a second cohort letter grade;
- tests, lint, and builds pass;
- the server runs the exact pushed commit, remains healthy and read-only, and reports live collector health without exposing secrets.
