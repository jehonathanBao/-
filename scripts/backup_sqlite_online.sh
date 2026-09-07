#!/usr/bin/env bash
set -euo pipefail

# Create a consistent SQLite backup without stopping the live monitor.
# The default destination is local only; operators can copy the resulting
# artifact to an external destination after the checksum and integrity check.
project_root="${TOXIC_PROJECT_ROOT:-/opt/toxic-order-monitor-rs}"
sqlite_path="${TOXIC_SQLITE_PATH:-${project_root}/data/btc-toxic-flow.sqlite}"
backup_dir="${TOXIC_SQLITE_BACKUP_DIR:-${project_root}/backups/sqlite}"
retention_days="${TOXIC_SQLITE_BACKUP_RETENTION_DAYS:-14}"
timeout_seconds="${TOXIC_SQLITE_BACKUP_TIMEOUT_SECONDS:-1800}"

if [[ ! -f "${sqlite_path}" ]]; then
  printf 'SQLite source does not exist: %s\n' "${sqlite_path}" >&2
  exit 1
fi
if ! [[ "${retention_days}" =~ ^[0-9]+$ ]]; then
  printf 'Invalid backup retention days: %s\n' "${retention_days}" >&2
  exit 1
fi
if ! [[ "${timeout_seconds}" =~ ^[0-9]+$ ]] || (( timeout_seconds < 60 )); then
  printf 'Invalid backup timeout seconds: %s\n' "${timeout_seconds}" >&2
  exit 1
fi
if ! command -v sqlite3 >/dev/null 2>&1; then
  printf 'sqlite3 is required for online backups\n' >&2
  exit 1
fi

mkdir -p "${backup_dir}"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
final_path="${backup_dir}/btc-toxic-flow-${timestamp}.sqlite"
temporary_path="${final_path}.tmp"
journal_path="${temporary_path}-journal"
trap 'rm -f "${temporary_path}" "${journal_path}"' EXIT

timeout_bin=""
if command -v timeout >/dev/null 2>&1; then
  timeout_bin="timeout"
elif command -v gtimeout >/dev/null 2>&1; then
  timeout_bin="gtimeout"
fi
if [[ -n "${timeout_bin}" ]]; then
  "${timeout_bin}" --signal=TERM --kill-after=30s "${timeout_seconds}s" \
    sqlite3 -cmd '.timeout 10000' "${sqlite_path}" ".backup '${temporary_path}'"
else
  printf 'timeout is unavailable; running backup without a process timeout\n' >&2
  sqlite3 -cmd '.timeout 10000' "${sqlite_path}" ".backup '${temporary_path}'"
fi
integrity="$(sqlite3 "${temporary_path}" 'PRAGMA integrity_check;')"
if [[ "${integrity}" != "ok" ]]; then
  printf 'SQLite integrity check failed: %s\n' "${integrity}" >&2
  exit 1
fi

mv "${temporary_path}" "${final_path}"
sha256sum "${final_path}" > "${final_path}.sha256"
find "${backup_dir}" -type f -name 'btc-toxic-flow-*.sqlite' -mtime "+${retention_days}" -delete
find "${backup_dir}" -type f -name 'btc-toxic-flow-*.sqlite.sha256' -mtime "+${retention_days}" -delete
printf 'SQLite online backup created: %s\n' "${final_path}"
