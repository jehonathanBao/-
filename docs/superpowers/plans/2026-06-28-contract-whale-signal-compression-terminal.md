# Contract Whale Signal Compression Terminal v2.5 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a separate trade-assist layer to `/contract-whale` that compresses noisy market events into up to three read-only trade ideas while preserving the existing analysis-first institutional terminal.

**Architecture:** Extend the existing backend `intelligence-terminal` response with `signalCompression`, `tradeIdeas`, and `riskContext`, then render those through local tabs inside `InstitutionalAnalysisTerminalPanel`. Keep the page-level event stream untouched and preserve read-only semantics end to end.

**Tech Stack:** Rust backend, Axum routes, React/Vite frontend, Vitest, Rust unit/integration tests.

## Global Constraints

- Must remain read-only monitoring and analysis output; no automatic execution, no order routing, no trading buttons.
- Keep `/api/contract-whale/intelligence-terminal` as the primary truth source; do not derive final trade ideas only in the frontend.
- Preserve existing event stream, lifecycle, retention, latest, and diagnostics behavior.
- Trade ideas may use medium-strength semantics only: direction bias, entry zone, invalidation, confidence, and structure rationale.
- Prefer `BULLISH_BIAS` / `BEARISH_BIAS` semantics over explicit `LONG` / `SHORT` instructions.

---

### Task 1: Extend backend intelligence response types for signal compression

**Files:**
- Modify: `src/contract_whale_monitor/types.rs`
- Modify: `src/contract_whale_monitor/intelligence/mod.rs`
- Test: `tests/contract_whale_routes_tests.rs`

**Interfaces:**
- Consumes: `ContractWhaleSignal`, `ContractWhaleNoiseSuppressionSummary`
- Produces: `ContractWhaleSignalCompressionSummary`, `ContractWhaleTradeIdea`, `ContractWhaleRiskContext`

- [ ] **Step 1: Write the failing test**

Add a new test in `tests/contract_whale_routes_tests.rs` asserting the intelligence builder returns fields for:

```rust
response.signal_compression.top_signal_count
response.trade_ideas
response.risk_context.no_trade_zones
```

and that `trade_ideas.len() <= 3`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test contract_whale_routes_tests intelligence_response_includes_signal_compression_trade_ideas_and_risk_context`

Expected: FAIL because the response fields do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add new response structs and fields to `src/contract_whale_monitor/types.rs` and thread them through `ContractWhaleIntelligenceResponse` with empty/default values.

- [ ] **Step 4: Run test to verify it passes or fails later**

Run: `cargo test --test contract_whale_routes_tests intelligence_response_includes_signal_compression_trade_ideas_and_risk_context`

Expected: FAIL on empty behavior or PASS on type existence, moving the next failure to content behavior.

### Task 2: Implement backend signal-compression and risk-context projections

**Files:**
- Create: `src/contract_whale_monitor/intelligence/signal_compression.rs`
- Create: `src/contract_whale_monitor/intelligence/risk.rs`
- Modify: `src/contract_whale_monitor/intelligence/mod.rs`
- Test: `tests/contract_whale_routes_tests.rs`

**Interfaces:**
- Consumes: `&[ContractWhaleSignal]`, `&ContractWhaleRegimeSnapshot`, `&[ContractWhaleLiquidityBehavior]`, `&[ContractWhaleRankedEvent]`
- Produces: `build_signal_compression_summary(...) -> ContractWhaleSignalCompressionSummary`
- Produces: `build_trade_ideas(...) -> Vec<ContractWhaleTradeIdea>`
- Produces: `build_risk_context(...) -> ContractWhaleRiskContext`

- [ ] **Step 1: Write the failing behavior test**

Add a seeded-signal test asserting:

- at most 3 trade ideas survive
- same-family multi-window signals compress into one idea
- idea direction uses bias labels instead of `LONG`/`SHORT`
- each idea exposes entry zone, invalidation, confidence, and structure context
- fake breakout risk populates risk context

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test contract_whale_routes_tests signal_compression_limits_trade_ideas_and_exposes_bias_semantics`

Expected: FAIL because the projection logic does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Implement:

- `signal_compression.rs`
  - dedup by same symbol + setup family + direction bias
  - score filter
  - truncate to top 3
- `risk.rs`
  - derive no-trade zones
  - derive fake-breakout risk summary
  - derive chop/conflict summary
- update `intelligence/mod.rs` to attach the new data

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test contract_whale_routes_tests signal_compression_limits_trade_ideas_and_exposes_bias_semantics`

Expected: PASS

### Task 3: Expose the new fields through the intelligence API route

**Files:**
- Modify: `src/api/contract_whale_routes.rs`
- Test: `tests/contract_event_routes_tests.rs`

**Interfaces:**
- Produces: `GET /api/contract-whale/intelligence-terminal?symbol=BTC&range=24h`

- [ ] **Step 1: Write the failing route test**

Extend the existing intelligence route test in `tests/contract_event_routes_tests.rs` to assert JSON includes:

```rust
payload["signalCompression"]
payload["tradeIdeas"]
payload["riskContext"]
```

and that the first trade idea, if present, includes:

```rust
payload["tradeIdeas"][0]["directionBias"]
payload["tradeIdeas"][0]["entryZone"]["label"]
payload["tradeIdeas"][0]["invalidation"]["priceLevel"]
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test contract_event_routes_tests contract_whale_intelligence_terminal_route_exposes_signal_compression_trade_ideas_and_risk_context`

Expected: FAIL because the route does not serialize these fields yet.

- [ ] **Step 3: Write minimal implementation**

Update `contract_whale_intelligence_terminal_route` serialization path to return the enriched response.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test contract_event_routes_tests contract_whale_intelligence_terminal_route_exposes_signal_compression_trade_ideas_and_risk_context`

Expected: PASS

### Task 4: Add frontend normalization for signal compression and trade ideas

**Files:**
- Modify: `toxic-order-monitor/src/api/contractWhale.js`
- Test: `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`

**Interfaces:**
- Consumes: intelligence-terminal JSON
- Produces: normalized `intelligenceTerminal.signalCompression`, `tradeIdeas`, `riskContext`

- [ ] **Step 1: Write the failing API normalizer test**

Add a test in `toxic-order-monitor/src/tests/ContractWhaleApi.test.js` for an intelligence-terminal payload that includes `signalCompression`, `tradeIdeas`, and `riskContext`, and assert the normalized result preserves:

- top signal count
- one trade idea with direction bias and entry zone
- no-trade zones in risk context

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- --run src/tests/ContractWhaleApi.test.js`

Expected: FAIL because the normalizer does not map the new fields yet.

- [ ] **Step 3: Write minimal implementation**

Extend the intelligence response normalizer and fallback object in `toxic-order-monitor/src/api/contractWhale.js`.

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- --run src/tests/ContractWhaleApi.test.js`

Expected: PASS

### Task 5: Add terminal tabs and render Trade Ideas / Risk Context

**Files:**
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: normalized `intelligenceTerminal`
- Produces: tabbed `InstitutionalAnalysisTerminalPanel`

- [ ] **Step 1: Write the failing UI test**

Extend `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx` to assert:

- default tab is `Market Intelligence`
- `Trade Ideas` tab exists
- `Risk / No-Trade` tab exists
- clicking `Trade Ideas` reveals:
  - direction bias
  - entry zone
  - invalidation
  - confidence
- clicking `Risk / No-Trade` reveals no-trade zones and fake-breakout risk
- no order-execution copy appears

- [ ] **Step 2: Run test to verify it fails**

Run: `npm test -- --run src/tests/ContractWhaleMonitor.test.jsx`

Expected: FAIL because the tabs and new fields are not rendered yet.

- [ ] **Step 3: Write minimal implementation**

Update `InstitutionalAnalysisTerminalPanel` to:

- manage local active-tab state
- render three tabs:
  - `Market Intelligence`
  - `Trade Ideas`
  - `Risk / No-Trade`
- keep the page-level event stream below untouched
- add presentational helpers for trade idea cards and risk context cards

- [ ] **Step 4: Run test to verify it passes**

Run: `npm test -- --run src/tests/ContractWhaleMonitor.test.jsx`

Expected: PASS

### Task 6: Verify, self-check, and sync server

**Files:**
- Modify: none unless fixes are needed after verification

**Interfaces:**
- Verifies backend, frontend, and deployment health

- [ ] **Step 1: Run targeted backend verification**

Run:

```bash
cargo test --test contract_whale_routes_tests
cargo test --test contract_event_routes_tests
```

Expected: PASS

- [ ] **Step 2: Run targeted frontend verification**

Run:

```bash
npm test -- --run src/tests/ContractWhaleApi.test.js
npm test -- --run src/tests/ContractWhaleMonitor.test.jsx
```

Expected: PASS

- [ ] **Step 3: Run integration sanity checks**

Run:

```bash
cargo check
npm run build
```

Expected: PASS

- [ ] **Step 4: Self-check the boundary**

Verify:

- trade ideas are visible only inside the separate terminal tab
- analysis tab remains the default and unchanged in tone
- no event-stream section gained execution language
- no buy/sell action verbs or buttons were added

- [ ] **Step 5: Sync server**

Run the established project sync path after local checks pass, then verify the page and API:

```bash
git status --short
git add <relevant files only>
git commit -m "add contract whale signal compression terminal"
git push origin main
```

Server:

```bash
cd /opt/toxic-order-monitor-rs
git pull --ff-only
docker compose up -d --build backend frontend
docker compose ps
curl -fsS "http://127.0.0.1:5173/api/contract-whale/intelligence-terminal?symbol=BTC" | head -c 4000
```

Expected:

- backend and frontend healthy
- intelligence terminal response includes `signalCompression`, `tradeIdeas`, and `riskContext`
- `/contract-whale` renders all three tabs and remains analysis-first
