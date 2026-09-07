# Testing workflow

The test suite is intentionally split into a fast feedback loop and a full
regression run. Both commands use the same tests; the split only avoids
re-running the full suite during local development.

## Rust

```bash
./scripts/test-rust-fast.sh
./scripts/test-rust-full.sh
```

The fast command covers the rating system, persistence, event routes, and
security contracts. Set `CARGO_TEST_JOBS` when the machine has enough memory
for parallel test binaries; the default is one job to keep SQLite-heavy tests
stable.

## Frontend

The frontend requires Node.js 20.19+ (or 22.12+). The repository pins the
recommended local version in `toxic-order-monitor/.nvmrc`.

```bash
cd toxic-order-monitor
npm run test:fast
npm run test:full
```

`test:fast` skips the large API and WebSocket suites while iterating on UI
components. `test:full` is the complete Vitest run used for release checks.
