# Production Smoke Checklist

Use this checklist after deploying with Docker Compose or a server reverse proxy. The smoke path is read-only and must not send Discord or Telegram messages.

## Safe Defaults

```text
OPERATOR_TOKEN=dummy-local-smoke-token
WS_SIGNAL_INTERVAL_MS=1000
DRY_RUN=true
DISCORD_WEBHOOK_URL=
TELEGRAM_BOT_TOKEN=
TELEGRAM_CHAT_ID=
```

## Checks

- Compose config renders successfully.
- Backend container starts and stays running.
- Frontend container starts and serves the Dashboard.
- `/healthz` returns 200.
- `/readyz` returns 200.
- `/api/toxicity/signal-inbox/recent` is reachable through the server-side token path.
- `/ws/signals` is reachable through the proxy path.
- Frontend refresh does not restart the backend container.
- Frontend container restart does not restart the backend container.
- `./data` is mounted to `/app/data`.
- `./config` is mounted to `/app/config`.
- Frontend HTML and JS do not contain `OPERATOR_TOKEN`.
- Discord and Telegram env vars are empty during smoke unless explicitly testing a non-production webhook.
- Backend host port is bound to `127.0.0.1:8000` unless a reverse proxy or firewall policy explicitly says otherwise.

## Commands

PowerShell:

```powershell
$env:OPERATOR_TOKEN="dummy-local-smoke-token"
$env:WS_SIGNAL_INTERVAL_MS="1000"
$env:DRY_RUN="true"
.\scripts\smoke-compose.ps1
```

Bash:

```bash
OPERATOR_TOKEN=dummy-local-smoke-token \
WS_SIGNAL_INTERVAL_MS=1000 \
DRY_RUN=true \
./scripts/smoke-compose.sh
```

Stop manually when finished:

```bash
docker compose down
```

## Evidence To Record

```text
StartedAt before frontend refresh:
StartedAt after frontend refresh:
RestartCount before frontend refresh:
RestartCount after frontend refresh:
RestartCount after frontend restart:
WebSocket proxy smoke result:
Token leak check result:
```
