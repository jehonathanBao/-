# Contract Whale Signal Compression Terminal v2.5 Design

## Goal

On `/contract-whale`, keep the existing read-only institutional analysis terminal as the primary view and add a separate `Trade Ideas` layer that compresses noisy contract-whale events into a small set of trade-assist signals without implying automated execution.

This feature is intentionally "tradeable but not auto-trading":

- allowed: direction bias, entry zone, invalidation, confidence, structure rationale
- forbidden: order buttons, auto execution, buy/sell commands, stop-loss automation

## Why This Design

The current page already has two distinct semantic layers:

1. **Analysis layer**: regime, liquidity behavior, ranked event strength, opportunity zones
2. **Trade-assist layer**: filtered setups derived from those structures

Blending them into one undifferentiated panel creates product risk. A user can start reading analysis as an execution instruction source. The v2.5 design therefore keeps the intelligence view intact and adds a separately labeled `Trade Ideas` tab/panel fed by backend truth rather than frontend-only recomputation.

## Chosen Approach

### Recommended option chosen: backend-owned trade-ideas projection

Reuse the existing `intelligence-terminal` response as the single read-only truth source and extend it with a separate `signalCompression` / `tradeIdeas` projection.

This is preferred over:

- reviving the old trading-decision panel unchanged, which carries stronger execution semantics than we want
- deriving trade ideas only in the frontend, which would create dual truth and drift from the backend

## Product Boundary

### What the operator sees

The upgraded `/contract-whale` page will expose:

- `Market Intelligence` tab: unchanged analysis-first panel
- `Trade Ideas` tab: compressed trade-assist signals
- `Risk / No-Trade` tab: chop, fake-breakout, and low-confidence suppression context

The historical event stream, ACTIVE/CLOSED lifecycle views, latest snapshots, and raw-flow diagnostics remain where they are today and are not semantically mixed into the new tabbed terminal.

### What the operator must not infer

The UI must not look like:

- an execution console
- a bot control panel
- a place where clicking or following the screen triggers a trade

All copy should preserve "reference / assist / context" language.

## Information Architecture

### Existing page structure to preserve

The current page already contains:

- top summary and status strip
- 60s contract flow
- data quality and market structure lite
- `InstitutionalAnalysisTerminalPanel`
- platform status
- filters
- latest signal table
- historical event stream and lifecycle sections

### New terminal structure

Inside `InstitutionalAnalysisTerminalPanel`, replace the current single long analysis panel with a tabbed terminal shell:

1. `Market Intelligence`
2. `Trade Ideas`
3. `Risk / No-Trade`

The tab set is local to the institutional terminal. It does not replace the page-level event stream below.

## Backend Design

### Response strategy

Keep `GET /api/contract-whale/intelligence-terminal` as the canonical route and extend its payload instead of creating a second primary truth route.

The response should continue to include:

- `marketRegime`
- `liquidityBehaviors`
- `rankedEvents`
- `opportunityMap`
- `noiseSuppression`

Add:

- `signalCompression`
- `tradeIdeas`
- `riskContext`

### New backend types

Add or extend types under `src/contract_whale_monitor/types.rs`:

- `ContractWhaleSignalCompressionSummary`
- `ContractWhaleTradeIdea`
- `ContractWhaleRiskContext`

Suggested shape:

```json
{
  "signalCompression": {
    "qualityScore": 82,
    "topSignalCount": 3,
    "discardedCount": 7,
    "compressionReason": "cross-window dedup + low-score suppression"
  },
  "tradeIdeas": [
    {
      "signalId": "btc-absorption-1",
      "rank": 1,
      "setupType": "Absorption continuation",
      "directionBias": "BULLISH_BIAS",
      "score": 87,
      "confidence": 84,
      "confidenceLabel": "HIGH",
      "entryZone": { "lowPrice": 60400, "highPrice": 60600, "label": "60400 - 60600" },
      "invalidation": { "priceLevel": 60020, "reason": "absorption structure lost" },
      "structureContext": "absorption + dominance + sweep",
      "regimeContext": "TRENDING_UP",
      "windowSec": 15
    }
  ],
  "riskContext": {
    "noTradeZones": [
      {
        "reason": "chop regime with conflicting flow",
        "rangeLabel": "60200 - 60600",
        "lowPrice": 60200,
        "highPrice": 60600
      }
    ],
    "fakeBreakoutRisk": "HIGH",
    "summary": "fake breakout clusters dominate inside the current range"
  }
}
```

### Computation strategy

Do not invent a second detector. Build trade-assist output from already filtered `ContractWhaleSignal` items.

Recommended layering:

1. `regime.rs`: current market state
2. `liquidity.rs`: behavior classification
3. `ranking.rs`: strength ordering
4. new `signal_compression.rs`: collapse all event evidence into top N tradeable ideas
5. new `risk.rs`: derive no-trade and fake-breakout context

### Trade-idea selection rules

Rules for `tradeIdeas`:

- maximum 3 ideas
- deduplicate same symbol + same direction-bias + same setup family across windows
- require minimum quality threshold
- keep medium execution semantics only:
  - direction bias
  - entry zone
  - invalidation
  - confidence
  - rationale
- do not emit explicit `LONG`, `SHORT`, `BUY`, `SELL`, `ENTRY NOW`

Preferred labels:

- `BULLISH_BIAS`
- `BEARISH_BIAS`
- `NEUTRAL_BIAS`

Not preferred:

- `LONG`
- `SHORT`

### Risk-context rules

`riskContext` must explain why the operator should ignore otherwise strong-looking signals.

Must cover:

- chop / ranging suppression
- fake breakout clusters
- high-volatility uncertainty
- conflicting flow directions

## Frontend Design

### Component strategy

Do not restore the old `TradingDecisionLayerPanel` directly. Instead:

- keep `InstitutionalAnalysisTerminalPanel`
- give it internal tabs
- add lightweight presentational subcomponents for:
  - `TradeIdeasTab`
  - `RiskContextTab`

This preserves the main read-only identity while clearly isolating the stronger decision-assist semantics.

### Tab behavior

Default active tab: `Market Intelligence`

Secondary tabs:

- `Trade Ideas`
- `Risk / No-Trade`

The operator should land in analysis mode first, not trading mode.

### Trade Ideas UI

Each idea card should show:

- setup type
- score
- confidence
- direction bias
- entry zone
- invalidation
- structure context
- regime context
- source window

Copy tone:

- "参考区"
- "失效参考位"
- "方向偏置"
- "结构原因"

Avoid:

- "立即做多"
- "立即做空"
- "买点"
- "止损位"

### Risk / No-Trade UI

Show:

- no-trade zones
- fake-breakout risk summary
- chop / conflict explanation

This tab exists to reduce misuse of trade ideas during bad conditions.

## Data Flow

```text
ContractWhaleSignal[]
  -> intelligence::regime
  -> intelligence::liquidity
  -> intelligence::ranking
  -> intelligence::signal_compression
  -> intelligence::risk
  -> /api/contract-whale/intelligence-terminal
  -> ContractWhaleMonitor.jsx
  -> InstitutionalAnalysisTerminalPanel tabs
```

The event stream below the terminal remains a separate projection:

```text
contract_whale_signals
  -> /api/contract-events
  -> historical stream + ACTIVE/CLOSED lifecycle
```

## Error Handling

If no fresh trade ideas exist:

- keep `Market Intelligence` populated if analysis exists
- show a quiet empty state in `Trade Ideas`
- explain whether the absence is due to:
  - no fresh signals
  - low scores after compression
  - chop / no-trade suppression

If latest is stale and no fresh history exists, keep the existing stale explanation:

- "latest 为旧快照"

and ensure the trade-ideas tab does not fabricate opportunities from stale latest-only data.

## Testing Strategy

### Backend

Add tests for:

- intelligence route includes `signalCompression`, `tradeIdeas`, `riskContext`
- at most 3 trade ideas
- trade ideas use bias semantics rather than explicit order verbs
- no-trade zones persist in the response

### Frontend

Add tests for:

- default `Market Intelligence` tab renders
- switching to `Trade Ideas` shows direction bias / entry zone / invalidation / confidence
- switching to `Risk / No-Trade` shows suppression context
- no explicit order-execution wording appears

## Non-Goals

This task does not:

- change detector thresholds
- change persistence behavior
- add Discord push logic
- add automated trading
- add order routing
- replace the existing event stream

## Acceptance Criteria

1. `/contract-whale` still opens in analysis-first mode.
2. The institutional terminal has separate tabs for analysis, trade ideas, and risk/no-trade.
3. `Trade Ideas` shows medium-strength trading semantics:
   - direction bias
   - entry zone
   - invalidation
   - confidence
   - rationale
4. No part of the UI implies auto execution.
5. Event stream and lifecycle views remain separate and unpolluted.
6. Backend and frontend use one shared truth source for the terminal response.
