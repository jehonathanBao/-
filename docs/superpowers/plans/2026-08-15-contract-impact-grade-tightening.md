# Contract Impact Grade Tightening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make A a rare major-impact grade and keep page, Discord, and retention semantics aligned.

**Architecture:** Keep the existing detector and API fields. Apply one fail-closed canonical downgrade function after the signal has all evidence fields, before persistence and Discord decisions. Use explicit threshold constants and boundary tests so the grade cannot drift with the page cohort.

**Tech Stack:** Rust, Cargo integration tests, React/Vitest, Docker Compose, PowerShell read-only health checks.

## Global Constraints

- Preserve read-only, analysis-only, dry-run, and execution-disabled behavior.
- Do not rewrite historical database rows or delete retention data in this rollout.
- Preserve the existing `C/B/A/S` JSON fields and `L1/L2/L3/S` mapping.
- Keep the existing hard-evidence S gate and only tighten raw A fallback behavior.
- Do not modify unrelated user changes, including `docs/superpowers/plans/2026-08-04-contract-whale-impact-grade-v3.md`.

---

### Task 1: Add failing canonical-grade boundary tests

**Files:**
- Modify: `tests/contract_whale_monitor_tests.rs`
- Test target: `sanitize_contract_whale_impact` through the existing detector sample helper

**Interfaces:**
- Consumes: a detector-produced `ContractWhaleSignal` with raw `A` or `S` fields.
- Produces: assertions for canonical `impact_level`, `signal_level`, and `signal_label`.

- [x] **Step 1: Write tests that describe the new behavior**

Import `sanitize_contract_whale_impact` beside `is_historic_s_impact`. Build each input from the existing detector sample, then set the listed fields before calling the sanitizer: ordinary raw A uses `data_quality=85`, `percentile_level=Some(99.0)`, `impact_score=Some(10.0)`, `impact_z_score=Some(10.0)`, `total_volume_btc=638.0`, `total_notional_usd=40_000_000.0`, `price_move_pct=Some(-0.104)`, and `multi_exchange_confirmed=false`; material raw A uses quality 70, percentile 99.0, both scores 4.0, volume 800.0, notional 50,000,000.0, price move 0.15, and no confirmation; confirmed major raw A uses quality 80, percentile 99.5, both scores 4.0, volume 2,500.0, notional 150,000,000.0, price move 0.5, and `multi_exchange_confirmed=true`. Assert exact pairs `C/L1/LOW IMPACT EVENT`, `B/L2/MEDIUM IMPACT EVENT`, and `A/L3/HIGH IMPACT EVENT` respectively.

- [x] **Step 2: Run the focused tests and verify the expected red failure**

Run:

```powershell
cargo test --test contract_whale_monitor_tests sanitize_contract_whale_impact -- --test-threads=1
```

Expected: the new tests fail because raw A currently remains A after the existing S-only sanitizer.

---

### Task 2: Implement the unified fail-closed A/B/C gate

**Files:**
- Modify: `src/contract_whale_monitor/discord.rs` near `sanitize_contract_whale_impact`
- Modify: `src/contract_whale_monitor/detector.rs` after canonical sanitization
- Modify: `tests/contract_whale_monitor_tests.rs`

**Interfaces:**
- Consumes: fully populated `ContractWhaleSignal`.
- Produces: canonical impact fields used by detector persistence, API, UI, Discord, and retention.

- [x] **Step 1: Add named constants and helper predicates**

Use these exact floors:

```rust
const A_MIN_DATA_QUALITY: u8 = 80;
const A_MIN_PERCENTILE: f64 = 99.5;
const A_MIN_MULTIPLE: f64 = 4.0;
const A_MIN_ABS_PRICE_MOVE_PCT: f64 = 0.5;
const A_MIN_VOLUME_BTC: f64 = 2_500.0;
const A_MIN_NOTIONAL_USD: f64 = 150_000_000.0;
const B_MIN_DATA_QUALITY: u8 = 70;
const B_MIN_PERCENTILE: f64 = 99.0;
const B_MIN_MULTIPLE: f64 = 2.5;
const B_MIN_ABS_PRICE_MOVE_PCT: f64 = 0.15;
const B_MIN_VOLUME_BTC: f64 = 800.0;
const B_MIN_NOTIONAL_USD: f64 = 50_000_000.0;
```

Use finite-value checks for every floating input. Independent confirmation is true when `multi_exchange_confirmed`, confirmed behavior is eligible, or live liquidation is positive and suspected.

- [x] **Step 2: Make raw A downgrade to B or C**

Inside `sanitize_contract_whale_impact`, preserve an existing hard-evidence S. For raw A, evaluate the A predicate first. If it fails, evaluate the B predicate; set the canonical triplet to `B/L2/MEDIUM IMPACT EVENT` or `C/L1/LOW IMPACT EVENT`.

- [x] **Step 3: Keep raw S fail-closed behavior unchanged**

Raw S without `is_historic_s_impact` remains `A` only after the A/B/C gate is applied, so it can no longer become an unqualified A. A confirmed historical S bypasses the A/B/C downgrade.

- [x] **Step 4: Close the C-grade Discord path**

After sanitization, set `discord_eligible` and `discord_would_send` to false with reason `impact_grade_c_display_only` for a C-grade signal unless the independent behavior lane is eligible. This prevents the pre-sanitization A decision from leaking into the outbox or status counters.

- [x] **Step 5: Preserve the explicit primary-source override**

Keep `high_primary_source_extreme` eligible through the low-score guard because it is already an explicit, data-quality-gated Discord override and has a dedicated regression test.

- [x] **Step 6: Run focused Rust tests and the adjacent suite**

Run sequentially:

```powershell
cargo test --test contract_whale_monitor_tests sanitize_contract_whale_impact -- --test-threads=1
cargo test --test contract_whale_monitor_tests detector_populates_market_impact_fields -- --test-threads=1
cargo test --test contract_whale_monitor_tests historic_s_impact_requires_hard_extreme_evidence -- --test-threads=1
```

Expected: all focused tests pass, including existing S hard-evidence coverage.

---

### Task 3: Verify frontend/API compatibility and build

**Files:**
- Inspect only: `toxic-order-monitor/src/api/contractWhale.js`, `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Test: existing contract-whale Vitest suites

**Interfaces:**
- Consumes: unchanged canonical JSON fields with new B/C values.
- Produces: unchanged field mapping and UI labels; no new execution control.

- [x] **Step 1: Run frontend contract-whale tests**

```powershell
npm --prefix toxic-order-monitor test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx
```

Expected: PASS with the existing `L3 / A`, cohort diagnostics, and canonical grade assertions intact.

- [x] **Step 2: Build the frontend**

```powershell
npm --prefix toxic-order-monitor run build
```

Expected: exit code 0 and a completed Vite build.

---

### Task 4: Full local verification and commit

**Files:**
- Review: all modified files and `git status --short`

- [x] **Step 1: Run formatting and focused backend suites**

```powershell
cargo fmt --all -- --check
cargo test --test contract_whale_monitor_tests -- --test-threads=1
cargo test --test contract_event_routes_tests -- --test-threads=1
```

- [x] **Step 2: Inspect the diff and ensure only intended files are changed**

```powershell
git diff --check
git status --short
```

The pre-existing untracked V3 plan remains untouched.

- [ ] **Step 3: Commit the verified implementation**

```powershell
git add src/contract_whale_monitor/discord.rs tests/contract_whale_monitor_tests.rs docs/superpowers/specs/2026-08-15-contract-impact-grade-tightening-design.md docs/superpowers/plans/2026-08-15-contract-impact-grade-tightening.md
git commit -m "fix: raise contract impact grade thresholds"
```

---

### Task 5: Sync and verify the read-only production runtime

**Files:**
- No additional source changes unless verification finds a defect in the files above.

- [ ] **Step 1: Push the verified commit**

```powershell
git push origin HEAD
```

- [ ] **Step 2: Sync the authorized server and recreate only backend/frontend**

Use the established SSH key and deployment path from `docs/server-deployment-runbook.md`. Pull fast-forward only, rebuild the backend/frontend images, and leave database rows and monitoring configuration intact.

- [ ] **Step 3: Verify service and API health**

Check `docker compose ps`, `/healthz`, `/readyz`, and the public BTC final-events endpoint. Confirm every newly returned A meets the new evidence floors and that the response remains `readOnly=true` / `executionEnabled=false`.

- [ ] **Step 4: Compare grade distribution before and after**

Fetch the seven-day BTC final-events payload and record counts for C/B/A/S, Discord-sent count, and any A rows that fail the thresholds. If any A fails, stop and report rather than claiming completion.
