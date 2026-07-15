# Contract Whale Detail Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the legacy BTC contract-whale detail modal with a compact institutional event inspector while preserving every existing read-only detail and interaction.

**Architecture:** Keep `ContractWhaleDetailModal` in `ContractWhaleMonitor.jsx` and change only its presentational hierarchy. Reuse `.workspace-dialog`, introduce scoped `contract-detail-*` classes in `index.css`, and extend the existing Testing Library regression test so the new layout contract cannot silently regress.

**Tech Stack:** React 19, JSX, Tailwind CSS 3, custom CSS, Vitest, Testing Library, Vite 7

## Global Constraints

- Preserve every API, formatter, field, section, selection path, and close action.
- Keep the modal read-only; add no trading, execution, signing, payment, deletion, deployment, or admin controls.
- Do not expose raw payloads, webhook URLs, tokens, secrets, or authorization values.
- Use the current graphite workspace tokens and existing dependencies only.
- Do not commit, push, deploy, or restart services without separate user authorization.

---

### Task 1: Lock the event-inspector structure with a failing test

**Files:**
- Modify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: existing signal-row click path and `aria-label="主力合约信号详情"`
- Produces: stable layout hooks `contract-detail-inspector`, `contract-detail-header`, `contract-detail-summary`, `contract-detail-body`, and `contract-detail-rail`

- [ ] **Step 1: Extend the existing detail-modal test**

After opening the dialog, assert:

```jsx
const dialog = screen.getByRole("dialog", { name: "主力合约信号详情" });
expect(dialog).toHaveClass("workspace-dialog", "contract-detail-inspector");
expect(screen.getByTestId("contract-detail-header")).toHaveTextContent("EVENT INSPECTOR");
expect(screen.getByTestId("contract-detail-summary")).toHaveTextContent("事件状态");
expect(screen.getByTestId("contract-detail-body")).toBeInTheDocument();
expect(screen.getByTestId("contract-detail-rail")).toHaveTextContent("Discord Gate");
expect(dialog).toHaveTextContent("READ ONLY");
```

- [ ] **Step 2: Verify RED**

Run:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleMonitor.test.jsx -t "opens a read-only detail modal from the signal row"
```

Expected: FAIL because the legacy dialog does not expose the new workspace classes or structural test IDs.

---

### Task 2: Build the institutional event inspector

**Files:**
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/index.css`
- Test: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: `signal`, `relatedSignals`, `summary`, `onClose`, and all existing detail helper functions
- Produces: the five stable `contract-detail-*` layout hooks from Task 1

- [ ] **Step 1: Replace the legacy modal frame**

Use a backdrop plus `.workspace-dialog.contract-detail-inspector`. The inspector owns internal scrolling and exposes the unchanged dialog name and close control.

- [ ] **Step 2: Add the command header and decision strip**

Render the symbol, signal type, direction, final result, `READ ONLY`, severity, lifecycle state, displayed volume, net direction, notional, and trigger price from the already selected signal.

- [ ] **Step 3: Create the asymmetric body**

Move core judgment and the unchanged Discord gate rows into a sticky `<aside data-testid="contract-detail-rail">`. Keep basic information and all other existing `DetailSection` content in `<main data-testid="contract-detail-body">`.

- [ ] **Step 4: Upgrade section and grid primitives**

Make `DetailSection` render a numbered-looking workspace heading and make `DetailGrid` use `.contract-detail-grid` / `.contract-detail-field` so long values wrap and numeric values stay scannable.

- [ ] **Step 5: Add scoped responsive CSS**

Define `contract-detail-*` styles in `index.css` with a `minmax(0, 1fr) 320px` desktop body, sticky rail, restrained surfaces, and single-column breakpoints at `960px` and `640px`.

- [ ] **Step 6: Verify GREEN**

Run the focused test command from Task 1. Expected: PASS with all pre-existing content and safety assertions still green.

---

### Task 3: Regression, build, and browser acceptance

**Files:**
- Verify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Verify: `toxic-order-monitor/src/index.css`
- Verify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: completed event inspector
- Produces: test, build, and visual evidence for the goal

- [ ] **Step 1: Run the complete component test file**

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleMonitor.test.jsx
```

Expected: all tests pass.

- [ ] **Step 2: Run the complete frontend suite**

```powershell
npm --prefix toxic-order-monitor test
```

Expected: all frontend tests pass.

- [ ] **Step 3: Run the production build**

```powershell
npm --prefix toxic-order-monitor run build
```

Expected: Vite exits with code 0 and emits `dist` assets.

- [ ] **Step 4: Inspect in a real browser**

Open `/contract-whale/btc`, select a historical event, and verify the command header, decision strip, sticky rail, all detail sections, close action, desktop layout, narrow layout, and absence of page-level horizontal overflow.

- [ ] **Step 5: Review the diff**

Run `git diff --check` and confirm only the approved detail-inspector files plus its spec and plan changed. Do not stage or commit.
