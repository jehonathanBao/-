# btc-toxic-flow.sqlite P0 Cleanup Runbook Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide a maintenance-window-safe cleanup runbook and batch-delete helper for the live `btc-toxic-flow.sqlite` P0 bloat tables without executing any destructive action during development.

**Architecture:** Keep this as a safety-first operational deliverable: one readonly sizing/reporting path, one guarded batch-delete helper for the three approved P0 tables, and one runbook that maps the live findings onto the real schema used by this repo. The script defaults to dry-run and refuses non-P0 tables so the maintenance operator cannot accidentally widen scope.

**Tech Stack:** Bash, SQLite, Python sqlite3 fallback, Rust integration tests, markdown operations docs

## Global Constraints

- Only target `flow_snapshots`, `venue_health_snapshots`, and `toxic_snapshots`.
- Default `DRY_RUN=1`; no mutation unless the operator explicitly sets `DRY_RUN=0`.
- Do not execute `DELETE`, `VACUUM`, `VACUUM INTO`, `PRAGMA wal_checkpoint`, service stop/start, or file replacement as part of implementation.
- Use the real live schema: P0 tables use integer millisecond `ts`, not `created_at`.
- `contract_flow_1s` is not a first-wave cleanup target.
- Current live environment does not have enough free space for `VACUUM INTO`; keep that as a later operational phase only.

---

## Live Readonly Findings

- Main DB size: about 84 GB
- WAL: 0
- SHM: 32 KB
- Root free space: about 9.5 GB
- `freelist_count`: 0
- `flow_snapshots`: about 8.13M rows; avg `payload_json` about 6931 bytes; payload-only estimate about 52.5 GB
- `toxic_snapshots`: about 2.03M rows; avg `payload_json` about 9596 bytes; payload-only estimate about 18.2 GB
- `venue_health_snapshots`: about 7.67M rows; low payload but very high row count
- `contract_flow_1s`: about 615k rows; roughly BTC/ETH 3.5 days; not the primary bloat source

## Actual Schema Notes

- `flow_snapshots(ts INTEGER NOT NULL, ...)`
- `toxic_snapshots(ts INTEGER NOT NULL, ...)`
- `venue_health_snapshots(ts INTEGER NOT NULL, ...)`
- Only `toxic_snapshots` currently has an explicit `ts` index in migrations.
- `flow_snapshots` and `venue_health_snapshots` are currently higher risk for long deletes because the live schema does not define matching time indexes.

## Maintenance Execution Notes

1. Stop backend writes before any delete window.
2. Use `scripts/sqlite_sizing_readonly.sh` and `scripts/sqlite_cleanup_plan_readonly.sh` immediately before the window to refresh counts.
3. Run `scripts/sqlite_safe_batch_delete.sh` table by table with `TIME_COLUMN=ts` and `TIME_MODE=epoch_ms`.
4. Start with conservative batches:
   - `flow_snapshots`: `BATCH_SIZE=10000`
   - `venue_health_snapshots`: `BATCH_SIZE=20000`
   - `toxic_snapshots`: `BATCH_SIZE=5000`
5. Do not attempt `VACUUM INTO` in the same window unless external free space is at least current DB size times 1.2 and an external backup or VPS snapshot exists.
6. Expect freespace to become reusable inside SQLite first; the main DB file will not immediately shrink.

## Script Usage

Dry run:

```bash
DB_PATH=/path/to/btc-toxic-flow.sqlite \
TABLE=flow_snapshots \
TIME_COLUMN=ts \
TIME_MODE=epoch_ms \
RETENTION_HOURS=24 \
DRY_RUN=1 \
scripts/sqlite_safe_batch_delete.sh
```

Single guarded execution slice:

```bash
DB_PATH=/path/to/btc-toxic-flow.sqlite \
TABLE=flow_snapshots \
TIME_COLUMN=ts \
TIME_MODE=epoch_ms \
RETENTION_HOURS=24 \
BATCH_SIZE=10000 \
MAX_BATCHES=100 \
SLEEP_SECONDS=0.5 \
DRY_RUN=0 \
scripts/sqlite_safe_batch_delete.sh
```

Emergency stop:

```bash
touch /tmp/stop_sqlite_cleanup
```

## Validation Checklist

- `cargo test --test sqlite_maintenance_scripts_tests`
- `bash -n scripts/sqlite_safe_batch_delete.sh`
- `bash -n scripts/sqlite_sizing_readonly.sh`
- `bash -n scripts/sqlite_cleanup_plan_readonly.sh`

## Operational Warning

This runbook is intentionally conservative. It is designed to stop uncontrolled growth and give the operator a bounded delete loop. It does not promise fast reclaim on the main DB file, and it intentionally avoids any online mutation during development and review.
