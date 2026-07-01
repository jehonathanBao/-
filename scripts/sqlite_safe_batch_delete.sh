#!/usr/bin/env bash
set -euo pipefail

DB_PATH="${DB_PATH:-}"
TABLE="${TABLE:-}"
TIME_COLUMN="${TIME_COLUMN:-}"
RETENTION_HOURS="${RETENTION_HOURS:-24}"
BATCH_SIZE="${BATCH_SIZE:-10000}"
MAX_BATCHES="${MAX_BATCHES:-100}"
SLEEP_SECONDS="${SLEEP_SECONDS:-0.5}"
DRY_RUN="${DRY_RUN:-1}"
STOP_FILE="${STOP_FILE:-/tmp/stop_sqlite_cleanup}"
SQLITE_TIMEOUT="${SQLITE_TIMEOUT:-30}"
TIME_MODE="${TIME_MODE:-epoch_ms}"
NOW_MS="${NOW_MS:-$(( $(date +%s) * 1000 ))}"

if [[ -z "${DB_PATH}" ]]; then
  echo "DB_PATH is required" >&2
  exit 1
fi

if [[ -z "${TABLE}" ]]; then
  echo "TABLE is required" >&2
  exit 1
fi

if [[ -z "${TIME_COLUMN}" ]]; then
  echo "TIME_COLUMN is required" >&2
  exit 1
fi

if [[ ! -f "${DB_PATH}" ]]; then
  echo "DB not found: ${DB_PATH}" >&2
  exit 1
fi

case "${TABLE}" in
  flow_snapshots|venue_health_snapshots|toxic_snapshots)
    ;;
  *)
    echo "Refuse to clean non-P0 table: ${TABLE}" >&2
    exit 1
    ;;
esac

case "${TIME_MODE}" in
  epoch_ms|epoch_s|sqlite_datetime)
    ;;
  *)
    echo "Unsupported TIME_MODE: ${TIME_MODE}" >&2
    exit 1
    ;;
esac

if ! command -v timeout >/dev/null 2>&1; then
  echo "timeout is required on PATH" >&2
  exit 1
fi

PYTHON_CMD=()

if command -v sqlite3 >/dev/null 2>&1; then
  SQL_BACKEND="sqlite3"
elif command -v python >/dev/null 2>&1 && python -c "import sqlite3" >/dev/null 2>&1; then
  SQL_BACKEND="python"
  PYTHON_CMD=(python)
elif command -v python3 >/dev/null 2>&1 && python3 -c "import sqlite3" >/dev/null 2>&1; then
  SQL_BACKEND="python"
  PYTHON_CMD=(python3)
elif command -v py >/dev/null 2>&1 && py -3 -c "import sqlite3" >/dev/null 2>&1; then
  SQL_BACKEND="python"
  PYTHON_CMD=(py -3)
else
  echo "sqlite3, python, python3, or py -3 is required on PATH" >&2
  exit 1
fi

run_sql_ro() {
  local sql="$1"
  if [[ "${SQL_BACKEND}" == "sqlite3" ]]; then
    timeout "${SQLITE_TIMEOUT}s" sqlite3 -readonly -noheader "${DB_PATH}" "${sql}"
  else
    SQL_QUERY="${sql}" timeout "${SQLITE_TIMEOUT}s" "${PYTHON_CMD[@]}" - "${DB_PATH}" <<'PY'
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

run_sql_rw() {
  local sql="$1"
  if [[ "${SQL_BACKEND}" == "sqlite3" ]]; then
    timeout "${SQLITE_TIMEOUT}s" sqlite3 -noheader "${DB_PATH}" "${sql}"
  else
    SQL_QUERY="${sql}" timeout "${SQLITE_TIMEOUT}s" "${PYTHON_CMD[@]}" - "${DB_PATH}" <<'PY'
import os
import sqlite3
import sys

db_path = sys.argv[1]
sql = os.environ["SQL_QUERY"]
conn = sqlite3.connect(db_path, timeout=5)
conn.executescript(sql)
conn.commit()
cur = conn.execute("SELECT changes()")
for row in cur.fetchall():
    print("|".join("" if value is None else str(value) for value in row))
PY
  fi
}

column_exists() {
  local column="$1"
  local result
  result="$(run_sql_ro "PRAGMA table_info(${TABLE});" | awk -F'|' '{print $2}' | grep -Fx "${column}" || true)"
  [[ -n "${result}" ]]
}

if ! column_exists "${TIME_COLUMN}"; then
  echo "TIME_COLUMN not found on ${TABLE}: ${TIME_COLUMN}" >&2
  exit 1
fi

cutoff_ms="$((NOW_MS - RETENTION_HOURS * 60 * 60 * 1000))"
cutoff_s="$((NOW_MS / 1000 - RETENTION_HOURS * 60 * 60))"

build_delete_where() {
  case "${TIME_MODE}" in
    epoch_ms)
      echo "${TIME_COLUMN} < ${cutoff_ms}"
      ;;
    epoch_s)
      echo "${TIME_COLUMN} < ${cutoff_s}"
      ;;
    sqlite_datetime)
      echo "${TIME_COLUMN} < datetime('now', '-${RETENTION_HOURS} hours')"
      ;;
  esac
}

DELETE_WHERE="$(build_delete_where)"
COUNT_SQL="SELECT COUNT(*) FROM ${TABLE} WHERE ${DELETE_WHERE};"

echo "DB_PATH=${DB_PATH}"
echo "TABLE=${TABLE}"
echo "TIME_COLUMN=${TIME_COLUMN}"
echo "TIME_MODE=${TIME_MODE}"
echo "RETENTION_HOURS=${RETENTION_HOURS}"
echo "BATCH_SIZE=${BATCH_SIZE}"
echo "MAX_BATCHES=${MAX_BATCHES}"
echo "DRY_RUN=${DRY_RUN}"
echo "STOP_FILE=${STOP_FILE}"
echo "delete_where=${DELETE_WHERE}"

initial_deletable_rows="$(run_sql_ro "${COUNT_SQL}" | tr -d '\r' | head -n 1)"
initial_deletable_rows="${initial_deletable_rows:-0}"
echo "initial_deletable_rows=${initial_deletable_rows}"

if [[ "${DRY_RUN}" != "0" ]]; then
  echo "DRY_RUN enabled. No deletion executed."
  exit 0
fi

batch=0
total_deleted=0

while (( batch < MAX_BATCHES )); do
  if [[ -f "${STOP_FILE}" ]]; then
    echo "STOP_FILE found: ${STOP_FILE}"
    break
  fi

  before="$(run_sql_ro "${COUNT_SQL}" | tr -d '\r' | head -n 1)"
  before="${before:-0}"
  if (( before <= 0 )); then
    echo "No more rows to delete."
    break
  fi

  delete_sql="
  DELETE FROM ${TABLE}
  WHERE rowid IN (
    SELECT rowid
    FROM ${TABLE}
    WHERE ${DELETE_WHERE}
    LIMIT ${BATCH_SIZE}
  );
  "

  deleted="$(run_sql_rw "${delete_sql}" | tr -d '\r' | tail -n 1)"
  deleted="${deleted:-0}"
  total_deleted=$((total_deleted + deleted))
  batch=$((batch + 1))

  after="$(run_sql_ro "${COUNT_SQL}" | tr -d '\r' | head -n 1)"
  after="${after:-0}"
  echo "batch=${batch} deleted=${deleted} remaining=${after} total_deleted=${total_deleted}"

  if (( deleted <= 0 )); then
    echo "No rows deleted in this batch. Stopping."
    break
  fi

  sleep "${SLEEP_SECONDS}"
done

echo "completed batches=${batch} total_deleted=${total_deleted}"
