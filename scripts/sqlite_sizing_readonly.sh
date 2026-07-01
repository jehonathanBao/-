#!/usr/bin/env bash
set -euo pipefail

DB_PATH="${DB_PATH:-}"
OUT="${OUT:-/tmp/sqlite_sizing_report_$(date +%F_%H%M%S).md}"
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

NOW_SEC="$(date +%s)"
NOW_MS="$((NOW_SEC * 1000))"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

TABLE_SPECS=(
  "contract_flow_1s|7|ts_bucket|P0|1s contract flow raw buckets"
  "contract_liquidation_1s|7|ts_bucket|P0|1s liquidation context buckets"
  "contract_oi_snapshots|30|ts|P1|open-interest snapshots"
  "contract_funding_snapshots|90|ts|P1|funding snapshots"
  "contract_whale_percentile_thresholds|30|computed_at|P1|CWM percentile thresholds"
  "toxic_events|30|ts|P1|toxic event stream"
  "toxic_snapshots|7|ts|P0|runtime toxic snapshots"
  "flow_snapshots|7|ts|P0|runtime flow snapshots"
  "venue_health_snapshots|7|ts|P1|venue health snapshots"
  "vpin_buckets|30|end_ts|P1|VPIN buckets"
  "replay_runs|30|COALESCE(finished_at, started_at)|P2|replay metadata"
  "contract_whale_signals|365|ts|P2|persisted contract whale signals"
  "spot_whale_signals|30|ts|P2|persisted spot whale signals"
  "main_force_events|0|started_at|P2|derived main force lifecycle events"
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

run_sql_preserve() {
  local sql="$1"
  local tmp_file="${TMP_DIR}/sql.$RANDOM.out"
  local status=0
  if [[ "${SQL_BACKEND}" == "sqlite3" ]]; then
    timeout "${SQLITE_TIMEOUT}s" sqlite3 -readonly -noheader "${DB_PATH}" "${sql}" >"${tmp_file}" 2>&1 || status=$?
  else
    SQL_QUERY="${sql}" timeout "${SQLITE_TIMEOUT}s" python3 - "${DB_PATH}" >"${tmp_file}" 2>&1 <<'PY'
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
    status=$?
  fi
  if [[ "${status}" -eq 0 ]]; then
    cat "${tmp_file}"
    rm -f "${tmp_file}"
    return 0
  fi
  cat "${tmp_file}"
  rm -f "${tmp_file}"
  return 1
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
  if [[ -n "${preferred}" && "${preferred}" != "AUTO" ]]; then
    if [[ "${preferred}" == *"COALESCE("* ]]; then
      echo "${preferred}"
      return 0
    fi
    if column_exists "${table}" "${preferred}"; then
      echo "${preferred}"
      return 0
    fi
  fi
  local candidates=(ts_bucket ts created_at updated_at bucket_ts timestamp end_ts started_at computed_at)
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

human_bytes() {
  local bytes="${1:-0}"
  python3 - "$bytes" <<'PY'
import sys
value = float(sys.argv[1] or 0)
units = ["B", "KB", "MB", "GB", "TB"]
idx = 0
while value >= 1024 and idx < len(units) - 1:
    value /= 1024.0
    idx += 1
print(f"{value:.2f} {units[idx]}")
PY
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
    main_force_events)
      echo ""
      ;;
    *)
      echo "${time_expr} < ${cutoff_ms}"
      ;;
  esac
}

page_size="$(scalar_or_na 'PRAGMA page_size;')"
page_count="$(scalar_or_na 'PRAGMA page_count;')"
freelist_count="$(scalar_or_na 'PRAGMA freelist_count;')"
journal_mode="$(scalar_or_na 'PRAGMA journal_mode;')"
wal_autocheckpoint="$(scalar_or_na 'PRAGMA wal_autocheckpoint;')"
busy_timeout="$(scalar_or_na 'PRAGMA busy_timeout;')"

page_size_num="${page_size//[^0-9]/}"
page_count_num="${page_count//[^0-9]/}"
freelist_count_num="${freelist_count//[^0-9]/}"

db_size_estimate_bytes=0
freelist_estimate_bytes=0
if [[ -n "${page_size_num}" && -n "${page_count_num}" ]]; then
  db_size_estimate_bytes="$((page_size_num * page_count_num))"
fi
if [[ -n "${page_size_num}" && -n "${freelist_count_num}" ]]; then
  freelist_estimate_bytes="$((page_size_num * freelist_count_num))"
fi

wal_path="${DB_PATH}-wal"
shm_path="${DB_PATH}-shm"
wal_size="missing"
shm_size="missing"
if [[ -f "${wal_path}" ]]; then
  wal_size="$(du -b "${wal_path}" | awk '{print $1}')"
fi
if [[ -f "${shm_path}" ]]; then
  shm_size="$(du -b "${shm_path}" | awk '{print $1}')"
fi

{
  echo "# SQLite Sizing Report"
  echo
  echo "- generated_at: $(date -Is)"
  echo "- db_path: ${DB_PATH}"
  echo "- sqlite_timeout_sec: ${SQLITE_TIMEOUT}"
  echo

  echo "## Disk"
  echo '```text'
  df -h "$(dirname "${DB_PATH}")" || true
  echo '```'
  echo

  echo "## Files"
  echo '```text'
  ls -lh "${DB_PATH}"* 2>/dev/null || true
  echo '```'
  echo

  echo "## SQLite Page Stats"
  echo
  echo "- page_size: ${page_size}"
  echo "- page_count: ${page_count}"
  echo "- freelist_count: ${freelist_count}"
  echo "- journal_mode: ${journal_mode}"
  echo "- wal_autocheckpoint: ${wal_autocheckpoint}"
  echo "- busy_timeout: ${busy_timeout}"
  echo "- db_size_estimate: $(human_bytes "${db_size_estimate_bytes}")"
  echo "- freelist_estimate: $(human_bytes "${freelist_estimate_bytes}")"
  echo "- wal_file_size: $( [[ "${wal_size}" =~ ^[0-9]+$ ]] && human_bytes "${wal_size}" || echo "${wal_size}" )"
  echo "- shm_file_size: $( [[ "${shm_size}" =~ ^[0-9]+$ ]] && human_bytes "${shm_size}" || echo "${shm_size}" )"
  echo

  echo "## Tables"
  echo '```text'
  run_sql "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;"
  echo '```'
  echo

  echo "## Top Objects by dbstat"
  echo '```text'
  if ! run_sql_preserve "
    SELECT
      name || CASE WHEN path <> '' THEN ' [' || path || ']' ELSE '' END AS object_name,
      ROUND(SUM(pgsize) / 1024.0 / 1024.0, 2) AS mb
    FROM dbstat
    GROUP BY name, path
    ORDER BY SUM(pgsize) DESC
    LIMIT 100;
  "; then
    echo "dbstat unavailable; fallback mode enabled"
  fi
  echo '```'
  echo

  echo "## Top Objects by Name"
  echo '```text'
  if ! run_sql_preserve "
    SELECT
      name,
      ROUND(SUM(pgsize) / 1024.0 / 1024.0, 2) AS mb
    FROM dbstat
    GROUP BY name
    ORDER BY SUM(pgsize) DESC
    LIMIT 50;
  "; then
    echo "dbstat unavailable; fallback mode enabled"
  fi
  echo '```'
  echo

  echo "## Candidate Retention Tables"
  echo
  printf "| table | purpose | priority | rows_total | time_expr | oldest | newest | retention_days | rows_deletable |\n"
  printf "|---|---|---:|---:|---|---:|---:|---:|---:|\n"

  for spec in "${TABLE_SPECS[@]}"; do
    IFS='|' read -r table retention_days preferred_time_expr priority purpose <<<"${spec}"
    if ! table_exists "${table}"; then
      printf "| %s | %s | %s | missing | - | - | - | %s | - |\n" "${table}" "${purpose}" "${priority}" "${retention_days}"
      continue
    fi

    time_expr="$(pick_time_expr "${table}" "${preferred_time_expr}")"
    rows_total="$(scalar_or_na "SELECT COUNT(*) FROM ${table};")"
    oldest="N/A"
    newest="N/A"
    deletable="N/A"

    if [[ -n "${time_expr}" ]]; then
      oldest="$(scalar_or_na "SELECT MIN(${time_expr}) FROM ${table};")"
      newest="$(scalar_or_na "SELECT MAX(${time_expr}) FROM ${table};")"
    fi

    if [[ "${retention_days}" != "0" && -n "${time_expr}" ]]; then
      cutoff_ms="$((NOW_MS - retention_days * 24 * 60 * 60 * 1000))"
      delete_where="$(estimate_delete_where "${table}" "${time_expr}" "${cutoff_ms}")"
      if [[ -n "${delete_where}" ]]; then
        deletable="$(scalar_or_na "SELECT COUNT(*) FROM ${table} WHERE ${delete_where};")"
      fi
    fi

    printf "| %s | %s | %s | %s | %s | %s | %s | %s | %s |\n" \
      "${table}" "${purpose}" "${priority}" "${rows_total}" \
      "${time_expr:-N/A}" "${oldest}" "${newest}" "${retention_days}" "${deletable}"
  done
  echo

  echo "## Symbol Breakdown"
  for table in contract_flow_1s contract_whale_signals spot_whale_signals main_force_events; do
    if ! table_exists "${table}" || ! column_exists "${table}" "symbol"; then
      continue
    fi
    echo
    echo "### ${table}"
    echo '```text'
    run_sql "
      SELECT symbol, COUNT(*) AS rows
      FROM ${table}
      GROUP BY symbol
      ORDER BY rows DESC
      LIMIT 20;
    "
    echo '```'
  done
  echo

  echo "## Index Inventory"
  echo '```text'
  run_sql "
    SELECT name, tbl_name
    FROM sqlite_master
    WHERE type = 'index'
    ORDER BY tbl_name, name;
  "
  echo '```'
  echo

  echo "## Initial Risk Readout"
  echo
  if [[ "${freelist_estimate_bytes}" -gt 0 ]]; then
    echo "- freelist_pages_present: yes"
  else
    echo "- freelist_pages_present: no"
  fi
  if [[ "${wal_size}" =~ ^[0-9]+$ && "${wal_size}" -gt $((1024 * 1024 * 1024)) ]]; then
    echo "- wal_state: large (>${wal_size} bytes)"
  else
    echo "- wal_state: moderate_or_missing"
  fi
  echo "- note: this report is read-only; no DELETE, VACUUM, checkpoint, truncate, or file mutation was performed."
} >"${OUT}"

echo "Report written to ${OUT}"
