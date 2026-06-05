# Live Data Deployment Checklist

Use this checklist when the monitor is connected to real market data and real notification channels. The bot remains read-only: it emits alerts and dashboard signals only. It must not trade, place orders, move funds, delete data, or mutate production rules from the browser.

## Local Server Configuration

Create or update server-local `.env`. Do not commit it.

```env
DRY_RUN=false
OPERATOR_TOKEN=replace-with-long-random-server-token
WS_SIGNAL_INTERVAL_MS=1000
SCAN_LOG_BUFFER_SIZE=200

DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/REPLACE/REPLACE
DISCORD_PUSH_COOLDOWN_SECONDS=60
TELEGRAM_ENABLED=true
TELEGRAM_BOT_TOKEN=replace-with-server-local-token
TELEGRAM_CHAT_ID=replace-with-chat-id
```

Keep these values server-side only:

- `OPERATOR_TOKEN`
- `DISCORD_WEBHOOK_URL`
- `TELEGRAM_BOT_TOKEN`
- `TELEGRAM_CHAT_ID`

Never expose them as `VITE_*` variables or commit them to Git.

## Git Ignore Checks

```bash
git check-ignore -v .env
git check-ignore -v config/replay.production.local.toml
git check-ignore -v data/production_replay/
```

Expected: each path is ignored.

## Start Backend With Live Data

```bash
docker compose up -d --build backend
curl -fsS http://127.0.0.1:8000/healthz
curl -fsS http://127.0.0.1:8000/readyz
```

## Verify Frontend

Use your SSH tunnel or reverse proxy URL. Example:

```text
http://127.0.0.1:55173/signals
```

Expected:

- `Live: connected`
- `扫描日志: connected`
- High / critical candidates appear in the primary list.
- Medium candidates remain folded by default.
- Low candidates do not appear in the primary list.
- Refreshing the page does not restart `toxic-bot`.

## Notification Boundary

Real notification sending is acceptable for this read-only bot, but messages must remain redacted:

- allowed: symbol, event/detector type, direction, final result, risk score, data quality
- forbidden: raw evidence, markout fields, stale flags, webhook URLs, tokens, authorization headers, raw payloads

Check logs:

```bash
docker compose logs -f backend
```

Check the Dashboard scan log panel. Expected events include startup, data-source
connection, candidate snapshot count, and Discord push queued / sent / skipped /
failed summaries. The panel must not display raw payloads, evidence, markout,
authorization headers, tokens, webhook URLs, or Telegram secrets.

Do not paste logs containing real tokens into tickets or chat.

## Live Readiness Script

Run:

```bash
./scripts/check-live-deployment.sh
```

PowerShell:

```powershell
.\scripts\check-live-deployment.ps1
```

The script reports whether live notification env vars are present without printing their values.

## Bot Continuity Check

```bash
docker inspect -f '{{.State.StartedAt}}' toxic-bot
docker inspect -f '{{.RestartCount}}' toxic-bot
docker compose restart frontend
docker inspect -f '{{.State.StartedAt}}' toxic-bot
docker inspect -f '{{.RestartCount}}' toxic-bot
```

Expected after frontend restart:

- `StartedAt` unchanged
- `RestartCount` unchanged

Backend rebuilds are different: updating Rust backend code and rebuilding `backend` will restart `toxic-bot` once by design.
