---
name: test-matrix
description: Design or review unit, integration, E2E, concurrency, permission, and log-redaction tests for toxic-order risk monitoring and operator workflows.
---

# Test Matrix

Use this skill when adding tests, reviewing coverage, or converting a task card into executable validation.

## Required Matrix

| Area | Required cases |
| --- | --- |
| Normal input | Ordinary order is not falsely blocked |
| High-risk input | Blacklist, abnormal phone, abnormal address, abnormal amount |
| Missing fields | Missing buyer_id, phone, address, amount |
| Duplicate order | Same order emits at most one alert |
| Boundary values | Amount 0, huge amount, long notes/address |
| Permission | Shop A cannot see or act on Shop B |
| Concurrency | Multiple workers process same order safely |
| Manual review | Released order is not auto-blocked later |
| Logs | No phone, address, ID number, full email, token |
| Production replay | Fixture smoke writes summary, signals, calibration, and CSV artifacts |
| CSV export | Formula-prefixed cells are escaped before spreadsheet import |
| Alert gate | High/critical can push only when score and data quality pass; medium/low never push |
| Persistent inbox | Refresh keeps candidates; clear cache hides cleared keys from later merges |

## Test Quality Rules

- Prefer behavior tests over implementation mirrors.
- Include adversarial inputs, not just happy paths.
- If a test fails, inspect whether production code is wrong before weakening the assertion.
- Avoid external network dependencies in automated tests.
- Isolate env var mutation with locks.
- Keep fixtures explicit and small.
- Use synthetic replay fixtures in CI; run real production replay only from ignored local data.
- Never require a real Discord, Telegram, or Langflow endpoint in unit tests.

## Current Repo Gate

```powershell
node --check web\app.js
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -j 1 --all-targets
cd toxic-order-monitor
npm run build
npm run test
npm audit --audit-level=high
```
