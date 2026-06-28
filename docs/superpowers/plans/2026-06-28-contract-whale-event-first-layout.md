# Contract Whale Event-First Layout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorder `/contract-whale` so the historical event stream is the first-screen priority, analysis becomes secondary, lifecycle remains available, and latency/status moves into a collapsed support area.

**Architecture:** Keep all data fetching and business logic intact. Refactor the React composition in `ContractWhaleMonitor.jsx` so the existing event/history/lifecycle/status content is re-grouped into an event-first layout with lightweight jump navigation and a smaller first-screen path to core event rows.

**Tech Stack:** React JSX, Vitest, Testing Library, Tailwind utility classes, existing `contractWhale.js` frontend API.

## Global Constraints

- Do not change backend detector, merge, lifecycle, or API semantics.
- Keep historical, active, and closed event copy aligned with the existing volume-label semantics.
- Preserve read-only terminal boundaries; do not introduce execution wording.
- Avoid touching unrelated dirty files in the repo.

---

### Task 1: Lock layout behavior with failing tests

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: `ContractWhaleMonitor`
- Produces: coverage for heading order, jump navigation presence, and event-first historical section treatment

- [ ] Add a failing test that asserts `HISTORICAL EVENTS (24h stream)` appears before `Institutional Analysis Terminal` and `ACTIVE EVENTS (updated)` in document order.
- [ ] Add a failing test that asserts top jump navigation exposes `Events`, `Market`, `Active`, and `Status`.
- [ ] Run the targeted Vitest file and confirm the new expectations fail for the current layout.

### Task 2: Refactor page composition into event-first sections

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`

**Interfaces:**
- Consumes: current `state`, `summary`, filters, event datasets, and helper components already in the file
- Produces: reordered sections, jump navigation, historical section anchor, lifecycle section anchor, and collapsed system-status block

- [ ] Add section anchors / ids for events, market intelligence, lifecycle events, and system status.
- [ ] Move the historical event stream block ahead of analysis and lifecycle sections.
- [ ] Move trend/data-quality/platform/status diagnostics into a collapsed support section.
- [ ] Keep the historical table expanded and sized for first-screen visibility.

### Task 3: Verify and refine

**Files:**
- Modify if needed: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify if needed: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: updated component and tests
- Produces: passing targeted tests and a buildable frontend

- [ ] Run the targeted Vitest file until green.
- [ ] Run `npm --prefix toxic-order-monitor run build`.
- [ ] Do a quick browser verification that `/contract-whale` shows historical events on first view and the jump nav lands on the expected sections.
