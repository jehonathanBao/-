#!/usr/bin/env bash
set -euo pipefail

BASE="${BASE:-http://127.0.0.1:5173}"

echo "== Docker compose status =="
docker compose ps || true

echo "== Frontend routes =="
for path in / /dashboard /contract-whale /spot-monitor /signal-history; do
  echo "--- $path"
  curl -fsS -o /tmp/frontend_check.html -w "status=%{http_code} time=%{time_total} content_type=%{content_type}\n" "$BASE$path" || true
  head -c 120 /tmp/frontend_check.html || true
  echo
done

echo "== API routes =="
for path in \
  "/api/contract-events?symbol=BTC&range=24h&limit=1" \
  "/api/final-events-v2?symbol=BTC&range=4h&limit=1" \
  "/api/contract-retention-status"
do
  echo "--- $path"
  curl -sS -o /tmp/api_check.json -w "status=%{http_code} time=%{time_total} content_type=%{content_type}\n" "$BASE$path" || true
  head -c 300 /tmp/api_check.json || true
  echo
done

echo "== Recent logs =="
docker compose logs --tail=80 frontend || true
docker compose logs --tail=80 backend || true
