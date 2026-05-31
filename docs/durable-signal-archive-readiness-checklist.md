# Durable Signal Archive MVP Readiness Checklist

## 1. Review Scope

This document is a readiness review only. It does not implement durable archive storage, a database writer, a JSONL writer, a SQLite writer, an API behavior change, a dashboard behavior change, runtime mutation, config write, apply/reload, notification sending, webhook/Telegram delivery, order placement, wallet/signing, live trading, or auto weight update.

S15 reviews whether the system is ready to enter a future Durable Signal Archive MVP task. It does not change S1-S14 signal semantics.

## 2. Readiness Conclusion

```text
ready_for_archive_mvp = false
blocked_reasons = [
  "archive schema is documented only and not finalized as a versioned code contract",
  "archive write flag is documented only and not defined in runtime configuration",
  "archive write-audit schema is not implemented or tested",
  "archive read API contract is not finalized",
  "archive failure-mode tests are not implemented",
  "archive storage choice and pruning policy are not approved for implementation"
]
recommended_next_card = "S15A - Durable Archive Schema Contract Draft"
```

The conservative result is intentional. S9 established the design boundary, but the next safe step is a schema contract draft, not a writer.

## 3. Current State

S7 remains the only signal history layer.

- `retentionMode=in_memory_bounded`
- `durableStorageEnabled=false`
- `databaseWriteEnabled=false`
- History may be lost after process restart.
- No durable archive is implemented.
- No archive write path exists.
- No archive read API exists.

S9 is a design document. S15 is a readiness review. Neither document authorizes durable writes.

## 4. Design Coverage Gate

S9 covers the required design areas at a planning level:

- difference between S7 `in_memory_bounded` history and future durable archive
- allowed fields
- forbidden fields
- storage options
- write-audit requirements
- retention and pruning policy
- API boundary
- dashboard boundary
- failure modes
- migration plan
- future acceptance gates

Readiness gap:

- these areas are not yet converted into a versioned schema contract
- acceptance tests for archive failure behavior do not exist
- implementation defaults have not been wired because S15 is documentation only

## 5. Allowed Data Gate

A future archive may only store signal-only evidence and review metadata:

- `signalId`
- `symbol`
- `signalKind`
- `directionBias`
- `severity`
- `confidence`
- `createdAtMs`
- `operatorAction`
- markout status
- quality bucket
- recommendation action
- alert preview status
- `notificationSent=false`
- `executionTriggered=false`
- source module
- `historyRecordedAtMs`
- `schemaVersion`

Allowed fields must remain descriptive. They must not become trading instructions.

## 6. Forbidden Data Gate

A future archive must never store:

- private keys
- wallet seed phrase
- API secrets
- exchange credentials
- signed transactions
- unsigned transaction payloads
- order placement payloads
- cancel/amend payloads
- position management instructions
- live trading instructions
- Telegram bot token
- webhook secret
- raw credentials

The archive must not become a secret store, execution queue, notification queue, or runtime control surface.

## 7. Write-audit Gate

Any future archive write must include:

- `writeId`
- `sourceModule`
- `recordType`
- `recordCount`
- `createdAtMs`
- `schemaVersion`
- `runtimeModified=false`
- `executionEnabled=false`
- `archiveWriteOnly=true`

Archive write semantics must remain narrow:

- archive write is not runtime mutation
- archive write is not config mutation
- archive write is not execution
- archive write does not trigger reload
- archive write does not send notification
- archive write does not trigger webhook/Telegram
- archive write does not update weights

## 8. Default-off Gate

A future MVP must start disabled by default:

- `archiveWriteEnabled=false`
- `durableStorageEnabled=false`
- `databaseWriteEnabled=false`

Only an explicitly reviewed configuration should be allowed to enter archive write dry-run or archive write MVP behavior.

S15 does not add these config fields. It only records the required future default.

## 9. Failure Mode Gate

A future implementation must fail closed for archive-specific work and fail open for live signal review:

- disk full
- permission denied
- schema mismatch
- partial write
- duplicate `signalId`
- corrupted archive file
- clock skew
- large file / slow query
- archive unavailable

Required isolation:

- archive failure must not affect live signal computation
- archive failure must not affect S7 in-memory history
- archive failure must not affect S1-S14 signal semantics
- archive failure must not trigger runtime reload
- archive failure must not trigger alert sending
- archive failure must not trigger webhook/Telegram
- archive failure must not trigger order placement
- archive failure must not trigger wallet/signing
- archive failure must not trigger live trading

## 10. Safety Boundary

The signal-only boundary remains active:

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
- No apply/reload
- No runtime mutation
- No config write

## 11. Implementation Decision

Do not start a durable archive writer yet.

Recommended next card:

```text
S15A - Durable Archive Schema Contract Draft
```

S15A should still avoid DB writes. It should produce a versioned schema contract, allowed/forbidden field list, write-audit record shape, failure-mode test plan, and default-off acceptance gates.

Only after S15A is complete should the project reconsider an MVP dry-run writer. Any later MVP must keep:

- `archiveWriteEnabled=false` by default
- `dryRun=true` by default
- no actual DB by default
- no execution linkage
- no notification sending
- no wallet/signing
- no auto weight update
