# Contract Event Feed Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce `/contract-whale/{symbol}` event-row time-to-visible and steady-state bandwidth without changing event semantics or removing detail evidence.

**Architecture:** Keep the existing canonical event and lifecycle projections intact. Optimize transport at nginx, split lightweight status polling from heavyweight event polling, and prevent unchanged event arrays from forcing the 26-column tables to render again.

**Tech Stack:** Rust/Axum, React 19, Vitest, nginx, Docker Compose.

## Global Constraints

- Preserve the complete API response shape and `sourceSignal` evidence.
- Do not change detector, scoring, retention, Discord, execution, or trading behavior.
- Keep summary/latest freshness at 5 seconds.
- Refresh heavyweight event projections every 15 seconds and immediately on first page load.
- Keep operator/debug routes inaccessible through the public nginx entrypoint.

---

### Task 1: Lock transport behavior with deployment tests

**Files:**
- Modify: `tests/deployment_assets_static_tests.rs`
- Modify: `deploy/nginx-site.toxic-order-monitor.conf`
- Modify: `toxic-order-monitor/nginx.conf.template`

**Interfaces:**
- Consumes: existing public nginx and container nginx templates.
- Produces: gzip compression for JSON/JS/CSS and immutable caching for `/assets/`.

- [ ] Add failing assertions for `gzip on`, required MIME types, `gzip_vary`, and immutable asset caching.
- [ ] Run `cargo test --test deployment_assets_static_tests` and verify the new assertions fail.
- [ ] Add matching nginx directives to both templates without changing operator route blocks.
- [ ] Re-run the deployment asset test and verify it passes.

### Task 2: Split lightweight and heavyweight polling

**Files:**
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: existing summary, latest, contract-events, final-events-v2, and intelligence API clients.
- Produces: 5-second status refresh and 15-second event projection refresh.

- [ ] Replace the existing timer test with a failing test that expects summary/latest at 5 seconds while event projections remain unchanged until 15 seconds.
- [ ] Run the focused Vitest file and verify the polling assertion fails for the current single 5-second timer.
- [ ] Split the polling functions and timers while retaining immediate initial requests and visibility-aware pause/resume behavior.
- [ ] Re-run the focused test and verify it passes.

### Task 3: Make event loading and table rendering independent

**Files:**
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: normalized contract/final event arrays.
- Produces: event-specific loading state, stable array references, and memoized event tables.

- [ ] Add a failing test where summary/latest resolve while contract-events remains pending and assert the event panel still shows its loading state.
- [ ] Run the focused Vitest file and confirm the current global loading state fails the assertion.
- [ ] Add `contractEventsLoading`, preserve previous arrays when event content is unchanged, and memoize `RawSignalDebugTable`.
- [ ] Re-run the focused tests and verify them.

### Task 4: Full validation and deployment

**Files:**
- Verify all changed files only.

**Interfaces:**
- Consumes: Tasks 1-3.
- Produces: tested commit, pushed branch, healthy production services, and before/after timing evidence.

- [ ] Run `cargo fmt --check` and relevant Rust tests.
- [ ] Run focused frontend tests and `npm --prefix toxic-order-monitor run build`.
- [ ] Run `git diff --check` and review the staged diff for secrets and unrelated files.
- [ ] Commit and push only task-related files.
- [ ] Pull on the server, rebuild frontend/backend only as required, reload host nginx, and verify both containers are healthy.
- [ ] Confirm `Content-Encoding: gzip`, immutable asset caching, endpoint health, and improved public transfer timings.
