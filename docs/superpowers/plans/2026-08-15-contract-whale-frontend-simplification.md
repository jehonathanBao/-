# Contract Whale Frontend Simplification

## Decision

Remove the large secondary intelligence presentation layer from the contract-whale page while preserving the event-first monitoring workflow.

## Evidence

- The live CWM summary is enabled and healthy.
- The intelligence endpoint is fresh and read-only, but current BTC/ETH responses commonly contain no ranked events, liquidity behaviors, opportunity zones, trade ideas, or no-trade zones.
- The historical event stream contains useful events independently of the intelligence projection.
- The large Pro Desk, structure, liquidity, setup, and risk cards repeat the compact insight rail and render mostly empty-state copy when no qualifying projection exists.

## Scope

- Remove the visible Pro Trading Desk overview bar.
- Remove the section jump tabs that only target the removed secondary panels.
- Remove the secondary structure/liquidity/setup grid.
- Remove the dedicated risk context panel and let lifecycle content use the full width.
- Remove unused legacy `TradeOpportunitiesPanel` and `InstitutionalAnalysisTerminalPanel` definitions.
- Keep the command bar, status ribbon, historical event stream, compact insight rail, lifecycle events, system diagnostics, trajectory view, and all backend APIs/data.

## Safety boundaries

- No backend route or persistence change.
- No change to alerting, Discord gates, execution, or trading behavior.
- No deletion of historical data.
- Keep the intelligence request because the compact insight rail still consumes its occasionally useful live projection.

## Acceptance

- The removed panel labels and anchors are absent from the rendered contract-whale page.
- The event stream, compact insight rail, lifecycle events, system status, and trajectory sections remain.
- Intelligence freshness no longer controls a large empty secondary canvas.
- Existing tests pass with updated assertions, and the frontend builds successfully.
