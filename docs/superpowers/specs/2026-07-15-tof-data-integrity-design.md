# TOF Data Integrity And Alert Isolation Design

**Status:** Approved by the user on 2026-07-15 through the instruction to implement the previously recommended approach and synchronize the completed change to the server.

## Objective

Make the toxic-order monitoring surfaces distinguish observed market evidence from calculated, inferred, and unavailable values. Remove synthetic TOF, order-book, open-interest, funding, and liquidation values from production risk and alert decisions. Prevent cross-symbol contamination in the core VPIN and markout paths while preserving the read-only monitoring boundary.

## Safety Boundary

- The product remains monitoring-only and analysis-only.
- No order placement, cancellation, signing, wallet, payment, withdrawal, or fund movement path is added.
- Server-local secrets and `.env` contents are never read into reports, committed, printed, or copied.
- Existing server notification mode is preserved. This change must not enable a disabled or dry-run notification path.
- Medium and low candidates remain in the inbox only. External alerts still require High/Critical severity, score at least `80`, and data quality at least `70`.
- Missing, stale, inferred, or symbol-mismatched evidence fails closed and cannot raise an alert score.

## Approaches Considered

### 1. Hide all TOF enrichment

This immediately removes false precision but discards the real VPIN, flow, sweep, and CWM evidence already available in the runtime.

### 2. Provenance-first compatibility layer (selected)

Keep the existing response field names so current clients continue to parse the payload. Add an explicit lineage object to each metric family and make unavailable numeric values serialize as `null`. Build metrics only from current symbol-matched runtime state or CWM evidence. Keep risk, hazard, and direction as separate concepts.

This is the smallest production-safe change that preserves useful real data and prevents inferred values from silently affecting alerts.

### 3. Replace the complete signal and scoring model

A new event schema and scoring engine would offer cleaner long-term boundaries, but it would touch unrelated monitoring families, persistence, replay, and historical API contracts. It is too risky for this correction.

## Metric Contract

Every TOF, perpetual-market, and advanced metric group carries:

- `provenance`: `observed`, `calculated_from_observed`, `inferred`, or `unavailable`;
- `available`: whether the required source evidence exists;
- `fresh`: whether the observation is within the configured freshness window;
- `observedAtMs`: the source timestamp when available;
- `source`: a stable source label such as `vpin_service`, `flow_window_service`, or `contract_whale_monitor`;
- `unavailableReason`: a stable fail-closed reason;
- `alertEligible`: true only for fresh observed or calculated-from-observed evidence.

Numeric values with no source are `null`, never synthetic zeroes. The application may retain pure classifier helpers for observed fixture tests, but production builders must not create market measurements from severity, direction, confidence, summary text, or candidate labels.

## Risk, Hazard, And Direction

The response exposes three independent concepts:

1. `riskScore`: the detector's persisted score. It is the only short-term score eligible for the existing external-alert gate.
2. `toxicityHazardScore`: a nullable score calculated from fresh observed flow, relative VPIN, spread/depth, and sweep evidence. It describes adverse-selection or volatility risk and does not predict direction.
3. `directionContext`: detector/CWM direction, confidence, and source. It is not derived from VPIN alone.

The optional setup classification is `continuation`, `reversal`, or `unclear`. Unavailable TOF or perpetual metrics cannot raise `riskScore`, `dataQuality`, confidence, or Discord eligibility.

## Observed Data Wiring

### TOF

- Flow imbalance and trade rate come from the symbol-filtered `FlowWindowService` state.
- VPIN, z-score, percentile, bucket count, and per-venue VPIN come from `VpinService`.
- Depth withdrawal, spread widening, and sweep state come from symbol-matched flow/sweep snapshots only when books are available.
- Old inbox candidates receive a current metric snapshot only when source and candidate timestamps satisfy the freshness contract; otherwise metrics are unavailable.

### Perpetual context

- OI and funding use only available CWM evidence for the same normalized symbol.
- Aggressive buy/sell volume is calculated from observed CWM total and net flow.
- Liquidation values are exposed only when the CWM evidence status is observed. Inferred liquidation remains explicitly inferred and cannot participate in alert scoring.
- Without the required evidence, individual values are `null` and the group explains why it is unavailable or incomplete.

### Advanced metrics

Advanced values are calculated only from alert-eligible observed TOF and perpetual inputs. If their prerequisites are missing, the advanced group is unavailable and its score is `null`. It never substitutes confidence for freshness or completeness.

### Relative VPIN and outcome calibration

- Rolling VPIN percentile and z-score are the primary toxicity context. Fixed high/extreme thresholds remain secondary guardrails and must not be presented as universal market constants.
- Calibration evaluates the claim TOF actually makes: volatility expansion. Primary outcomes are absolute return, realized volatility, maximum absolute excursion, and liquidity recovery. Directional markout remains a secondary continuation/reversal label.
- Detection-time evidence and future outcome measurement stay separate. A future markout cannot be used as if it were known at the candidate timestamp.

## Symbol Isolation

- `VpinService` creates a VPIN engine scoped to the configured normalized symbol and rejects trades for other symbols.
- VPIN bucket IDs and stored buckets retain the symbol. Per-venue VPIN is calculated independently from venue contributions inside the symbol-scoped lookback.
- `MarkoutService` accepts only configured-symbol trades and resolves future mids through the symbol-specific price index.
- Markout sample dedupe keys include symbol.
- Flow and sweep computations continue to use the existing symbol-filtered rolling-window path.
- A mismatched runtime state is reported as unavailable rather than relabeled as the requested symbol.

## API And Alert Flow

- Inbox items preserve the persisted detector `toxicityScore` and numeric `dataQuality` instead of reconstructing them from severity or quality buckets.
- HTTP, WebSocket, automatic Discord evaluation, and manual Discord evaluation use the same metric and alert input contract.
- The backend alert request uses the persisted detector score, persisted data quality, and detector confidence. It does not use TOF, perpetual, or advanced synthetic/fused values.
- The frontend gate refuses a push when the runtime boundary is unknown, execution safety fields conflict, or the score/data-quality source is unavailable.
- Existing dedupe and cooldown behavior remains unchanged.

## Operator UI

- Metric panels show a source badge and freshness state.
- `null` is rendered as `不可用`, never as `0`.
- Labels distinguish `毒性/波动风险` from `方向上下文`.
- The liquidation panel reports transport or data failure as unavailable and retains last-success age where available; it must not turn failure into `CALM`, `NEUTRAL`, or `ACCUMULATION`.
- Header and sidebar render the backend-provided runtime boundary. If it cannot be loaded, they display `RUNTIME UNKNOWN` instead of hard-coded `READ ONLY`.
- Stale signals remain visibly stale. Acknowledged and false-positive review states are not counted as unhandled.

## Compatibility

- Existing top-level field names remain available wherever practical.
- Numeric metric fields become nullable and gain lineage metadata.
- Frontend normalizers accept both the new nullable contract and older payloads during a rolling deployment.
- No database migration is required for metric provenance. Persisted detector score and data quality already exist in the source signal.

## Test Strategy

Backend behavior tests must prove:

- no L2 evidence produces no depth or spread number;
- no OI/funding evidence produces no OI/funding number;
- inferred-only evidence cannot raise risk or pass an external-alert gate;
- BTC and ETH trades cannot share VPIN buckets, markout summaries, or price resolution;
- per-venue VPIN remains symbol-scoped;
- relative VPIN percentile/z-score remains primary while fixed thresholds remain secondary;
- observed, fresh, same-symbol evidence produces calculated metrics;
- persisted detector score and data quality survive inbox and WebSocket mapping unchanged;
- stale or mismatched observations fail closed.
- volatility outcomes can succeed even when directional continuation is false;
- future outcome fields do not enter detection-time scoring.

Frontend behavior tests must prove:

- `null` metrics render as `不可用`;
- provenance and freshness badges are visible;
- network failure renders unavailable, not calm/neutral market state;
- runtime safety labels reflect backend values and show unknown on failure;
- stale/reviewed candidates are visible and excluded from the unhandled count as appropriate;
- medium/low or unavailable-source signals cannot be pushed.

The final local gate is:

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -j 1 --all-targets
npm --prefix toxic-order-monitor test
npm --prefix toxic-order-monitor run build
npm --prefix toxic-order-monitor audit --audit-level=high
docker compose config --quiet
git diff --check
```

## Deployment And Rollback

After local verification and review, commit and push the verified result. On `/opt/toxic-order-monitor-rs`, record the pre-deploy commit and container state, run `git pull --ff-only`, rebuild both backend and frontend, and recreate only those services. Verify the deployed commit, `healthz`, `readyz`, container health, runtime monitoring state, safety fields, signal inbox, WebSocket/public routes, and absence of startup error bursts.

Rollback uses the recorded pre-deploy commit with a normal revert or fast-forward-safe rollback commit, followed by rebuilding the same two services. Runtime data, server-local `.env`, tokens, and replay inputs are preserved.
