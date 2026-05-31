# Workspace Policy

## Active Repository

- Active development repo: `C:\Users\byhdo_ocup4f5\Documents\有毒订单监控-rs`
- Legacy reference repo: `C:\Users\byhdo_ocup4f5\Documents\有毒订单监控`

All new R/T task execution must target the Rust repo above unless a task card explicitly says otherwise.

## Policy Decision

- `有毒订单监控-rs` is the only active development repo.
- `有毒订单监控` is legacy TypeScript/Node reference only.
- Wholesale merge from the legacy repo into the Rust repo is forbidden.
- Selective migration is allowed only when a specific missing Rust behavior is identified and reviewed manually.

## Why Wholesale Merge Is Forbidden

- The legacy repo overlaps heavily with the Rust repo in `src/api`, `src/connectors`, `src/normalizers`, `src/safety`, `src/toxicity`, and shared market-data concepts.
- The legacy repo is an alternate implementation surface, not a clean additive module.
- Copying source wholesale would mix TypeScript/Node and Rust/Cargo implementations, increase conflict risk, and blur the current mainline.
- The Rust repo already contains the active dashboard, calibration/manual-apply governance flow, and newer operator review surfaces.

## Selective Migration Policy

Use legacy files only as read-only reference material when:

1. A Rust feature gap is clearly identified.
2. The missing behavior can be traced to a specific legacy file or test.
3. The migration is reimplemented intentionally in Rust rather than copied blindly.
4. Runtime state, generated artifacts, and local environment files stay excluded.

Examples of acceptable reference-only inputs:

- legacy test cases that describe expected behavior
- legacy route behavior notes
- legacy design intent embedded in comments or small helper modules
- non-secret config examples if the Rust repo lacks equivalent documentation

## Forbidden Migration List

Never migrate these from the legacy repo into the Rust repo:

- `.runtime/`
- `node_modules/`
- database files
- logs
- generated reports
- local runtime state
- secrets, keys, or non-example env files
- stale generated fixtures
- wholesale source directories that duplicate Rust mainline modules

## Required Guard For Future R/T Tasks

Every future R/T task must start with:

```powershell
cd C:\Users\byhdo_ocup4f5\Documents\有毒订单监控-rs
```

Then verify:

- `current_repo == C:\Users\byhdo_ocup4f5\Documents\有毒订单监控-rs`
- `Cargo.toml` exists
- `src/` exists
- `tests/` exists
- `web/` exists

If the guard fails, the task must stop with:

```text
PAUSED
reason: workspace_mismatch
action: do not modify current workspace
```

## Operational Boundaries

- Do not modify the legacy repo during normal feature work.
- Do not copy legacy runtime/config/db/log artifacts into the Rust repo.
- Do not bypass manual-apply governance boundaries when working in the Rust repo.
- Do not treat reference-only legacy code as current production truth without manual review.
