# 有毒订单监控-rs

Rust-based order monitoring system.

## Detection Engine Limitations

This project emits toxic-order risk candidates for read-only monitoring. The
signals are not legal, compliance, or enforcement conclusions.

- Public market data without native order IDs can only support probabilistic
  price-level evidence.
- Spoofing, Layering, and Iceberg labels are emitted as candidates unless a
  complete order-lifecycle evidence chain is available.
- Dashboard alerts represent risk candidates and should not be treated as
  confirmed market manipulation.
- Candidate confidence is downgraded when markout, fill participation, venue
  reliability, or order-level evidence is missing.

## Replay & Calibration

Candidate replay is a read-only validation loop for synthetic or historical L2
market events. It does not affect the live detection path.

- Replay fixtures live under `fixtures/replay/`.
- Replay input supports `.jsonl` and simple header-based `.csv` files.
- Supported replay event types are `trade`, `book_delta`, `snapshot`, and
  `snapshot_reset`.
- `snapshot_reset` is never treated as cancel evidence.
- Markout evaluation records 1s / 5s / 30s directional movement when enough
  future price data exists; missing future data remains `None`.
- Calibration reports group signals by score bucket, detector, venue data
  quality, high-score/no-markout samples, and low-score/high-markout samples.
- Current fixtures are synthetic examples and do not represent real exchange
  behavior. Before production confidence upgrades, replay must use real
  historical L2 / trade data and reviewed labels.
- Production validation should call `run_candidate_calibration_file(path)` with
  externally prepared historical data; the replay engine remains read-only and
  does not write DB / JSONL / SQLite / archive outputs.
- CI runs candidate replay, markout, and calibration tests so the validation
  loop stays healthy across pull requests.

Example test entrypoint:

```powershell
cargo test --test candidate_replay_runner_tests
cargo test --test candidate_markout_evaluator_tests
cargo test --test candidate_calibration_report_tests
```

## Production Replay & Calibration

Production replay turns externally prepared historical L2 / trade data into a
read-only candidate validation report. Real market data is intentionally ignored
by git and should be placed under `data/production_replay/`.

Supported formats:

- JSONL using the candidate replay schema.
- Simple header-based CSV with fields such as `type`, `venue`, `symbol`,
  `tsMs`, `side`, `price`, `qty`, `qtyBefore`, `qtyAfter`, `sequence`,
  `tradeId`, `orderId`, and `aggressorSide`.

Run locally after placing real data at the configured input path:

```powershell
cargo run --bin replay_production -- --config config/replay.production.example.toml
```

Optional helper scripts:

```powershell
scripts/run_production_replay.ps1
```

```bash
scripts/run_production_replay.sh
```

Reports are written under:

```text
data/production_replay/reports/<venue>_<symbol>_<timestamp>/
```

Report files:

- `summary.json`: event counts, signal counts, venue / symbol / detector
  breakdowns, and read-only candidate notice.
- `signals.json`: generated `ToxicSignal` candidate records.
- `calibration.json`: score buckets, detector markout, venue data quality, and
  FP/FN candidate samples.
- `calibration.md`: human-readable summary and recommendation.
- `high_score_candidates.csv`: candidates meeting configured `score` and
  `dataQuality` gates.
- `possible_false_positives.csv`: high-score candidates with weak or missing
  markout.
- `possible_false_negatives.csv`: lower-score candidates with stronger adverse
  markout.

Interpretation rules:

- `high_score_candidates` are priority review candidates, not confirmed
  manipulation.
- `possible_false_positives` should be reviewed before reducing sensitivity.
- `possible_false_negatives` should be reviewed before increasing sensitivity.
- Without real historical L2 / trade data, production replay validation is not
  complete.
- Without reviewed FP/FN labels, scoreBreakdown weights must not be changed
  automatically.
- Discord only sends high / critical candidates that meet the configured score
  and data-quality gate.
- Discord messages include event type, detector type, direction, final result
  description, risk score, data quality, and an explicit candidate-only notice.
- Discord messages must not include raw evidence, markout fields, stale flags,
  webhook URLs, or private payloads.

Alert gate defaults:

```env
ALERT_MIN_SCORE=80
ALERT_MIN_DATA_QUALITY=70
VITE_ALERT_MIN_SCORE=80
VITE_ALERT_MIN_DATA_QUALITY=70
```

## API Security

Default local mode binds the API to `127.0.0.1` and keeps the dashboard usable
without a token.

When binding to a non-loopback address such as `0.0.0.0`, protect operator API
reads and writes with an operator token:

```env
API_HOST=0.0.0.0
OPERATOR_TOKEN=<strong local token>
ALLOW_LAN_DASHBOARD=true
ALLOWED_DASHBOARD_ORIGIN=http://127.0.0.1:3000
```

Requests should send:

```text
X-Operator-Token: <strong local token>
```

Do not put `OPERATOR_TOKEN` in Vite env or browser bundles. For shared or remote
access, prefer VPN, SSH tunnel, or a reverse proxy such as Nginx with Basic Auth
that injects the operator header server-side.

## Docker Deployment

See `docs/server-deployment-runbook.md` for the Compose deployment. The frontend
Vite server runs separately from the Rust backend, so browser refresh and HMR do
not restart the monitor process. The Compose setup keeps `OPERATOR_TOKEN`
server-side in the Vite proxy instead of exposing it as a `VITE_*` browser env.
The optional `/ws/signals` stream sends redacted signal summaries only and is
covered by the same non-loopback token boundary as `/api`.
The Dashboard also uses `/api/runtime/scan-log/recent` and `/ws/scan-logs` for
the left-side scan log panel. Those endpoints are read-only, share the same
operator-token boundary, and emit sanitized runtime summaries only.

### WebSocket Authentication Model

The browser must not receive `OPERATOR_TOKEN`.

Development / current Compose mode:

- Browser connects to the frontend origin: `/ws/signals`.
- Browser connects to the frontend origin: `/ws/signals` and `/ws/scan-logs`.
- Vite proxy forwards `/ws/*` to the backend.
- Vite proxy injects `x-operator-api-token` server-side.
- Backend validates the token for non-loopback WS requests.

Production static frontend mode:

- Do not connect the browser directly to backend `/ws/signals` or
  `/ws/scan-logs` with
  `OPERATOR_TOKEN`.
- Use a reverse proxy such as Nginx, Caddy, or Traefik to inject
  `x-operator-api-token` server-side.
- Or implement a cookie/session-based auth layer.

See `docs/reverse-proxy-production-example.md` for a placeholder-only Nginx
example. Do not commit real tokens.

### Runtime Scan Log

The scan log panel is an operator-facing, read-only status feed for startup,
market-data scanning, signal snapshots, and Discord push decisions. Configure
the in-memory ring buffer with:

```env
SCAN_LOG_BUFFER_SIZE=200
DISCORD_PUSH_COOLDOWN_SECONDS=60
```

Scan log payloads must not contain operator tokens, Discord / Telegram secrets,
authorization headers, raw payloads, evidence, markout, or webhook URLs. The
local "清空显示" button only clears the browser panel; it does not clear the
backend buffer.
