# Contract Whale Read-Model Decoupling Design

**Status:** Approved for implementation on 2026-07-15. Production database cutover remains a separate operational gate.

## Problem

The contract-whale page starts a retention-status request three seconds after mount. That route runs several synchronous aggregate scans against the same multi-gigabyte SQLite database used by high-frequency writers. Browser timeouts cancel the HTTP waiter, but not the SQLite work. Five-second status polling and multi-tab usage then create new work while the abandoned scan is still running, eventually starving unrelated Axum requests.

Indexed summary, latest, and event queries are fast when executed alone. The failure is therefore request amplification and blocking/lock contention, not a missing index or a slow public proxy.

## Goals

- Normal dashboard and contract-whale page loads never trigger retention table scans.
- Retention diagnostics return immediately from a bounded snapshot and never execute SQLite work on a Tokio worker.
- At most one retention snapshot calculation runs at a time.
- A client timeout or disconnect cannot start duplicate retention work.
- SQLite WAL mode is initialized once per store instead of being rewritten on every connection.
- Existing monitoring, alerting, thresholds, lifecycle, dry-run, and read-only behavior remains unchanged.
- The code remains ready for a later PostgreSQL/TimescaleDB read-model migration without coupling that migration to the immediate production repair.

## Non-goals

- No live trading, signing, payment, deletion, or mutation endpoint.
- No detector, score, threshold, Discord, or event-visibility change.
- No automatic PostgreSQL cutover, bulk backfill, or destructive SQLite cleanup.
- No automatic commit, push, deployment, or service restart.

## Considered Approaches

### Increase browser and proxy timeouts

Rejected. It keeps request-driven full-table scans and only allows each blocked request to consume resources longer.

### Immediate full PostgreSQL/TimescaleDB cutover

Deferred. It is the long-term storage direction, but combining schema creation, historical backfill, writer cutover, read cutover, and rollback with an active incident creates unnecessary data-loss and downtime risk.

### Staged read-model decoupling, then database migration

Selected. Remove the accidental heavy request first, isolate the diagnostic scan behind a single-flight snapshot runtime, and reduce SQLite connection lock coordination. After the latency gate is stable, introduce TimescaleDB dual-write/backfill and switch readers in a separately verified deployment.

## Backend Design

Add a retention snapshot runtime owned by `AppState`:

- The runtime stores the last successful `ContractRetentionTables` value and completion time.
- Fresh lifetime is 15 minutes; a successful value remains usable as stale for 24 hours.
- Only one calculation may be in flight.
- A request with no cached value starts a detached `spawn_blocking` calculation and immediately returns a degraded read-only snapshot with `errorCode=contract_retention_refresh_in_progress` and `retryAfterMs=2000`.
- A request with a stale value returns it immediately, marks it stale, and starts one background refresh.
- A request with a fresh value returns it without opening SQLite.
- Failed refreshes retain the last successful value and expose a stable error code without leaking SQL or filesystem details.

The public response remains HTTP 200 because this is an optional diagnostic panel and callers already consume per-table availability. The response gains `dataState`, `degraded`, `lastKnownDataAvailable`, `generatedAt`, `cacheAgeSec`, `retryAfterMs`, and `errorCode` metadata.

The SQLite store initializes `journal_mode=WAL` during `SqliteStore::open`. Per-operation connections retain `busy_timeout` but no longer attempt a journal-mode write. This preserves WAL readers while avoiding a write-like pragma during every API query and persistence flush.

## Frontend Design

`ContractWhaleMonitor` never schedules `fetchContractRetentionStatus`. Normal and diagnostic-enabled `/dashboard` and `/contract-whale/:symbol` mounts do not make the request at any point; operators may query the protected diagnostic endpoint directly when needed.

Status and event polling keep the existing in-flight guards and visibility pause. This change does not add another retry layer; the backend single-flight runtime is the authoritative duplication boundary.

## PostgreSQL/TimescaleDB Migration Boundary

After this change passes local and production latency gates, migrate in a separate reversible sequence:

1. Create TimescaleDB hypertables for raw contract flow/OI/funding/liquidation and a relational signal table.
2. Enable idempotent dual writes with per-batch source watermarks while SQLite remains authoritative.
3. Backfill bounded time ranges and compare counts, min/max timestamps, and sampled projections.
4. Switch API readers by configuration while retaining SQLite fallback.
5. Observe for at least one retention window before disabling SQLite writes.

No database password or connection URL is committed. Every cutover switch defaults to SQLite until explicitly configured.

## Testing

Backend tests prove:

- the first retention request returns before a forced slow calculation completes;
- repeated concurrent requests start one calculation;
- a later request serves the completed cache;
- a stale value is returned immediately while one refresh runs;
- summary/latest remain responsive during a forced slow retention calculation;
- WAL and busy-timeout behavior remains correct after removing the per-connection journal update.

Frontend tests prove:

- normal page mounts never request retention status, including after the old three-second delay;
- operator diagnostics do not automatically request retention status;
- existing status/event polling and recovery behavior remains green.

## Acceptance Criteria

- No normal-page request to `/api/contract-retention-status` appears in browser or Nginx logs.
- Retention endpoint response latency remains below 250 ms while its background scan is running.
- Summary/latest remain below 800 ms in the forced-contention integration test.
- Concurrent retention requests produce one underlying calculation.
- Focused Rust and frontend tests pass, followed by the full project validation gates.
- No execution, alert eligibility, auth, or secret-handling boundary changes.
