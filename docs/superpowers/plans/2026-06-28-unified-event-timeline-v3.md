# Unified Event Timeline v3

## Goal

Converge `/contract-whale` onto one canonical market-event timeline so latest, history, final lifecycle views, and latency diagnostics all describe the same event clock.

## Why

- Current UX exposes multiple time semantics at once (`latest`, `history`, `final`, `flow`), which makes normal lag look like data inconsistency.
- `contract-events` and `final-events-v2` already derive from `contract_whale_signals`; we should promote that persisted event stream into the canonical timeline source instead of letting every endpoint describe freshness differently.

## Scope

1. Add a shared contract-whale timeline helper + route.
2. Attach canonical timeline metadata to:
   - `/api/contract-whale/latest`
   - `/api/contract-events`
   - `/api/final-events-v2`
   - `/api/contract-whale/latency-debug`
3. Update `/contract-whale` to present:
   - Market Time
   - System Lag
   - per-view drift versus canonical timeline
4. Keep existing endpoint payloads backward-compatible; add fields rather than remove fields.

## Canonical timeline semantics

- `eventTs`: newest canonical event timestamp visible for the symbol/range
- `persistedTs`: newest DB write time known for the canonical source
- `processedTs`: newest projection/update time derived from canonical source
- `servedTs`: API response time
- `timelineLagSec`: `servedTs - eventTs`
- `source`: one of `contract_whale_signals`, `final_events_v2`, `latest_snapshot`, `flow_state`, `none`

## TDD plan

### Backend

1. Add failing route test for `/api/contract-whale/timeline`
2. Add failing assertions that `/api/contract-events` and `/api/final-events-v2` expose matching timeline metadata
3. Add failing assertion that `/api/contract-whale/latest` exposes canonical timeline metadata

### Frontend

1. Add failing API normalization test for timeline metadata
2. Update latency guard test to expect canonical Market Time/System Lag wording

## Guardrails

- Do not change detector thresholds
- Do not change persistence semantics
- Do not remove existing payload fields relied on by current UI/tests
- Do not touch unrelated dirty files in `src/contract_whale_monitor/intelligence/*` or `src/contract_whale_monitor/trading/*`

## Verification

- `cargo test --test contract_event_routes_tests`
- `npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx`
- `npm --prefix toxic-order-monitor run build`
