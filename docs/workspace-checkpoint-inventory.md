# Workspace Checkpoint Inventory

Current snapshot date: `2026-06-01`

## Purpose

This document inventories the current Git workspace before a future GitHub release checkpoint.

It classifies changed and untracked files into:

- `COMMIT`
- `DO_NOT_COMMIT`
- `LOCAL_EVIDENCE_ONLY`
- `NEEDS_REVIEW`

It does not delete files.
It does not stage files.
It does not change business logic.

## Current Validation State

- runtime acceptance: `PASS`
- secrets audit: `PASS_WITH_LOCAL_ENV_NOTE`
- repo-wide cargo test / clippy: `BLOCKED_BY_LOCAL_ENV`

## Commands Run

- `git status --short`
- `git status --short --untracked-files=all`
- `git diff --stat`
- `git diff --name-only`
- `rg -n "discord\.com/api/webhooks" .`
- `rg -n "DISCORD_WEBHOOK_URL" .`
- `rg -n "BOT_TOKEN|TOKEN|WEBHOOK" .`
- `rg -n "ENABLE_DEV_TEST_ALERTS" .`
- targeted file and path checks for:
  - `docs/project-runtime-acceptance-matrix.md`
  - `docs/final-safety-and-secrets-audit.md`
  - `.runtime/`
  - `data/`
  - `.env`
  - `checkpoint.json`
  - `toxic-flow-alerts.jsonl`

## Snapshot Summary

### Git Status

Tracked modified files:

- `src/alerts/alert_service.rs`
- `src/alerts/sidecar.rs`
- `src/api/mod.rs`
- `src/api/routes.rs`
- `src/api/server.rs`
- `src/app.rs`
- `src/connectors/binance.rs`
- `src/connectors/manager.rs`
- `src/normalizers/trade.rs`
- `src/types/status.rs`
- `tests/alert_service_tests.rs`
- `tests/normalizer_tests.rs`
- `tests/status_api_tests.rs`
- `tests/venue_enablement_api_tests.rs`
- `web/app.js`

Untracked files:

- `data/runtime-acceptance/archive/2026-06-01T03-05-26-430Z-toxic-flow-alerts.jsonl`
- `data/runtime-acceptance/checkpoint.json`
- `data/runtime-acceptance/toxic-flow-alerts.jsonl`
- `data/runtime-acceptance-real/archive/2026-06-01T03-08-21-060Z-toxic-flow-alerts.jsonl`
- `data/runtime-acceptance-real/checkpoint.json`
- `data/runtime-acceptance-real/toxic-flow-alerts.jsonl`
- `docs/final-safety-and-secrets-audit.md`
- `docs/project-runtime-acceptance-matrix.md`
- `docs/windows-rust-build-stability-runbook.md`
- `src/api/dev_alert_routes.rs`
- `tests/dev_test_alert_api_tests.rs`

### Diff Summary

`git diff --stat` for tracked files:

- `15 files changed`
- `651 insertions`
- `27 deletions`

This summary does not include untracked files.

## Security Scan Summary

### Real Discord Webhook / Token Scan

Results:

- `rg -n "discord\.com/api/webhooks" .` -> no matches
- `rg -n "DISCORD_WEBHOOK_URL" .` -> matches only in documentation placeholders
- `rg -n "BOT_TOKEN|TOKEN|WEBHOOK" .` -> no Discord secret values found; matches include:
  - documentation text about secret handling
  - non-Discord API token code such as `OPERATOR_API_TOKEN`
  - Telegram env variable name `TELEGRAM_BOT_TOKEN`
- targeted scan of current runtime evidence files and `data/runtime-acceptance*` artifacts -> `MATCHED_FILES=0`

Conclusion:

- no real Discord webhook URL was found in this repository snapshot
- no Discord bot token was found in this repository snapshot
- current evidence supports the prior secrets audit result: `PASS_WITH_LOCAL_ENV_NOTE`

### Dev Alert Endpoint Exposure Check

Results:

- `ENABLE_DEV_TEST_ALERTS` appears in:
  - `src/api/dev_alert_routes.rs`
  - `tests/dev_test_alert_api_tests.rs`
  - documentation
- no evidence was found that this endpoint is enabled by default in checked-in startup paths

Conclusion:

- the dev alert endpoint remains explicit-opt-in and should stay that way

## Special Path Checks

### `docs/project-runtime-acceptance-matrix.md`

- current Git state: untracked
- recommendation: `COMMIT`
- reason:
  - it is now the canonical project-level runtime acceptance matrix
  - it includes the completed live Binance public-stream acceptance

### `docs/final-safety-and-secrets-audit.md`

- current Git state: untracked
- recommendation: `COMMIT`
- reason:
  - it records the final repository-side safety review
  - it contains placeholder text, not real secrets

### `docs/toxic-flow-discord-runtime-acceptance-runbook.md`

- current Git state in this repository: not present
- sibling-repo location:
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\docs\toxic-flow-discord-runtime-acceptance-runbook.md`
- recommendation: `NEEDS_REVIEW`
- reason:
  - this file belongs to the sibling Discord project, not the monitor repo
  - do not expect it to be committed from this workspace

### `.runtime/`

- current Git ignore state: ignored by `.gitignore`
- recommendation: `LOCAL_EVIDENCE_ONLY`
- reason:
  - it contains local logs, screenshots, sqlite state, temporary target directories, and runtime snapshots
  - it is too broad and too machine-specific for a release checkpoint

Relevant local evidence files for this acceptance cycle:

- `.runtime/live-venue-diagnostics.json`
- `.runtime/live-status.json`
- `.runtime/live-venue-acceptance.out.log`
- `.runtime/live-venue-acceptance.err.log`
- `.runtime/dashboard-live-acceptance-waited.png`

### `data/`

- current Git ignore state: not ignored by default
- current untracked content:
  - `data/runtime-acceptance/checkpoint.json`
  - `data/runtime-acceptance/toxic-flow-alerts.jsonl`
  - `data/runtime-acceptance/archive/2026-06-01T03-05-26-430Z-toxic-flow-alerts.jsonl`
  - `data/runtime-acceptance-real/checkpoint.json`
  - `data/runtime-acceptance-real/toxic-flow-alerts.jsonl`
  - `data/runtime-acceptance-real/archive/2026-06-01T03-08-21-060Z-toxic-flow-alerts.jsonl`
- recommendation: `LOCAL_EVIDENCE_ONLY`
- reason:
  - these are runtime sidecar artifacts, checkpoint state, and archived acceptance events
  - they are useful for local proof but should not be treated as source fixtures unless a future card explicitly promotes them

### `.env`

- current file presence in this repository root:
  - `.env` not present
  - `.env.example` present
- current Git ignore state:
  - `.env` is ignored
  - `.env.*` is ignored by pattern
- recommendation:
  - `.env` -> `DO_NOT_COMMIT`
  - `.env.example` -> `NEEDS_REVIEW` only if changed in a future card
- reason:
  - no local `.env` file is present in this workspace snapshot
  - `.env.example` is a checked-in placeholder file, not a local secret file

### `checkpoint.json`

- files found:
  - `data/runtime-acceptance/checkpoint.json`
  - `data/runtime-acceptance-real/checkpoint.json`
- recommendation: `LOCAL_EVIDENCE_ONLY`
- reason:
  - checkpoint files are operational state, not source

### `toxic-flow-alerts.jsonl`

- files found:
  - `data/runtime-acceptance/toxic-flow-alerts.jsonl`
  - `data/runtime-acceptance-real/toxic-flow-alerts.jsonl`
- recommendation: `LOCAL_EVIDENCE_ONLY`
- reason:
  - these are local runtime event streams used for acceptance verification

## File Classification

### A. Source Changes That Should Be Committed

| File | Recommendation | Reason |
| --- | --- | --- |
| `src/alerts/alert_service.rs` | `COMMIT` | accepted sidecar test-alert and alert-path changes |
| `src/alerts/sidecar.rs` | `COMMIT` | accepted sidecar schema / writer support |
| `src/api/mod.rs` | `COMMIT` | wiring for accepted API surface |
| `src/api/routes.rs` | `COMMIT` | accepted diagnostics/status contract updates |
| `src/api/server.rs` | `COMMIT` | accepted route registration and runtime surface updates |
| `src/api/dev_alert_routes.rs` | `COMMIT` | accepted dev-only sidecar test alert endpoint |
| `src/app.rs` | `COMMIT` | accepted app wiring for test and diagnostics surfaces |
| `src/connectors/binance.rs` | `COMMIT` | accepted Binance public-stream fixes |
| `src/connectors/manager.rs` | `COMMIT` | accepted ingestion and venue-health updates |
| `src/normalizers/trade.rs` | `COMMIT` | accepted normalizer compatibility fix |
| `src/types/status.rs` | `COMMIT` | accepted diagnostics/status typing additions |
| `web/app.js` | `COMMIT` | accepted dashboard diagnostics/status rendering updates |

### B. Test Changes That Should Be Committed

| File | Recommendation | Reason |
| --- | --- | --- |
| `tests/alert_service_tests.rs` | `COMMIT` | covers sidecar writer and test-alert behavior |
| `tests/dev_test_alert_api_tests.rs` | `COMMIT` | covers dev endpoint default-off and sidecar write path |
| `tests/normalizer_tests.rs` | `COMMIT` | fixture alignment for current normalizer shape |
| `tests/status_api_tests.rs` | `COMMIT` | covers bus -> flow window -> API runtime path |
| `tests/venue_enablement_api_tests.rs` | `COMMIT` | covers diagnostics states including connected-but-no-events and subscribe failures |

### C. Documentation Changes That Should Be Committed

| File | Recommendation | Reason |
| --- | --- | --- |
| `docs/final-safety-and-secrets-audit.md` | `COMMIT` | final repo-side safety report |
| `docs/project-runtime-acceptance-matrix.md` | `COMMIT` | canonical runtime acceptance matrix |
| `docs/windows-rust-build-stability-runbook.md` | `COMMIT` | Windows build stability runbook for repo-wide validation |
| `docs/workspace-checkpoint-inventory.md` | `COMMIT` | this checkpoint inventory |

### D. Local Runtime Evidence Files

| File or Path | Recommendation | Reason |
| --- | --- | --- |
| `.runtime/` | `LOCAL_EVIDENCE_ONLY` | ignored machine-local runtime evidence and build outputs |
| `.runtime/live-venue-diagnostics.json` | `LOCAL_EVIDENCE_ONLY` | local acceptance evidence |
| `.runtime/live-status.json` | `LOCAL_EVIDENCE_ONLY` | local acceptance evidence |
| `.runtime/live-venue-acceptance.out.log` | `LOCAL_EVIDENCE_ONLY` | local acceptance evidence |
| `.runtime/live-venue-acceptance.err.log` | `LOCAL_EVIDENCE_ONLY` | local acceptance evidence |
| `.runtime/dashboard-live-acceptance-waited.png` | `LOCAL_EVIDENCE_ONLY` | local dashboard screenshot evidence |
| `data/runtime-acceptance/` | `LOCAL_EVIDENCE_ONLY` | sidecar dry-run / acceptance artifacts |
| `data/runtime-acceptance-real/` | `LOCAL_EVIDENCE_ONLY` | sidecar real-send / acceptance artifacts |
| `data/runtime-acceptance/checkpoint.json` | `LOCAL_EVIDENCE_ONLY` | watcher checkpoint state |
| `data/runtime-acceptance/toxic-flow-alerts.jsonl` | `LOCAL_EVIDENCE_ONLY` | local event stream |
| `data/runtime-acceptance/archive/...` | `LOCAL_EVIDENCE_ONLY` | archived acceptance event stream |
| `data/runtime-acceptance-real/checkpoint.json` | `LOCAL_EVIDENCE_ONLY` | watcher checkpoint state |
| `data/runtime-acceptance-real/toxic-flow-alerts.jsonl` | `LOCAL_EVIDENCE_ONLY` | local event stream |
| `data/runtime-acceptance-real/archive/...` | `LOCAL_EVIDENCE_ONLY` | archived acceptance event stream |

### E. Files That Must Not Be Committed

| File or Pattern | Recommendation | Reason |
| --- | --- | --- |
| `.env` | `DO_NOT_COMMIT` | local secrets or overrides if created later |
| `.env.*` local secret files | `DO_NOT_COMMIT` | local secrets or overrides if created later |
| any file containing a real Discord webhook URL or bot token | `DO_NOT_COMMIT` | secret material |

Current note:

- no local `.env` file is present in this repository root at the time of this inventory
- no committed Discord webhook or token value was found by the scans run for this card

### F. Files Needing Human Review

| File or Path | Recommendation | Reason |
| --- | --- | --- |
| `docs/toxic-flow-discord-runtime-acceptance-runbook.md` | `NEEDS_REVIEW` | belongs to the sibling Discord repo, not this workspace |
| `data/` as a top-level path | `NEEDS_REVIEW` | not ignored by default; if the team never wants runtime JSONL/checkpoints in Git, a future hygiene card should decide whether to ignore or relocate them |
| `.env.example` | `NEEDS_REVIEW` | safe placeholder file today, but any future edits should be checked to ensure no secrets were copied into it |

## Recommended Release Checkpoint Boundary

Recommended release-checkpoint include set for this repository:

- accepted source files under `src/`
- accepted dashboard file under `web/`
- accepted test files under `tests/`
- accepted project docs under `docs/`

Recommended release-checkpoint exclude set for this repository:

- `.runtime/`
- `data/runtime-acceptance/`
- `data/runtime-acceptance-real/`
- `.env`
- any local secret-bearing file

## Honest Boundary Notes

- runtime acceptance remains `PASS`
- secrets audit remains `PASS_WITH_LOCAL_ENV_NOTE`
- repo-wide cargo test / clippy remains `BLOCKED_BY_LOCAL_ENV`
- this inventory does not claim repo-wide validation is fully green
- this inventory does not promote Bybit / OKX live public-stream acceptance beyond their current expected disabled state
