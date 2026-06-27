#!/usr/bin/env bash
set -euo pipefail

BASE="${BASE:-http://127.0.0.1:5173}"
SYMBOL="${SYMBOL:-BTC}"
RANGE="${RANGE:-24h}"

echo "== contract-events =="
curl -sS "$BASE/api/contract-events?symbol=$SYMBOL&range=$RANGE&limit=50" | head -c 2000
echo
echo

echo "== contract-events include_hidden =="
curl -sS "$BASE/api/contract-events?symbol=$SYMBOL&range=$RANGE&limit=50&include_hidden=true" | head -c 3000
echo
echo

echo "== debug-counts =="
curl -sS "$BASE/api/contract-events/debug-counts?symbol=$SYMBOL&range=$RANGE&include_hidden=true" | head -c 4000
echo
echo

echo "== final-events-v2 =="
curl -sS "$BASE/api/final-events-v2?symbol=$SYMBOL&range=4h&limit=50" | head -c 3000
echo
echo

echo "== latest =="
curl -sS "$BASE/api/contract-whale/latest?symbol=$SYMBOL" | head -c 3000
echo
