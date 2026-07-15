# Unified Institutional Workspace UI Design

**Date:** 2026-07-15

**Status:** Approved for specification review

## Goal

Extend the compact institutional trading-workstation visual language currently used by `/contract-whale/:symbol` to every route exposed by the primary sidebar, while preserving each route's existing data, behavior, copy, controls, and information order.

The result must remain a read-only monitoring console. It must not introduce order placement, signing, execution, payment, withdrawal, deletion, deployment, or other irreversible controls.

## Problem Statement

Commit `ee64090` introduced the new graphite workspace as an explicit contract-route variant:

- `Dashboard.jsx` enables `contract-workspace-shell` and `contract-workspace-main` only when `viewMode === "contract-whale"`.
- `Sidebar.jsx` enables its compact variant only when the `compact` prop is true.
- The new CSS is primarily scoped to `.contract-*` selectors.
- All other routes continue to render the earlier padded blue shell, large header, rounded cards, and older table/form styling.

This split makes navigation feel like switching between two products. The fix must remove the route-specific shell distinction and provide one shared workspace foundation without rewriting business behavior.

## Chosen Approach

Use a shared workspace foundation plus incremental component styling.

This approach was selected over two alternatives:

1. Global CSS overrides of existing Tailwind classes would be quicker but brittle, difficult to test, and likely to create unintended cross-component regressions.
2. Rewriting every page component would produce cleaner markup but would risk changing behavior, information order, and established test contracts.

The chosen approach introduces reusable workspace primitives at the shell and page-header levels, then migrates existing components to explicit shared classes where needed. Existing APIs, state, polling, filtering, dialogs, and table interactions remain intact.

## Route Coverage

The unified workspace must cover all primary and alias routes:

- `/` and `/dashboard`
- `/contract-whale/btc` and `/contract-whale/eth`
- `/spot-monitor/btc` and `/spot-monitor/eth`
- `/spot-monitor` and `/spot-whale` aliases
- `/liquidation-cascade`
- `/alt-contract-monitor`
- `/new-token-watch`
- `/signals`
- `/history`
- `/rules`
- `/usage-guide`
- `/discord`
- `/settings`

Unknown paths continue to use the existing redirect behavior.

## Architecture

### Shared Application Shell

`Dashboard.jsx` will render one workspace shell for every route:

- graphite background and precise separators;
- compact sidebar on desktop;
- horizontally scrollable compact navigation on narrow screens;
- edge-to-edge main workspace with controlled route-level padding;
- persistent read-only status in the sidebar;
- active navigation state expressed through text, background, and border, not color alone.

`Sidebar.jsx` will no longer switch between visually unrelated compact and legacy variants. Existing link labels, paths, icons, aliases, focus behavior, and accessible navigation name remain unchanged.

### Shared Page Header

Non-contract routes will use a shared compact page-header component that preserves the existing header's information while matching the contract command bar's density and visual hierarchy.

The shared header will expose stable slots for:

- product or module eyebrow;
- route title;
- concise context or safety description;
- high-risk count;
- Discord configuration state;
- current time or existing operational status.

The contract monitor keeps its current real-data command bar. Other pages do not invent prices, market metrics, or health values that their current data does not provide.

### Shared Visual Primitives

The stylesheet will define route-neutral workspace primitives for:

- shell and main content surfaces;
- compact page headers;
- primary and muted panels;
- status ribbons and metric cells;
- filter labels, selects, inputs, and buttons;
- tables, rows, headers, and horizontal overflow containers;
- semantic badges and notices;
- loading, empty, degraded, disabled, and error states;
- detail panels and read-only dialogs.

Semantic colors remain restrained:

- cyan for active navigation and informational focus;
- emerald for healthy or positive state;
- rose for critical or negative state;
- amber for warning, stale, degraded, or review-needed state;
- slate for neutral, disabled, or unavailable state.

Status meaning must always remain visible in text and must not depend on color alone.

### Existing Page Components

The implementation will preserve the current component and information order for:

- dashboard risk summary, filters, signal inbox, detail, logs, and charts;
- spot monitor flow, venue health, filters, event table, pagination, and detail behavior;
- Binance alt-contract monitor status, filters, event stream, audit, and diagnostics;
- new-token monitor content and controls;
- liquidation cascade predictor content;
- signal, history, and rule views;
- usage guide content;
- Discord and settings routes as currently resolved by `Dashboard.jsx`.

Components may receive route-neutral workspace class names or small presentation-only wrappers. No API or state-management logic will move unless required to make a shared presentational component possible.

## Data and Interaction Flow

All existing data flow remains unchanged:

```text
route -> Dashboard view selection -> existing page/component -> existing API/store/hooks -> rendered monitoring state
```

The unified shell and page-header primitives only consume already available presentation data. They do not fetch, mutate, cache, normalize, or filter business data.

Existing confirmation gates remain unchanged for frontend cache clearing, Discord test messages, and Discord signal pushes. Pending-state button protection and sensitive-value masking must not regress.

## Responsive Behavior

Desktop remains optimized for dense scanning:

- compact fixed-width sidebar;
- edge-to-edge workspace;
- sharp panel boundaries;
- tables keep meaningful minimum widths and horizontal overflow where needed.

At narrow widths:

- navigation becomes a horizontally scrollable top strip;
- command/header metrics wrap without page-level horizontal overflow;
- panels become a single column in their current document order;
- tables scroll inside their own containers;
- controls remain labeled and keyboard accessible;
- touch targets remain usable without introducing oversized card layouts.

Reduced-motion preferences must continue to disable non-essential transitions.

## Error, Empty, and Disabled States

The redesign must preserve every existing user-visible state and message. Presentation will be normalized so that:

- loading is calm and does not resemble a critical alert;
- empty data clearly states that no matching monitoring data is available;
- stale or degraded sources use amber text plus an explicit explanation;
- disabled, dry-run, spot-only, and unconfigured states are neutral operational states rather than failures;
- actual errors remain distinguishable and readable;
- malformed or failed data does not cause shell layout collapse.

## Testing Strategy

Implementation follows test-driven development.

### Route Shell Contract

Add failing tests first to prove that every primary route renders:

- the shared workspace shell;
- the compact workspace sidebar;
- a route-appropriate compact header or the existing contract command bar;
- the correct active navigation item;
- the read-only safety state;
- no legacy shell variant.

Alias and redirect tests remain intact.

### Component Regression Tests

Preserve existing accessible labels and test the shared presentation contracts without asserting CSS implementation details. Existing page tests must continue to prove:

- filters and tabs operate normally;
- tables remain accessible and horizontally contained;
- detail views and dialogs open normally;
- empty, loading, network-failure, malformed-response, and degraded states remain visible;
- confirmation gates and pending states remain enforced;
- no execution controls appear.

### Verification

Run, in order:

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/AppRoutes.test.jsx src/tests/Dashboard.test.jsx
npm --prefix toxic-order-monitor test -- --run src/tests/SpotWhaleMonitor.test.jsx src/tests/BinanceAltContractMonitor.test.jsx src/tests/NewTokenWatch.test.jsx
npm --prefix toxic-order-monitor test
npm --prefix toxic-order-monitor run build
```

Then inspect every covered route in a real browser at desktop and narrow responsive widths. Prefer Brave for user-facing preview. Browser acceptance must confirm no page-level horizontal overflow, readable state labels, operable controls, correct active navigation, and visual continuity across routes.

## Acceptance Criteria

The goal is complete when:

1. Every sidebar route uses the same graphite institutional workspace shell.
2. The compact sidebar and read-only boundary remain visible and consistent.
3. No route returns to the large blue legacy header or padded rounded-card application shell.
4. Existing data, behavior, copy, filters, tables, dialogs, and information order are preserved.
5. Contract-whale pages retain their current command bar, event-first hierarchy, and diagnostics.
6. Responsive layouts contain overflow within tables or navigation rather than the page.
7. Route and component regression tests pass.
8. The complete frontend test suite and production build pass.
9. Visual inspection covers all primary routes at desktop width and representative narrow widths.
10. No backend, live execution, trading, signing, payment, deletion, deployment, or alert-policy behavior changes.

## Non-Goals

- Backend or API changes.
- New trading, execution, signing, payment, delete, deploy, or admin controls.
- Changes to thresholds, classifiers, polling, caching, persistence, or Discord delivery rules.
- Reordering or removing existing page content.
- Inventing data to fill command bars or metric panels.
- A new design system package or broad dependency upgrade.
- Commit, push, deployment, or server restart without separate user authorization.
