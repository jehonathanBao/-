---
name: rust-backend-security
description: Review or implement Rust backend changes for order-risk security, tenant isolation, auditability, safe queries, SSRF protection, PII-safe logging, and Tokio-safe async behavior.
---

# Rust Backend Security

Use this skill when a task touches backend routes, services, repositories, external HTTP clients, audit logs, or risk decisions.

## Guardrails

- Preserve read-only market-monitoring paths unless a task explicitly opens a write path.
- Scope every order query by `tenant_id`, `shop_id`, and `user_id` before returning data.
- Never build SQL with string interpolation. Use sqlx bind parameters or an ORM query builder.
- Never log phone numbers, addresses, ID numbers, full emails, tokens, webhook URLs, or raw order payloads.
- Treat external URLs as untrusted. Block localhost, private IP ranges, link-local, metadata IPs, and non-HTTPS destinations unless explicitly allowlisted.
- Require authorization checks for alert actions, manual review actions, blacklist updates, whitelist updates, threshold updates, and rule changes.
- Record audit entries for rule updates, threshold changes, manual release, manual block, blacklist changes, and whitelist changes.
- Do not block Tokio runtime with sync file, network, sleep, or CPU-heavy work. Use async APIs or `spawn_blocking`.

## Review Checklist

- Security: tenant/shop/user scoping, parameter binding, no PII logs, SSRF guard, explicit permission checks.
- Risk correctness: dedupe alerts by stable order key, manual release overrides automated block, explicit blacklist/whitelist/rule priority.
- Auditability: model score, rule hits, inputs hash, actor, action, and timestamp are traceable.
- Performance: paginated list queries, indexes for hot filters, no N+1 query loops, bounded caches.
- Tests: normal, high-risk, missing fields, duplicate order, boundary values, cross-shop denial, manual release precedence, log redaction.

## Toxic Signal Alert Boundary

- Discord and Telegram pushes are alert-only and must remain read-only.
- Preserve the gate: high or critical only, score `>= 80`, data quality `>= 70`.
- Medium and low candidates may enter replay reports and inbox display but must not push external alerts.
- Alert payloads may include symbol, detector/event type, direction, final result, risk score, and data quality.
- Do not expose markout, stale flags, raw evidence, private tokens, webhook URLs, or raw payloads in alert messages.

## Production Replay Boundary

- Real L2/trade files belong only in ignored `data/production_replay/` paths.
- Local runs should use ignored `config/replay.production.local.toml`.
- `snapshot_reset` events must not count as cancel evidence.
- Replay outputs should remain reports: summary, signals, calibration JSON/Markdown, high-score CSV, false-positive/false-negative CSVs.
- Escape CSV cells that start with formula prefixes: `=`, `+`, `-`, `@`, including leading-whitespace variants.

## Required Verification

Run the relevant subset first, then the full backend gate before merging:

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test -j 1 --all-targets
```

## Output Format

For reviews, list findings first with `file:line`, impact, and a concrete fix. For implementation, summarize only changed safety boundaries, tests, and residual risks.
