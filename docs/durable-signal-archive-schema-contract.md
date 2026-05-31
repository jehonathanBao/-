# Durable Signal Archive Schema Contract

## 1. Purpose

This document is a schema draft only. It does not create an archive writer, database table, JSONL writer, SQLite writer, runtime mutation path, API write endpoint, notification sender, execution path, wallet/signing path, or durable replay mutation path.

S15A defines the contract boundary for a future durable archive record. It does not make the archive available.

## 2. Current Status

The current system still remains in signal-only mode:

- `archiveWriteEnabled=false`
- `durableStorageEnabled=false`
- `databaseWriteEnabled=false`
- `jsonlWriteEnabled=false`
- `sqliteWriteEnabled=false`
- `runtimeModified=false`
- `executionEnabled=false`
- `notificationSent=false`
- `executionTriggered=false`
- `ready_for_archive_mvp=false`

No order placement. No wallet/signing. No live trading.

## 3. Minimal Archive Record

The minimum durable archive record should preserve one signal snapshot and its evidence references:

```rust
struct ArchiveRecordDraft {
    archiveRecordId: String,
    schemaVersion: u32,
    createdAtMs: u64,
    sourceSignalId: String,
    sourceSignalType: String,
    symbol: String,
    signalTsMs: u64,
    signalLayer: String,
    direction: String,
    toxicityScore: f64,
    confidence: f64,
    evidenceRefs: EvidenceRefsDraft,
    replayRef: Option<String>,
    markoutRef: Option<String>,
    governanceRef: Option<String>,
    safetyFlags: ArchiveSafetyFlagsDraft,
    writeMode: String,
    archiveWriteEnabled: bool,
}
```

This is a Rust-like sketch for documentation only. It is not a real business type and must not be treated as implementation.

## 4. Field Semantics

Required field meanings:

- `archiveRecordId`: future durable archive record identifier
- `schemaVersion`: contract version for archive record evolution
- `createdAtMs`: archive record creation time, not signal generation time
- `sourceSignalId`: original signal identifier from S1-S15 surfaces
- `sourceSignalType`: source classification such as inbox, grouped, detail, report, or alert preview
- `symbol`: market symbol visible to the operator
- `signalTsMs`: source signal timestamp from the snapshot
- `signalLayer`: source layer such as `signal_inbox`, `signal_groups`, `signal_detail`, `signal_history`, `signal_report`, or `signal_alert_preview`
- `direction`: signal direction snapshot only, never an order instruction
- `toxicityScore`: source toxicity value or derived normalized score captured for audit
- `confidence`: signal confidence snapshot
- `evidenceRefs`: references to replay, markout, governance, and supporting evidence modules
- `replayRef`: reference to replay evidence, if present
- `markoutRef`: reference to markout evidence, if present
- `governanceRef`: reference to governance evidence, if present
- `safetyFlags`: fixed signal-only safety assertions
- `writeMode`: future mode label such as `disabled`, `dry_run`, or `archive_write_only`
- `archiveWriteEnabled`: explicit future gate, defaulting to false

## 5. Evidence Reference Contract

`evidenceRefs` must point to existing read-only evidence surfaces. It must not embed live execution payloads.

Suggested shape:

```rust
struct EvidenceRefsDraft {
    signalInboxRef: Option<String>,
    signalGroupRef: Option<String>,
    signalDetailRef: Option<String>,
    signalHistoryRef: Option<String>,
    replayRef: Option<String>,
    markoutRef: Option<String>,
    governanceRef: Option<String>,
    alertPreviewRef: Option<String>,
    reportRef: Option<String>,
}
```

Reference intent:

- T1-T14 evidence should be linked by stable identifiers or source module references
- replay should be linked by replay record id or signal id
- markout should be linked by signal id or markout record id
- governance should be linked by proposal id, review id, signoff id, or symbol-scoped reference

`evidenceRefs` are references only. They are not copied transaction payloads, credential blobs, or execution instructions.

## 6. Source Snapshot vs Derived Fields

Source snapshot fields:

- `sourceSignalId`
- `sourceSignalType`
- `symbol`
- `signalTsMs`
- `signalLayer`
- `direction`
- `confidence`
- replay / markout / governance references

Derived fields:

- `archiveRecordId`
- `schemaVersion`
- `createdAtMs`
- `toxicityScore`
- `writeMode`
- `safetyFlags`

The rule is simple: source snapshots describe what the operator saw; derived fields describe how archival would classify and protect that snapshot.

## 7. Safety Flags Contract

The archive safety flags must remain fixed and explicit:

```rust
struct ArchiveSafetyFlagsDraft {
    archiveWriteEnabled: bool,      // false
    durableStorageEnabled: bool,    // false
    databaseWriteEnabled: bool,     // false
    jsonlWriteEnabled: bool,        // false
    sqliteWriteEnabled: bool,       // false
    runtimeModified: bool,          // false
    executionEnabled: bool,         // false
    notificationSent: bool,         // false
    executionTriggered: bool,       // false
}
```

Fixed draft values for the current system:

- `archiveWriteEnabled=false`
- `durableStorageEnabled=false`
- `databaseWriteEnabled=false`
- `jsonlWriteEnabled=false`
- `sqliteWriteEnabled=false`
- `runtimeModified=false`
- `executionEnabled=false`
- `notificationSent=false`
- `executionTriggered=false`

No order placement. No wallet/signing. No live trading.

## 8. Forbidden Data

The durable archive schema must never include:

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

The contract must remain evidence-only and review-only.

## 9. Schema Version Evolution

`schemaVersion` should evolve conservatively:

1. Additive fields may advance the version when older readers can ignore them safely.
2. Breaking field renames or semantic changes require a new major contract version.
3. Evidence reference meaning must never change silently across versions.
4. Safety flags must remain explicit in every version.
5. Any future migration plan must document old-to-new mapping before a writer exists.

Until that migration plan exists in code and tests, archive write must remain disabled.

## 10. Why Archive Write Is Still Disabled

Current blockers remain active:

- the schema is drafted but not implemented as a tested contract
- no archive writer gate exists in runtime configuration
- no write-audit record type is finalized
- no failure-mode test matrix is implemented
- no archive read contract is finalized
- no storage backend has been approved for MVP

That is why:

- `archiveWriteEnabled=false`
- `durableStorageEnabled=false`
- `databaseWriteEnabled=false`

## 11. Remaining Gates Before Any Implementation

Before any dry-run writer is considered, the project still needs:

- versioned schema contract acceptance
- write-audit contract draft
- allowed/forbidden field review signoff
- failure-mode test plan
- archive read contract draft
- default-off config gate definition
- storage backend decision
- migration and pruning policy review

The next recommended card remains:

```text
S15B - Durable Archive Write Plan / Dry-run Writer Contract
```

That future card should still avoid real persistence by default.

## 12. Non-goals

This schema draft does not:

- create an archive writer
- create a DB table
- write JSONL
- write SQLite
- add POST, PUT, or PATCH archive APIs
- change `AppState`
- change runtime behavior
- change S1-S15 signal semantics
- trigger notification sending
- trigger execution
- connect wallet/signing/order paths
