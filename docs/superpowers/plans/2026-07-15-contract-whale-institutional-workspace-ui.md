# Contract Whale Institutional Workspace UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Recreate the approved institutional trading-workstation concept on `/contract-whale/:symbol`, with the event tape dominating the first viewport and all existing read-only monitoring behavior preserved.

**Architecture:** Keep the existing API, polling, cache, filtering, signal-detail, and downstream analysis components unchanged. Add a contract-route-specific shell in `Dashboard.jsx`/`Sidebar.jsx`, then reshape the top of `ContractWhaleMonitor.jsx` into a command bar, status ribbon, compact event tape, and right insight rail; retain the deeper lifecycle, diagnostics, trajectory, and analysis sections below the fold.

**Tech Stack:** React 19, JSX, Tailwind CSS 3, custom CSS in `index.css`, Heroicons, Vitest, Testing Library, Vite

## Global Constraints

- Preserve read-only monitoring; do not add buy, sell, order, signing, or execution controls.
- Do not change backend routes, thresholds, classifier semantics, polling intervals, or the event-feed session cache.
- Preserve the existing BTC `窗口总流量 ≥ 500 BTC` display semantics and ETH route isolation.
- Keep all current filters accessible by their existing labels so API/filter interaction tests remain valid.
- Render only real values already present in summary, event, intelligence, and platform-capability payloads; show `N/A` instead of invented market values.
- Keep keyboard-accessible rows, focus states, detail buttons, and reduced-motion behavior.
- Do not commit, push, or deploy unless the user separately authorizes those actions.

---

### Task 1: Lock the approved workspace hierarchy with failing UI tests

**Files:**
- Modify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`
- Modify: `toxic-order-monitor/src/tests/Dashboard.test.jsx`

**Interfaces:**
- Consumes: `ContractWhaleMonitor`, `Dashboard`, existing mocked contract-whale payloads
- Produces: stable test IDs `contract-workspace-command-bar`, `contract-workspace-status-ribbon`, `contract-event-tape`, and `contract-insight-rail`

- [ ] **Step 1: Extend the existing layout test with the approved first-viewport contract**

```jsx
expect(screen.getByTestId("contract-workspace-command-bar")).toHaveTextContent("BTC / PERP");
expect(screen.getByTestId("contract-workspace-command-bar")).toHaveTextContent("只读监控");
expect(screen.getByTestId("contract-workspace-status-ribbon")).toBeInTheDocument();
expect(screen.getByTestId("contract-event-tape")).toContainElement(
  screen.getByTestId("raw-contract-whale-signals"),
);
expect(screen.getByTestId("contract-insight-rail")).toHaveTextContent("市场结构");
expect(screen.getByTestId("contract-insight-rail")).toHaveTextContent("流动性与 OI");
expect(screen.getByTestId("contract-insight-rail")).toHaveTextContent("交易机会 / 风险");
```

- [ ] **Step 2: Assert the contract route uses the compact route shell**

```jsx
expect(screen.getByTestId("contract-workspace-main")).toHaveClass("contract-workspace-main");
expect(screen.queryByText("盘口异常监控大屏")).not.toBeInTheDocument();
expect(screen.getByTestId("contract-workspace-sidebar")).toBeInTheDocument();
```

- [ ] **Step 3: Run the focused tests and confirm RED**

Run:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleMonitor.test.jsx src/tests/Dashboard.test.jsx
```

Expected: FAIL because the workspace test IDs and compact contract shell do not exist.

---

### Task 2: Introduce the contract-route-specific page shell

**Files:**
- Modify: `toxic-order-monitor/src/pages/Dashboard.jsx`
- Modify: `toxic-order-monitor/src/components/Sidebar.jsx`
- Modify: `toxic-order-monitor/src/index.css`

**Interfaces:**
- Consumes: `viewMode === "contract-whale"`, `useLocation().pathname`, existing navigation links
- Produces: compact contract sidebar and edge-to-edge contract workspace main surface

- [ ] **Step 1: Make `Dashboard` select the route-specific shell without changing other routes**

```jsx
<div className={`flex min-h-screen flex-col lg:flex-row ${isContractWhaleView ? "contract-workspace-shell" : "bg-[#07111f]"}`}>
  <Sidebar compact={isContractWhaleView} />
  <main
    className={isContractWhaleView ? "contract-workspace-main w-full min-w-0 flex-1" : "w-full min-w-0 flex-1 p-4 lg:p-6"}
    data-testid={isContractWhaleView ? "contract-workspace-main" : undefined}
  >
    {!isContractWhaleView ? <Header ... /> : null}
    {isContractWhaleView ? <ContractWhalePage symbol={mainstreamSymbol} /> : ...}
  </main>
</div>
```

- [ ] **Step 2: Add a compact `Sidebar` variant while preserving link names and routes**

```jsx
export default function Sidebar({ compact = false }) {
  return (
    <aside
      className={compact ? "contract-sidebar ..." : "...existing classes..."}
      data-testid={compact ? "contract-workspace-sidebar" : undefined}
    >
      {/* Existing NavLink accessible names remain unchanged. */}
    </aside>
  );
}
```

- [ ] **Step 3: Add route-scoped graphite surfaces and precise separators**

```css
.contract-workspace-shell { background: #090c10; color: #e6edf3; }
.contract-workspace-main { background: #090c10; padding: 0; }
.contract-sidebar { background: #080b0f; border-color: #20262e; }
```

- [ ] **Step 4: Run `Dashboard.test.jsx` and confirm the shell passes without breaking other routes**

Run:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/Dashboard.test.jsx
```

Expected: PASS.

---

### Task 3: Build the real-data command bar and status ribbon

**Files:**
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/index.css`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: `assetSymbol`, `summary`, `latestItems`, `contractEvents`, `currentDisplayIntelligence`, `state.dataSlices`
- Produces: `ContractWorkspaceCommandBar` and `ContractWorkspaceStatusRibbon`

- [ ] **Step 1: Derive market metrics without fabricating values**

```jsx
function deriveWorkspaceMarketMetrics({ contractEvents, latestItems, summary }) {
  const latest = latestItems[0] || contractEvents[0] || null;
  return {
    price: signalTriggerPrice(latest),
    priceMovePct: numberOrNull(latest?.priceMovePct),
    fundingRate: numberOrNull(latest?.fundingRate),
    oiDeltaBtc: numberOrNull(latest?.oiDelta ?? latest?.eventLifecycle?.netOiDeltaBtc),
    notional24h: contractEvents.reduce((sum, item) => sum + (Number(item.notionalUsd) || 0), 0),
    sourceCount: activeContractSourceNames(summary).length,
  };
}
```

- [ ] **Step 2: Render the command bar with a live local clock and explicit read-only state**

The command bar must contain the selected symbol/perpetual market, latest available price, move, funding, OI delta, visible-event notional, source count, clock, health dot, and `只读监控` copy. Any unavailable metric renders `N/A`.

- [ ] **Step 3: Render the compact status ribbon**

The ribbon must expose regime, direction, signal grade, health, threshold profile, and display filter in a single scan line. It consumes the same summary/intelligence values already rendered in the old status cards.

- [ ] **Step 4: Run the targeted component tests**

Run:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleMonitor.test.jsx
```

Expected: command bar and ribbon assertions PASS.

---

### Task 4: Replace the historical debug grid with the compact event tape

**Files:**
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/index.css`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: normalized contract event items and existing formatting helpers
- Produces: `ContractEventTapeTable({ items, onOpenSignal, testId, volumeLabel, volumeTooltip })`

- [ ] **Step 1: Keep the current data and interactions but reduce the visible columns to trader-priority fields**

```jsx
const columns = [
  "时间",
  "事件 / 价格",
  "方向",
  "等级",
  "换手量",
  "净流量",
  "名义价值",
  "价格响应",
  "OI 背景",
  "来源 / Discord",
];
```

Each row must keep `tabIndex=0`, Enter/Space activation, the existing signal-detail target, severity/impact labels, exact price text, volume semantics, OI explanation, source, Discord state, and an accessible detail button.

- [ ] **Step 2: Add a deterministic micro-trace based on the event's real price move**

```jsx
function EventImpactTrace({ value }) {
  const magnitude = Math.min(1, Math.abs(Number(value) || 0) / 1.5);
  return <span aria-hidden="true" className="contract-impact-trace">...</span>;
}
```

The trace is a visual encoding only; the exact signed percentage remains visible as text.

- [ ] **Step 3: Compress explanatory copy without removing diagnostics**

Healthy state shows one-line filter/freshness metadata. Stale/unavailable state keeps the existing warning copy in a compact amber inline status. Loading and empty states keep their existing user-visible wording.

- [ ] **Step 4: Preserve lifecycle/debug tables below the fold**

Only `HistoricalEventStreamPanel` switches to `ContractEventTapeTable`; lifecycle and diagnostic tables continue using `RawSignalDebugTable` so no forensic fields are lost.

- [ ] **Step 5: Run event, filter, cache, keyboard, and detail tests**

Run:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleMonitor.test.jsx
```

Expected: all component tests PASS, including session-cache refresh and detail-modal coverage.

---

### Task 5: Assemble the first-viewport insight rail and preserve deep analysis below

**Files:**
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/index.css`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: `currentDisplayIntelligence`, `summary`, `latestItems`, derived trade ideas
- Produces: `ContractDeskInsightRail`

- [ ] **Step 1: Render three compact, real-data modules**

```jsx
<aside data-testid="contract-insight-rail">
  <DeskMarketStructureSnapshot intelligence={intelligence} summary={summary} />
  <DeskLiquidityOiSnapshot intelligence={intelligence} latestItems={latestItems} />
  <DeskOpportunityRiskSnapshot intelligence={intelligence} summary={summary} />
</aside>
```

Market structure shows regime, bias, and confidence. Liquidity/OI shows the strongest real liquidity behavior and OI delta/context. Opportunity/risk shows the top derived setup plus the first no-trade/invalidation zone. Empty data renders calm `暂无明确结构` text.

- [ ] **Step 2: Use a 65/35 first-viewport grid and move jump tabs under it**

```jsx
<section className="contract-primary-grid" data-testid="primary-analysis-grid">
  <HistoricalEventStreamPanel ... />
  <ContractDeskInsightRail ... />
</section>
<EventFirstJumpNavigation />
```

- [ ] **Step 3: Retain the existing full structure, liquidity, setup, lifecycle, risk, diagnostics, trajectory, and event sections below the fold**

No API data or forensic content is removed; only the first-viewport summary hierarchy changes.

- [ ] **Step 4: Run the focused tests and confirm document order**

Expected order: command bar → status ribbon → filters/health → event tape + insight rail → jump tabs → deep analysis.

---

### Task 6: Verify the complete redesign

**Files:**
- Verify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Verify: `toxic-order-monitor/src/components/Sidebar.jsx`
- Verify: `toxic-order-monitor/src/pages/Dashboard.jsx`
- Verify: `toxic-order-monitor/src/index.css`
- Verify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`
- Verify: `toxic-order-monitor/src/tests/Dashboard.test.jsx`

**Interfaces:**
- Consumes: completed workspace redesign
- Produces: passing suite, production build, and visual evidence

- [ ] **Step 1: Run focused tests**

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleMonitor.test.jsx src/tests/Dashboard.test.jsx
```

- [ ] **Step 2: Run the complete frontend suite**

```powershell
npm --prefix toxic-order-monitor test
```

- [ ] **Step 3: Build production assets**

```powershell
npm --prefix toxic-order-monitor run build
```

- [ ] **Step 4: Start or reuse the local frontend and inspect `/contract-whale/btc` in a real browser**

Verify at desktop width and a narrow responsive width that the event tape stays first, no horizontal page overflow occurs, filters remain operable, rows open the read-only detail modal, and no buy/sell execution actions appear.

- [ ] **Step 5: Compare the browser screenshot with the approved concept**

Acceptance: graphite shell, compact navigation, real-data command bar, one-line status ribbon, dominant event tape, three-module insight rail, restrained semantic colors, sharp separators, no giant recovery banner, and no generic dashboard header.

---

## Self-Review

- Spec coverage: shell, command bar, status ribbon, event tape, insight rail, deep-analysis preservation, read-only boundary, tests, build, and browser verification are all mapped.
- Placeholder scan: no `TBD`, `TODO`, or unspecified implementation step remains.
- Type consistency: test IDs and component names are consistent across tasks; all data inputs already exist in `ContractWhaleMonitor` state.
- Scope: frontend-only redesign; no backend, execution, alert, threshold, polling, or deployment change.
