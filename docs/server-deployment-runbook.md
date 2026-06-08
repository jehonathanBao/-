# Server Deployment Runbook

This deployment keeps the Rust monitor process independent from the React/Vite frontend. Browser refreshes and Vite HMR reloads do not restart the backend container.

## Directory Layout

```text
有毒订单监控-rs/
├── Dockerfile.backend
├── docker-compose.yml
├── config/
│   └── default.toml
├── data/
│   ├── production_replay/
│   └── reports/
├── docs/
├── src/
└── toxic-order-monitor/
    ├── Dockerfile.frontend
    ├── package.json
    ├── vite.config.js
    └── src/
```

## Services

- `backend`: Rust monitor and API server.
- `frontend`: Vite dev server with HMR.

The backend listens on `0.0.0.0:3000` inside Compose and is exposed only on host loopback port `127.0.0.1:8000`.

The frontend listens on `0.0.0.0:5173` inside Compose and is exposed on host port `0.0.0.0:5173` by default so other terminals on the network can open the dashboard. Set `DASHBOARD_BIND_HOST=127.0.0.1` before `docker compose up` if you need to restrict it back to local-only access.

Open:

```text
http://<server-ip>:5173
```

The frontend calls `/api/...` with relative URLs. Vite proxies API calls to `http://backend:3000`.
It also proxies `/ws/signals` and `/ws/scan-logs` to the backend so browser refreshes reconnect without restarting `toxic-bot`.
The backend and operator token are still kept server-side; browsers should not call `http://<server-ip>:8000` directly.

## Required Token

When the Rust API binds to a non-loopback address, `/api` requests require an operator token.

PowerShell:

```powershell
$env:OPERATOR_TOKEN = "replace-with-a-strong-local-token"
docker compose up -d --build
```

Bash:

```bash
export OPERATOR_TOKEN="replace-with-a-strong-local-token"
docker compose up -d --build
```

Do not put this token in `VITE_*` env vars. The Vite proxy injects it server-side as `x-operator-api-token`; the browser bundle should not receive it.

## WebSocket Authentication Model

The browser must not receive `OPERATOR_TOKEN`.

Development / current Compose mode:

- Browser connects to the frontend origin: `/ws/signals`.
- Vite proxy forwards `/ws/signals` to `ws://backend:3000`.
- Vite proxy injects `x-operator-api-token` server-side.
- Backend validates the token for non-loopback WS requests.

Production static frontend mode:

- Do not connect the browser directly to backend `/ws/signals` with `OPERATOR_TOKEN`.
- Use a reverse proxy such as Nginx, Caddy, or Traefik to inject `x-operator-api-token` server-side.
- Or implement a cookie/session-based auth layer.

See `docs/reverse-proxy-production-example.md` for a placeholder-only Nginx example.

## Logs

```powershell
docker compose logs -f backend
docker compose logs -f frontend
```

Keep Discord and Telegram webhook URLs out of logs.
The Dashboard scan log panel displays sanitized startup, scan, candidate, and Discord push status only.

For live notification deployment, see `docs/live-data-deployment-checklist.md`.

## Persistent Data

The Compose file mounts:

```text
./data:/app/data
./config:/app/config:ro
```

Runtime state and reports should stay under `./data`, including:

- SQLite runtime database
- replay reports
- production replay inputs
- calibration reports

Real production replay files are ignored by git.

## Frontend HMR

The frontend container runs:

```text
npm run dev -- --host 0.0.0.0
```

Refreshing the browser or triggering Vite HMR affects only the frontend session. It does not restart the Rust backend container.
The expected deployment boundary is that browser refresh and HMR never interrupt the backend bot runtime.

## WebSocket Boundary

`/ws/signals` streams redacted toxic signal snapshots to the Dashboard. `/ws/scan-logs` streams sanitized runtime scan logs. Both use the same non-loopback token boundary as `/api` and should normally be reached through the Vite `/ws` proxy.

`toxic-order-monitor/src/hooks/useReconnectingWebSocket.js` reconnects automatically with exponential backoff. The Dashboard merges incoming snapshots into the same persistent inbox used by HTTP polling.

Keep the stream read-only:

- keep it read-only
- require the same operator token policy as `/api`
- send redacted signal summaries only
- do not send markout fields, raw evidence, stale flags, tokens, webhook URLs, or private payloads
- tune `WS_SIGNAL_INTERVAL_MS` for snapshot frequency
- tune `SCAN_LOG_BUFFER_SIZE` for the in-memory scan log ring buffer
- tune `TOF_SCAN_LOG_INTERVAL_SECONDS` to avoid metrics summary spam
- keep `DISCORD_AUTO_PUSH_ENABLED=true` for real High/Critical candidate notifications
- keep `DISCORD_AUTO_PUSH_CACHED_ON_BOOT=false` to avoid restart-time cached candidate bursts
- keep `DISCORD_PUSH_COOLDOWN_SECONDS=60` or higher for real notification channels

## TOF-Lite Metrics

The backend augments signal inbox and WebSocket snapshots with TOF-lite aggregate
metrics:

```text
tofMetrics, tofScore, finalRiskScore, candidateType, explainTags, directionConfidence
```

These fields are read-only operator summaries. They must not include raw order
books, raw trades, evidence, markout, tokens, or webhook values. If real L2 /
trade data is incomplete, TOF-lite falls back to the existing risk score and
lower metrics completeness.

## Contract-Side TOF Metrics

v0.5 also adds read-only perp-side TOF aggregate metrics to the same inbox,
WebSocket, Discord payload, scan log, and Dashboard surfaces:

```text
perpTofMetrics, perpScore, perpCandidateType, finalCandidateType, metricsDirection, mergedConfidence
```

The supported candidate families are `OpenInterestCandidate`,
`CrowdedLongCandidate`, `CrowdedShortCandidate`, `LongSqueezeCandidate`,
`ShortSqueezeCandidate`, and `AggressiveOrderFlowCandidate`. These fields are
safe summaries only: OI change, funding rate, liquidation pressure, aggressive
buy/sell volume, direction, score, data quality, and explain tags. Do not send
raw payloads, raw evidence, markout, tokens, or webhook values.

Useful defaults:

```env
PERP_TOF_ENABLED=true
PERP_OI_BUCKET_SIZE=100000
PERP_FUNDING_THRESHOLD=0.05
PERP_LIQUIDATION_WINDOW=5m
PERP_AGF_VOLUME_THRESHOLD=1000000
ADVANCED_TOF_ENABLED=true
```

High/Critical Discord delivery still uses the same alert gate: score `>= 80`,
data quality `>= 70`, dedupe, and cooldown. Medium and Low contract candidates
remain Dashboard-only.

## Advanced TOF Fusion

The v0.6 advanced layer merges spot risk, spot TOF-lite, and perp TOF into a
single read-only candidate summary:

```text
advancedTofMetrics, advancedScore, advancedCandidateType
```

The fusion formula is:

```text
finalRiskScore = 0.4 * spotRisk + 0.3 * spotTofScore + 0.3 * perpScore
```

Advanced indicators include VPIN Enhanced, large order-flow clusters,
historical Funding / OI trend, and market pressure heatmap. Scan logs emit
`advanced_metrics_computed` with aggregate scores only. Discord embeds may show
these aggregate indicators and explain tags, but must still omit raw payloads,
evidence, markout, tokens, and webhook values.

## Remote Access

For remote browser access, prefer a reverse proxy with TLS in front of the frontend and backend.

Do not expose the backend API publicly without:

- HTTPS
- a strong `OPERATOR_TOKEN`
- server-side token injection or an authenticated gateway
- restricted CORS origins
- log redaction

## Stop

```powershell
docker compose down
```

To keep data, do not delete `./data`.

## Validation

```powershell
docker compose config
docker compose build
docker compose up -d
docker compose logs -f backend
docker compose logs -f frontend
```

Then verify:

```text
http://localhost:5173
http://localhost:8000/api/status
ws://localhost:5173/ws/signals
ws://localhost:5173/ws/scan-logs
```

Direct `http://localhost:8000/api/status`, `ws://localhost:8000/ws/signals`,
and `ws://localhost:8000/ws/scan-logs` may require an operator token when the
backend is bound to `0.0.0.0`. The frontend proxy path should inject it
automatically.
