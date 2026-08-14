# Main-Force Shadow and Impact S Hardening Implementation Plan

> **For agentic workers:** This plan is executed inline in the current task with test-first checkpoints. All changes preserve read-only monitoring and dry-run notification defaults.

**Goal:** Separate market-impact grading from main-force behavior, prevent ordinary windows from being represented as S impact, and expose a bounded read-only Shadow candidate lane sourced from persisted 1-second contract flow.

**Architecture:** Impact S is normalized at the signal boundary from hard evidence only, then reused by API, Discord, and retention metadata. Shadow is an independent candidate detector over persisted 1-second buckets; it emits suspect/watching/corroborated/invalidated observations and never claims confirmed main-force behavior or sends Discord automatically. Corroborated Shadow observations are compatible with the existing main-force event model but do not replace its authoritative confirmation gate.

**Tech Stack:** Rust, Axum, SQLite, serde, existing contract-whale repository, Cargo integration tests.

## Global Constraints

- `read_only=true`, `analysis_only=true`, and `execution_enabled=false` remain unchanged.
- Shadow never triggers order execution and defaults to observation/dry-run only.
- Missing OI, stale data, inferred liquidation, or missing cross-source confirmation must fail closed.
- Existing contract and spot retention policies remain bounded at 7/30/365 days.
- Existing dirty files remain untouched.

---

### Task 1: Impact S hard-evidence gate

**Files:** `src/contract_whale_monitor/discord.rs`, `src/contract_whale_monitor/detector.rs`, `src/api/contract_whale_routes.rs`, `src/storage/retention_policy.rs`, `tests/contract_whale_monitor_tests.rs`, `tests/retention_policy_tests.rs`

**Acceptance:** A signal can retain S only with live liquidation of at least 2,500 BTC or extraordinary turnover of at least 20,000 BTC over at least 60 seconds with at least two confirmed exchanges, dynamic multiple at least 10, percentile at least 99.5, and dominance at least 0.65. Otherwise S is downgraded to A for display/push/retention semantics. Missing evidence never upgrades a grade.

### Task 2: Persisted 1-second footprint features

**Files:** `src/contract_whale_monitor/types.rs`, `src/contract_whale_monitor/aggregator.rs`, `src/storage/migrations.rs`, `src/storage/contract_whale_repo.rs`, `tests/contract_whale_monitor_tests.rs`

**Acceptance:** Buckets expose directional trade counts and concentration fields while remaining backward-compatible with existing rows. A one-trade pulse and many smaller same-direction trades produce different footprint values.

### Task 3: Shadow candidate detector

**Files:** create `src/contract_whale_monitor/shadow.rs`, modify `src/contract_whale_monitor/mod.rs`, add `tests/contract_whale_shadow_tests.rs`

**Acceptance:** Sub-High, persistent same-direction flow can become `suspect`/`watching`; three contiguous observations with usable OI, data quality at least 70, and cross-source confirmation can become `corroborated`; a single pulse, missing OI, inferred liquidation, stale gap, or poor quality cannot corroborate.

### Task 4: Read-only Shadow API and operator semantics

**Files:** create `src/api/contract_whale_shadow_routes.rs`, modify `src/api/server.rs`, add `tests/contract_whale_shadow_routes_tests.rs`

**Acceptance:** `GET /api/contract-whale/shadows` returns bounded candidate observations with explicit lane `shadow`, state, evidence, and read-only flags. It does not call the Discord outbox and never uses “主力确认” for suspect/watching states.

### Task 5: Verification and rollout

**Files:** `docs/usage-guide.md` and existing Rust/frontend test suites.

**Acceptance:** Focused tests, relevant Rust tests, frontend tests/build, `git diff --check`, server rebuild/restart, health/ready checks, and authenticated Shadow API smoke test all pass.
