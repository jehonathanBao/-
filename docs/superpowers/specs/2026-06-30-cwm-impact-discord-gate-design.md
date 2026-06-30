# CWM Impact Discord Gate Design

## Goal

Allow Contract Whale Monitor Discord pushes when backend-authoritative market impact level is `B`, `A`, or `S`, even when the signal severity is `Medium`, while preserving data-quality, warmup, duplicate, cooldown, dry-run, and webhook gates.

## Current Problem

The `/contract-whale` UI displays two independent concepts:

- `severity`: `Medium`, `High`, `Critical`, `S`
- `impactLevel`: `C`, `B`, `A`, `S`

The CWM Discord gate currently uses `severity`. As a result, `severity = Medium` and `impactLevel = B` appears on the UI as `L2 / B` but remains observe-only in Discord.

## Chosen Approach

Use a backend-authoritative impact layer:

- Add impact fields to `ContractWhaleSignal`.
- Populate those fields when detector creates a signal.
- Reuse `src/normalization/market_impact.rs` for impact classification.
- Let CWM Discord gate read those backend fields.
- Keep frontend display derived from backend fields when present, with existing fallback only for older records.

## Backend Fields

Add these optional fields to `ContractWhaleSignal`:

- `impact_level: Option<String>` with values `C`, `B`, `A`, `S`
- `signal_level: Option<String>` with values `L1`, `L2`, `L3`, `S`
- `signal_label: Option<String>`
- `normalized_strength: Option<String>`
- `impact_score: Option<f64>`
- `impact_z_score: Option<f64>`

Older persisted records may omit these fields. Deserialization must remain backward-compatible.

## Gate Behavior

The original severity gate remains valid:

- `Critical` and `S` continue through the existing gate.
- `High` continues through existing BTC, multi-exchange, and primary-source override logic.
- `Medium` remains observe-only unless impact gate qualifies it.

New impact gate:

- `impact_level in ["B", "A", "S"]` is eligible.
- `impact_level = "C"` is not eligible.
- The feature is enabled by default.
- Data quality below the configured minimum blocks impact gate.
- Warmup remains a hard block.
- Duplicate and cooldown gates remain unchanged.

Reason precedence:

- If severity gate qualifies, keep the existing severity reason.
- If severity does not qualify and impact gate qualifies, use `impact_level_gate`.
- If both qualify, Discord payload still includes impact details, but the main reason remains the severity reason.

## Configuration

Add CWM runtime config:

- `contract_whale_monitor.discord.impact_level_push_enabled = true`
- `contract_whale_monitor.discord.push_impact_levels = ["B", "A", "S"]`
- `contract_whale_monitor.discord.impact_level_min_data_quality = 70`

Delivery settings remain in `ContractWhaleDiscordSettings` and environment variables:

- `CONTRACT_WHALE_DISCORD_ENABLED`
- `CONTRACT_WHALE_DISCORD_WEBHOOK_URL`
- `CONTRACT_WHALE_DISCORD_TIMEOUT_MS`
- `CONTRACT_WHALE_DISCORD_MAX_ATTEMPTS`
- `CONTRACT_WHALE_DISCORD_COOLDOWN_SEC`

## Discord Payload

Add safe final fields:

- Signal severity
- Market impact as `B / L2`, `A / L3`, or `S / S`
- Push reason

For `Medium + B`, the message explains that the signal severity is still Medium and the push happened because market impact reached B.

## Frontend Behavior

The UI must keep severity and market impact separate:

- Severity badge remains `Medium`, `High`, `Critical`, or `S`.
- Impact badge remains `L1/C`, `L2/B`, `L3/A`, or `S/S`.
- Discord detail panel shows a human-readable reason such as `market impact B` for `impact_level_gate`.

The frontend must not decide Discord eligibility. It only displays backend fields.

## Persistence And Old Data

Existing historical records are not backfilled and are not retroactively pushed. New signals carry authoritative impact fields. The existing frontend fallback can continue to render impact labels for old payloads.

## Tests

Backend tests must cover:

- `Medium + B` is eligible with `impact_level_gate`.
- `Medium + A` is eligible.
- `Medium + S` is eligible.
- `Medium + C` remains observe-only.
- `Medium + B` with low data quality is blocked.
- `Medium + B` during warmup remains blocked.
- Existing `High`, `Critical`, and `S` gates do not regress.
- Duplicate and cooldown suppression still apply.

Frontend tests must cover:

- `Medium + B` displays severity as Medium, impact as `L2 / B`, and Discord reason as market impact B.
- `Medium + C` displays observe-only behavior.
- No `undefined` or `NaN` is rendered.

## Out Of Scope

- No market-structure gate changes.
- No retroactive Discord push for old records.
- No broad opening of all Medium signals.
- No cooldown-key change.
