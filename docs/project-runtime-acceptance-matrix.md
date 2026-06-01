# Project Runtime Acceptance Matrix

Current snapshot date: `2026-06-01`

## Scope

This matrix tracks the runtime acceptance state of the monitor repo and its sidecar-based Discord notification path.

It covers:

- local API startup
- public venue diagnostics
- flow window population
- sidecar writer output
- the dev-only sidecar test alert entrypoint
- the sibling Discord watcher / webhook path
- dedupe and checkpoint behavior
- repo-wide validation state
- security boundary checks

It does not change trading logic.
It does not enable live trading.
It does not move Discord transport logic into this repository.

## Status Legend

- `PASS`: acceptance path and evidence are present
- `PARTIAL PASS`: the target scope passed, but a broader repo-wide or environment-wide gate is still incomplete
- `FAIL`: acceptance path is currently red or incomplete
- `EXPECTED_DISABLED`: the surface is intentionally disabled by env or config and is not counted as a failure
- `BLOCKED_BY_LOCAL_ENV`: validation is limited by the current Windows toolchain or machine environment
- `OPTIONAL_NOT_STARTED`: not implemented yet and not required for the current delivery boundary
- `PENDING_PUBLIC_NETWORK`: local contract is implemented, but live public-network verification is still required

## Summary Matrix

| ID | Runtime Surface | Current Status | Blocks Delivery |
| --- | --- | --- | --- |
| 1 | API server startup | `PASS` | `No` |
| 2 | Venue diagnostics | `PASS` | `No` |
| 3 | Flow windows | `PASS` | `No` |
| 4 | Sidecar writer | `PASS` | `No` |
| 5 | Dev test alert endpoint | `PASS` | `No` |
| 6 | Discord watcher | `PASS` | `No` |
| 7 | Discord real webhook | `PASS` | `No` |
| 8 | Dedupe | `PASS` | `No` |
| 9 | Checkpoint | `PASS` | `No` |
| 10 | Repo-wide verification | `PASS` | `No` |
| 11 | Security review | `PASS` | `No` |
| 12 | Discord HTTP bridge | `OPTIONAL_NOT_STARTED` | `No` |

## 1. API Server Startup

- Acceptance command:
  - `cargo run -- serve`
  - open [http://127.0.0.1:3000](http://127.0.0.1:3000)
  - launcher path: `一键启动监控面板.cmd`
- Expected result:
  - server binds to `127.0.0.1:3000`
  - `/dashboard` returns HTML
  - if the service is already running, the launcher opens the existing dashboard instead of starting a duplicate instance
- Current status:
  - `PASS`
- Evidence location:
  - `一键启动监控面板.cmd`
  - `src/api/server.rs`
  - `tests/dev_test_alert_api_tests.rs`
  - `tests/venue_enablement_api_tests.rs`
- Failure triage:
  - if port `3000` is already in use, run:
    - `Get-NetTCPConnection -LocalPort 3000 -ErrorAction SilentlyContinue | Select-Object LocalAddress,LocalPort,State,OwningProcess`
    - `Get-Process -Id <PID>`
  - if `cargo` is missing, use a Rust-enabled shell
  - if MSVC link tools are missing, use the steps in `docs/windows-rust-build-stability-runbook.md`
- Blocks delivery:
  - `No`

## 2. Venue Diagnostics

- Acceptance command:
  - start monitor runtime with Binance enabled
  - `Invoke-WebRequest http://127.0.0.1:3000/api/venues/diagnostics -UseBasicParsing`
  - optional UI check: open `/dashboard` and review `Venue Stream Diagnostics`
- Expected result:
  - when all venues are disabled, diagnostics explain `disabled_by_env`
  - when Binance is enabled but no market events arrive yet, status can be `connected_but_no_events`
  - after real public trade/book events arrive, diagnostics move to `public_stream_active`
  - Bybit / OKX remain explicitly disabled if their enable flags are false; this is an expected disabled state, not a failure
- Current status:
  - `PASS`
- Evidence location:
  - `src/api/routes.rs`
  - `src/api/server.rs`
  - `tests/venue_enablement_api_tests.rs`
  - `tests/status_api_tests.rs`
  - `web/app.js`
  - `.runtime/live-venue-diagnostics.json`
  - `.runtime/live-status.json`
  - `.runtime/live-venue-acceptance.out.log`
  - `.runtime/live-venue-acceptance.err.log`
  - `.runtime/dashboard-live-acceptance-waited.png`
- Verified live acceptance snapshot:
  - `diagnosticStatus=public_stream_active`
  - `latestVenueTradeAvailable=true`
  - `latestVenueBookAvailable=true`
  - `flowWindowsPopulated=true`
  - `connectedVenues=1`
  - `activeVenues=1`
  - Binance trade/book message counts increased during polling
  - `venueDiagnosticStatuses.bybit=disabled_by_env`
  - `venueDiagnosticStatuses.okx=disabled_by_env`
- Failure triage:
  - if all venues read disabled, confirm `ENABLE_BINANCE`, `ENABLE_BYBIT`, and `ENABLE_OKX`
  - if Binance is stuck at `connected_but_no_events`, compare `/api/venues/diagnostics` with `/api/status`
  - if live public traffic is unavailable, treat this as runtime/network validation pending, not as a contract failure
  - do not count Bybit / OKX `disabled_by_env` as a regression unless their enable flags were intentionally set true for that run
- Blocks delivery:
  - `No`

## 3. Flow Windows

- Acceptance command:
  - `Invoke-WebRequest http://127.0.0.1:3000/api/status -UseBasicParsing`
  - targeted regression: `cargo test --test status_api_tests`
- Expected result:
  - `marketDataQuality.flowWindowsPopulated == true` after normalized trade flow reaches the aggregator
  - `Visible Windows` reflects configured flow windows
  - `Active Venues` becomes non-zero when a venue is truly ingesting
- Current status:
  - `PASS`
- Evidence location:
  - `tests/status_api_tests.rs`
  - `web/app.js`
  - `src/api/routes.rs`
  - `.runtime/live-status.json`
  - `.runtime/dashboard-live-acceptance-waited.png`
- Verified live acceptance snapshot:
  - `/api/status` returned `marketDataQuality.status=healthy`
  - `/api/status` returned `marketDataQuality.flowWindowsPopulated=true`
  - dashboard rendered `MARKET DATA QUALITY = HEALTHY`
  - dashboard rendered `Flow Windows Populated = true`
  - dashboard rendered `Visible Windows = 4`
  - dashboard rendered `Active Venues = 1`
- Failure triage:
  - if `flowWindowsPopulated == false`, inspect venue trade activity first
  - if diagnostics are green but flow windows stay empty, rerun `cargo test --test status_api_tests`
  - compare `latestVenueTradeAvailable`, `tradeMessageCount`, and `flowWindowsPopulated` together
- Blocks delivery:
  - `No`

## 4. Sidecar Writer

- Acceptance command:
  - set:
    - `TOXIC_FLOW_SIDECAR_ENABLED=true`
    - `TOXIC_FLOW_SIDECAR_EVENTS_PATH=<shared-jsonl-path>`
  - emit an alert through `AlertService` or the dev test endpoint
  - inspect the JSONL file
- Expected result:
  - a line is appended in `toxic-flow-rs.sidecar.v1`
  - output stays producer-only and does not contain Discord webhook or token fields
- Current status:
  - `PASS`
- Evidence location:
  - `src/alerts/sidecar.rs`
  - `src/alerts/alert_service.rs`
  - `tests/alert_service_tests.rs`
- Failure triage:
  - if the file is not created, confirm both sidecar env vars are set
  - if no line is written, confirm an alert path was actually triggered
  - if secrets appear in output, fail closed and inspect the sidecar serialization contract immediately
- Blocks delivery:
  - `No`

## 5. Dev Test Alert Endpoint

- Acceptance command:
  - default-off check:
    - `POST /api/dev/alerts/test-sidecar` without `ENABLE_DEV_TEST_ALERTS=true`
  - enabled check:
    - set `ENABLE_DEV_TEST_ALERTS=true`
    - set sidecar env vars
    - `POST /api/dev/alerts/test-sidecar`
- Expected result:
  - default-off path returns `404` with `reason=dev_test_alerts_disabled`
  - missing sidecar config returns `409` with `error=sidecar_disabled_or_path_missing`
  - enabled path writes a `runtime_acceptance_test` sidecar event
- Current status:
  - `PASS`
- Evidence location:
  - `src/api/dev_alert_routes.rs`
  - `src/api/server.rs`
  - `tests/dev_test_alert_api_tests.rs`
- Failure triage:
  - `404`: `ENABLE_DEV_TEST_ALERTS` is not enabled
  - `409`: sidecar is disabled or path is missing
  - if event content is wrong, compare the response body and written JSONL against the test fixture assertions
- Blocks delivery:
  - `No`

## 6. Discord Watcher

- Acceptance command:
  - in the sibling repo `C:\Users\byhdo_ocup4f5\Documents\discord解决方案`
  - set:
    - `TOXIC_FLOW_SIDECAR_PATH=<shared-jsonl-path>`
    - `DISCORD_DRY_RUN=true`
  - run:
    - `npm.cmd run notification:run-toxic-flow-daemon`
    - optional smoke: `npm.cmd run notification:smoke:toxic-flow-daemon`
- Expected result:
  - watcher starts without crashing
  - watcher consumes new `toxic-flow-rs.sidecar.v1` lines
  - dry-run logs a formatted Discord preview without sending to the public network
- Current status:
  - `PASS`
- Evidence location:
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\package.json`
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\docs\toxic-flow-discord-runtime-acceptance-runbook.md`
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\tests\toxicFlowJsonlWatcher.test.ts`
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\scripts\verify-toxic-flow-jsonl-daemon.ts`
- Failure triage:
  - if the watcher is silent, confirm the monitor repo and Discord repo point to the exact same JSONL path
  - if the file is absent, keep the watcher running and emit a test alert from the monitor repo
  - if dry-run sends real traffic, fail immediately because `DISCORD_DRY_RUN=true` was violated
- Blocks delivery:
  - `No`

## 7. Discord Real Webhook

- Acceptance command:
  - in the sibling repo `C:\Users\byhdo_ocup4f5\Documents\discord解决方案`
  - set:
    - `TOXIC_FLOW_SIDECAR_PATH=<shared-jsonl-path>`
    - `DISCORD_DRY_RUN=false`
    - `DISCORD_WEBHOOK_URL=<manual-local-secret>`
  - run:
    - `npm.cmd run notification:run-toxic-flow-daemon`
  - then emit a new sidecar event with a fresh `dedupeKey`
- Expected result:
  - Discord receives exactly one message
  - watcher / audit output records `deliveryStatus=sent`
  - no raw webhook URL appears in logs or audit records
- Current status:
  - `PASS`
- Evidence location:
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\docs\discord-real-send-verification.md`
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\docs\toxic-flow-discord-runtime-acceptance-runbook.md`
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\tests\rustSidecar.test.ts`
- Failure triage:
  - if nothing arrives, confirm `DISCORD_DRY_RUN=false`
  - if the audit shows `failed_config`, verify the webhook URL locally
  - if the audit shows `failed_network`, treat it as a runtime connectivity problem, not a producer-side failure
- Blocks delivery:
  - `No`

## 8. Dedupe

- Acceptance command:
  - emit two events with the same `dedupeKey`
  - emit a third event with a new `dedupeKey`
  - for Discord-side verification, run `npm.cmd run notification:verify-toxic-flow-daemon`
- Expected result:
  - first event is processed
  - duplicate event is recorded as `skipped_deduped`
  - new key is processed again
- Current status:
  - `PASS`
- Evidence location:
  - `tests/alert_service_tests.rs`
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\tests\rustSidecar.test.ts`
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\tests\toxicFlowJsonlWatcher.test.ts`
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\scripts\verify-toxic-flow-jsonl-daemon.ts`
- Failure triage:
  - if duplicates are not suppressed, compare the exact `dedupeKey`
  - if only monitor-side dedupe works, check the Discord-side watcher dedupe window and audit output
  - if dedupe suppresses too much, inspect reused keys across test runs
- Blocks delivery:
  - `No`

## 9. Checkpoint

- Acceptance command:
  - run the Discord watcher once against a JSONL file with known events
  - stop the watcher
  - start the watcher again against the same path
  - observe the second pass
- Expected result:
  - after restart, already-processed lines are not replayed
  - second pass shows `processed=0`
- Current status:
  - `PASS`
- Evidence location:
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\tests\toxicFlowJsonlWatcher.test.ts`
  - `C:\Users\byhdo_ocup4f5\Documents\discord解决方案\docs\toxic-flow-discord-runtime-acceptance-runbook.md`
- Failure triage:
  - if old events replay, inspect `checkpoint.json`
  - confirm the watcher is pointed at the same base directory
  - if replay was intentional, delete the checkpoint before restarting
- Blocks delivery:
  - `No`

## 10. Repo-wide Verification

- Acceptance command:
  - `cargo fmt --check`
  - `cargo check -j 1`
  - `cargo clippy -j 1 --all-targets -- -D warnings`
  - `cargo test -j 1`
- Expected result:
  - all commands pass under a stable Windows MSVC shell
- Current status:
  - `PASS`
- Evidence location:
  - `docs/windows-rust-build-stability-runbook.md`
  - `tests/replay_heatmap_dashboard_tests.rs`
  - `tests/fixtures/replay_heatmap_ui_spec.json`
  - `web/index.html`
- Failure triage:
  - stable Windows validation requires a `VsDevCmd.bat` shell plus `-j 1` or `CARGO_BUILD_JOBS=1`
  - if `cargo clean` fails with `os error 5`, stop the local `btc-toxic-flow-monitor-rs.exe` process before retrying
  - if Windows-local failures such as `1455`, `rustc panic`, `can't find crate for std`, or metadata-stub errors recur, follow `docs/windows-rust-build-stability-runbook.md`
- Blocks delivery:
  - `No`

## 11. Security Review

- Acceptance command:
  - repo secret scan:
    - `rg -n "DISCORD_WEBHOOK|discord\\.com/api/webhooks|DISCORD_BOT_TOKEN" src tests config docs -S`
  - dev endpoint default-off test:
    - `cargo test --test dev_test_alert_api_tests`
  - signal-only boundary checks:
    - `cargo test --test alert_service_tests`
    - `cargo test --test toxicity_api_safety_contract_tests`
- Expected result:
  - no Discord webhook or bot token is stored in this repository
  - the dev-only test endpoint stays default-off
  - runtime alert acceptance stays signal-only and does not trigger trading execution
- Current status:
  - `PASS`
- Evidence location:
  - `src/api/dev_alert_routes.rs`
  - `src/api/routes.rs`
  - `docs/toxic-signal-operator-runbook.md`
  - `tests/dev_test_alert_api_tests.rs`
  - `tests/alert_service_tests.rs`
  - `tests/toxicity_api_safety_contract_tests.rs`
- Failure triage:
  - if a Discord secret appears in repo search, stop and treat it as a security incident
  - if the dev endpoint answers while disabled, treat it as a regression
  - if any runtime surface shows `executionEnabled=true`, fail the acceptance review immediately
- Blocks delivery:
  - `No`

## 12. Discord HTTP Bridge

- Acceptance command:
  - none for the current delivery scope
- Expected result:
  - if implemented later, it should remain optional and should not replace the working sidecar JSONL bridge by default
- Current status:
  - `OPTIONAL_NOT_STARTED`
- Evidence location:
  - `docs/project-runtime-acceptance-matrix.md`
- Failure triage:
  - none for the current delivery scope
  - do not treat lack of an HTTP bridge as a failure while the sidecar path is passing
- Blocks delivery:
  - `No`

## Recommended Next Action

The repo-wide validation blocker is cleared for the current snapshot.

Recommended follow-up focus:

- keep using the stabilized Windows validation sequence from `docs/windows-rust-build-stability-runbook.md`
- preserve honest release notes about Bybit / OKX remaining `disabled_by_env`
- keep Discord HTTP bridge as optional scope, not a hidden requirement
