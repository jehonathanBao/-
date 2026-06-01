# GitHub Release Checkpoint Plan

Current snapshot date: `2026-06-01`

## Scope

This report prepares a GitHub release checkpoint from the current workspace state.

It provides:

- a recommended stage set
- a recommended exclude set
- secret-scan summary
- a suggested commit message
- a suggested tag
- rollback guidance
- a final human checklist

It does not:

- run `git add`
- run `git commit`
- run `git tag`
- run `git push`
- delete files
- modify business logic

## Current Acceptance Boundary

Validated runtime surfaces:

- Binance live public stream: `PASS`
- `/api/venues/diagnostics`: `PASS`
- `/api/status` and flow windows: `PASS`
- dashboard live render: `PASS`
- Discord sidecar bridge: `PASS`
- real Discord webhook send: `PASS`
- monitor-generated sidecar test alert: `PASS`
- final safety and secrets audit: `PASS_WITH_LOCAL_ENV_NOTE`
- project runtime acceptance matrix: `PASS`
- workspace checkpoint inventory: `PASS`

Remaining honest boundaries:

- repo-wide cargo test / clippy: `PARTIAL PASS / BLOCKED_BY_LOCAL_ENV`
- Discord HTTP bridge: `OPTIONAL_NOT_STARTED`
- Bybit / OKX are currently `disabled_by_env`; this is expected and is not a public-network PASS

## Workspace Status Snapshot

Commands used for this report:

- `git status --short`
- `git diff --stat`
- `git diff --name-only`

### `git status --short`

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
- `docs/workspace-checkpoint-inventory.md`
- `src/api/dev_alert_routes.rs`
- `tests/dev_test_alert_api_tests.rs`

### `git diff --stat`

- `15 files changed`
- `651 insertions`
- `27 deletions`

This stat covers tracked modifications only.
It does not include untracked files.

### `git diff --name-only`

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

## Recommended Stage Set

This recommendation is based on:

- current `git status`
- current `git diff`
- [workspace-checkpoint-inventory.md](/C:/Users/byhdo_ocup4f5/Documents/有毒订单监控-rs/docs/workspace-checkpoint-inventory.md)

### A. Source Files to Include

- `src/alerts/alert_service.rs`
- `src/alerts/sidecar.rs`
- `src/api/mod.rs`
- `src/api/routes.rs`
- `src/api/server.rs`
- `src/api/dev_alert_routes.rs`
- `src/app.rs`
- `src/connectors/binance.rs`
- `src/connectors/manager.rs`
- `src/normalizers/trade.rs`
- `src/types/status.rs`

### B. Test Files to Include

- `tests/alert_service_tests.rs`
- `tests/dev_test_alert_api_tests.rs`
- `tests/normalizer_tests.rs`
- `tests/status_api_tests.rs`
- `tests/venue_enablement_api_tests.rs`

### C. Frontend File to Include

- `web/app.js`

### D. Documentation Files to Include

- `docs/project-runtime-acceptance-matrix.md`
- `docs/final-safety-and-secrets-audit.md`
- `docs/windows-rust-build-stability-runbook.md`
- `docs/workspace-checkpoint-inventory.md`
- `docs/github-release-checkpoint-plan.md`

## Explicit Exclude Set

Do not stage these paths for the release checkpoint:

- `.env`
- any real Discord webhook or token file
- `.runtime/`
- `data/runtime-acceptance/`
- `data/runtime-acceptance-real/`
- any `checkpoint.json` file under runtime acceptance directories
- any `toxic-flow-alerts.jsonl` file under runtime acceptance directories
- local screenshots, logs, and JSON snapshots unless the team explicitly decides to version runtime evidence

Concrete local-only paths currently present:

- `.runtime/live-venue-diagnostics.json`
- `.runtime/live-status.json`
- `.runtime/live-venue-acceptance.out.log`
- `.runtime/live-venue-acceptance.err.log`
- `.runtime/dashboard-live-acceptance-waited.png`
- `data/runtime-acceptance/checkpoint.json`
- `data/runtime-acceptance/toxic-flow-alerts.jsonl`
- `data/runtime-acceptance/archive/2026-06-01T03-05-26-430Z-toxic-flow-alerts.jsonl`
- `data/runtime-acceptance-real/checkpoint.json`
- `data/runtime-acceptance-real/toxic-flow-alerts.jsonl`
- `data/runtime-acceptance-real/archive/2026-06-01T03-08-21-060Z-toxic-flow-alerts.jsonl`

## Secret Scan Summary

Recommended or verified scans:

- `rg -n "discord\.com/api/webhooks" .`
- `rg -n "DISCORD_WEBHOOK_URL" .`
- `rg -n "BOT_TOKEN|TOKEN|WEBHOOK" .`
- `rg -n "ENABLE_DEV_TEST_ALERTS" .`

Observed summary:

- `discord.com/api/webhooks`
  - no matches
- `DISCORD_WEBHOOK_URL`
  - matches only in documentation placeholders and examples
  - current known paths:
    - `docs/final-safety-and-secrets-audit.md`
    - `docs/project-runtime-acceptance-matrix.md`
- `BOT_TOKEN|TOKEN|WEBHOOK`
  - no Discord secret value identified
  - matches include placeholder docs and non-Discord token identifiers such as:
    - `OPERATOR_API_TOKEN`
    - `TELEGRAM_BOT_TOKEN`
- `ENABLE_DEV_TEST_ALERTS`
  - appears in:
    - `src/api/dev_alert_routes.rs`
    - `tests/dev_test_alert_api_tests.rs`
    - documentation
  - current interpretation:
    - this is expected
    - it documents and enforces the default-off dev test endpoint

Security interpretation:

- no real Discord webhook or token was found in the current workspace scan
- no evidence suggests the dev test alert endpoint is enabled by default
- do not stage any local `.env` file if one appears later

## Suggested Stage Commands

These are suggestions only.
Do not run them blindly before checking `git diff --cached`.

### Option A: Explicit `git add` File List

```powershell
git add src/alerts/alert_service.rs
git add src/alerts/sidecar.rs
git add src/api/mod.rs
git add src/api/routes.rs
git add src/api/server.rs
git add src/api/dev_alert_routes.rs
git add src/app.rs
git add src/connectors/binance.rs
git add src/connectors/manager.rs
git add src/normalizers/trade.rs
git add src/types/status.rs
git add tests/alert_service_tests.rs
git add tests/dev_test_alert_api_tests.rs
git add tests/normalizer_tests.rs
git add tests/status_api_tests.rs
git add tests/venue_enablement_api_tests.rs
git add web/app.js
git add docs/project-runtime-acceptance-matrix.md
git add docs/final-safety-and-secrets-audit.md
git add docs/windows-rust-build-stability-runbook.md
git add docs/workspace-checkpoint-inventory.md
git add docs/github-release-checkpoint-plan.md
```

### Option B: Safer Review-First Flow

```powershell
git add -N src/alerts/alert_service.rs src/alerts/sidecar.rs src/api/mod.rs src/api/routes.rs src/api/server.rs src/api/dev_alert_routes.rs src/app.rs src/connectors/binance.rs src/connectors/manager.rs src/normalizers/trade.rs src/types/status.rs tests/alert_service_tests.rs tests/dev_test_alert_api_tests.rs tests/normalizer_tests.rs tests/status_api_tests.rs tests/venue_enablement_api_tests.rs web/app.js docs/project-runtime-acceptance-matrix.md docs/final-safety-and-secrets-audit.md docs/windows-rust-build-stability-runbook.md docs/workspace-checkpoint-inventory.md docs/github-release-checkpoint-plan.md
git diff --cached --stat
git diff --cached
```

If the dry staging view looks correct, replace `git add -N` with real `git add` on the same file list.

## Suggested Commit Message

Suggested subject:

```text
checkpoint: validate live venue stream and discord sidecar notifications
```

Suggested body:

```text
Binance live public stream PASS
Discord sidecar bridge PASS
real Discord webhook send PASS
monitor-generated sidecar test alert PASS
safety/secrets audit PASS_WITH_LOCAL_ENV_NOTE
repo-wide verification remains BLOCKED_BY_LOCAL_ENV
Discord HTTP bridge remains OPTIONAL_NOT_STARTED
```

## Suggested Tag

Suggested tag name:

```text
runtime-acceptance-binance-discord-sidecar-20260601
```

Suggested annotated tag message:

```text
Runtime acceptance checkpoint for Binance live public stream and Discord sidecar notification flow.

This checkpoint confirms:
- Binance live public stream PASS
- /api/venues/diagnostics PASS
- /api/status and flow windows PASS
- dashboard live render PASS
- Discord sidecar bridge PASS
- real Discord webhook send PASS
- monitor-generated sidecar test alert PASS

This checkpoint does not mean:
- repo-wide cargo test/clippy is fully green
- Bybit / OKX public-network acceptance is complete
- Discord HTTP bridge is implemented
```

## Rollback Strategy

### 1. Inspect the Checkpoint Commit

After a future commit is created:

```powershell
git log --oneline --decorate -n 10
git show --stat <checkpoint-commit>
```

### 2. Roll Back to the Previous Commit

Safe options depend on whether the checkpoint has already been pushed.

If the checkpoint is local-only and you want to discard it entirely:

```powershell
git reset --hard <previous-commit>
```

If the checkpoint may already be shared and you want a safe public rollback:

```powershell
git revert <checkpoint-commit>
```

### 3. Revert Only One File

If only one tracked file from the checkpoint needs to be undone:

```powershell
git restore --source=<previous-commit> -- path/to/file
```

Or, after the commit exists:

```powershell
git checkout <previous-commit> -- path/to/file
```

### 4. Avoid Deleting Local Runtime Evidence

Do not store local runtime evidence under the checkpoint commit expectation.
Keep these paths out of the release checkpoint:

- `.runtime/`
- `data/runtime-acceptance/`
- `data/runtime-acceptance-real/`

If a rollback is needed, leave those directories untouched unless you explicitly want to purge local evidence.

## Human Checklist

- [ ] Confirm `git status` does not include `.env`
- [ ] Confirm `.runtime/` is not staged
- [ ] Confirm `data/runtime-acceptance*` is not staged
- [ ] Confirm `git diff --cached` contains no webhook or token value
- [ ] Confirm docs do not contain a real Discord webhook
- [ ] Confirm repo-wide verification is still labeled `BLOCKED_BY_LOCAL_ENV`
- [ ] Confirm Bybit / OKX are not being claimed as public-network `PASS`
- [ ] Confirm Discord HTTP bridge is still labeled `OPTIONAL_NOT_STARTED`
- [ ] Run the chosen `git add` flow
- [ ] Run `git commit`
- [ ] Run `git tag`
- [ ] Run `git push`
- [ ] Run `git push --tags`

## Final Release Checkpoint Recommendation

This release checkpoint is reasonable to prepare now because the runtime-critical Binance and Discord sidecar flows are accepted.

It should still be described honestly as a runtime-acceptance checkpoint, not a repo-wide full-green release checkpoint, because:

- repo-wide cargo validation remains `PARTIAL PASS / BLOCKED_BY_LOCAL_ENV`
- Bybit / OKX remain expected disabled, not live-network accepted
- Discord HTTP bridge remains `OPTIONAL_NOT_STARTED`
