#!/usr/bin/env bash
set -euo pipefail

export OPERATOR_TOKEN="${OPERATOR_TOKEN:-dummy-local-smoke-token}"
export WS_SIGNAL_INTERVAL_MS="${WS_SIGNAL_INTERVAL_MS:-1000}"
export DRY_RUN="${DRY_RUN:-true}"
export DISCORD_WEBHOOK_URL="${DISCORD_WEBHOOK_URL:-}"
export TELEGRAM_BOT_TOKEN="${TELEGRAM_BOT_TOKEN:-}"
export TELEGRAM_CHAT_ID="${TELEGRAM_CHAT_ID:-}"

docker compose config
docker compose up -d --build

wait_for_http() {
  local url="$1"
  local label="$2"
  local attempts="${3:-60}"
  echo "[smoke] waiting for ${label}..."
  for _ in $(seq 1 "$attempts"); do
    if curl -fsS "$url" >/dev/null; then
      return 0
    fi
    sleep 1
  done
  echo "[smoke] ${label} did not become ready: ${url}" >&2
  return 1
}

backend_started_at() {
  docker inspect -f '{{.State.StartedAt}}' toxic-bot
}

backend_restart_count() {
  docker inspect -f '{{.RestartCount}}' toxic-bot
}

echo "[smoke] checking containers..."
docker compose ps

echo "[smoke] checking frontend..."
wait_for_http http://127.0.0.1:5173/ "frontend"

echo "[smoke] checking backend health..."
wait_for_http http://127.0.0.1:8000/healthz "backend /healthz"
wait_for_http http://127.0.0.1:8000/readyz "backend /readyz"

echo "[smoke] checking backend API with server-side token..."
curl -fsS \
  -H "x-operator-api-token: ${OPERATOR_TOKEN}" \
  http://127.0.0.1:8000/api/toxicity/signal-inbox/recent >/dev/null

echo "[smoke] checking token is not exposed in frontend HTML..."
if curl -fsS http://127.0.0.1:5173/ | grep -q "${OPERATOR_TOKEN}"; then
  echo "OPERATOR_TOKEN leaked in frontend HTML"
  exit 1
fi

echo "[smoke] checking token is not present in frontend app files..."
docker compose exec -T -e SMOKE_TOKEN="${OPERATOR_TOKEN}" frontend sh -lc \
  'if grep -R -- "$SMOKE_TOKEN" /app --exclude-dir=node_modules --exclude-dir=.vite --exclude-dir=dist >/dev/null 2>&1; then echo "OPERATOR_TOKEN leaked in frontend files"; exit 1; fi'

echo "[smoke] checking websocket through frontend proxy..."
docker compose exec -T frontend node --input-type=module -e '
const url = "ws://127.0.0.1:5173/ws/signals";
const timeout = setTimeout(() => {
  console.error("websocket smoke timed out");
  process.exit(1);
}, 10000);
const ws = new WebSocket(url);
ws.onmessage = (event) => {
  const payload = JSON.parse(event.data);
  if (payload.type !== "signal_snapshot") {
    console.error("unexpected websocket payload type", payload.type);
    process.exit(1);
  }
  clearTimeout(timeout);
  ws.close();
  process.exit(0);
};
ws.onerror = (event) => {
  console.error("websocket smoke failed", event?.message || "");
  process.exit(1);
};
'

started_before="$(backend_started_at)"
restart_before="$(backend_restart_count)"
echo "[smoke] backend StartedAt before frontend refresh: ${started_before}"
echo "[smoke] backend RestartCount before frontend refresh: ${restart_before}"

echo "[smoke] simulating frontend refresh requests..."
for _ in $(seq 1 20); do
  curl -fsS http://127.0.0.1:5173/ >/dev/null
  sleep 1
done

started_after_refresh="$(backend_started_at)"
restart_after_refresh="$(backend_restart_count)"
if [[ "$started_after_refresh" != "$started_before" ]]; then
  echo "[smoke] backend StartedAt changed after frontend refresh" >&2
  exit 1
fi
if [[ "$restart_after_refresh" != "$restart_before" ]]; then
  echo "[smoke] backend RestartCount changed after frontend refresh" >&2
  exit 1
fi

echo "[smoke] restarting frontend and checking backend stays up..."
docker compose restart frontend
wait_for_http http://127.0.0.1:5173/ "frontend after restart"
started_after_frontend_restart="$(backend_started_at)"
restart_after_frontend_restart="$(backend_restart_count)"
if [[ "$started_after_frontend_restart" != "$started_before" ]]; then
  echo "[smoke] backend StartedAt changed after frontend restart" >&2
  exit 1
fi
if [[ "$restart_after_frontend_restart" != "$restart_before" ]]; then
  echo "[smoke] backend RestartCount changed after frontend restart" >&2
  exit 1
fi

echo "[smoke] checking data volume mount..."
docker compose exec -T backend sh -lc "test -d /app/data && test -d /app/config"

echo "[smoke] final backend container state..."
docker compose ps backend

echo "[smoke] done"
