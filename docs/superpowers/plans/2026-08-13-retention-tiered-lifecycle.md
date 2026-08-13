# Tiered Market Data Retention Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Apply a bounded 7-day / 30-day / 365-day retention policy to contract and spot monitoring data without deleting protected evidence or blocking the live read-only monitor.

**Architecture:** A shared retention policy classifies signal/event facts into ordinary, important, or critical tiers. Contract and spot signal persistence stores the computed tier and deadline, while existing batch pruners use indexed columns and bounded deletes. Runtime snapshot cleanup receives the same tiered defaults, and dry-run/status evidence is emitted before production cleanup is widened.

**Tech Stack:** Rust, SQLite/WAL, Tokio, rusqlite, TOML configuration, cargo test.

## Global Constraints

- Ordinary data: 7 days.
- Important data: 30 days.
- Critical data: 365 days.
- Contract and spot data use asset-native units; BTC and ETH thresholds are not interchangeable.
- Pending/retry/sending notification rows are never deleted by retention.
- Read-only monitoring and dry-run behavior remain enabled; no trading or account-state mutation is introduced.
- Existing untracked user files remain untouched.

### Task 1: Shared retention policy and boundary tests

**Files:**
- Create: `src/storage/retention_policy.rs`
- Modify: `src/storage/mod.rs`
- Test: `tests/retention_policy_tests.rs`

Implement `RetentionClass`, `RetentionPolicy`, `ContractRetentionFacts`, `SpotRetentionFacts`, and deterministic classification functions with exact 7/30/365 boundaries.

### Task 2: Contract retention configuration and persistence

**Files:**
- Modify: `src/contract_whale_monitor/config.rs`
- Modify: `config/default.toml`
- Modify: `src/storage/migrations.rs`
- Modify: `src/storage/sqlite.rs`
- Modify: `src/storage/contract_whale_repo.rs`
- Test: `tests/contract_whale_persistence_tests.rs`

Set raw contract flow and ordinary signals to 7 days, important impact to 30 days, and critical evidence to 365 days. Persist retention metadata and prune using it, while preserving pending outbox rows.

### Task 3: Spot retention tiers

**Files:**
- Modify: `src/spot_whale_monitor/config.rs`
- Modify: `config/default.toml`
- Modify: `src/storage/migrations.rs`
- Modify: `src/storage/sqlite.rs`
- Modify: `src/storage/spot_whale_repo.rs`
- Modify: `src/spot_whale_monitor/service.rs`
- Test: `tests/spot_whale_monitor_tests.rs`

Replace the current 7-day/permanent split with 7/30/365-day metadata-driven pruning, preserving existing BTC/ETH unit thresholds.

### Task 4: Runtime snapshot and auxiliary data defaults

**Files:**
- Modify: `src/storage/runtime_retention_repo.rs`
- Modify: `src/storage/snapshot_service.rs`
- Modify: `src/binance_alt_contract_monitor/config.rs`
- Modify: `src/binance_alt_contract_monitor/service.rs`
- Test: `tests/storage_tests.rs`, `tests/system_mode_config_tests.rs`

Align generic snapshots, BACM signal/outcome/event cleanup, and hourly/outbox cleanup with bounded tiers; do not delete pending notifications.

### Task 5: Dry-run evidence, migration checks, and release verification

**Files:**
- Modify: `src/api/contract_event_routes.rs`
- Modify: `src/storage/storage_health.rs`
- Test: relevant retention suites and existing frontend smoke tests

Expose retention tier counts, oldest/newest timestamps, next cleanup, and last-run outcome through existing read-only status surfaces. Run migration dry-run first; only then enable bounded cleanup.

Verification commands:

```text
cargo fmt --check
cargo test -j 1 --test retention_policy_tests --test contract_whale_persistence_tests --test spot_whale_monitor_tests --test storage_tests --test system_mode_config_tests
npm test -- --run
npm run build
```
