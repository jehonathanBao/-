# Binance Alt Contract Monitor Runbook

## Scope

BACM is a read-only Binance altcoin perpetual monitor. It consumes public Binance USD-M futures market data only. It does not place orders, cancel orders, transfer funds, read private account state, or modify exchange account state.

## Default Universe

The default mode is full Binance USDT perpetual altcoin monitoring:

```toml
[binance_alt_contract_monitor]
universe_mode = "all_binance_usdt_perp"
symbol_limit = 0
exclude_symbols = ["BTCUSDT", "ETHUSDT"]
whitelist = []
blacklist = []

[binance_alt_contract_monitor.symbol_filter]
quote_asset = "USDT"
contract_type = "PERPETUAL"
status = "TRADING"
min_24h_quote_volume_usd = 0
```

Rules:

- `symbol_limit = 0` means no Top-N limit.
- `min_24h_quote_volume_usd = 0` means low-volume contracts are not filtered out.
- `BTCUSDT` and `ETHUSDT` are excluded because they are covered by BTC/ETH CWM.
- `blacklist` always applies.

## Debug Modes

Use these only for testing:

```toml
universe_mode = "top_n"
symbol_limit = 40
```

or:

```toml
universe_mode = "whitelist_only"
whitelist = ["SOLUSDT", "DOGEUSDT"]
```

## Liquidity Tiers

BACM monitors all matching symbols, then assigns liquidity tiers from 24h quote volume:

- Tier A: `>= $500M`
- Tier B: `$100M - $500M`
- Tier C: `$20M - $100M`
- Tier D: `$5M - $20M`
- Tier E: `< $5M`

Tier E is intentionally conservative. Tier E signals may appear on the frontend, but Discord dry-run/would-send is blocked unless abnormal score, build score, dynamic multiple, OI confirmation, and non-liquidation checks all pass.

## WebSocket Sharding

The aggTrade collector builds shards from the active universe and subscribes in batches. Current default shard size is 200 streams per connection. This keeps the monitor away from single-connection stream pressure while still covering the full Binance USDT perpetual universe.

## All-Market Context

BACM uses all-market public streams for context instead of opening one stream per symbol:

- `!markPrice@arr@1s`: updates mark price and funding-rate context.
- `!ticker@arr`: refreshes last price, 24h quote volume, 24h price change, and liquidity tier.
- `!forceOrder@arr`: tracks recent liquidation snapshots.

The summary API exposes `allMarketContext` with:

- `markPriceConnected`
- `tickerConnected`
- `forceOrderConnected`
- `candidateSymbols`
- `hotOiSymbols`

These fields are operational status only. They never include private account data or credentials.

## Force Order Context

The Binance `!forceOrder@arr` stream is a liquidation snapshot stream, not a complete liquidation tape. BACM uses it as context to mark liquidation-driven risk, not as proof of total liquidation volume.

## OI Polling

Open Interest polling is adaptive:

- All monitored symbols receive a slow baseline poll.
- Symbols that pass light-scan candidate conditions enter a Hot OI pool.
- Hot OI symbols are polled at the shorter interval configured by `hot_symbols_interval_sec`.
- Candidate and Hot OI membership expires after `candidate_ttl_sec`.

This prevents personal deployments from hammering REST endpoints while still refreshing context for symbols that are close to signal generation.

## Light Scan

BACM does not fully score every symbol on every trade. The runtime first performs a light scan using notional size, dynamic multiple, price move, directional strength, and force-order context. Only candidate symbols proceed to full abnormal/build scoring and Discord gate evaluation.

## Logic v2: Abnormal Flow vs Main-Force Build

BACM v2 separates contract impulse from suspected main-force positioning. A high `abnormalScore` means the symbol has unusual contract flow, but it does not automatically mean there is main-force accumulation or distribution.

Frontend and Discord wording should follow this split:

- `abnormalScore`: short-lived contract-flow impulse, useful for "abnormal volume" or "price impact" alerts.
- `buildScore`: raw build-style context score, still useful but no longer sufficient by itself.
- `mainForceConfidence`: final suspected main-force confidence after evidence, OI quality, market-wide filtering, liquidation context, and funding crowding penalties.
- `evidenceCount` / `evidenceTags`: the evidence chain used to justify stronger wording.

Use main-force wording only when the signal has a strong evidence chain:

- `mainForceConfidence >= 75`
- `evidenceCount >= 4`
- not liquidation-driven
- OI is fresh or directionally supportive
- the move is not just a broad market impulse, unless the symbol is a relative-strength leader

If these conditions are not met, describe the signal as abnormal contract flow, aggressive pump/dump, absorption, resistance, squeeze, or liquidation context instead of "main-force build".

## Multi-Window Confirmation

Signals include `windowConfirmations` for the active scan windows. A window is confirmed when notional size, dynamic multiple, and directional strength pass the configured candidate bar. Multiple confirmed windows increase `mainForceConfidence`; a single-window spike remains more conservative.

## Market-Wide Move Filter

BACM marks `marketWideMove` when many monitored alt contracts move in the same direction at the same time. In that case, non-leading symbols are down-weighted because the move may be market beta rather than symbol-specific main-force behavior.

Use these fields when reviewing a signal:

- `marketWideMove`: broad-market impulse detected.
- `marketWideDirection`: broad impulse direction.
- `marketImpulseRatio`: share of monitored symbols joining that impulse.
- `relativeStrengthRank`: lower is stronger; top-ranked symbols can still keep stronger wording.

## Post-Signal Validation

Every new signal starts as `postSignalStatus = pending`. After enough time and follow-through data, the runtime can mark it as:

- `validated`: price defended the signal VWAP in the expected direction.
- `failed`: price lost the signal VWAP in the expected direction.
- `trap`: the move reversed against the signal.
- `pending`: not enough time/data yet.

This is review context only. The module stays read-only and never trades from validation status.

## OI, Funding, And Depth Context

OI freshness and quality are separated from the raw score:

- `oiQuality = fresh`: recent OI data is available.
- `oiQuality = stale`: OI data exists but is old.
- `oiQuality = missing`: no usable OI context.

Funding crowding reduces main-force confidence when the trade direction is already overcrowded. For example, aggressive long flow with very positive funding is treated more carefully than aggressive long flow with neutral or contrarian funding.

Depth fields such as `spreadBps`, `depth0_5PctUsd`, `depth1PctUsd`, and `flowToDepthRatio` are currently candidate-only placeholders. Do not treat missing depth fields as a backend failure.

## Event Merge

Repeated signals for the same symbol, direction, and type within the event window are merged into the same event snapshot. Review these fields for continuity:

- `eventId`
- `eventSignalCount`
- `eventPeakAbnormalScore`
- `eventPeakBuildScore`

The merged event view prevents one market impulse from appearing as many unrelated alerts.

## Storage Strategy

The personal edition keeps high-frequency trades bounded in memory and persists signal history as JSONL. Long-term SQLite-style full 1s bucket persistence is intentionally avoided for full-universe monitoring because it can grow by millions of rows per day. Future persistence should keep all-symbol 1m aggregates and store 1s buckets only around hot candidate windows.

Default storage guard:

```toml
[binance_alt_contract_monitor.storage]
persist_all_1s = false
persist_all_1m = true
persist_hot_1s = true
hot_1s_retention_hours = 24
flow_1m_retention_days = 14
signals_retention_days = 180
```

## Dry-Run Discord

Personal deployments should start with:

```env
BINANCE_ALT_CONTRACT_ENABLED=true
BINANCE_ALT_CONTRACT_DRY_RUN=true
BINANCE_ALT_CONTRACT_UNIVERSE_MODE=all_binance_usdt_perp
```

Dry-run allows signal generation and would-send accounting without sending real Discord messages.

## Frontend Checks

Open:

```text
/alt-contract-monitor
```

Expected:

- The page says `全 Binance USDT 永续`.
- The monitored count is greater than the old fixed hotlist.
- Tier counts include A/B/C/D/E.
- Runtime chips show markPrice / ticker / ForceOrder state.
- Candidate and Hot OI counts are visible.
- BTCUSDT and ETHUSDT are absent.
- The page remains read-only and does not expose tokens or webhook URLs.
