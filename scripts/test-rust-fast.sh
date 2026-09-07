#!/usr/bin/env bash
set -euo pipefail

# Fast feedback loop for changes to the rating, persistence, and API layers.
# The CI workflow still runs the complete suite separately.
cargo test --lib --all-features
cargo test --all-features \
  --test contract_whale_impact_grade_tests \
  --test contract_whale_impact_grade_persistence_tests \
  --test contract_whale_persistence_tests \
  --test contract_event_routes_tests \
  --test contract_whale_routes_tests \
  --test api_security_guard_tests \
  --test sqlite_store_pragmas_tests
