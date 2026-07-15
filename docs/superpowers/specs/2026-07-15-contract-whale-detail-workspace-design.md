# Contract Whale Detail Workspace Design

**Date:** 2026-07-15

**Status:** Approved by the user (方案 1)

## Goal

Replace the legacy `ContractWhaleDetailModal` presentation with the same compact institutional trader-workspace language used by the current BTC contract monitor, while preserving every existing read-only field, formatter, data source, close action, and safety boundary.

## Root Cause

The unified workspace change migrated the shell and several other dialogs to `.workspace-dialog`, but did not modify `ContractWhaleMonitor.jsx`. `ContractWhaleDetailModal` still owns a legacy rounded Tailwind modal, a symmetric two-column opening layout, and the old generic `DetailSection` / `DetailGrid` presentation. Clicking a signal therefore crosses back into the old visual system even though the route behind it uses the new trader workspace.

## Chosen Design

Use a large read-only event inspector rather than a generic card modal.

### Command Header

- Fixed at the top of the inspector while the detail body scrolls.
- Shows the `CONTRACT WHALE / EVENT INSPECTOR` eyebrow, symbol, signal type, direction, final result, and explicit `READ ONLY` state.
- Keeps the existing accessible close button and visible Chinese `关闭` label.

### Decision Strip

Immediately below the header, show the values a trader scans first:

- severity;
- event lifecycle state;
- total displayed volume;
- net direction;
- notional value;
- trigger price.

Values use tabular or monospace numerals and semantic text. Color is supplementary and never the only status indicator.

### Inspector Body

Desktop uses an asymmetric grid:

- the main evidence column contains basic information and every existing analytical section;
- the narrower sticky decision rail contains the core judgment and Discord gate;
- long values wrap inside their own cells instead of widening the page.

The existing sections remain available: core judgment, market-driver engine, liquidity force, cluster/persistence, whale trajectory, spot confirmation, source snapshot, time windows, venue breakdown, dominant venue, score breakdown, price response, and glossary.

### Visual Language

- graphite surfaces with sharp separators and restrained cyan focus;
- radius between `0.25rem` and `0.45rem`, not the old `rounded-2xl` card language;
- compact uppercase labels, denser spacing, tabular data, and clear section numbering;
- no glow shadow, cyan outline card, or two equal generic cards at the top;
- backdrop remains dark and modal content remains visually separate from the live tape.

### Responsive Behavior

- At widths below `960px`, the decision rail joins the document flow above the evidence sections.
- At widths below `640px`, the summary strip and detail grids become one column.
- The inspector owns its internal scroll; the page behind it does not shift horizontally.

## Data And Safety Boundary

No API, polling, cache, classifier, threshold, Discord, signal-selection, or backend behavior changes. The modal remains read-only and must not expose raw payloads, tokens, webhooks, or execution controls.

## Testing

Extend the existing detail-modal regression test to prove:

- the dialog uses `.workspace-dialog` and `.contract-detail-inspector`;
- command header, decision strip, evidence body, and decision rail render;
- the explicit `READ ONLY` state is visible;
- all existing detail copy and safety assertions remain green;
- close behavior still removes the dialog.

Run the focused component test, the full frontend suite, and a production build. Then inspect the real BTC route at desktop and narrow widths.

## Acceptance Criteria

1. Clicking any BTC contract event opens the new event inspector, not the legacy rounded modal.
2. The first viewport exposes decision-critical data before the long raw field list.
3. Every existing detail field and analytical section remains present.
4. Long Discord and merge-source values wrap without horizontal page overflow.
5. Close behavior and accessible dialog naming remain unchanged.
6. Focused tests, full frontend tests, and production build pass.
7. Browser inspection confirms visual continuity with the current contract workspace.
8. No commit, push, deployment, or service restart occurs without separate authorization.
