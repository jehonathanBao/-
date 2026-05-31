# Whale Flow Operator Runbook

Whale Flow is a signal-only monitoring layer.
It detects large public trade-flow candidates and explains their evidence quality.
It does not place orders.
It does not cancel or amend orders.
It does not manage wallets or signing.
It does not trigger live trading.
It does not auto-apply threshold changes.

Whale Flow 只用于巨额成交信号监控、复盘、数据质量解释和阈值证据评估。
它不是交易执行模块，不下单，不撤单，不签名，不自动交易，也不自动修改阈值。

## 1. Purpose

Whale Flow exists to help an operator answer a simple question:

> "Is this large BTC flow worth reviewing, and do we have enough evidence to trust the view?"

Use it for:

- large public trade-flow monitoring
- candidate replay and explanation
- evidence quality review
- calibration readiness review
- bounded in-memory history lookup

Do not use it as a trade instruction engine.

## 2. Safety Boundary

Whale Flow is read-only from the operator perspective.

- `readOnly=true`
- `analysisOnly=true`
- `executionEnabled=false`
- `runtimeModified=false`
- `no order placement`
- `no cancel/amend`
- `no wallet/signing`
- `no live trading`
- `no auto threshold update`
- `no auto calibration`
- `no DB/JSONL/SQLite/archive write`
- `no notification/webhook/Telegram`

If a panel looks operational, remember that it is still a review surface.

## 3. What Whale Flow Detects

Whale Flow looks for large BTC trade-flow candidates across multiple windows:

- `1s`
- `5s`
- `15s`
- `60s`

Default thresholds:

```text
1s  >= 100 BTC
5s  >= 300 BTC
15s >= 800 BTC
60s >= 2000 BTC
directionRatio >= 70%
relativeVolumeMultiple >= 5x
minVenueConfirmations >= 2
```

The monitor combines:

- trade volume
- direction ratio
- relative volume versus history
- venue confluence
- price impact
- depth drop

Important:

- A candidate is evidence of unusual flow, not a buy or sell command.
- Absence of a candidate does not prove there was no large money movement.

## 4. Whale Flow Candidate Types

Candidate types are labels for operator review.

- `aggressive_buy`
- `aggressive_sell`
- `absorption`
- `liquidation_sweep`
- `trap`

Interpretation guide:

- `aggressive_buy` usually means a large buyer-initiated flow was observed.
- `aggressive_sell` usually means a large seller-initiated flow was observed.
- `absorption` usually means one side of the book was repeatedly hit but did not move as expected.
- `liquidation_sweep` usually means a forced sweep or liquidation-like event may have occurred.
- `trap` usually means the flow may have been诱导 or misdirection.

These labels are still signal labels, not execution instructions.

## 5. Thresholds and Evidence Gates

Thresholds answer "did the event qualify as large enough?"
Evidence gates answer "do we trust the conclusion?"

Threshold checks include:

- window size
- BTC volume
- direction ratio
- relative volume multiple
- venue confluence
- price impact
- depth drop

Evidence gates include:

- resolved markout evidence
- not enough data rate
- candidate count floor
- current snapshot only fallback detection

Key rule:

- candidate count is not evidence count
- resolved markout evidence is required
- current snapshot only cannot support threshold tuning
- `needs_more_data` is the safe default when evidence is thin

## 6. Data Quality / Venue Coverage

Do not trust an empty or thin result set without first checking data quality.

Review these fields first:

- `venueCoverage`
- `baselineQuality`
- `dataQuality`
- `noCandidateReasons`
- `degradationWarnings`
- `whyCandidate`
- `missingInputs`
- `confidenceModifiers`

Common meanings:

- no Whale Flow is not the same as no large money move
- no candidate may mean venue confluence is insufficient
- no candidate may mean baseline is insufficient
- no candidate may mean depth data is unavailable
- no candidate may mean flow is below threshold

If the data is degraded, the correct response is to downgrade confidence, not to invent a stronger signal.

## 7. Baseline Source Meaning

Baseline source tells you what the comparison frame came from.

Common values:

- `one_hour_normalized`
- `sixty_second_fallback`
- `longer_window_fallback`
- `insufficient_history`

Meaning:

- `one_hour_normalized` means there was enough history for a normal baseline.
- `sixty_second_fallback` means the system had to fall back to a shorter baseline window.
- `longer_window_fallback` means it had to fall back to a different longer window.
- `insufficient_history` means the historical baseline was too thin.

If the baseline is weak, confidence should be lower.

## 8. Replay Overlay / Heatmap

Replay overlay and heatmap are review tools.

They help the operator inspect:

- `whaleClassification`
- `baselineSource`
- `dataQuality`
- `markout aligned`
- `markout adverse`
- `markout neutral`
- `markout not_enough_data`

Important rule:

- `not_enough_data` must not be treated as `aligned`
- `directionBias` is a signal attribute, not an order instruction

Use replay overlays to understand what happened after the candidate, not to justify an immediate trade.

## 9. Calibration Report

The calibration report explains whether thresholds look trustworthy.

It should be read as evidence, not as an automatic tuning command.

The safe interpretation is:

- review the current thresholds
- review sample quality
- review resolved evidence
- review blocked reasons
- do not auto-apply changes

When the report says `needs_more_data`, that is intentional.

Do not treat `keep`, `raise`, or `lower` style ideas as automatic actions.
Those are only meaningful when resolved evidence gates pass and manual review still agrees.

## 10. Candidate History

Candidate history is bounded in-memory only.

- `retentionMode=in_memory_bounded`
- `currentCandidates`
- `maxCandidates`
- `recordedCount`
- `deduplicatedCount`
- `evictedCount`

Behavior:

- history is stored only in runtime memory
- history is deduplicated by `candidate_id`
- old items may be evicted after `maxCandidates`
- history is lost after restart
- no DB write
- no file write

Do not treat the history layer as durable archive storage.

## 11. Calibration Readiness Badge

The readiness badge answers a smaller question:

> "Do we currently have enough resolved evidence to trust calibration?"

Badge states:

- `READY`
- `NOT READY`

Relevant fields:

- `calibrationReady`
- `resolvedMarkoutEvidenceCount`
- `blockedReasons`
- `currentCandidates`
- `notEnoughDataRate`

Rules:

- `READY` does not mean auto-apply
- `READY` does not mean the operator should stop checking evidence
- `NOT READY` means collect more evidence
- `NOT READY` means manual review is still required

Common blocked reasons:

- `candidate_count_too_low`
- `resolved_markout_evidence_too_thin`
- `not_enough_data_rate_too_high`
- `current_snapshot_only`

## 12. Common Status Meanings

Use these statuses as review hints only:

- `healthy` means the current view is internally consistent.
- `partial` means some evidence exists, but not everything is complete.
- `degraded` means the data path is weaker than normal.
- `no_data` means there is not enough data to draw a strong conclusion.
- `aligned` means the observed outcome matched the expected direction.
- `adverse` means the observed outcome went against the expected direction.
- `neutral` means the observed outcome was flat or inconclusive.
- `not_enough_data` means the outcome cannot be classified strongly.
- `needs_more_data` means the safe default is to wait for more evidence.
- `calibrationReady=false` means the calibration gate is not open.
- `resolved_markout_evidence_too_thin` means too few resolved outcomes exist.
- `not_enough_data_rate_too_high` means too many outcomes are unresolved.
- `candidate_count_too_low` means there are not enough samples yet.

## 13. Recommended Operator Workflow

1. Open `BTC Whale / Large Flow Monitor`.
2. Check `Data Quality` and `Venue Coverage` before trusting candidates.
3. Use `Whale Flow Overlay` to inspect one candidate.
4. Use `Replay Heatmap` to review post-signal behavior.
5. Check `Candidate History` to confirm enough samples exist.
6. Check `Calibration Readiness Badge`.
7. If `NOT READY`, collect more evidence.
8. If `READY`, review the calibration report manually.
9. Do not treat whale flow as a trade instruction.
10. Do not apply threshold changes automatically.

## 14. What Not To Do

Do not trade directly from whale flow.
Do not treat `directionBias` as an order instruction.
Do not treat `aggressive_buy` as a buy signal.
Do not treat `aggressive_sell` as a sell signal.
Do not treat `not_enough_data` as `aligned`.
Do not apply threshold notes without manual review.
Do not assume `calibrationReady` means auto-apply.
Do not persist whale candidate history unless a separate durable archive task is approved.
Do not send alerts, webhooks, or Telegram from this layer.

## 15. Future Extensions

Future work should stay inside the signal-only boundary unless a separate card explicitly opens a write path.

Reasonable next steps:

- richer operator documentation
- notification preview policy refinement
- replay explanation improvements
- calibration evidence reporting improvements

If durable archival is ever needed, it should be introduced as a separately reviewed design and audit track.
