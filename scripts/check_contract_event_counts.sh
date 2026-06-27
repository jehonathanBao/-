#!/usr/bin/env bash
set -euo pipefail

BASE="${BASE:-http://127.0.0.1:5173}"
SYMBOL="${SYMBOL:-BTC}"
RANGE="${RANGE:-24h}"
OPERATOR_TOKEN="${OPERATOR_TOKEN:-}"

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

declare -a CURL_HEADERS=()
if [[ -n "$OPERATOR_TOKEN" ]]; then
  CURL_HEADERS=(-H "Authorization: Bearer ${OPERATOR_TOKEN}")
fi

print_json_preview() {
  local name="$1"
  local url="$2"
  local limit="$3"
  local outfile="$tmp_dir/${name}.json"
  local status_file="$tmp_dir/${name}.status"
  echo "== $name =="
  local http_status
  http_status="$(curl -sS -w "%{http_code}" "${CURL_HEADERS[@]}" "$url" -o "$outfile")"
  printf '%s' "$http_status" > "$status_file"
  echo "HTTP $http_status"
  head -c "$limit" "$outfile"
  echo
  echo
}

print_json_preview \
  "contract-events" \
  "$BASE/api/contract-events?symbol=$SYMBOL&range=$RANGE&limit=50" \
  2000
print_json_preview \
  "contract-events include_hidden" \
  "$BASE/api/contract-events?symbol=$SYMBOL&range=$RANGE&limit=50&include_hidden=true" \
  3000
print_json_preview \
  "debug-counts" \
  "$BASE/api/contract-events/debug-counts?symbol=$SYMBOL&range=$RANGE&include_hidden=true" \
  4000
print_json_preview \
  "pipeline-debug" \
  "$BASE/api/contract-whale/pipeline-debug?symbol=$SYMBOL&range=$RANGE" \
  4000
print_json_preview \
  "raw-flow-debug" \
  "$BASE/api/contract-whale/raw-flow-debug?symbol=$SYMBOL&range=$RANGE" \
  4000
print_json_preview \
  "final-events-v2" \
  "$BASE/api/final-events-v2?symbol=$SYMBOL&range=4h&limit=50" \
  3000
print_json_preview \
  "latest" \
  "$BASE/api/contract-whale/latest?symbol=$SYMBOL" \
  3000
