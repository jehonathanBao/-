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

## Check Runtime Health

```bash
curl http://127.0.0.1:3000/api/contract-whale/summary
```

Expected health states:

- `healthy`: Binance or OKX data is recent, with primary sources online.
- `degraded`: at least one source is recent, but another source is disconnected or stale.
- `unhealthy`: all sources are stale or unavailable.
- `warming_up`: startup warmup is still collecting samples.
- `disabled`: the module is configured off.

If a source is connected but has no trades for 30 seconds on Binance/OKX, or 60 seconds on Bitfinex, treat it as stale.

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

- OKX `ctVal` missing: do not guess contract size. Keep data quality reduced until instrument metadata is restored.
- Bitfinex disconnected: summary should degrade, not stop Binance/OKX monitoring.
- Discord 429: keep cooldown enabled and review `cwm.discord.skipped` reasons.
- Cargo cache corruption: follow `docs/dev/cargo-cache-recovery.md`.
- No frontend updates: check `/api/contract-whale/summary`, `/api/contract-whale/latest`, and browser network errors before restarting the backend.
