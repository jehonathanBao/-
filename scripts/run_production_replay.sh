#!/usr/bin/env bash
set -euo pipefail

config="${1:-config/replay.production.local.toml}"

if [[ ! -f "$config" ]]; then
  echo "Replay config not found: $config. Copy config/replay.production.example.toml to config/replay.production.local.toml and point it at local production data." >&2
  exit 1
fi

if ! find data/production_replay -maxdepth 1 -type f \( -name '*.jsonl' -o -name '*.csv' \) | grep -q .; then
  echo "WARN: No real JSONL/CSV production replay input found under data/production_replay/." >&2
fi

cargo run --bin replay_production -- --config "$config"
