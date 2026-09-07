#!/usr/bin/env bash
set -euo pipefail

cargo test --all-targets --all-features --no-fail-fast -j "${CARGO_TEST_JOBS:-1}"
