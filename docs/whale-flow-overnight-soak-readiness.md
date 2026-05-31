# Whale Flow Overnight Soak Readiness Checklist

This checklist is for local overnight signal-only data collection.
It does not enable trading.
It does not enable order placement.
It does not enable wallet/signing.
It does not enable notification sending.
It does not enable durable archive writes.

这份清单只用于本地整晚运行前的信号采集准备检查。
它不启用交易，不下单，不撤单，不签名，不发送通知，也不写入 durable archive。

## 1. Purpose

Use this checklist before leaving Whale Flow running overnight on the local machine.

The goal is not to prove that a whale candidate will appear.
The goal is to confirm that the system is in a safe, visible, signal-only collection state.

The operator should confirm:

- venue enabled / connected
- market data quality healthy or acceptable
- latest trade/book available
- event bus lagged/dropped visible and understood
- candidate history retention active
- calibration does not need to be ready before collection
- no DB/write/execution path is enabled

## 2. Safety Boundary

The overnight soak remains inside the same read-only operator boundary:

- `readOnly=true`
- `analysisOnly=true`
- `executionEnabled=false`
- `runtimeModified=false`
- `notificationSent=false`
- `executionTriggered=false`
- `archiveWriteEnabled=false`
- `databaseWriteEnabled=false`
- `jsonlWriteEnabled=false`
- `sqliteWriteEnabled=false`
- No order placement
- No cancel/amend
- No wallet/signing
- No transaction construction
- No live trading
- No apply/reload
- No auto threshold update

If any part of the surface suggests execution, treat that as out of scope for this soak checklist.

## 3. Pre-run Checklist

Before the overnight run, confirm:

- [ ] Confirm the project path is `C:\Users\byhdo_ocup4f5\Documents\有毒订单监控-rs`
- [ ] Confirm the dashboard opens at `http://127.0.0.1:3000/dashboard`
- [ ] Confirm `monitoringStarted=false` before manual start
- [ ] Confirm `Start Monitoring` is manual-only
- [ ] Confirm no API keys are required
- [ ] No API keys should be added for this soak checklist
- [ ] Confirm no wallet/signing/live trading env vars are set
- [ ] Confirm archive write remains disabled
- [ ] Confirm candidate history is bounded in-memory only

The soak should begin only after these baseline checks are visible.

## 4. Venue Connectivity Checklist

Check:

- `/api/status`
- `/api/venues/diagnostics`

If `/api/venues/diagnostics` is not available yet, use `/api/status` venue fields.

Before an overnight run, confirm:

- [ ] `ENABLE_BINANCE=true` is visible in current process or config source
- [ ] `ENABLE_BYBIT=true` is visible in current process or config source
- [ ] `ENABLE_OKX=true` is visible in current process or config source
- [ ] At least one venue `enabled=true`
- [ ] At least one venue status is connected/active or has recent messages
- [ ] Latest venue trade is available
- [ ] Latest venue book is available if depth-based diagnostics are expected
- [ ] `connectedVenues > 0`
- [ ] `activeVenues > 0`

Important:

- `monitoringStarted=true` does not guarantee venue streams are active

## 5. Market Data Quality Checklist

Before and during the soak, confirm:

- [ ] Flow windows are populated
- [ ] Latest trade timestamp is recent
- [ ] Latest book timestamp is recent
- [ ] Event bus lagged events = 0 or clearly understood
- [ ] Dropped events = 0 or clearly understood
- [ ] Market Data Quality is healthy or acceptable

If lagged/dropped counters are not implemented yet, treat unavailable counters as a known visibility gap.

## 6. Event Bus / Consumer Lag Checklist

The operator should know whether the feed is clean or only partially visible.

Review:

- `laggedEvents`
- `droppedEvents`
- latest trade timestamp
- latest book timestamp
- connector last message timestamps if available

Use this rule:

- zero is best
- small non-zero counts are not automatically fatal if the cause is known
- unavailable counters should be documented as a visibility gap, not silently ignored

## 7. Whale Flow Candidate History Checklist

Check:

- `/api/toxicity/whale-flow/history/status`
- `/api/toxicity/whale-flow/history/recent`

Before leaving the soak running, confirm:

- [ ] `retentionMode=in_memory_bounded`
- [ ] `durableStorageEnabled=false`
- [ ] `databaseWriteEnabled=false`
- [ ] `currentCandidates` visible
- [ ] `maxCandidates` visible
- [ ] `recordedCount` visible
- [ ] `deduplicatedCount` visible
- [ ] `evictedCount` visible
- [ ] oldest/latest candidate timestamps visible

Notes:

- Candidate history may be empty at the beginning of a soak
- Empty history is normal before any whale candidate is detected

## 8. Calibration Readiness Notes

Calibration readiness is not required before running overnight data collection.

Use these rules:

- `calibrationReady=false` is acceptable during early data collection
- `NOT READY` means collect more evidence
- `READY` does not mean auto-apply
- candidate count is not evidence count
- resolved markout evidence is required for calibration
- `needs_more_data` is the safe default

The soak can still be valid even if calibration remains blocked the whole night.

## 9. Data Collection Expectations

These outcomes can be normal during a quiet or partial night:

- No whale candidate detected all night can be normal if market is quiet
- No suspicious toxic order can be normal if multi-layer confluence is not met
- `not_enough_data` can be normal early in the run
- venue confluence may be partial if one exchange stream is down
- baseline may use fallback until enough history accumulates

Do not treat an empty candidate list as proof that the monitor failed.

## 10. During-run Monitoring

Periodically review:

- `/api/status`
- `/api/toxicity/whale-flow/status`
- `/api/toxicity/whale-flow/recent`
- `/api/toxicity/whale-flow/history/status`
- `/api/toxicity/whale-flow/calibration/report`
- `/api/toxicity/signal-health/summary`

Dashboard panels to watch:

- `BTC Whale / Large Flow Monitor`
- `Whale Flow Operator Presets`
- `Whale Candidate History`
- `Whale Calibration Readiness`
- `Signal Health / Completeness`
- `可能有毒订单列表`

If the dashboard becomes stale, refresh it before assuming the runtime stopped.

## 11. After-run Review

After the overnight run, review:

- Whale Flow recent candidates
- Candidate History
- Rolling Signal Quality Digest
- Whale Flow Calibration Report
- Replay Overlay / Markout Heatmap
- Alert Preview / Explainability

Do not manually change thresholds based on one night unless resolved evidence is sufficient.

## 12. What Not To Do

Do not treat whale flow as a trade instruction.
Do not treat `aggressive_buy` as a buy signal.
Do not treat `aggressive_sell` as a sell signal.
Do not treat `directionBias` as an order instruction.
Do not apply threshold changes automatically.
Do not enable archive writes during a soak unless a separate approved task exists.
Do not add API keys, wallet keys, private keys, or exchange secrets.
Do not expose the dashboard on LAN without the API security guard and operator token path.

## 13. Troubleshooting

### Case A: `monitoringStarted=true` but venues disabled

Runtime start succeeded, but venue streams are not enabled.
Check `ENABLE_BINANCE` / `ENABLE_BYBIT` / `ENABLE_OKX` and config source.

### Case B: venues enabled but no recent trades

Connector may be connecting, blocked, timed out, or symbol mapping may be wrong.
Check `lastError`, `lastMessageTs`, and `lastTradeTs`.

### Case C: candidate history empty

No whale flow candidate has been detected yet, or data quality is insufficient.

### Case D: `calibrationReady=false`

This is expected until enough resolved markout evidence is collected.

### Case E: dashboard stale

Refresh dashboard.
Check backend service logs.
Check whether the web service is still running.
