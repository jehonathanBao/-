# Toxic Signal Operator Runbook

## 1. System Scope

This system is signal-only.
It provides monitoring, signal review, evidence replay, quality digest, alert preview, and bounded in-memory history.
It does not place orders.
It does not cancel or amend orders.
It does not manage wallets or signing.
It does not trigger live trading.
It does not auto-apply parameter or weight changes.

本系统只做信号监控、复盘、质量摘要、告警预览和内存历史查询。
不做下单，不撤单，不管理钱包，不签名，不自动交易，不自动落参。

The current operator surface covers:

- `S1` Signal Inbox
- `S2` Signal Groups
- `S3` Signal Detail / Evidence Timeline
- `S4` Signal Symbol Filter
- `S5` Daily Report / Quality Digest
- `S6` Alert Preview
- `S7` Signal History

## 2. Safety Boundary

The signal system is intentionally constrained by a read-only operator boundary.

- `readOnly=true`
- `runtimeModified=false`
- `analysisOnly=true`
- `executionEnabled=false`
- No order placement
- No cancel/amend
- No wallet/signing
- No transaction construction
- No live trading
- No runtime config mutation
- No apply/reload
- No auto-apply parameter or weight changes

For S6 and S7, the operator should also assume:

- `notificationSent=false`
- `executionTriggered=false`
- `retentionMode=in_memory_bounded`
- `durableStorageEnabled=false`
- `databaseWriteEnabled=false`

## 3. Dashboard Overview

The dashboard is an operator review surface, not an execution console. The main signal sequence is:

1. Check `Operator Home Summary`.
2. Review `Signal Symbol Filter`.
3. Open `Toxic Signal Inbox`.
4. Check `Toxic Signal Groups`.
5. Open `Signal Detail / Evidence Timeline`.
6. Review `Toxic Signal History`.
7. Review `Toxic Signal Daily Report`.
8. Review `Signal Alert Rules / Notification Preview`.

Each panel exists to help the operator understand signal quality, evidence, grouping, and review status. None of these panels is an order-entry surface.

## 4. S1 — Signal Inbox

`Signal Inbox` is the unified signal entry point.

- View recent toxic signals.
- Review `symbol`, `signalKind`, `severity`, `confidence`, `markout`, `quality`, and `recommendation` summaries.
- Use it as the first pass for triage.
- Treat it as read-only context.

What it does not do:

- It does not place trades.
- It does not imply entry or exit execution.
- It does not override governance or review requirements.

## 5. S2 — Signal Groups

`Signal Groups` reduces duplicate noise through burst grouping.

- Signals with the same `symbol`, `signalKind`, and `directionBias` inside the cooldown window can be grouped.
- `originalSignalsPreserved=true` means grouping is a display and review aid only.
- Grouping does not delete original signals.
- Grouping does not change the signal algorithm.

Use this panel to avoid overreacting to repeated short-window bursts that represent the same operator review event.

## 6. S3 — Signal Detail / Evidence Timeline

`Signal Detail` is the single-signal replay and explanation view.

- Review `fusion`, `replay`, `markout`, `quality`, `recommendation`, and `governance` context.
- Use it to answer why a signal appeared.
- Use it to inspect whether evidence aligns or conflicts.

What it is not:

- It is not an entry/exit recommendation.
- It is not a trade confirmation screen.
- It is not an instruction to long or short.

## 7. S4 — Signal Symbol Filter

`Signal Symbol Filter` is a display-only focus tool.

- `viewOnly=true`
- `persistentWatchlistEnabled=false`
- `runtimeMonitorModified=false`

Use it when you want to focus the current page on one symbol. It only changes the view. It does not change the runtime monitoring scope, it does not save a watchlist, and it does not persist operator selection.

## 8. S5 — Daily Report / Quality Digest

`Daily Report` is the end-of-day or point-in-time digest for signal quality review.

- Summarizes `totalSignals`, `groupedSignals`, `highSeveritySignals`, `noTradeOnlyCandidates`, `downgradeCandidates`, and `notEnoughDataSignals`.
- Summarizes markout counts such as `aligned`, `adverse`, `neutral`, and `notEnoughData`.
- Provides JSON and Markdown views of the same report.

Operator expectation:

- Markdown and JSON numbers must agree.
- The report is for review and monitoring only.
- The report does not imply an execution instruction.

## 9. S6 — Alert Preview

`Alert Preview` shows which signals would be worth reminding an operator about if notification were enabled.

- `notificationSent=false`
- `executionTriggered=false`
- No Telegram send
- No webhook send
- No trade trigger

Use this panel to review candidate notifications, suppression reasons, and manual-review reasons.

Do not assume:

- A preview means a real notification was sent.
- A notify candidate means the system traded.
- A preview status is equivalent to execution readiness.

## 10. S7 — Signal History

`Signal History` is a bounded in-memory history query layer.

- `retentionMode=in_memory_bounded`
- `durableStorageEnabled=false`
- `databaseWriteEnabled=false`

Current first-version behavior:

- Keeps recent signal snapshots in runtime memory only.
- Supports recent signal lookup, symbol filtering, signal ID lookup, alert preview history, and report history.
- History may be lost after restart.
- No DB write.
- No file write.

Do not treat S7 as durable archival storage.

## 11. Common Status Meanings

`not_enough_data`

- Evidence is incomplete.
- Do not reinterpret it as bullish or bearish.
- It means the system cannot support a stronger conclusion yet.

`no_trade_only_candidate`

- The signal should be reviewed as a risk or monitoring event only.
- It is explicitly not a trade instruction.

`downgrade_candidate`

- The signal type or setup may need weaker trust during governance review.
- It is a review hint, not an automatic parameter action.

`notify_candidate`

- If notifications were enabled, this signal might be worth reminding an operator about.
- It is still preview-only.

`review_candidate`

- The signal is worth manual review but has not cleared a stronger reminder threshold.

`suppressed_*`

- The preview or review layer intentionally held the signal back from stronger surfacing.
- Read the suppression reason before drawing conclusions.

`ledgerAvailable=false`

- Governance ledger context is unavailable for that item.
- Do not assume approval, rejection, or suppression if ledger context is missing.

`retentionMode=in_memory_bounded`

- History is runtime memory only.
- It is bounded.
- It is not durable archive behavior.

`viewOnly=true`

- The filter or panel only affects what the operator sees.
- It does not change runtime monitoring scope.

`executionEnabled=false`

- No execution path is available from this surface.
- Nothing on the page should be treated as an order workflow.

## 12. Recommended Operator Workflow

1. Check `Operator Home Summary`.
2. Open `Signal Inbox`.
3. Use `Symbol Filter` when focusing on one symbol.
4. Check `Signal Groups` to avoid duplicate noise.
5. Open `Signal Detail` for high severity signals.
6. Review markout and quality before trusting a signal type.
7. Use `Daily Report` for end-of-day review.
8. Use `Alert Preview` only as notification candidate review.
9. Use `Signal History` for recent lookup.
10. Do not treat any signal as an automatic trade instruction.

## 13. What Not To Do

Do not trade directly from a signal.
Do not treat `directionBias` as an order instruction.
Do not manually apply weight changes without governance review.
Do not assume `not_enough_data` is bullish or bearish.
Do not assume `Alert Preview` means a real notification was sent.
Do not assume `Signal History` is durable storage.
Do not treat grouped signals as deleted originals.
Do not treat daily digest counts as execution permissions.

## 14. Future Extensions

Reasonable next steps can stay inside the signal-only boundary:

- richer operator documentation
- notification preview policy refinement
- durable archive design documentation
- governance and review workflow documentation

Future storage work should remain explicitly separated from the current runtime-only history layer. If durable archival is needed later, it should be introduced as a separately reviewed design and audit track rather than silently extending S7 behavior.
