# Final Safety And Secrets Audit

Current audit date: `2026-06-01`

## Scope

This audit is a final read-only review across:

- monitor repository: `C:\Users\byhdo_ocup4f5\Documents\有毒订单监控-rs`
- Discord repository: `C:\Users\byhdo_ocup4f5\Documents\discord解决方案`
- source code
- docs
- tests
- scripts
- audit files under `data/` and `.runtime/`
- `.env` and `.env.example`
- `git status`
- `git diff`

This audit does not print real secrets.
This audit does not call Discord over the public network.

## Overall Verdict

`PASS_WITH_LOCAL_ENV_NOTE`

No blocking evidence of committed Discord webhook or bot token leakage was found in the inspected repositories, docs, tests, scripts, diffs, or runtime audit files.

One local operational note remains:

- the Discord project has a local `.env` file present on disk
- it is gitignored
- it is not tracked
- it is not in the current Git diff

That local file should remain local-only and must not be staged or committed.

## Commands Used

```powershell
git status --short
git diff --stat
rg -l "discord\.com/api/webhooks|discordapp\.com/api/webhooks"
rg -l "DISCORD_WEBHOOK_URL|DISCORD_BOT_TOKEN|BOT_TOKEN|WEBHOOK|TOKEN"
rg -n "ENABLE_DEV_TEST_ALERTS|test-sidecar"
```

Additional read-only checks were used to confirm:

- `.env` tracking state
- audit log masking state
- endpoint default-off behavior in code and tests
- dry-run / mocked transport behavior in Discord-side tests

## Git State Summary

### Monitor repository

- `git status --short` shows normal in-progress code changes and new docs/tests
- `git diff` does not contain Discord webhook or bot token patterns

### Discord repository

- `git status --short` shows the working tree as untracked from this local checkout
- local `.env` exists, but:
  - `.gitignore` includes `.env`
  - `git ls-files --error-unmatch .env` failed
  - `.env` is not tracked
  - `.env` is not in `git diff`
- `git diff` does not contain Discord webhook or bot token patterns

## Findings

## 1. `DISCORD_WEBHOOK_URL` has no real committed value

- Result:
  - `PASS`
- Evidence:
  - monitor repo secret search returned no webhook URL hits
  - Discord repo `.env.example` contains empty placeholders only
  - Discord repo docs use variable names or manual placeholder language, not live webhook values
  - diff scan reported:
    - monitor repo: `diff_has_discord_secret_pattern=false`
    - Discord repo: `diff_has_discord_secret_pattern=false`
- Notes:
  - webhook-shaped strings exist in a few Discord tests as obvious placeholders such as fake IDs or `alias-secret`
  - these are test-only strings, not live secrets

## 2. Discord bot token has not been committed

- Result:
  - `PASS`
- Evidence:
  - no tracked `.env` in the Discord repo
  - `.env.example` uses blank placeholder values
  - secret-related code paths live in config loaders and tests, not in committed real credentials
- Notes:
  - local `.env` exists but is ignored and untracked

## 3. Audit files do not contain a full webhook URL

- Result:
  - `PASS`
- Evidence:
  - full webhook regex scan over Discord `data/` and `.runtime/` returned:
    - `full_webhook_match_count=0`
  - masked webhook scan returned exactly two masked files:
    - `.runtime/notification-audit.jsonl`
    - `.runtime/runtime-acceptance-real-audit.jsonl`
  - those records expose `webhookMasked` only, not a full secret
- Notes:
  - masked URLs are acceptable for audit visibility

## 4. Test snapshots and test records do not expose a full webhook

- Result:
  - `PASS`
- Evidence:
  - `tests/discordRealSendVerification.test.ts` asserts persisted audit files do not match a full webhook regex
  - `tests/discordNotificationEgress.test.ts` checks masking and `rawWebhookUrlExposed=false`
  - `tests/rustSidecar.test.ts` and `tests/toxicFlowJsonlWatcher.test.ts` run through mocked or disabled notifier paths
- Notes:
  - some tests intentionally use fake webhook-shaped strings as placeholders
  - those are not evidence of real secret leakage

## 5. Docs do not contain a real webhook

- Result:
  - `PASS`
- Evidence:
  - `docs/toxic-flow-discord-runtime-acceptance-runbook.md` uses:
    - `DISCORD_WEBHOOK_URL="fill this manually on your machine"`
  - `docs/discord-real-send-verification.md` documents setup generically and does not embed a live webhook
  - monitor-repo docs do not contain webhook URLs

## 6. `ENABLE_DEV_TEST_ALERTS` is default-off

- Result:
  - `PASS`
- Evidence:
  - `src/api/dev_alert_routes.rs` only enables the endpoint when env var equals `"true"`
  - the monitor `.env.example` does not turn it on by default
  - `tests/dev_test_alert_api_tests.rs` explicitly verifies disabled behavior

## 7. `/api/dev/alerts/test-sidecar` returns `404` when disabled

- Result:
  - `PASS`
- Evidence:
  - `tests/dev_test_alert_api_tests.rs`
  - when `ENABLE_DEV_TEST_ALERTS` is unset, the endpoint returns:
    - `404`
    - `reason=dev_test_alerts_disabled`

## 8. The dev test endpoint does not trigger trading execution

- Result:
  - `PASS`
- Evidence:
  - `src/api/dev_alert_routes.rs` response safety boundary includes:
    - `No order placement`
    - `No wallet/signing`
    - `No live trading`
  - `src/alerts/alert_service.rs` dev test path writes a runtime acceptance sidecar event only
  - it does not call order placement or execution paths

## 9. The dev test endpoint does not trigger Telegram

- Result:
  - `PASS`
- Evidence:
  - `src/api/dev_alert_routes.rs` returns `telegramTriggered=false`
  - `tests/dev_test_alert_api_tests.rs` asserts `telegramTriggered == false`
  - `src/alerts/alert_service.rs` `emit_runtime_acceptance_test_alert(...)` writes sidecar output only and does not call the Telegram send branch

## 10. Discord watcher dry-run remains safe by default

- Result:
  - `PASS`
- Evidence:
  - Discord `.env.example` sets:
    - `DISCORD_NOTIFY_ENABLED=false`
    - `DISCORD_DRY_RUN=true`
  - `src/notifications/discord/discordNotificationConfig.ts` computes:
    - `enabled = DISCORD_NOTIFY_ENABLED && !dryRun`
  - `tests/discordNotificationEgress.test.ts` verifies dry-run does not hit transport
  - `tests/toxicFlowJsonlWatcher.test.ts` and `tests/rustSidecar.test.ts` use disabled or mocked notifier paths

## 11. Automated tests do not call the public Discord network

- Result:
  - `PASS`
- Evidence:
  - Discord-side transport tests inject `fetchImpl` mocks returning `new Response(...)`
  - watcher tests use mocked notifier functions or disabled-mode responses such as `discord_notify_disabled`
  - runtime safety tests assert `rawWebhookUrlExposed=false`
  - no audit requirement in this pass required a live Discord call
- Notes:
  - real webhook acceptance exists as a manual runtime procedure, not as an automated test

## File-Level Evidence

### Monitor repository

- `src/api/dev_alert_routes.rs`
- `src/alerts/alert_service.rs`
- `src/config/env.rs`
- `tests/dev_test_alert_api_tests.rs`
- `tests/alert_service_tests.rs`
- `.env.example`

### Discord repository

- `.gitignore`
- `.env.example`
- `src/notifications/discord/discordNotificationConfig.ts`
- `src/notifications/discord/discordWebhookTransport.ts`
- `tests/discordNotificationEgress.test.ts`
- `tests/discordRealSendVerification.test.ts`
- `tests/rustSidecar.test.ts`
- `tests/toxicFlowJsonlWatcher.test.ts`
- `docs/toxic-flow-discord-runtime-acceptance-runbook.md`
- `docs/discord-real-send-verification.md`
- `.runtime/notification-audit.jsonl`
- `.runtime/runtime-acceptance-real-audit.jsonl`

## Non-Blocking Notes

- The Discord project has a local `.env` file on disk.
- It is currently ignored and untracked.
- That is acceptable for local runtime use, but it must remain out of Git.

- The monitor repo still contains unrelated runtime/build artifacts under `data/` and `.runtime/`.
- The inspected text-like files in those areas did not contain full Discord webhook URLs.

## Blocking Findings

- None for secret leakage or default safety boundary.

## Recommended Next Action

Before any future commit or publication step:

1. rerun the secret grep commands
2. confirm `.env` is still ignored and untracked
3. confirm `.runtime` audit files still only contain masked webhook values
4. keep `ENABLE_DEV_TEST_ALERTS` unset in default developer startup flows
