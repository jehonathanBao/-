# Contract Whale Trading Decision Layer v1

## Goal

Upgrade `/contract-whale` from a noise-reduced market-event monitor into a read-only trading decision support surface without changing detector fundamentals, persistence behavior, or introducing any execution path.

## Requirements Split

### Backend

1. Add a dedicated trading decision computation layer under `src/contract_whale_monitor/trading/`.
2. Reuse stabilized contract whale events as the only input; do not change detector thresholds or generate synthetic signals.
3. Expose `GET /api/contract-whale/trading-decisions?symbol=BTC`.
4. Return:
   - `symbol`
   - `timestamp`
   - `marketBias`
   - `biasConfidence`
   - `noiseSuppression`
   - `topSetups[]`
   - `noTradeZones[]`
5. Keep the existing summary `tradeOpportunities` panel backward-compatible during rollout.

### Frontend

1. Add a dedicated fetcher for `/api/contract-whale/trading-decisions`.
2. Render a new "Trading Decision Layer" panel on `/contract-whale`.
3. Show:
   - market bias
   - top setup ranking
   - entry zone
   - invalidation
   - confidence / score
   - no-trade zones
4. Do not remove the existing event feed or historical views.

## Design Decisions

1. `summary.tradeOpportunities` remains as a compact monitor-side summary.
2. The new trading-decisions API becomes the richer trader-facing projection.
3. Entry / invalidation remain read-only reference levels derived from event price context, not order instructions.
4. Score bands:
   - `< 70` filtered from `topSetups`
   - `70-84` tradeable
   - `85+` high conviction

## TDD Steps

1. Add backend route red test for `/api/contract-whale/trading-decisions`.
2. Add backend computation red tests for:
   - bias classification
   - top setup ranking
   - no-trade zone generation
3. Add frontend red test for the dedicated trading decision panel.
4. Implement trading module types and pure scoring / classification logic.
5. Wire API route and keep existing summary helpers stable.
6. Wire frontend API + panel.
7. Run focused backend + frontend verification, then full targeted build checks.

## File Boundaries

- `src/contract_whale_monitor/mod.rs`
- `src/contract_whale_monitor/trading/mod.rs`
- `src/contract_whale_monitor/trading/scoring.rs`
- `src/contract_whale_monitor/trading/classifier.rs`
- `src/contract_whale_monitor/trading/bias.rs`
- `src/contract_whale_monitor/trading/noise_filter.rs`
- `src/contract_whale_monitor/types.rs`
- `src/api/contract_whale_routes.rs`
- `src/api/server.rs`
- `tests/contract_event_routes_tests.rs`
- `tests/contract_whale_routes_tests.rs`
- `toxic-order-monitor/src/api/contractWhale.js`
- `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

## Safety Boundary

- No detector threshold changes in this task.
- No auto entry / exit / order routing.
- No Discord behavior changes.
- No persistence schema changes.
- No hidden write side-effects outside current read-only monitoring flow.
