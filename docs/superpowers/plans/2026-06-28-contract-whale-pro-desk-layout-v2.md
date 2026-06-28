# Contract Whale Pro Desk Layout v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild `/contract-whale` into an event-first professional desk layout where event flow, structure, liquidity, setups, risk, and system status each have their own stable panel hierarchy.

**Architecture:** Keep the existing data-fetch layer and signal semantics unchanged, but replace the current mixed dashboard/tab presentation with a fixed desk layout. Reuse existing event/history/lifecycle/system diagnostics components where possible, then carve market structure, liquidity behavior, trade setups, and risk context into separate panels wired to the existing intelligence payload.

**Tech Stack:** React, JSX, Tailwind utility classes, Vitest, Testing Library, Vite

## Global Constraints

- Do not change detector thresholds, merge semantics, lifecycle semantics, or backend scoring behavior.
- Keep historical events as the primary first-screen panel.
- Trade setups must remain a distinct panel and not blend into the historical event list.
- Risk must stay visible in the top strip and in its own panel.
- Preserve existing server/API contracts and only refactor frontend presentation for this task.

---

### Task 1: Lock the desired desk layout with failing UI tests

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: `ContractWhaleMonitor` default component export
- Produces: failing assertions for desk panel hierarchy and standalone setup/risk panels

- [ ] Add assertions that `/contract-whale` renders standalone `Market Structure`, `Liquidity Map`, `Trade Setups`, and `Risk Context` headings.
- [ ] Add assertions that `HISTORICAL EVENTS (24h stream)` appears before `Market Structure`, `Market Structure` appears before `Trade Setups`, and `Trade Setups` appears before `System Status / Latency / Retention`.
- [ ] Add assertions that `Institutional Analysis Terminal` is no longer the visible panel title.
- [ ] Run: `npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleMonitor.test.jsx`
- [ ] Confirm the new expectations fail before implementation.

### Task 2: Split the current analysis terminal into desk panels

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`

**Interfaces:**
- Consumes: `state.intelligenceTerminal`, `summary`, `selectedSignalId`, `setSelectedSignalId`
- Produces: standalone desk panels for top bar, market structure, liquidity map, trade setups, and risk context

- [ ] Introduce a desk-level top strip that keeps regime, risk, and freshness visible.
- [ ] Replace the single `InstitutionalAnalysisTerminalPanel` mount with a multi-row grid layout:
  - historical events + market structure
  - liquidity map + trade setups
  - lifecycle + risk context
  - system status
- [ ] Make trade setup cards selectable so clicking a setup highlights its source signal through `selectedSignalId`.
- [ ] Keep lifecycle and system status below the main desk rows.

### Task 3: Preserve interaction clarity and event-first navigation

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`

**Interfaces:**
- Consumes: section ids for jump navigation
- Produces: updated jump-nav targets and setup-to-event focus behavior

- [ ] Expand jump navigation so it reflects the new desk sections.
- [ ] Keep event-first scroll focus by preserving the historical panel anchor and viewport height.
- [ ] Add setup click behavior that scrolls the event stream into view while opening/highlighting the related signal context.

### Task 4: Verify the desk layout locally

**Files:**
- Modify if needed after test feedback: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Verify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: updated desk panels
- Produces: green local verification

- [ ] Run: `npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleMonitor.test.jsx`
- [ ] Run: `npm --prefix toxic-order-monitor run build`
- [ ] Confirm the page builds with the new panel titles and layout.

### Task 5: Sync the verified frontend to the live server

**Files:**
- Modify only if deployment verification finds a frontend-only issue: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`

**Interfaces:**
- Consumes: committed local desk-layout changes
- Produces: updated live `/contract-whale`

- [ ] Stage only the files touched by this task.
- [ ] Commit with a desk-layout-specific message.
- [ ] Push `main`.
- [ ] Sync server with:
  - `git pull --ff-only`
  - `docker compose up -d --build frontend backend`
- [ ] Verify the live page shows the event-first desk layout.
