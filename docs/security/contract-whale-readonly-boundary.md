# Contract Whale Monitor Read-Only Boundary

Contract Whale Monitor is an alerting and dashboard module only. It is read-only by design.

## Allowed Capabilities

- Consume public market-data WebSocket streams.
- Normalize public contract trades into BTC and USD notional units.
- Aggregate one-second buckets and rolling windows.
- Generate local candidate signals and history records.
- Show signals in the dashboard.
- Send redacted Discord notifications when enabled and eligible.

## Explicitly Forbidden Capabilities

The module must never:

- place orders
- cancel orders
- block orders
- transfer funds
- withdraw funds
- modify exchange account state
- require private account permissions
- read exchange API key secrets
- expose webhook URLs, tokens, raw payloads, evidence, or markout data in frontend output or logs

## Configuration Boundary

Default configuration keeps the module disabled and dry-run:

```toml
[contract_whale_monitor]
enabled = false
dry_run = true
```

When `enabled = false`, collectors, detector runtime, and Discord tasks must not start. API routes may still return an empty disabled state so the frontend can show `未启用`.

When `dry_run = true`, the module may collect public data, generate signals, and write history, but real Discord sends must be skipped and logged as would-send only.

## Verification

Before enabling the module in a runtime environment:

```powershell
rg -n "place_order|cancel_order|transfer_funds|withdraw|api_secret|private_key" src/contract_whale_monitor
```

The expected result is no executable account-state-changing logic under `src/contract_whale_monitor`.
