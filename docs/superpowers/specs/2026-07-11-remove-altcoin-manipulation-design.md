# Remove Altcoin Manipulation Feature Design

## Goal

Remove the dedicated Altcoin Manipulation feature from the backend and frontend without changing the independent Binance alt-contract anomaly monitor, new-token watch, BTC/ETH contract monitoring, spot monitoring, or liquidation cascade calculations.

## Chosen Approach

Remove the feature completely rather than merely hiding it:

- delete the dedicated Rust manipulation engine and API routes;
- remove the `/altcoin-manipulation` route, sidebar entry, dashboard view, component, API client functions, and feature-specific tests;
- remove the `AltcoinManipulation` market domain and its fusion-only control-score path;
- keep liquidation cascade data available for BTC and ETH, but remove its altcoin manipulation/context requests and UI labels.

## Preserved Systems

The following remain intact and must keep their current contracts:

- `binance_alt_contract_monitor` and the 山寨合约异常 page;
- new-token monitoring and its watchlist persistence;
- BTC and ETH contract-whale routes and intelligence terminal;
- BTC and ETH spot monitoring;
- liquidation cascade, leverage map, and liquidity-gap APIs;
- shared raw market-data and contract-flow persistence.

## Backend Changes

Remove:

- `src/altcoin_manipulation_engine.rs`;
- `src/api/altcoin_routes.rs`;
- `/api/altcoin/manipulation`, `/api/altcoin/regime`, `/api/altcoin/fusion`, and `/api/altcoin/signals` registrations;
- `MarketDomain::AltcoinManipulation` and fusion branches that depend on its control score;
- static tests whose purpose is to assert the removed split.

The remaining market-domain behavior is deliberately simple: BTC uses the BTC structure domain; non-BTC symbols no longer receive a dedicated manipulation domain. Any shared fusion consumers must use their non-manipulation path.

## Frontend Changes

Remove:

- the standalone `/altcoin-manipulation` route;
- the sidebar entry and dashboard view mode;
- `AltcoinManipulationDashboard` and its local-storage watchlist;
- `fetchAltcoinManipulation` and its normalizers/fallbacks;
- altcoin manipulation cards/labels from the liquidation cascade dashboard.

The liquidation cascade dashboard remains read-only and displays only cascade, leverage, liquidity-gap, and its independent market-state context.

## Compatibility and Error Handling

Removed API paths return the normal application 404 response. They are not redirected to new-token or alt-contract endpoints, because those systems have different semantics.

Old browser local-storage key `altcoin_manipulation_watchlist_v1` may remain inert in browsers; no code reads or writes it after removal.

## Testing Strategy

Before removal, add tests that specify:

- the sidebar and router do not expose the removed route;
- the liquidation cascade dashboard loads without requesting an altcoin manipulation endpoint;
- server routing no longer registers `/api/altcoin/*`;
- the remaining alt-contract and new-token routes still compile and retain their existing tests.

Then remove production code until those tests pass, followed by the focused Rust and frontend regression suites and production builds.

## Non-Goals

This task does not alter detector thresholds, persistence retention, Discord behavior, execution behavior, or data collection for remaining monitors.

## Acceptance Criteria

1. No dedicated altcoin manipulation page, nav item, component, engine, or API route remains.
2. No frontend request targets `/api/altcoin/*`.
3. Liquidation cascade still loads its independent data without manipulation context.
4. Alt-contract anomaly monitoring, new-token watch, BTC/ETH contract, and BTC/ETH spot monitors continue to work.
5. Rust and frontend tests/builds pass without relying on the deleted feature.
