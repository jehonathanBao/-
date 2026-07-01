#!/usr/bin/env bash
set -euo pipefail

DB_PATH="${DB_PATH:-}"
OUT="${OUT:-/tmp/sqlite_cleanup_plan_$(date +%F_%H%M%S).md}"
SQLITE_TIMEOUT="${SQLITE_TIMEOUT:-30}"

if [[ -z "${DB_PATH}" ]]; then
  echo "DB_PATH is required" >&2
  echo "Example: DB_PATH=/path/to/btc-toxic-flow.sqlite $0" >&2
  exit 1
fi

if [[ ! -f "${DB_PATH}" ]]; then
  echo "DB not found: ${DB_PATH}" >&2
  exit 1
fi

if ! command -v timeout >/dev/null 2>&1; then
  echo "timeout is required on PATH" >&2
  exit 1
fi

if command -v sqlite3 >/dev/null 2>&1; then
  SQL_BACKEND="sqlite3"
elif command -v python3 >/dev/null 2>&1; then
  SQL_BACKEND="python3"
else
  echo "sqlite3 or python3 is required on PATH" >&2
  exit 1
fi

NOW_MS="$(( $(date +%s) * 1000 ))"

PLAN_SPECS=(
  "contract_flow_1s|7|ts_bucket|P0|contract raw flow buckets"
  "contract_liquidation_1s|7|ts_bucket|P0|contract liquidation buckets"
  "contract_oi_snapshots|30|ts|P1|OI snapshots"
  "contract_funding_snapshots|90|ts|P1|funding snapshots"
  "contract_whale_percentile_thresholds|30|computed_at|P1|CWM percentile thresholds"
  "toxic_events|30|ts|P1|toxic events"
  "toxic_snapshots|7|ts|P0|toxic snapshots"
  "flow_snapshots|7|ts|P0|flow snapshots"
  "venue_health_snapshots|7|ts|P1|venue health snapshots"
  "vpin_buckets|30|end_ts|P1|VPIN buckets"
  "replay_runs|30|COALESCE(finished_at, started_at)|P2|replay runs"
  "contract_whale_signals|365|ts|P2|protected contract whale signals"
  "spot_whale_signals|30|ts|P2|protected spot whale signals"
)

run_sql() {
  local sql="$1"
  if [[ "${SQL_BACKEND}" == "sqlite3" ]]; then
    timeout "${SQLITE_TIMEOUT}s" sqlite3 -readonly -noheader "${DB_PATH}" "${sql}" 2>/dev/null || true
  else
    SQL_QUERY="${sql}" timeout "${SQLITE_TIMEOUT}s" python3 - "${DB_PATH}" <<'PY' 2>/dev/null || true
import os
import sqlite3
import sys

db_path = sys.argv[1]
sql = os.environ["SQL_QUERY"]
conn = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True, timeout=5)
cur = conn.execute(sql)
for row in cur.fetchall():
    print("|".join("" if value is None else str(value) for value in row))
PY
  fi
}

table_exists() {
  local table="$1"
  [[ "$(run_sql "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='${table}';")" == "1" ]]
}

list_columns() {
  local table="$1"
  run_sql "PRAGMA table_info(${table});" | awk -F'|' '{print $2}'
}

column_exists() {
  local table="$1"
  local column="$2"
  list_columns "${table}" | grep -Fxq "${column}"
}

pick_time_expr() {
  local table="$1"
  local preferred="$2"
  if [[ -n "${preferred}" ]]; then
    if [[ "${preferred}" == *"COALESCE("* ]]; then
      echo "${preferred}"
      return 0
    fi
    if column_exists "${table}" "${preferred}"; then
      echo "${preferred}"
      return 0
    fi
  fi
  local candidates=(ts_bucket ts created_at updated_at end_ts computed_at started_at)
  local candidate
  for candidate in "${candidates[@]}"; do
    if column_exists "${table}" "${candidate}"; then
      echo "${candidate}"
      return 0
    fi
  done
  echo ""
}

scalar_or_na() {
  local sql="$1"
  local result
  result="$(run_sql "${sql}" | head -n 1 | tr -d '\r')"
  if [[ -z "${result}" ]]; then
    echo "N/A"
  else
    echo "${result}"
  fi
}

estimate_delete_where() {
  local table="$1"
  local time_expr="$2"
  local cutoff_ms="$3"
  case "${table}" in
    contract_whale_signals)
      echo "${time_expr} < ${cutoff_ms} AND severity <> 'S' AND ABS(COALESCE(net_volume_btc, 0)) < 500"
      ;;
    spot_whale_signals)
      if column_exists "${table}" "is_permanent"; then
        echo "${time_expr} < ${cutoff_ms} AND COALESCE(is_permanent, 0) = 0 AND ABS(COALESCE(net_volume_base, 0)) < 50"
      else
        echo "${time_expr} < ${cutoff_ms} AND ABS(COALESCE(net_volume_base, 0)) < 50"
      fi
      ;;
    *)
      echo "${time_expr} < ${cutoff_ms}"
      ;;
  esac
}

{
  echo "# Cleanup Plan Preview"
  echo
  echo "- generated_at: $(date -Is)"
  echo "- db_path: ${DB_PATH}"
  echo "- mode: readonly"
  echo

  echo "## Candidate Deletes"
  echo
  printf "| table | priority | retention_days | rows_total | rows_deletable | delete_condition |\n"
  printf "|---|---:|---:|---:|---:|---|\n"

  for spec in "${PLAN_SPECS[@]}"; do
    IFS='|' read -r table retention_days preferred_time_expr priority purpose <<<"${spec}"
    if ! table_exists "${table}"; then
      printf "| %s | %s | %s | missing | - | table missing |\n" "${table}" "${priority}" "${retention_days}"
      continue
    fi
    time_expr="$(pick_time_expr "${table}" "${preferred_time_expr}")"
    rows_total="$(scalar_or_na "SELECT COUNT(*) FROM ${table};")"
    if [[ -z "${time_expr}" ]]; then
      printf "| %s | %s | %s | %s | N/A | no timestamp column detected |\n" \
        "${table}" "${priority}" "${retention_days}" "${rows_total}"
      continue
    fi
    cutoff_ms="$((NOW_MS - retention_days * 24 * 60 * 60 * 1000))"
    delete_where="$(estimate_delete_where "${table}" "${time_expr}" "${cutoff_ms}")"
    rows_deletable="$(scalar_or_na "SELECT COUNT(*) FROM ${table} WHERE ${delete_where};")"
    printf "| %s | %s | %s | %s | %s | %s |\n" \
      "${table}" "${priority}" "${retention_days}" "${rows_total}" "${rows_deletable}" "${delete_where}"
  done
  echo

  cat <<'EOF'
## Maintenance Window Guardrails

1. Stop backend writes before any DELETE.
2. Confirm an external backup or VPS snapshot exists before mutation.
3. Use batched rowid deletes only; never run one giant DELETE.
4. Run WAL checkpoint only after writes are stopped and batched deletes finish.
5. VACUUM INTO is allowed only with external disk capacity >= current DB size * 1.2.

## Explicitly Not Executed

- DELETE
- VACUUM
- VACUUM INTO
- PRAGMA wal_checkpoint
- service stop/restart
- any mutation of SQLite files
EOF
} >"${OUT}"

echo "Plan written to ${OUT}"
