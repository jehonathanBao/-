# Contract Whale Institutional Analysis Terminal v3.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade `/contract-whale` from a trade-setup view into a read-only institutional analysis terminal that explains market structure, liquidity behavior, ranked event strength, regime, and opportunity zones.

**Architecture:** Keep the existing detector / event / lifecycle pipeline intact and add a new intelligence layer on top of normalized `ContractWhaleSignal` data. Expose the analysis layer through a dedicated read-only API and swap the frontend panel from trade suggestions to institutional market interpretation.

**Tech Stack:** Rust backend, Axum routes, existing `contract_whale_monitor` domain types, React frontend, Vitest, Rust integration tests.

## Global Constraints

- Must remain read-only analysis output; no buy/sell advice, no entry/exit, no stop-loss, no execution path changes.
- Keep existing event system, merge, lifecycle, retention, and monitoring APIs intact.
- Reuse existing signal data and summary metadata where possible; do not invent synthetic market data.
- Add tests first for backend response shape and frontend rendering before implementation changes.
- Preserve public `/api/contract-whale/trading-decisions` compatibility even if the main UI stops surfacing it.

---

### Task 1: Define intelligence response types and module boundaries

**Files:**
- Create: `src/contract_whale_monitor/intelligence/mod.rs`
- Create: `src/contract_whale_monitor/intelligence/regime.rs`
- Create: `src/contract_whale_monitor/intelligence/liquidity.rs`
- Create: `src/contract_whale_monitor/intelligence/strength.rs`
- Create: `src/contract_whale_monitor/intelligence/opportunity.rs`
- Create: `src/contract_whale_monitor/intelligence/ranking.rs`
- Modify: `src/contract_whale_monitor/mod.rs`
- Modify: `src/contract_whale_monitor/types.rs`

**Interfaces:**
- Consumes: `ContractWhaleSignal`, `ContractWhaleMarketStructureLite`, `ContractWhaleNoiseSuppressionSummary`
- Produces: `ContractWhaleIntelligenceResponse`, `ContractWhaleRegimeSnapshot`, `ContractWhaleLiquidityBehavior`, `ContractWhaleOpportunityZone`, `ContractWhaleRankedEvent`

- [ ] **Step 1: Write the failing backend type-driven test**

Add assertions in `tests/contract_whale_routes_tests.rs` for a new builder that returns:
- `market_regime.regime`
- `liquidity_behaviors[]`
- `ranked_events[]`
- `opportunity_map[]`
- `market_bias` removed from the new response

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test contract_whale_routes_tests institutional_analysis_response_surfaces_regime_strength_and_opportunities`

Expected: FAIL because `build_contract_whale_intelligence_response` and the new types do not exist.

- [ ] **Step 3: Add minimal types and module skeleton**

Create the intelligence module files and add placeholder response structs to `types.rs` so the test compiles.

- [ ] **Step 4: Run test to verify the failure moved to behavior**

Run: `cargo test --test contract_whale_routes_tests institutional_analysis_response_surfaces_regime_strength_and_opportunities`

Expected: FAIL on incorrect/default field values rather than missing symbols.


### Task 2: Implement backend intelligence scoring and classification

**Files:**
- Modify: `src/contract_whale_monitor/intelligence/regime.rs`
- Modify: `src/contract_whale_monitor/intelligence/liquidity.rs`
- Modify: `src/contract_whale_monitor/intelligence/strength.rs`
- Modify: `src/contract_whale_monitor/intelligence/opportunity.rs`
- Modify: `src/contract_whale_monitor/intelligence/ranking.rs`
- Modify: `src/contract_whale_monitor/intelligence/mod.rs`
- Modify: `src/api/contract_whale_routes.rs`
- Test: `tests/contract_whale_routes_tests.rs`

**Interfaces:**
- Consumes: slices of `ContractWhaleSignal`
- Produces: `build_contract_whale_intelligence_response(symbol, items, market_structure_lite, noise_suppression, timestamp)`

- [ ] **Step 1: Write the failing behavior test**

Extend `tests/contract_whale_routes_tests.rs` with seeded signals covering:
- one trending-up aggressive buy
- one fake breakout / no follow-through suppression
- one absorption zone

Assert:
- regime resolves to `TRENDING_UP`
- top ranked events sorted by strength descending
- liquidity behaviors include `absorption` and `fake_breakout`
- opportunity map includes at least one absorption or breakout pressure zone

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test contract_whale_routes_tests institutional_analysis_response_surfaces_regime_strength_and_opportunities`

Expected: FAIL with mismatched regime, empty ranking, or missing opportunity entries.

- [ ] **Step 3: Implement minimal scoring/classification**

Implement:
- `strength.rs`: 0-100 score using volume, price response, dominance, persistence, cross-window consistency
- `regime.rs`: map signals into `TRENDING_UP`, `TRENDING_DOWN`, `RANGING`, `HIGH_VOLATILITY`, `LIQUIDATION_PHASE`
- `liquidity.rs`: detect `absorption`, `distribution`, `fake_breakout`, `liquidity_sweep`, `order_block_behavior`
- `opportunity.rs`: produce non-trading zones like `absorption_zone`, `reversal_zone`, `fake_breakout_risk_zone`
- `ranking.rs`: rank strongest events without entry/exit fields

- [ ] **Step 4: Run targeted test to verify it passes**

Run: `cargo test --test contract_whale_routes_tests institutional_analysis_response_surfaces_regime_strength_and_opportunities`

Expected: PASS


### Task 3: Expose a dedicated intelligence API route

**Files:**
- Modify: `src/api/contract_whale_routes.rs`
- Modify: `src/api/server.rs`
- Test: `tests/contract_event_routes_tests.rs`

**Interfaces:**
- Produces: `GET /api/contract-whale/intelligence-terminal?symbol=BTC&range=24h`
- Returns: serialized `ContractWhaleIntelligenceResponse`

- [ ] **Step 1: Write the failing route test**

Add a route test in `tests/contract_event_routes_tests.rs` that calls:
`/api/contract-whale/intelligence-terminal?symbol=BTC&range=24h`

Assert:
- HTTP 200
- JSON contains `symbol`, `marketRegime`, `rankedEvents`, `liquidityBehaviors`, `opportunityMap`, `noiseSuppression`
- response does not include `topSetups` or `entryZone`

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test contract_event_routes_tests contract_whale_intelligence_terminal_route_exposes_read_only_market_analysis`

Expected: FAIL because the route does not exist.

- [ ] **Step 3: Implement the route**

Add `contract_whale_intelligence_terminal_route` using the same freshness/stale filtering pattern as the latest and trading routes, but serialize the new intelligence response.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test contract_event_routes_tests contract_whale_intelligence_terminal_route_exposes_read_only_market_analysis`

Expected: PASS


### Task 4: Replace the frontend trade panel with the institutional analysis terminal

**Files:**
- Modify: `toxic-order-monitor/src/api/contractWhale.js`
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Produces: `fetchContractWhaleIntelligenceTerminal({ symbol, range })`
- Consumes: `/api/contract-whale/intelligence-terminal`

- [ ] **Step 1: Write the failing frontend test**

Add a Vitest case asserting the `/contract-whale` view shows:
- `Institutional Analysis Terminal`
- `Market Regime`
- `Liquidity Behavior`
- `Signal Strength Ranking`
- `Opportunity Map`

Also assert it does **not** show trade copy like `Entry Zone` or `Invalidation` in the new terminal panel.

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- --run src/tests/ContractWhaleMonitor.test.jsx`

Expected: FAIL because the new panel and API function do not exist.

- [ ] **Step 3: Implement minimal frontend integration**

Add:
- `fetchContractWhaleIntelligenceTerminal` in `contractWhale.js`
- state slot `intelligenceTerminal`
- polling refresh alongside summary/latest/history
- `InstitutionalAnalysisTerminalPanel` replacing `TradingDecisionLayerPanel` in the main render tree

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- --run src/tests/ContractWhaleMonitor.test.jsx`

Expected: PASS


### Task 5: Full verification, self-check, and sync

**Files:**
- Modify: none if all green

**Interfaces:**
- Verifies the new route and frontend build
- Syncs to server via the established deploy path

- [ ] **Step 1: Run backend verification**

Run:
- `cargo test --test contract_whale_routes_tests`
- `cargo test --test contract_event_routes_tests`
- `cargo check`

Expected: all PASS

- [ ] **Step 2: Run frontend verification**

Run:
- `npm test -- --run src/tests/ContractWhaleMonitor.test.jsx`
- `npm run build`

Expected: PASS

- [ ] **Step 3: Self-check the response boundary**

Verify:
- no entry/exit/stop loss language in the new intelligence API
- old trading route still compiles for compatibility
- UI now leads with regime/liquidity/ranking/opportunity, not trade setups

- [ ] **Step 4: Commit and push only the relevant files**

Run:
- `git status --short`
- `git add <relevant files>`
- `git commit -m "add contract whale institutional analysis terminal"`
- `git push origin main`

- [ ] **Step 5: Sync server and verify health**

Run:
- `ssh -i C:\\Users\\byhdo\\.ssh\\codex_contabo_tokyo_root -o BatchMode=yes -o StrictHostKeyChecking=accept-new -b 192.168.1.229 root@5.104.80.120 "cd /opt/toxic-order-monitor-rs && git pull --ff-only && docker compose up -d --build frontend backend && docker compose ps && curl -fsS http://127.0.0.1:8000/healthz && echo && curl -fsS http://127.0.0.1:8000/readyz && echo && curl -fsS 'http://127.0.0.1:5173/api/contract-whale/intelligence-terminal?symbol=BTC' | head -c 3000"`

Expected:
- `backend` and `frontend` healthy
- health endpoints return `ok`
- intelligence terminal API returns JSON with regime/ranking/opportunity data
