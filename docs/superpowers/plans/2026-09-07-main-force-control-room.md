# Main-force Control Room Implementation Plan

> Execute inline with executing-plans. The user requested direct bounded work; no subagents, automatic commits or deployment.

**Goal:** Replace the full UI layout and branding with the approved dynamic financial control room while preserving monitoring semantics.

**Architecture:** Keep domain adapters/stores and change presentation composition. Replace Sidebar's implementation with two-level top navigation, introduce a small event timeline, reorganize existing monitoring components, replace the previous terminal theme with a fresh full-width control-room system.

**Tech Stack:** React 19, Vite 7, Tailwind 3, Heroicons, SVG, Vitest.

## Global Constraints

No new dependencies, API/Rust/grade/notification/polling changes, secrets, commits or server operations. Preserve unrelated untracked work. Source baseline: 8de0c2c. Existing PRODUCT.md sidebar/layout instructions are superseded by the user's approved overhaul; safety requirements remain.

## Task 1 — Top navigation and branding

Files: components/Sidebar.jsx, Header.jsx, WorkspacePageHeader.jsx, pages/Dashboard.jsx, BinanceOrderflowRoute.jsx, App.jsx, index.html, public/whale-desk.svg, components/UsageGuide.jsx; tests/FinancialTerminal.test.jsx and WorkspaceShell.test.jsx (frontend paths relative to toxic-order-monitor/src).

Interface: `Sidebar({ runtimeBoundary })` remains compatible but renders a header containing `主导航`, `当前工作区`, and a native module directory. Group links navigate to existing routes; secondary links expose the active asset.

- [x] Add assertions for new brand, top navigation and group switching. Example: `expect(screen.getByRole('link', {name: '合约主力监控'})).toHaveAttribute('href', '/dashboard')`; after clicking 合约市场, expect BTC/ETH links in 当前工作区.
- [x] Run `npm test -- --run src/tests/FinancialTerminal.test.jsx`; verify new requirements fail before implementation.
- [x] Implement accessible primary/secondary navigation with normal Link/NavLink, native details directory, existing MotionControl, runtime badge and skip link. Remove desktop row/sidebar layout in both route shells; retain Header safety props.
- [x] Update route-shell assertions for top navigation, preserve active route/alias/runtime truth tests, run both suites.

## Task 2 — Market canvas, timeline and evidence composition

Files: new components/EventTraceTimeline.jsx; components/MonitorFlowDashboard.jsx, ContractWhaleMonitor.jsx, SpotWhaleMonitor.jsx, BinanceAltContractMonitor.jsx; tests/MonitorFlowDashboard.test.jsx, ContractWhaleMonitor.test.jsx, new EventTraceTimeline.test.jsx.

Interface: `EventTraceTimeline({ items = [], onSelect, selectedId })` consumes existing event `{ id, ts, symbol, direction }`, displays at most eight valid timestamped events in chronological order and calls `onSelect(item.id)`. No fetching, grading, interpolation or mutation. Without onSelect, items remain static readable observations.

- [x] Add new homepage and contract timeline expectations to the existing real component tests; verify them fail with `npm test -- --run src/tests/MonitorFlowDashboard.test.jsx src/tests/ContractWhaleMonitor.test.jsx`.
- [x] Recompose homepage around price chart + market links + focused evidence; retain snapshot aggregation, filters, pause and event normalization unchanged.
- [x] Add timeline to contract chart workspace, move filters/table to full width, group deeper evidence in an explicitly labeled research area, preserve all original data/detail callbacks and panels.
- [x] Test timeline invalid dates, duplicate IDs, bounded chronological ordering, callback target, keyboard click, empty state. Run timeline/home/contract suites and fix only failures caused by this task.

## Task 3 — Complete visual system and acceptance

Files: replace src/styles/terminal.css with src/styles/control-room.css; src/main.jsx; shared page, panel, table, forms and detail selectors; PRODUCT.md frontend visual baseline; local ignored browser acceptance artifacts.

- [x] Build deep-navy/electric-blue design tokens, full-width responsive grid, aligned numeric type, labeled states, sticky top navigation, scoped table scroll, visible keyboard focus, short transform/opacity feedback. Keep direction/grade colors semantic.
- [x] Honor `html[data-motion='off']` and `prefers-reduced-motion: reduce`; no random prices or activity. Update favicon and boot loading visual to the new brand.
- [x] Run `npm run test:full`, `npm run build`, and `git diff --check`; report and diagnose failures before proceeding.
- [x] Inspect desktop and mobile full routes using isolated Playwright/Brave fixtures, block mutation requests, clearly label screenshots SIMULATED. Verify navigation, timeline/details, chart controls, pause, motion, reduced motion and unavailable states. Preserve user browser sessions.
- [x] Record results and show a usable local preview. Keep code uncommitted and production unchanged.

## Acceptance record — 2026-09-07

- Implemented inline on `codex/main-force-behavior-v4`, baseline `8de0c2c`. No subagents, commits, push, SSH, deployment, backend/API/store/config/dependency changes.
- Final full suite: **41 files / 347 tests passed** (56.77s). Build passed (15.43s); existing large-bundle advisory remains. `git diff --check` passed.
- Browser: isolated headless Brave, all 15 destinations, 320/390/1024/1536-width checks. Covered primary/secondary navigation, 15-link directory, mobile directory clicks, chart range/keyboard, contract/spot timeline detail selection, pause, manual/system reduced motion and unavailable state. **No page script errors or mutation requests** (259 API reads in the final run).
- SIMULATED screenshots and report remain local and ignored under `.superpowers/ui-preview/control-room/`; no fixture data was added to the application. Actual local preview: http://127.0.0.1:5173/dashboard . Without a local backend the real page may correctly show waiting/unavailable states.
- Screenshot review corrected the mobile directory stacking and removed residual old page-header borders.
- Additional bounded presentation fix: the existing orderflow chart's fixed 60 future categories hid all real candles in its default view when only 1–24 samples were available. The future gutter now scales with sample count up to the existing 60-slot cap. Three regression tests cover short, full and empty histories; no synthetic series data added, default long-history zoom unchanged.
- Preserved canonical grades, detector/cohort distinction, read-only runtime warnings, filters, pagination, notifications and all existing domain panels. The previous terminal stylesheet was replaced with `control-room.css` and remains recoverable from Git.
- Scope is local UI delivery. A new explicit sync/deploy request is required to publish this redesign.

## Follow-up publishing authorization — 2026-09-07

The user subsequently requested “同步”, authorizing this completed UI to be committed, pushed and deployed to the existing server. Fresh preflight confirmed local/Git/server baseline 8de0c2c, 347 passing tests and a successful build. Publish frontend only, retain a rollback image and the server's untracked backups, compare served build provenance and verify backend container identity/start time remain unchanged. No backend, rating, notification or runtime configuration changes are included.
