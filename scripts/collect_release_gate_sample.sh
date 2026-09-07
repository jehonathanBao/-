#!/usr/bin/env bash
set -euo pipefail

umask 027
project_root="${TOXIC_PROJECT_ROOT:-/opt/toxic-order-monitor-rs}"
base_url="${TOXIC_PUBLIC_BASE_URL:-http://127.0.0.1:5173}"
report_dir="${TOXIC_RELEASE_GATE_REPORT_DIR:-${project_root}/.runtime/reports/release-gates}"
sqlite_path="${TOXIC_SQLITE_PATH:-${project_root}/data/btc-toxic-flow.sqlite}"

mkdir -p "${report_dir}"
temporary_dir="$(mktemp -d)"
trap 'rm -rf "${temporary_dir}"' EXIT

curl --fail --silent --show-error --max-time 10 \
  "${base_url}/api/status" > "${temporary_dir}/status.json"
curl --fail --silent --show-error --max-time 10 \
  "${base_url}/api/contract-whale/rating-health" > "${temporary_dir}/rating-health.json"

python3 - "${temporary_dir}/status.json" "${temporary_dir}/rating-health.json" \
  "${report_dir}/release-gates-$(date -u +%Y%m%d).jsonl" "${sqlite_path}" <<'PY'
import json
import os
import sys
import time
from pathlib import Path

status_path, rating_path, report_path, sqlite_path = sys.argv[1:]
status = json.loads(Path(status_path).read_text())
rating = json.loads(Path(rating_path).read_text())
quality = status.get("marketDataQuality") or {}
storage = status.get("storage") or {}
snapshot = rating.get("snapshot") or {}
sample = {
    "sampledAt": int(time.time() * 1000),
    "marketDataQuality": {
        key: quality.get(key)
        for key in (
            "status",
            "eventBusDroppedEvents",
            "eventBusSendErrors",
            "flowWindowLaggedEvents",
            "markoutLaggedEvents",
            "vpinLaggedEvents",
            "recentLaggedEvents",
            "historicalLaggedEvents",
            "lastLaggedAtMs",
        )
    },
    "storage": {
        "status": storage.get("status"),
        "lastWriteTs": storage.get("lastWriteTs"),
        "sqliteBytes": os.stat(sqlite_path).st_size if os.path.exists(sqlite_path) else None,
    },
    "rating": {
        key: snapshot.get(key)
        for key in (
            "assessmentCount",
            "gradeCount",
            "missingCount",
            "missingRate",
            "latencyP50Ms",
            "latencyP95Ms",
            "gradeDistribution",
            "stateDistribution",
        )
    },
}
with open(report_path, "a", encoding="utf-8") as handle:
    handle.write(json.dumps(sample, ensure_ascii=False, separators=(",", ":")) + "\n")
PY

find "${report_dir}" -type f -name 'release-gates-*.jsonl' -mtime +14 -delete
