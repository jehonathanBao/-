# Durable Signal Archive Design / Write-audit Plan

## 1. Purpose

This document is a design plan only. It does not implement a database, JSONL writer, SQLite writer, API behavior change, dashboard behavior change, runtime mutation, config write, apply path, reload path, notification sender, or execution path.

The future durable signal archive, if approved separately, may only support:

- historical review
- audit reconstruction
- offline research
- operator investigation
- regression analysis

The archive must never become a trading system, alert sending system, runtime control system, or automatic parameter update system.

## 2. Current State

S7 Signal History is the current history layer. It is intentionally runtime-local and bounded.

- `retentionMode=in_memory_bounded`
- `durableStorageEnabled=false`
- `databaseWriteEnabled=false`
- History may be lost after process restart.
- S7 does not write a database.
- S7 does not write JSONL.
- S7 does not write files.
- S7 does not mutate S1-S6 signal logic.

Any future durable archive must be designed as a new reviewed capability. It must not silently change the meaning of S7.

## 3. Non-goals

S9 explicitly does not add implementation.

- no DB implementation
- no JSONL writer
- no SQLite writer
- no API behavior change
- no dashboard behavior change
- no runtime mutation
- no config write
- no apply/reload
- no order placement
- no wallet/signing
- no live trading
- no auto weight update

## 4. Safety Boundary

The archive design inherits the signal-only boundary.

- `readOnly=true`
- `analysisOnly=true`
- `executionEnabled=false`
- `notificationSent=false`
- `executionTriggered=false`
- No order placement
- No cancel/amend
- No wallet/signing
- No transaction construction
- No live trading
- No auto weight update
- No runtime config mutation
- No apply/reload

Archive records are evidence. They are not instructions. They must not be interpreted as orders, wallet actions, execution intents, or weight changes.

## 5. Allowed Future Uses

A future durable archive may be used for:

- replaying historical signal context
- reviewing daily or weekly quality drift
- reconstructing which signal evidence was visible to the operator
- audit checks for signal-only behavior
- offline research on signal quality
- regression fixture generation after manual review

A future durable archive may not be used for:

- order placement
- cancel/amend
- wallet or signing flows
- live trading
- Telegram sending
- webhook sending
- alert delivery
- runtime config mutation
- auto weight update
- calibration runner trigger
- apply/reload

## 6. Write-audit Principles

If durable storage is approved in a later task, every write must be auditable and boring in the best possible way.

Required write-audit properties:

- Every archive write has a write timestamp.
- Every archive write records the source surface, such as `signal_inbox`, `signal_groups`, `signal_report`, `alert_preview`, or `signal_history`.
- Every archive write records schema version.
- Every archive write records whether the source response was `readOnly=true`.
- Every archive write records `executionEnabled=false`.
- Every archive write records `notificationSent=false` and `executionTriggered=false` when alert-preview data is archived.
- Every archive write records `archiveWriteEnabled` if a future config gate exists.
- Every archive write result is observable without implying operator action.

The write-audit log must prove that archival happened, not that trading or notification happened.

## 7. Candidate Archive Record Shape

This is a future schema sketch only. It is not implemented by S9.

```json
{
  "schemaVersion": 1,
  "archiveRecordId": "archive_xxx",
  "source": "signal_inbox",
  "sourceSignalId": "signal_xxx",
  "symbol": "BTCUSDT",
  "signalKind": "short_bias_toxic_flow",
  "directionBias": "short",
  "severity": "high",
  "confidence": 0.72,
  "sourceCreatedAtMs": 0,
  "archivedAtMs": 0,
  "readOnly": true,
  "analysisOnly": true,
  "executionEnabled": false,
  "notificationSent": false,
  "executionTriggered": false,
  "writeAudit": {
    "writer": "durable_signal_archive",
    "archiveWriteEnabled": true,
    "databaseWriteEnabled": true,
    "runtimeModified": false,
    "configModified": false
  }
}
```

Allowed archived fields should be limited to signal evidence, summary fields, timestamps, source identifiers, and safety flags.

Forbidden archived fields:

- private keys
- wallet identifiers not already public market metadata
- signing payloads
- transaction construction data
- order request payloads
- exchange credentials
- notification secrets
- runtime config patches
- auto-apply decisions

## 8. Failure Modes

Archive failure must never affect realtime signal generation or operator review.

Required failure behavior for a future implementation:

- If archive write fails, S1-S7 responses still work.
- If archive storage is unavailable, realtime signals still work.
- If archive schema migration fails, realtime signals still work.
- If archive read fails, dashboard signal panels still render current data.
- If archive backpressure appears, the system must degrade archive capture before blocking signal computation.
- Archive failure must be visible as archive health, not as trading or alert status.

Archive failure cannot:

- place orders
- send alerts
- trigger apply/reload
- trigger calibration runner
- trigger weight update
- change runtime monitor scope
- mutate signal algorithms

## 9. Future Config Gates

If durable archive implementation is approved later, the default posture should be disabled.

Suggested future gates:

- `archiveWriteEnabled=false` by default
- `archiveReadEnabled=true` can be considered separately
- `archiveSchemaVersion=1`
- `archiveMaxWriteBatchSize`
- `archiveWriteAuditEnabled=true`

These gates are design notes only. S9 does not add config fields.

## 10. Future API Shape

Any future API must stay read-only for operator query surfaces.

Potential future read APIs:

- `GET /api/toxicity/signal-archive/status`
- `GET /api/toxicity/signal-archive/recent`
- `GET /api/toxicity/signal-archive/:symbol`
- `GET /api/toxicity/signal-archive/signal/:signal_id`

Potential future write path, if approved, should be internal and audited. It should not be exposed as a dashboard button named Save, Persist, Apply, Execute, Send, Trade, or Reload.

S9 does not add any API.

## 11. Future Dashboard Rules

A future dashboard archive panel may show archive health and query results.

Allowed controls:

- Refresh Archive Status
- Load Archived Signals
- Copy Archive JSON

Forbidden controls:

- Trade
- Execute
- Send Notification
- Apply
- Reload
- Update Weight
- Save Watchlist
- Wallet
- Sign

S9 does not add or change any dashboard panel.

## 12. Readiness Checklist Before Implementation

Do not implement archive storage until a later readiness review confirms:

- allowed fields are explicit
- forbidden fields are explicit
- write-audit schema is explicit
- failure modes are explicit
- default `archiveWriteEnabled=false` is accepted
- no execution linkage is preserved
- no notification sending is preserved
- no wallet/signing is preserved
- no auto weight update is preserved
- archive failure isolation is tested

Until that review passes, S7 remains the only signal history layer:

- `retentionMode=in_memory_bounded`
- `durableStorageEnabled=false`
- `databaseWriteEnabled=false`
