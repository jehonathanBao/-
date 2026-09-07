# Financial Terminal UI Implementation Plan

> Execute inline using executing-plans. User requested bounded direct implementation, no subagents or automatic commits.

**Goal:** Deliver the approved dynamic dark fintech interface without changing monitoring or notification semantics.

**Architecture:** Keep existing API hooks, stores, adapters and routes. Refine shared shell and CSS; use a small presentation-only price chart for existing homepage candles and contract event samples. Dynamic effects are controlled globally and never fabricate data.

**Tech Stack:** React 19, Vite 7, Tailwind 3, Heroicons, Vitest.

## Global Constraints

Preserve all routes, canonical grades, risk colors, confirmation gates and secret masking. No API or Rust edits, new dependencies, automatic commits, deployment or notification tests against production. Preserve pre-existing untracked files.

## Task 1 — Shared shell and motion

Files: Sidebar.jsx, Header.jsx, App.jsx, Dashboard.jsx, BinanceOrderflowRoute.jsx, new MotionControl.jsx, styles/terminal.css, tests/FinancialTerminal.test.jsx (under toxic-order-monitor/src).

- [x] Add failing tests for grouped navigation preserving hrefs, working settings link and reduced-motion/visibility-aware motion control.
- [x] Run `npm test -- --run src/tests/FinancialTerminal.test.jsx` and inspect expected missing-feature failures.
- [x] Implement grouped persistent navigation, compact command header, skip link and a single global motion preference control.
- [x] Run the new tests plus WorkspaceShell and AppRoutes tests.

## Task 2 — Market overview and contract emphasis

Files: new components/PriceChart.jsx, components/MonitorFlowDashboard.jsx, components/ContractWhaleMonitor.jsx, tests/PriceChart.test.jsx, tests/MonitorFlowDashboard.test.jsx, index.css, styles/terminal.css.

- [x] Test missing/invalid samples, chronological sorting, sample range selection and keyboard chart inspection before implementation.
- [x] Replace decorative homepage animation with actual provided price samples, readable data cards and unified event tape. Keep buildMonitorFlowEvents and API calls unchanged.
- [x] Add a presentation-only event-trigger price chart to contract pages; preserve event tables, filters and detailed evidence. Correct asset glyph and explicitly label sampled/event-only metrics.
- [x] Apply one coherent shell/card/table/input system across remaining routes. Include mobile layout, focus, loading, empty/error, data update and reduced-motion styles.
- [x] Run chart, homepage, contract and shell component tests.

## Task 3 — Acceptance

- [x] Run `npm run test:full` and `npm run build`; diagnose any failure before proceeding.
- [x] Use a loopback-only dev server and Playwright to inspect desktop/mobile screenshots, grouped navigation, event filtering, chart controls, reduced motion and unavailable-data states. Browser test fixtures must remain clearly identified as simulated; never send production notifications.
- [x] Review diff for API/grade/notification changes and secrets. Keep changes local and report results with a usable preview.

## Acceptance evidence — 2026-09-07

- Full frontend suite: 337 tests passed across 39 files. Production build passed; the existing warning about chunks larger than 500 kB remains.
- Isolated browser smoke: 18 checks passed, including 1536 px desktop and 390 px mobile layouts, event filtering, chart range/keyboard interaction, detail open/close, manual motion control and OS reduced motion. No page script errors or mutation requests; 96 API reads were intercepted with fixtures.
- Screenshots and browser report are in the ignored `.superpowers/ui-preview/` directory. Fixture screenshots are explicitly labeled SIMULATED and are not production evidence or shipped demo data.
- Missing, disabled or unavailable data is shown as waiting/unknown rather than a fabricated zero. Disabled contract monitoring is not labeled LIVE. Contract price points are labeled event-trigger samples, not continuous market history.
- Local preview: http://127.0.0.1:5173/dashboard. The local monitoring backend is not connected; unavailable-data notices are expected outside the isolated fixture preview. The existing public price fallback remains unchanged.
- No changes to Rust, API adapters, polling cadence, canonical grading, notification delivery, dependency manifests or production configuration. No commit, push, server deployment or production notification test was performed. Pre-existing untracked work was preserved.
