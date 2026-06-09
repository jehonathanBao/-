# Contract Whale Monitor Runbook

This module is read-only. It consumes public market data, writes local aggregate/history data, and sends alerts only when enabled by configuration. It never places orders, cancels orders, transfers funds, or reads private exchange credentials.

## Enable Dry Run

Use dry-run for the first production boot:

```toml
[contract_whale_monitor]
enabled = true
dry_run = true
```

Dry-run allows collection, signal generation, history writes, and `cwm.discord.would_send` logs. It does not call the Discord webhook.

## Personal MVP Quick Start

For a personal local run that enables Binance/Bitfinex perp monitoring, keeps
Coinbase as spot-only confirmation, disables OKX, and keeps Discord dry-run:

```powershell
Copy-Item config\cwm.personal.example.toml config\cwm.personal.local.toml
$env:APP_CONFIG_FILE = "config/cwm.personal.local"
$env:RUST_LOG = "info,contract_whale_monitor=debug"
cargo run -- serve
```

`config/cwm.personal.local.toml` is ignored by Git. Keep webhook URLs and any
operator tokens in `.env` or server environment variables, never in the config
file.

Expected startup behavior:

- CWM summary shows `enabled=true` and `dryRun=true`.
- `thresholdProfile` resolves to `binance_bitfinex`.
- Active contract sources are Binance and Bitfinex perp.
- Coinbase appears as `spot_only` and can be used for spot context, but it is
  not counted as a contract source.
- OKX appears as `disabled`, not as a data failure.
- Discord logs use `cwm.discord.would_send`; no real webhook request is made.

Quick checks:

```powershell
curl "http://127.0.0.1:3000/api/contract-whale/summary?symbol=BTC"
curl "http://127.0.0.1:3000/api/contract-whale/latest?symbol=BTC&limit=50"
curl "http://127.0.0.1:3000/api/contract-whale/history?symbol=BTC&limit=50"
```

To bind the API to a non-loopback address, set `API_HOST` and also set
`OPERATOR_TOKEN`; non-loopback GET APIs reject unauthenticated requests.

## Check Runtime Health

```bash
curl http://127.0.0.1:3000/api/contract-whale/summary
```

Expected health states:

- `healthy`: all enabled contract sources are recent, with Binance as the primary source.
- `degraded`: at least one source is recent, but another source is disconnected or stale.
- `unhealthy`: all sources are stale or unavailable.
- `warming_up`: startup warmup is still collecting samples.
- `disabled`: the module is configured off.

If a source is connected but has no trades for 30 seconds on Binance, or 60 seconds on Bitfinex, treat it as stale. Disabled sources are not health failures.

## OKX Disabled Mode

Current production mode intentionally disables OKX for CWM:

```env
ENABLE_OKX=false
CONTRACT_WHALE_OKX_ENABLED=false
```

When OKX is disabled:

- `/api/contract-whale/summary` should show `thresholdProfile=binance_bitfinex`.
- `enabledExchanges` should be `["binance", "bitfinex"]`.
- `disabledExchanges` should contain `okx`.
- OKX must not contribute to total volume, exchange count, data quality, multi-exchange confirmation, main exchange, or net-flow contribution.
- The frontend health card should show OKX as `未启用`, not as a red disconnected source.

In this profile, Binance is the primary liquidity source and Bitfinex is a
confirmation source. Binance-only extreme flow can support High/Critical CWM
evidence but should stay capped below S-like component strength. Bitfinex-only
flow is capped as High-level evidence. S-grade structure requires Binance and
Bitfinex same-direction confirmation.

The active two-source BTC thresholds are:

| Window | High | Critical | S |
| --- | ---: | ---: | ---: |
| 5s | 650 BTC | 1200 BTC | 2000 BTC |
| 15s | 1200 BTC | 2200 BTC | 3600 BTC |
| 60s | 2800 BTC | 5200 BTC | 8000 BTC |

USD notional thresholds are `High=$40M`, `Critical=$95M`, and `S=$200M`.

## Check WebSocket Data

Watch logs with the CWM namespace:

```bash
rg "contract_whale_monitor|\\[cwm\\]" logs
```

Important events:

- `cwm.config.loaded`
- `cwm.runtime.started`
- `cwm.ws.connected`
- `cwm.ws.disconnected`
- `cwm.trade.normalized`
- `cwm.bucket.flushed`
- `cwm.signal.generated`
- `cwm.discord.would_send`
- `cwm.discord.sent`
- `cwm.discord.skipped`
- `cwm.retention.pruned`

## Check 1s Aggregates

Use SQLite inspection on the configured local database:

```sql
SELECT ts_bucket, exchange, symbol, buy_volume_btc, sell_volume_btc, trade_count
FROM contract_flow_1s
WHERE symbol = 'BTC'
ORDER BY ts_bucket DESC
LIMIT 20;
```

If this table is empty while WebSocket logs show connected, check stale source warnings and collector normalization errors.

## Check Signals

```sql
SELECT ts, symbol, window_sec, signal_type, direction, severity, score,
       discord_eligible, discord_sent
FROM contract_whale_signals
ORDER BY ts DESC
LIMIT 20;
```

The API equivalent:

```bash
curl "http://127.0.0.1:3000/api/contract-whale/latest?limit=50&symbol=BTC"
curl "http://127.0.0.1:3000/api/contract-whale/history?symbol=BTC&severity=critical&limit=50"
```

## Enable Real Discord

Keep webhook secrets in the server environment or local `.env`; never commit them.

```env
DISCORD_WEBHOOK_URL=https://discord.com/api/webhooks/...
```

Then switch dry-run off:

```toml
[contract_whale_monitor]
enabled = true
dry_run = false
```

Critical and S signals can send. High signals only send when the score and multi-exchange gate allow it. Cooldown prevents repeated same-direction alerts.

## Metrics

```bash
curl http://127.0.0.1:3000/api/contract-whale/metrics
```

Metrics include CWM WebSocket connection status, reconnect counts, generated signals, Discord sent/skipped counts, data quality, and recent 60s flow. They do not include webhook URLs, tokens, raw payloads, evidence, or markout data.

## Retention

Default retention:

```toml
[contract_whale_monitor.retention]
flow_1s_days = 14
signals_days = 365
```

The service prunes old 1s flow buckets and old signals on an hourly background task. Signal history is intentionally retained longer than raw aggregates.

## Rollback

1. Set `contract_whale_monitor.enabled = false`.
2. Restart the backend.
3. Confirm `/api/contract-whale/summary` returns `healthStatus=disabled`.
4. Leave existing history tables intact for audit unless explicitly approved for deletion.

## Common Issues

- OKX disabled: this is expected in current production mode and should not lower data quality.
- OKX `ctVal` missing: only relevant when OKX is explicitly re-enabled; do not guess contract size.
- Bitfinex disconnected: summary should degrade, not stop Binance monitoring.
- Discord 429: keep cooldown enabled and review `cwm.discord.skipped` reasons.
- Cargo cache corruption: follow `docs/dev/cargo-cache-recovery.md`.
- No frontend updates: check `/api/contract-whale/summary`, `/api/contract-whale/latest`, and browser network errors before restarting the backend.
