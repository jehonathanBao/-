# Production Replay Runbook

This runbook is for local, read-only replay of real L2/trade data. It must not enable execution, deletion, deployment, trading, payment, or irreversible actions.

## Inputs

Put real production replay files under:

```text
data/production_replay/
```

Supported input files:

- JSONL
- CSV

Each event should include:

- `event_type`
- `venue`
- `symbol`
- `ts_ms`
- `side`
- `price`
- `qty`
- `qty_before`
- `qty_after`
- `sequence`

Optional fields:

- `trade_id`
- `order_id`

`snapshot_reset` events must be treated as state resets and must not count as cancel evidence.

## Local Config

Copy the example config to the ignored local config:

```powershell
Copy-Item config/replay.production.example.toml config/replay.production.local.toml
```

Update:

- `input.path`
- `input.venue`
- `input.symbol`
- `output.report_dir`

Do not commit `config/replay.production.local.toml` or production replay data.

## Run

PowerShell:

```powershell
scripts/run_production_replay.ps1
```

Bash:

```bash
scripts/run_production_replay.sh
```

Direct cargo command:

```powershell
cargo run --bin replay_production -- --config config/replay.production.local.toml
```

## Expected Outputs

The report directory should contain:

- `summary.json`
- `signals.json`
- `calibration.json`
- `calibration.md`
- `high_score_candidates.csv`
- `possible_false_positives.csv`
- `possible_false_negatives.csv`

CSV outputs must escape formula-prefixed cells before spreadsheet import.

## Alert Boundary

- High and critical candidates may be considered for Discord or Telegram only when score `>= 80` and data quality `>= 70`.
- Medium candidates should appear in calibration reports and the frontend folded inbox only.
- Low candidates should remain display-only unless explicitly filtered.
- Alert payloads should include only symbol, event/detector type, direction, final result description, risk score, and data quality.
- Do not include raw evidence, markout fields, stale flags, tokens, webhook URLs, or raw data rows in alert payloads.

## Validation

Run the safe local checks:

```powershell
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features --no-fail-fast -j 1
cd toxic-order-monitor
npm run build
npm run test
npm audit --audit-level=high
```

If Rust compilation fails on Windows, check:

- MSVC linker `link.exe` is installed and visible in the shell.
- Cargo registry cache is not corrupted. Remove the broken crate cache and retry if modules are missing from a crate source directory.

## Acceptance Template

```text
[PASS] Production Replay Run
[PASS] Signals Written to Inbox
[PASS] High/CRITICAL Candidates Sent to Discord
[PASS] Medium Candidates Only Displayed
[PASS] Persistent Inbox Maintains Signals
[PASS] WebSocket Signal Stream Emits Redacted Snapshots
[PASS] Clear Cache Works
[PASS] Frontend Build
[PASS] Frontend Tests Passed
[PASS] npm audit high: 0 vulnerabilities
[WARN] Real production L2/trade data not provided
[WARN] Score weight tuning requires human labels
```

Use `[BLOCKED]` instead of `[PASS]` for any item that cannot run because local Rust tooling, real data, labels, or alert credentials are unavailable.
