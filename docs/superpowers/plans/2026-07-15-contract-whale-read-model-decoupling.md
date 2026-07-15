# Contract Whale Read-Model Decoupling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate request-driven retention scans and SQLite journal-mode lock coordination from the contract-whale page load path.

**Architecture:** Keep SQLite authoritative for this repair, but put retention aggregates behind an AppState-owned single-flight stale-while-revalidate snapshot runtime. Remove retention polling from normal React mounts and configure WAL only when the store is opened. The resulting boundaries are migration-ready for a later TimescaleDB dual-write cutover.

**Tech Stack:** Rust, Axum, Tokio, rusqlite, React 19, Vitest.

## Global Constraints

- Preserve read-only monitoring and all existing alert/trading safety gates.
- Do not change detector, score, threshold, retention policy, lifecycle, Discord, auth, or secret behavior.
- Do not perform a live database cutover, destructive cleanup, commit, push, deploy, or service restart in this implementation cycle.
- Run Rust tests sequentially in this repository.

---

### Task 1: Add a non-blocking retention snapshot runtime

**Files:**
- Create: `src/api/contract_retention_runtime.rs`
- Modify: `src/api/mod.rs`
- Modify: `src/app.rs`
- Modify: `src/api/contract_event_routes.rs`
- Modify: `tests/contract_event_routes_tests.rs`

**Interfaces:**
- Produces: `ContractRetentionRuntime::get_or_spawn(compute)` and `ContractRetentionSnapshotOutcome`.
- Consumes: existing `retention_tables` calculation and `ContractRetentionTables` response value.

- [ ] Write a failing integration test that forces a 1.2-second retention calculation, verifies the first route response returns within 250 ms, starts eight equivalent requests, and asserts one calculation started.
- [ ] Run `cargo test -j 1 --test contract_event_routes_tests contract_retention_status -- --nocapture` and verify the latency/single-flight assertion fails.
- [ ] Implement the AppState-owned runtime with 15-minute fresh TTL, 24-hour stale TTL, one in-flight job, detached `spawn_blocking`, stable degraded metadata, and test-only delay/stats hooks.
- [ ] Re-run the focused test and verify the first response is degraded immediately, the completed cache is later served, and the underlying calculation count is one.
- [ ] Add and pass a test showing summary/latest stay responsive while the retention job is running.

### Task 2: Remove retention scans from normal frontend mounts

**Files:**
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: the existing contract-whale component mount lifecycle.
- Produces: no automatic retention diagnostics request from the page.

- [ ] Add a failing fake-timer test that advances beyond three seconds and asserts `fetchContractRetentionStatus` was never called during a normal mount.
- [ ] Run `npm test -- src/tests/ContractWhaleMonitor.test.jsx` and verify it fails on the existing unconditional timeout.
- [ ] Remove retention scheduling from the component; keep the protected endpoint available for direct operator diagnostics.
- [ ] Re-run the focused frontend test and verify it passes.

### Task 3: Stop rewriting WAL mode on every SQLite connection

**Files:**
- Modify: `src/storage/sqlite.rs`
- Modify: `tests/sqlite_store_pragmas_tests.rs`

**Interfaces:**
- Produces: one-time WAL initialization in `SqliteStore::open`; per-operation connections only set connection-local busy timeout.

- [ ] Add a failing lock-contention regression test that holds a read transaction and opens another store connection without attempting a journal-mode mutation.
- [ ] Run `cargo test -j 1 --test sqlite_store_pragmas_tests -- --nocapture` and verify the regression fails with the current per-connection pragma path.
- [ ] Split store initialization from per-connection configuration and move `journal_mode=WAL` to the initialization connection.
- [ ] Re-run the pragma tests and verify WAL, busy timeout, and concurrent reader behavior remain green.

### Task 4: Validate the repair and migration boundary

**Files:**
- Verify all changed files.

**Interfaces:**
- Consumes: Tasks 1-3.
- Produces: local regression, safety, and build evidence.

- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test -j 1 --all-targets`.
- [ ] Run `npm test` and `npm run build` from `toxic-order-monitor`.
- [ ] Run `git diff --check` and inspect `git status --short` for secrets, runtime data, or unrelated files.
- [ ] Report local results and leave TimescaleDB backfill/cutover and production deployment behind their separate operational gates.
