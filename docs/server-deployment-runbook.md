# Server Deployment Runbook

This deployment keeps the Rust monitor process independent from the React/Vite frontend. Browser refreshes and Vite HMR reloads do not restart the backend container.

## Directory Layout

```text
有毒订单监控-rs/
├── Dockerfile.backend
├── docker-compose.yml
├── config/
│   └── default.toml
├── data/
│   ├── production_replay/
│   └── reports/
├── docs/
├── src/
└── toxic-order-monitor/
    ├── Dockerfile.frontend
    ├── package.json
    ├── vite.config.js
    └── src/
```

## Services

- `backend`: Rust monitor and API server.
- `frontend`: nginx-served SPA container behind host nginx.

The backend listens on `0.0.0.0:3000` inside Compose and is exposed only on host loopback port `127.0.0.1:8000`.

The frontend listens on `5173` inside the container and is published on host loopback as `127.0.0.1:5174`. Host nginx owns the public `:80` and `:5173` entrypoints and reverse proxies browser SPA requests to `http://127.0.0.1:5174`, while `/api` and `/ws` continue to proxy to the backend on `http://127.0.0.1:8000`.

Open:

```text
http://<server-ip>:5173/contract-whale
```

The frontend calls `/api/...` with relative URLs. Host nginx proxies those requests to `http://127.0.0.1:8000` and injects the operator token server-side. Browsers should not call `http://<server-ip>:8000` directly.

## Required Token

When the Rust API binds to a non-loopback address, `/api` requests require an operator token.

PowerShell:

```powershell
$env:OPERATOR_TOKEN = "replace-with-a-strong-local-token"
docker compose up -d --build
```

Bash:

```bash
export OPERATOR_TOKEN="replace-with-a-strong-local-token"
docker compose up -d --build
```

Do not put this token in `VITE_*` env vars. The Vite proxy injects it server-side as `x-operator-api-token`; the browser bundle should not receive it.

## WebSocket Authentication Model

The browser must not receive `OPERATOR_TOKEN`.

Current deployment mode:

- Browser connects to the public origin: `/ws/signals` or `/ws/scan-logs`.
- Host nginx proxies `/ws` to `http://127.0.0.1:8000`.
- Host nginx injects `x-operator-api-token` server-side through `/etc/nginx/snippets/toxic-order-monitor-token.conf`.
- Backend validates the token for non-loopback WS requests.

Use `deploy/nginx-site.toxic-order-monitor.conf` as the host nginx site template.

## Logs

```powershell
docker compose logs -f backend
docker compose logs -f frontend
```

Keep Discord and Telegram webhook URLs out of logs.
The Dashboard scan log panel displays sanitized startup, scan, candidate, and Discord push status only.

For live notification deployment, see `docs/live-data-deployment-checklist.md`.

## Persistent Data

The Compose file mounts:

```text
./data:/app/data
./config:/app/config:ro
```

Runtime state and reports should stay under `./data`, including:

- SQLite runtime database
- replay reports
- production replay inputs
- calibration reports

Real production replay files are ignored by git.

## Frontend Runtime

The frontend container serves the built SPA through nginx for both internal health supervision and the public browser shell. Host nginx is the only public ingress, but it now proxies the SPA to `127.0.0.1:5174` instead of reading a second host-side `dist` tree. This keeps browser refreshes, route handling, and asset versions on the same deployment path as the rebuilt frontend container.

## WebSocket Boundary

`/ws/signals` streams redacted toxic signal snapshots to the Dashboard. `/ws/scan-logs` streams sanitized runtime scan logs. Both use the same non-loopback token boundary as `/api` and should normally be reached through the host nginx `/ws` proxy.

`toxic-order-monitor/src/hooks/useReconnectingWebSocket.js` reconnects automatically with exponential backoff. The Dashboard merges incoming snapshots into the same persistent inbox used by HTTP polling.

Keep the stream read-only:

- keep it read-only
- require the same operator token policy as `/api`
- send redacted signal summaries only
- do not send markout fields, raw evidence, stale flags, tokens, webhook URLs, or private payloads
- tune `WS_SIGNAL_INTERVAL_MS` for snapshot frequency
- tune `SCAN_LOG_BUFFER_SIZE` for the in-memory scan log ring buffer
- tune `TOF_SCAN_LOG_INTERVAL_SECONDS` to avoid metrics summary spam
- keep `DISCORD_AUTO_PUSH_ENABLED=true` for real High/Critical candidate notifications
- keep `DISCORD_AUTO_PUSH_CACHED_ON_BOOT=false` to avoid restart-time cached candidate bursts
- keep `DISCORD_PUSH_COOLDOWN_SECONDS=60` or higher for real notification channels

## TOF-Lite Metrics

## New-Token L2 Sessions

`NEW_TOKEN_L2_ENABLED=false` is the safe default. When enabled, each
operator-selected new-token symbol receives its own public Binance USD-M depth
bookTicker, aggTrade, mark-price, and low-frequency open-interest context
session. A session remains `flow_only` until a REST snapshot and at least one
contiguous depth update have synchronized successfully.

The runtime stores only compact per-second session metrics for seven days and
delayed shadow outcomes for one year. It does not persist raw depth frames,
identify participants, send Discord alerts, or expose an execution path. L2
outcomes are exposed read-only at
`GET /api/new-token-watch/outcomes?symbol=...`; they remain explicitly
`shadowOnly=true`. New-token add/remove/restart and runtime diagnostics are
operator-token protected; keep those endpoints off the public nginx token
injection path.

The backend augments signal inbox and WebSocket snapshots with TOF-lite aggregate
metrics:

```text
tofMetrics, tofScore, finalRiskScore, candidateType, explainTags, directionConfidence
```

These fields are read-only operator summaries. They must not include raw order
books, raw trades, evidence, markout, tokens, or webhook values. If real L2 /
trade data is incomplete, TOF-lite falls back to the existing risk score and
lower metrics completeness.

## Contract-Side TOF Metrics

v0.5 also adds read-only perp-side TOF aggregate metrics to the same inbox,
WebSocket, Discord payload, scan log, and Dashboard surfaces:

```text
perpTofMetrics, perpScore, perpCandidateType, finalCandidateType, metricsDirection, mergedConfidence
```

The supported candidate families are `OpenInterestCandidate`,
`CrowdedLongCandidate`, `CrowdedShortCandidate`, `LongSqueezeCandidate`,
`ShortSqueezeCandidate`, and `AggressiveOrderFlowCandidate`. These fields are
safe summaries only: OI change, funding rate, liquidation pressure, aggressive
buy/sell volume, direction, score, data quality, and explain tags. Do not send
raw payloads, raw evidence, markout, tokens, or webhook values.

Useful defaults:

```env
PERP_TOF_ENABLED=true
PERP_OI_BUCKET_SIZE=100000
PERP_FUNDING_THRESHOLD=0.05
PERP_LIQUIDATION_WINDOW=5m
PERP_AGF_VOLUME_THRESHOLD=1000000
ADVANCED_TOF_ENABLED=true
```

Short-term toxic-order Discord delivery uses a stricter alert gate:
`toxicScore >= 85`, `confidence >= 70`, `dataQuality >= 70`, dedupe, and
cooldown `>= 60s`. Medium and Low contract candidates remain Dashboard-only.
Short-term toxic Discord copy must say "短线有毒订单" and must not imply
main-force intervention.

Recommended short-term toxic Discord env vars:

```env
SHORT_TOXIC_DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
SHORT_TOXIC_DISCORD_AUTO_PUSH_ENABLED=true
SHORT_TOXIC_ALERT_MIN_SCORE=85
SHORT_TOXIC_ALERT_MIN_CONFIDENCE=70
SHORT_TOXIC_ALERT_MIN_DATA_QUALITY=70
SHORT_TOXIC_DISCORD_COOLDOWN_SECONDS=60
```

If these are unset, the backend can still fall back to the generic
`DISCORD_WEBHOOK_URL`, `DISCORD_AUTO_PUSH_ENABLED`, and `ALERT_MIN_*` values.
For production, prefer a family-specific short-term toxic webhook so short-line
alerts do not mix with main-force or whale-flow channels.

Recommended market-structure Discord env vars:

```env
MARKET_STRUCTURE_DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
MARKET_STRUCTURE_DISCORD_AUTO_PUSH_ENABLED=true
MARKET_STRUCTURE_ALERT_MIN_SCORE=80
MARKET_STRUCTURE_EXTREME_MIN_SCORE=85
MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE=70
MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY=70
MARKET_STRUCTURE_DISCORD_COOLDOWN_SECONDS=900
```

This family is for medium/longer-term spot + perp structure alerts, not
short-term toxic-order sweeps. Its Discord wording must distinguish:

- `主力结构异动`: main-force / accumulation / distribution / absorption / resistance
- `极端行情冲击`: violent liquidation or squeeze impact that is not yet confirmed as main-force behavior

## Split Risk Systems

The v0.6 dashboard exposes two read-only scoring systems instead of forcing
spot, TOF, perp, and CWM into one final score:

```text
shortTermToxic: toxicScore, shortPressure, toxicType, ttlSec, expiresAt, halfLifeSec, decayedScore, reasons
marketStructureScore/mainForceStructure:
  mainForceScore, extremeImpactScore, structureBias, confidence, dataQuality,
  severity, regimeType, spotScore, contractScore, crossConfirmScore, oiScore,
  liquidationScore, fundingCrowdingScore, cwmScore, reasons
```

The short-term toxic score is used for ordinary toxic-order Discord gating:

```text
High/Critical/S && toxicScore >= 85 && confidence >= 70 && dataQuality >= 70 && cooldown >= 60s
```

Short-term severity bands are:

```text
0-39 Calm
40-59 Watch
60-74 High
75-89 Critical
90-100 S
```

Short-term scores decay quickly:

```text
decayedScore = previousScore * exp(-elapsedSec / halfLifeSec)
```

The half-life is 30-45 seconds and max TTL is 3-5 minutes depending on severity.

The market-structure score is medium/longer-term context built from spot, perp,
and cross-market confirmation. `contractScore` is the contract-side composite
from perp TOF, OI, liquidation pressure, funding crowding, and CWM:

```text
spotScore =
  0.30*SpotCvdScore
+ 0.25*SpotVolumeAnomaly
+ 0.20*SpotAbsorption
+ 0.15*SpotLiquidityShift
+ 0.10*SpotPriceResponse

contractScore =
  0.30*CwmAggressiveFlow
+ 0.20*OiImpulse
+ 0.15*LiquidationContext
+ 0.15*FundingCrowding
+ 0.10*BasisPremium
+ 0.10*ActiveExchangeConfirmation

crossConfirmScore =
  0.40*SpotContractDirectionConsistency
+ 0.25*MultiWindowConsistency
+ 0.20*PriceResponseConsistency
+ 0.15*SourceCoverage

structureRaw =
  0.40*spotScore
+ 0.40*contractScore
+ 0.20*crossConfirmScore

mainForceScore =
  0.65*structureRaw
+ 0.25*min(spotScore, contractScore)
+ 0.10*durationScore
- liquidationPenalty
- crowdingPenalty
```

Main-force severity bands are:

```text
0-39 Calm
40-59 Watch
60-74 Confirmed
75-89 Major
90-100 Extreme
```

`mainForceConfirmed=true` requires `mainForceScore>=75`, `confidence>=70`,
`dataQuality>=70`, and at least 3 of 7 confirmation checks:
`spotScore>=60`, `contractScore>=70`, `crossConfirmScore>=60`, OI aligned with
direction, clear price-response or absorption/suppression structure,
liquidation not primary-driven, and at least two consistent time windows.

Treat `extremeImpactConfirmed` as a separate read-only flag:

- `extremeImpactConfirmed=true` means violent market impact is present.
- It does not automatically imply `mainForceConfirmed=true`.
- Example: `regimeType=long_liquidation_cascade` can be extreme while still not
  being confirmed as active main-force building.

`mainForceScore` estimates whether behavior resembles real main-force
activity. `extremeImpactScore` tracks violent market impact separately because
a liquidation cascade can be extreme without being active main-force building.
`structureBias` is the separate `-100..+100` direction score. It must not be
treated as a synonym for `mainForceScore`.
The `min(spotScore, contractScore)` term intentionally prevents contract-only
volume spikes from becoming very high main-force scores without spot
confirmation.
`ActiveExchangeConfirmation` must be computed from enabled venues only. If OKX
is disabled, it is not counted in total volume, exchange count, data-quality
penalties, or multi-exchange confirmation.
`SourceCoverage` uses `healthyEnabledSources / enabledSources`; disabled venues
are not included in the denominator. With Binance and Bitfinex enabled and OKX
disabled, two healthy sources means `2 / 2 = 100%`.
In the current Binance + Bitfinex profile, Binance is the primary liquidity
source and Bitfinex is a confirmation source. Binance-only extremes can support
High/Critical contract evidence but are capped below S-like CWM component
strength. Bitfinex-only extremes are capped as High-level evidence. S-grade
contract structure requires Binance + Bitfinex same-direction confirmation.
Funding crowding is a risk correction, not a simple bullish/bearish boost, and
BasisPremium remains a low-weight leverage-context indicator.
`confidence` must remain distinct from `dataQuality`:

```text
confidence =
  0.35*dataQuality
+ 0.25*SourceCoverage
+ 0.20*MultiWindowConsistency
+ 0.20*SignalAgreement
```

High data quality only means the feeds are healthy enough. It does not imply
the structure interpretation is reliable if spot, contract, OI, and price
response disagree.

Current `regimeType` values exposed to the Dashboard / WebSocket / inbox are:

- `main_force_long_build`
- `main_force_short_build`
- `spot_accumulation`
- `spot_distribution`
- `contract_short_squeeze`
- `long_liquidation_cascade`
- `downside_absorption`
- `upside_resistance`
- `range_rotation`
- `unclear`

`finalRiskScore` may still appear as a compatibility field for older clients.
For ordinary toxic-order candidates it mirrors `toxicScore`; do not treat it as
a fused spot/perp/CWM score. CWM/spot-whale large-flow alerts keep independent
Discord gates and cooldowns. Scan logs emit aggregate scores only. Discord
embeds may show aggregate indicators and explain tags, but must still omit raw
payloads, evidence, markout, tokens, and webhook values.

## Remote Access

For remote browser access, prefer host nginx (or another reverse proxy with TLS) in front of the frontend and backend.

Do not expose the backend API publicly without:

- HTTPS
- a strong `OPERATOR_TOKEN`
- server-side token injection or an authenticated gateway
- restricted CORS origins
- log redaction

## Stop

```powershell
docker compose down
```

To keep data, do not delete `./data`.

## Validation

```powershell
docker compose config
docker compose build
docker compose up -d
docker compose logs -f backend
docker compose logs -f frontend
```

Then verify:

```text
http://localhost:5173/dashboard
http://localhost:8000/api/status
ws://localhost:5173/ws/signals
ws://localhost:5173/ws/scan-logs
```

Direct `http://localhost:8000/api/status`, `ws://localhost:8000/ws/signals`,
and `ws://localhost:8000/ws/scan-logs` may require an operator token when the
backend is bound to `0.0.0.0`. The frontend proxy path should inject it
automatically.
