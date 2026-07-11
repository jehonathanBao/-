# Remove Altcoin Manipulation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the dedicated altcoin manipulation feature while keeping all other monitors and liquidation cascade data intact.

**Architecture:** Delete the isolated engine, routes, frontend page, nav item, and client. Simplify fusion to remove its manipulation-only branch. The cascade view keeps cascade, leverage, liquidity-gap, and generic market-state data without a control-score dependency.

**Tech Stack:** Rust 2021, Axum, React 19, Vite, Vitest.

## Global Constraints

- Preserve Binance alt-contract anomalies, new-token watch, BTC/ETH contract and spot monitors, and liquidation cascade APIs.
- Do not change detection thresholds, persistence, retention, Discord, or execution behavior.
- Do not modify pre-existing unrelated dirty files.
- Do not commit unless the user asks.

---

### Task 1: Define removal behavior with tests

**Files:** `tests/market_intelligence_split_static_tests.rs`, `toxic-order-monitor/src/tests/Dashboard.test.jsx`, `toxic-order-monitor/src/tests/LiquidationCascadeDashboard.test.jsx`

- [ ] Write a static Rust test named `altcoin_manipulation_engine_and_routes_are_removed` asserting that `src/lib.rs` has no `altcoin_manipulation_engine`, `src/api/mod.rs` has no `altcoin_routes`, and `src/api/server.rs` has no `/api/altcoin/`, while still containing `/api/binance-alt-contract/summary` and `/api/new-token-watch/list`.
- [ ] Run `cargo test --test market_intelligence_split_static_tests altcoin_manipulation_engine_and_routes_are_removed`; it must fail because the feature currently exists.
- [ ] Update frontend tests to assert no sidebar link named `妖币控盘监控`, retained links for `山寨合约异常` and `新币合约监控`, and no cascade text/request containing `控盘分` or `/api/altcoin/`.
- [ ] Run `npm test -- --run src/tests/Dashboard.test.jsx src/tests/LiquidationCascadeDashboard.test.jsx`; it must fail because the feature is still wired in.

### Task 2: Delete the backend feature boundary

**Files:** delete `src/altcoin_manipulation_engine.rs`, delete `src/api/altcoin_routes.rs`, modify `src/lib.rs`, `src/api/mod.rs`, `src/api/server.rs`, `src/market_domain.rs`, `src/multi_timeframe_orderflow_fusion.rs`, `src/api/fusion_routes.rs`, and `tests/market_intelligence_split_static_tests.rs`.

- [ ] Remove the public modules/imports and all four `/api/altcoin/*` route registrations.
- [ ] Remove `MarketDomain::AltcoinManipulation` and only fusion paths that read `altcoin_control_score` or produce manipulation-specific labels; keep BTC structure behavior intact and use the existing neutral/general path for non-BTC inputs.
- [ ] Delete the engine and route files; keep the new absence test and delete only feature-presence test assertions.
- [ ] Run `cargo test --test market_intelligence_split_static_tests altcoin_manipulation_engine_and_routes_are_removed` and `cargo check`; both must pass.

### Task 3: Delete the frontend feature and decouple cascade

**Files:** delete `toxic-order-monitor/src/components/AltcoinManipulationDashboard.jsx`, modify `toxic-order-monitor/src/App.jsx`, `toxic-order-monitor/src/components/Sidebar.jsx`, `toxic-order-monitor/src/pages/Dashboard.jsx`, `toxic-order-monitor/src/api/liquidationCascade.js`, `toxic-order-monitor/src/components/LiquidationCascadeDashboard.jsx`, and focused frontend tests.

- [ ] Delete the standalone route, sidebar entry, dashboard view mode/page component, path parser branch, filter label, and standalone component.
- [ ] Delete `fetchAltcoinManipulation`, its normalizer, and fallback. Keep all non-`/api/altcoin/*` liquidation API helpers.
- [ ] Remove manipulation, market-signal, and altcoin-domain requests from the cascade dashboard. Keep cascade, leverage map, liquidity gap, and generic regime calls. Remove `控盘分` and manipulation-only copy.
- [ ] Run `npm test -- --run src/tests/Dashboard.test.jsx src/tests/LiquidationCascadeDashboard.test.jsx`; it must pass.

### Task 4: Run preservation and quality gates

**Files:** modify only if compilation or tests prove a direct deleted-feature reference remains.

- [ ] Run `rg -n -i --glob '!target/**' --glob '!node_modules/**' "altcoin[_ -]?manipulation|AltcoinManipulation|妖币控盘|/api/altcoin" src toxic-order-monitor tests`; no production reference may remain.
- [ ] Run `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test -j 1 --all-targets`; all must exit 0.
- [ ] Run `npm test -- --run` and `npm run build`; both must exit 0.
- [ ] Run `git diff --check` and `git status --short`; confirm no unrelated dirty files were changed by this task.
