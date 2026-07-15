# Unified Institutional Workspace UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply the existing compact institutional contract-whale workspace visual language to every primary frontend route without changing page data, behavior, copy, controls, or information order.

**Architecture:** Replace the route-specific shell switch with one shared graphite workspace shell and compact sidebar. Keep the existing contract command bar, add a compact shared global header and reusable page intro for non-contract routes, then use route-neutral workspace CSS tokens plus a scoped compatibility bridge to restyle existing panels, fields, tables, and notices without rewriting their business logic.

**Tech Stack:** React 19, JSX, React Router 6, Tailwind CSS 3, custom CSS, Vitest, Testing Library, Vite 7, Docker Compose, nginx

## Global Constraints

- Preserve the current read-only monitor boundary; do not add order, execution, signing, payment, withdrawal, delete, deploy, admin, or runtime-rule controls to the frontend.
- Preserve existing APIs, stores, WebSocket behavior, polling, caches, filters, thresholds, dialogs, Discord delivery rules, and information order.
- Keep the existing contract-whale command bar, event-first hierarchy, signal details, and diagnostic sections intact.
- Keep active navigation, status, loading, empty, stale, disabled, `spot_only`, dry-run, and unconfigured states understandable through visible text rather than color alone.
- Keep every existing accessible label and route path unless the plan explicitly adds a stable workspace test ID.
- Keep page-level horizontal overflow disabled; navigation and tables own their narrow-screen overflow.
- Do not expose `.env`, tokens, webhook URLs, authorization headers, wallet material, or other secrets in code, tests, build output, screenshots, commits, or deployment diagnostics.
- Deploy the verified frontend only; the backend container must keep the same `StartedAt` and restart count.

---

### Task 1: Lock full-route workspace coverage with a failing shell test

**Files:**
- Create: `toxic-order-monitor/src/tests/WorkspaceShell.test.jsx`
- Test: `toxic-order-monitor/src/tests/WorkspaceShell.test.jsx`

**Interfaces:**
- Consumes: `Dashboard`, `MemoryRouter`, existing sidebar routes
- Produces: stable test IDs `workspace-shell`, `workspace-main`, `workspace-sidebar`, `workspace-command-header`, and `workspace-page-header`

- [ ] **Step 1: Create an isolated route-shell test with business components mocked**

The test renders the real `Dashboard`, `Header`, and `Sidebar`, while replacing data-heavy monitor panels with inert read-only probes. It covers:

```jsx
const ROUTES = [
  ["/dashboard", "监控首页", false],
  ["/contract-whale/btc", "BTC 合约监控", true],
  ["/contract-whale/eth", "ETH 合约监控", true],
  ["/spot-monitor/btc", "BTC 现货监控", false],
  ["/spot-monitor/eth", "ETH 现货监控", false],
  ["/liquidation-cascade", null, false],
  ["/alt-contract-monitor", "山寨合约异常", false],
  ["/new-token-watch", "新币合约监控", false],
  ["/signals", "异常信号", false],
  ["/history", "信号历史", false],
  ["/rules", "告警规则", false],
  ["/usage-guide", "使用指南", false],
  ["/discord", "Discord 设置", false],
  ["/settings", "系统设置", false],
];

it.each(ROUTES)("uses the unified workspace on %s", (path, activeLabel, contractRoute) => {
  render(
    <MemoryRouter initialEntries={[path]}>
      <Dashboard />
    </MemoryRouter>,
  );

  expect(screen.getByTestId("workspace-shell")).toHaveClass("workspace-shell");
  expect(screen.getByTestId("workspace-main")).toHaveClass("workspace-main");
  expect(screen.getByTestId("workspace-sidebar")).toBeInTheDocument();
  expect(screen.getByText("READ ONLY")).toBeInTheDocument();

  if (activeLabel) {
    expect(screen.getByRole("link", { name: activeLabel })).toHaveAttribute("aria-current", "page");
  }

  if (contractRoute) {
    expect(screen.queryByTestId("workspace-command-header")).not.toBeInTheDocument();
    expect(screen.getByTestId("contract-monitor-probe")).toBeInTheDocument();
  } else {
    expect(screen.getByTestId("workspace-command-header")).toBeInTheDocument();
  }
});
```

- [ ] **Step 2: Run the route-shell test and verify RED**

Run:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/WorkspaceShell.test.jsx
```

Expected: FAIL because `workspace-shell`, `workspace-main`, `workspace-sidebar`, and `workspace-command-header` do not exist on all routes.

---

### Task 2: Introduce the shared shell, compact sidebar, and compact command header

**Files:**
- Modify: `toxic-order-monitor/src/pages/Dashboard.jsx`
- Modify: `toxic-order-monitor/src/components/Sidebar.jsx`
- Modify: `toxic-order-monitor/src/components/Header.jsx`
- Modify: `toxic-order-monitor/src/index.css`
- Modify: `toxic-order-monitor/src/tests/Dashboard.test.jsx`
- Test: `toxic-order-monitor/src/tests/WorkspaceShell.test.jsx`
- Test: `toxic-order-monitor/src/tests/Dashboard.test.jsx`

**Interfaces:**
- Consumes: `viewMode`, `isContractWhaleView`, existing header status props, existing sidebar menu entries
- Produces: shared shell test IDs and route classes `workspace-route-${viewMode}`

- [ ] **Step 1: Make `Dashboard` render one shell on every route**

Use the following shell contract:

```jsx
<div className="workspace-shell flex min-h-screen flex-col lg:flex-row" data-testid="workspace-shell">
  <Sidebar />
  <main
    className={[
      "workspace-main w-full min-w-0 flex-1",
      `workspace-route-${viewMode}`,
      isContractWhaleView ? "contract-workspace-main" : "",
    ].filter(Boolean).join(" ")}
    data-testid="workspace-main"
  >
    {!isContractWhaleView ? (
      <Header discordConnected={discordConnected} highUnhandledCount={highUnhandledCount} />
    ) : null}
    <div className={isContractWhaleView ? undefined : "workspace-content"}>
      {/* Preserve the existing route-selection order and content. */}
    </div>
  </main>
</div>
```

- [ ] **Step 2: Make `Sidebar` permanently use the compact route-neutral workspace variant**

Remove the `compact` prop and render one visual branch. Keep every `menuItems` entry, label, icon, alias, and route. The root must use:

```jsx
<aside
  className="workspace-sidebar contract-sidebar w-full shrink-0 border-b px-3 py-3 lg:sticky lg:top-0 lg:h-screen lg:w-[212px] lg:border-b-0 lg:border-r lg:px-2 lg:py-4"
  data-testid="workspace-sidebar"
>
```

The brand keeps the `W`, `Whale Desk`, and `有毒订单监控` labels. The bottom status keeps `READ ONLY` and `No execution · No signing`.

- [ ] **Step 3: Convert `Header` into the compact non-contract command bar**

Preserve its title, high-risk count, Discord state, time, and settings button. Use explicit classes and stable test ID:

```jsx
<header className="workspace-command-header" data-testid="workspace-command-header">
  <div className="workspace-command-brand">
    <p>Toxic Order Monitor</p>
    <h2>盘口异常监控大屏</h2>
    <span>READ-ONLY RISK WORKSPACE</span>
  </div>
  <div className="workspace-command-metrics">
    <div className="workspace-command-metric workspace-command-metric-danger">高风险未处理 …</div>
    <div className="workspace-command-metric">Discord …</div>
    <div className="workspace-command-metric">…current time…</div>
    <button aria-label="系统设置" className="workspace-command-settings" type="button">…</button>
  </div>
</header>
```

- [ ] **Step 4: Add the shared shell/header CSS tokens**

Define `--workspace-bg`, `--workspace-panel`, `--workspace-panel-raised`, `--workspace-line`, `--workspace-line-strong`, and `--workspace-muted` on `.workspace-shell`. Use a graphite grid background on `.workspace-main`, a sticky compact header on desktop, sharp separators, tabular status values, visible focus rings, and responsive stacking below `820px`.

- [ ] **Step 5: Update the existing contract-route assertions**

Replace `contract-workspace-main` and `contract-workspace-sidebar` test-ID expectations with `workspace-main` and `workspace-sidebar`, while retaining the assertion that the contract route has no generic `盘口异常监控大屏` header.

- [ ] **Step 6: Run focused tests and verify GREEN**

Run:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/WorkspaceShell.test.jsx src/tests/Dashboard.test.jsx
```

Expected: PASS with all routes on the shared shell and existing dashboard behavior intact.

---

### Task 3: Add reusable compact page intros without changing content order

**Files:**
- Create: `toxic-order-monitor/src/components/WorkspacePageHeader.jsx`
- Modify: `toxic-order-monitor/src/pages/Dashboard.jsx`
- Modify: `toxic-order-monitor/src/index.css`
- Modify: `toxic-order-monitor/src/tests/WorkspaceShell.test.jsx`
- Test: `toxic-order-monitor/src/tests/Dashboard.test.jsx`

**Interfaces:**
- Consumes: `eyebrow`, `title`, `description`, `badge`
- Produces: `WorkspacePageHeader` and stable test ID `workspace-page-header`

- [ ] **Step 1: Extend the shell test with route-specific page-intro expectations and verify RED**

Add assertions for the existing dedicated page routes:

```jsx
const PAGE_INTROS = [
  ["/spot-monitor/btc", "BTC 现货监控"],
  ["/spot-monitor/eth", "ETH 现货监控"],
  ["/liquidation-cascade", "强平瀑布预测"],
  ["/alt-contract-monitor", "山寨合约异常监控"],
  ["/new-token-watch", "新币合约监控"],
  ["/usage-guide", "用户使用指南"],
];

it.each(PAGE_INTROS)("uses the compact page intro on %s", (path, title) => {
  renderDashboard(path);
  expect(screen.getByTestId("workspace-page-header")).toHaveTextContent(title);
});
```

Run the single test file and confirm it fails because `workspace-page-header` is missing.

- [ ] **Step 2: Create `WorkspacePageHeader`**

```jsx
export default function WorkspacePageHeader({ eyebrow, title, description, badge }) {
  return (
    <section className="workspace-page-header" data-testid="workspace-page-header">
      <div className="workspace-page-heading">
        <p>{eyebrow}</p>
        <h2>{title}</h2>
        <span>{description}</span>
      </div>
      {badge ? <div className="workspace-page-badge">{badge}</div> : null}
    </section>
  );
}
```

- [ ] **Step 3: Replace only the repeated page-intro markup in `Dashboard.jsx`**

Use `WorkspacePageHeader` in `LiquidationCascadePage`, `SpotWhalePage`, `AltContractPage`, `NewTokenWatchPage`, and `UsageGuidePage`. Pass their current eyebrow, title, description, and read-only badge text verbatim, then render the existing monitor component immediately afterward.

- [ ] **Step 4: Add compact intro CSS**

Implement a low-height bordered strip with a restrained eyebrow, route title, description, and right-aligned read-only badge. At narrow widths it stacks without reordering the page content.

- [ ] **Step 5: Run page and shell tests and verify GREEN**

Run:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/WorkspaceShell.test.jsx src/tests/Dashboard.test.jsx
```

Expected: PASS with all dedicated route titles and safety descriptions unchanged.

---

### Task 4: Unify legacy panels, fields, tables, notices, and responsive overflow

**Files:**
- Modify: `toxic-order-monitor/src/index.css`
- Modify: `toxic-order-monitor/src/components/SpotWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/tests/SpotWhaleMonitor.test.jsx`
- Test: `toxic-order-monitor/src/tests/BinanceAltContractMonitor.test.jsx`
- Test: `toxic-order-monitor/src/tests/NewTokenWatch.test.jsx`

**Interfaces:**
- Consumes: `.workspace-content`, existing `console-panel`, `console-panel-muted`, `console-field`, `console-row`, and legacy panel utility classes
- Produces: route-neutral institutional panel styling with contained overflow

- [ ] **Step 1: Add a failing spot-monitor presentation contract**

Assert that the spot monitor root has `workspace-monitor-panel`, every history filter has `console-field`, and rendered data rows have `console-row`. Run the focused spot test and confirm these explicit classes are missing.

- [ ] **Step 2: Add explicit workspace primitives to `SpotWhaleMonitor`**

Add `workspace-monitor-panel console-panel` to the root section, `console-field` to each history select/date input, and `console-row` to interactive table rows. Keep every existing class, label, event handler, pagination rule, detail button, and status message.

- [ ] **Step 3: Add a scoped compatibility bridge for existing panels**

Under `.workspace-content`, normalize existing `console-panel` and legacy `section.rounded-2xl` / `article.rounded-2xl` surfaces to the shared graphite variables. Remove decorative glow, reduce large radii, keep semantic borders, and style nested muted panels, table headers, rows, inputs, selects, and buttons without altering DOM order or event behavior.

The bridge must be excluded from `.contract-workspace-main` so the approved contract-specific hierarchy remains unchanged.

- [ ] **Step 4: Contain narrow-screen overflow**

Set `min-width: 0` on workspace grid children, `overflow-x: clip` on the page shell, and preserve `overflow-x: auto` on navigation/table containers. Below `820px`, reduce content padding and stack page headers and command metrics.

- [ ] **Step 5: Run focused component regressions**

Run:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/SpotWhaleMonitor.test.jsx src/tests/BinanceAltContractMonitor.test.jsx src/tests/NewTokenWatch.test.jsx src/tests/Dashboard.test.jsx src/tests/WorkspaceShell.test.jsx
```

Expected: PASS with existing data, filter, table, detail, empty-state, and safety behavior intact.

---

### Task 5: Verify, review, commit, push, deploy, and prove the live frontend

**Files:**
- Verify: `toxic-order-monitor/src/pages/Dashboard.jsx`
- Verify: `toxic-order-monitor/src/components/Header.jsx`
- Verify: `toxic-order-monitor/src/components/Sidebar.jsx`
- Verify: `toxic-order-monitor/src/components/WorkspacePageHeader.jsx`
- Verify: `toxic-order-monitor/src/components/SpotWhaleMonitor.jsx`
- Verify: `toxic-order-monitor/src/index.css`
- Verify: `toxic-order-monitor/src/tests/WorkspaceShell.test.jsx`
- Verify: `docs/superpowers/specs/2026-07-15-unified-institutional-workspace-ui-design.md`
- Verify: `docs/superpowers/plans/2026-07-15-unified-institutional-workspace-ui.md`

**Interfaces:**
- Consumes: completed frontend changes and the existing Compose/nginx deployment
- Produces: verified commit on `main`, rebuilt frontend container, live route evidence, unchanged backend process

- [ ] **Step 1: Run final local validation**

Run:

```powershell
npm --prefix toxic-order-monitor test
npm --prefix toxic-order-monitor run build
git diff --check
```

Expected: all frontend tests pass, Vite build exits `0`, and `git diff --check` emits no errors.

- [ ] **Step 2: Inspect all routes locally in a real browser**

Start Vite on a free loopback port and inspect every primary route at desktop and narrow widths. Verify the graphite shell, compact sidebar/header, active navigation, page intro, contained tables, readable state labels, no page-level overflow, and no execution controls.

- [ ] **Step 3: Review the final diff and secret boundary**

Run:

```powershell
git status --short
git diff --stat
git diff -- toxic-order-monitor/src docs/superpowers/specs/2026-07-15-unified-institutional-workspace-ui-design.md docs/superpowers/plans/2026-07-15-unified-institutional-workspace-ui.md
git diff --cached --check
```

Stage only the files listed in this plan. Do not stage `.env`, `.runtime`, logs, `data`, `dist`, or `node_modules`.

- [ ] **Step 4: Commit and push**

```powershell
git add docs/superpowers/specs/2026-07-15-unified-institutional-workspace-ui-design.md docs/superpowers/plans/2026-07-15-unified-institutional-workspace-ui.md toxic-order-monitor/src/pages/Dashboard.jsx toxic-order-monitor/src/components/Header.jsx toxic-order-monitor/src/components/Sidebar.jsx toxic-order-monitor/src/components/WorkspacePageHeader.jsx toxic-order-monitor/src/components/SpotWhaleMonitor.jsx toxic-order-monitor/src/index.css toxic-order-monitor/src/tests/Dashboard.test.jsx toxic-order-monitor/src/tests/SpotWhaleMonitor.test.jsx toxic-order-monitor/src/tests/WorkspaceShell.test.jsx
git commit -m "feat: unify monitoring workspace ui"
git push origin main
```

Expected: one scoped commit pushed to `origin/main`.

- [ ] **Step 5: Record backend continuity before deployment**

Over SSH, record the deployed commit plus `toxic-bot` `StartedAt` and `RestartCount` without printing environment values.

- [ ] **Step 6: Pull and rebuild only the frontend service**

On `/opt/toxic-order-monitor-rs`:

```bash
git pull --ff-only
docker compose build frontend
docker compose up -d --no-deps frontend
docker compose ps frontend backend
```

Expected: server HEAD matches the pushed commit; `toxic-frontend` and `toxic-bot` are healthy.

- [ ] **Step 7: Verify live health, route responses, assets, and backend continuity**

Check `/healthz`, `/readyz`, and all primary public SPA routes through `http://127.0.0.1:5173`. Confirm each returns `200`, references the current hashed frontend asset, and the backend `StartedAt` / `RestartCount` values are unchanged.

- [ ] **Step 8: Inspect representative live routes in the browser**

Open the deployed `/contract-whale/btc`, `/spot-monitor/btc`, `/alt-contract-monitor`, `/new-token-watch`, `/signals`, `/usage-guide`, `/discord`, and `/settings` routes. Confirm the same workspace shell spans every page and no hashed-asset refresh error appears.

---

## Self-Review

- Spec coverage: route shell, sidebar, global header, dedicated page intros, shared primitives, responsive behavior, read-only boundary, tests, browser inspection, commit, and frontend-only deployment are mapped.
- Placeholder scan: no deferred implementation marker or unspecified code step remains.
- Interface consistency: all tasks use the same workspace test IDs and `WorkspacePageHeader` prop names.
- Scope: frontend presentation plus tests/docs only; backend, API, alert, threshold, polling, cache, data, and runtime-rule behavior remain unchanged.
