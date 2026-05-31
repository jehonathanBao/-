# Durable Archive Write Disabled-by-default

This is a disabled-by-default write gate.
It does not enable durable archive writes.
It does not write DB, JSONL, SQLite, or files.
It only proves that unsafe write attempts are rejected by default.

## Contract

`GET /api/archive/write/status` reports the current write gate status.
`POST /api/archive/write` rejects every write request by default.

The response must keep these values fixed:

```text
archiveWriteEnabled=false
durableStorageEnabled=false
databaseWriteEnabled=false
jsonlWriteEnabled=false
sqliteWriteEnabled=false
fileArchiveWriteEnabled=false
runtimeModified=false
executionEnabled=false
notificationSent=false
executionTriggered=false
manualReviewRequired=true
dryRunContractPreserved=true
reviewPackContractPreserved=true
recordsWritten=0
bytesWritten=0
writeRejected=true
archive_write_disabled_by_default
```

## Boundary

No order placement.
No cancel/amend.
No wallet/signing.
No live trading.
No DB write.
No JSONL write.
No SQLite write.
No archive file write.
No runtime mutation.
No config write.
No apply/reload.
No notification sending.
No webhook/Telegram.
No auto weight update.

## Relationship To Dry-run

S16 does not replace the dry-run endpoints. These contracts remain preserved:

```text
POST /api/archive/dry-run/write
GET /api/archive/dry-run/review-pack/latest
GET /api/archive/dry-run/review-pack/:dry_run_id
```

The write gate exists only to prove that archive write attempts are rejected while
the durable archive writer remains disabled.
