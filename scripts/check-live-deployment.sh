#!/usr/bin/env bash
set -euo pipefail

echo "[live] validating compose without printing resolved secrets..."
docker compose config --quiet 2>/dev/null || docker compose config >/dev/null

echo "[live] checking ignored local secret/data paths..."
repo_dir="$(pwd)"
git -c "safe.directory=${repo_dir}" check-ignore -q .env
git -c "safe.directory=${repo_dir}" check-ignore -q config/replay.production.local.toml

if git -c "safe.directory=${repo_dir}" ls-files --error-unmatch .env >/dev/null 2>&1; then
  echo "[live] .env is tracked by git; remove it from the index before deploying" >&2
  exit 1
fi

if grep -R "VITE_.*TOKEN\|VITE_API_TOKEN\|VITE_OPERATOR_TOKEN" \
  docker-compose.yml .env.example toxic-order-monitor/vite.config.js >/dev/null 2>&1; then
  echo "[live] found forbidden VITE token environment name" >&2
  exit 1
fi

echo "[live] checking backend health endpoints if containers are running..."
if [ -n "$(docker compose ps -q --status running backend 2>/dev/null)" ]; then
  curl -fsS http://127.0.0.1:8000/healthz >/dev/null
  curl -fsS http://127.0.0.1:8000/readyz >/dev/null
fi

echo "[live] checking backend notification env presence without printing values..."
if [ -n "$(docker compose ps -q --status running backend 2>/dev/null)" ]; then
  docker compose exec -T backend sh -lc '
    if [ "${DRY_RUN:-true}" = "false" ]; then echo "[live] DRY_RUN=false"; else echo "[live] DRY_RUN is not false"; fi
    if [ -n "${DISCORD_WEBHOOK_URL:-}" ]; then echo "[live] Discord webhook configured"; else echo "[live] Discord webhook not configured"; fi
    if [ -n "${TELEGRAM_BOT_TOKEN:-}" ] && [ -n "${TELEGRAM_CHAT_ID:-}" ]; then echo "[live] Telegram configured"; else echo "[live] Telegram not fully configured"; fi
  '
fi

echo "[live] done"
