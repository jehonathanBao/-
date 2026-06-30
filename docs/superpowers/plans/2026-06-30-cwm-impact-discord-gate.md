# CWM Impact Discord Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** CWM Discord pushes become eligible for backend-authoritative impact levels `B`, `A`, and `S` while preserving existing severity and safety gates.

**Architecture:** `ContractWhaleSignal` receives authoritative impact fields from the detector. CWM Discord gate evaluates `severity_gate || impact_level_gate`, with data quality and warmup still blocking before delivery. Frontend only displays backend eligibility and reason.

**Tech Stack:** Rust 2021, Axum, rusqlite, serde, React, Vite, Vitest.

## Global Constraints

- Do not open all `Medium` signals.
- Do not treat `impactLevel` as `severity`.
- Do not change `market_structure` Discord gate.
- Do not bypass data-quality, warmup, duplicate, cooldown, dry-run, or webhook gates.
- Do not change cooldown key from `symbol + direction + signal_type`.
- Do not retroactively push old historical records.
- Do not modify unrelated `spot_whale` working tree changes.

---

### Task 1: Backend Red Tests For Impact Gate

**Files:**
- Modify: `tests/contract_whale_discord_notifier_tests.rs`

**Interfaces:**
- Consumes: `evaluate_contract_whale_discord_gate`, `ContractWhaleDiscordCooldownStore`, `ContractWhaleDiscordSettings`
- Produces: failing tests defining `Medium + impactLevel B/A/S` behavior

- [ ] **Step 1: Add failing tests**

Add tests that clone an existing sample signal, set `severity = Medium`, set impact fields, and assert gate decisions:

```rust
#[test]
fn cwm_discord_gate_allows_medium_b_a_s_impact_levels() {
    let settings = live_settings_for_tests();
    for level in ["B", "A", "S"] {
        let cooldown = ContractWhaleDiscordCooldownStore::new();
        let mut signal = sample_medium_impact_signal(level);
        let decision =
            evaluate_contract_whale_discord_gate(&settings, &signal, &cooldown, signal.ts);
        assert!(decision.allowed, "impact level {level} should be allowed");
        assert_eq!(decision.reason, "eligible");
        assert_eq!(signal.discord_reason, "impact_level_gate");
    }
}
```

- [ ] **Step 2: Run red test**

Run: `cargo test --test contract_whale_discord_notifier_tests cwm_discord_gate_allows_medium_b_a_s_impact_levels`

Expected: fail because `ContractWhaleSignal` has no impact fields or gate still rejects Medium.

### Task 2: Backend Impact Fields And Detector Population

**Files:**
- Modify: `src/contract_whale_monitor/types.rs`
- Modify: `src/contract_whale_monitor/detector.rs`
- Modify: `src/normalization/market_impact.rs`
- Modify: `tests/contract_whale_monitor_tests.rs`

**Interfaces:**
- Produces: `ContractWhaleSignal.impact_level`, `signal_level`, `signal_label`, `normalized_strength`, `impact_score`, `impact_z_score`

- [ ] **Step 1: Add failing detector test**

Add a test asserting detector-generated signals carry impact fields.

- [ ] **Step 2: Run red test**

Run: `cargo test --test contract_whale_monitor_tests detector_populates_market_impact_fields`

Expected: fail because fields do not exist or are empty.

- [ ] **Step 3: Implement minimal impact field population**

Use `MarketImpactBaseline` with available signal stats volume context, using current signal volume as the tested raw value. Preserve the same classification thresholds from `market_impact.rs`.

- [ ] **Step 4: Run green tests**

Run: `cargo test --test contract_whale_monitor_tests detector_populates_market_impact_fields`

Expected: pass.

### Task 3: Backend Gate And Notifier

**Files:**
- Modify: `src/contract_whale_monitor/config.rs`
- Modify: `src/contract_whale_monitor/discord_gate.rs`
- Modify: `src/contract_whale_monitor/discord.rs`
- Modify: `src/contract_whale_monitor/discord_notifier.rs`
- Modify: `tests/contract_whale_discord_notifier_tests.rs`

**Interfaces:**
- Produces: `impact_level_gate` reason for `Medium + B/A/S`
- Preserves: severity reason priority for existing `High/Critical/S`

- [ ] **Step 1: Add red tests for blocked cases**

Cover `Medium + C`, low quality, warmup, duplicate, cooldown, and existing High behavior.

- [ ] **Step 2: Run red tests**

Run: `cargo test --test contract_whale_discord_notifier_tests cwm_discord_gate`

Expected: impact tests fail before implementation.

- [ ] **Step 3: Implement impact-level gate**

Add config defaults and gate helper. Keep delivery checks and cooldown unchanged.

- [ ] **Step 4: Add payload fields**

Add Discord fields for `Signal Severity`, `Market Impact`, and `Push Reason`.

- [ ] **Step 5: Run green tests**

Run: `cargo test --test contract_whale_discord_notifier_tests`

Expected: pass.

### Task 4: Frontend Normalization And Display

**Files:**
- Modify: `toxic-order-monitor/src/api/contractWhale.js`
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`
- Modify: `toxic-order-monitor/src/tests/ContractWhaleApi.test.js`
- Modify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: backend `impactLevel`, `signalLevel`, `discordReason`, `discordEligible`, `discordWouldSend`
- Produces: clear Discord reason text for impact-gated signals

- [ ] **Step 1: Add red frontend tests**

Cover `Medium + B` display and `Medium + C` observe-only display.

- [ ] **Step 2: Run red frontend tests**

Run: `cd toxic-order-monitor && npm test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx`

Expected: impact-gate wording assertions fail.

- [ ] **Step 3: Implement frontend display**

Normalize backend impact fields and map `impact_level_gate` to `market impact B/A/S` labels without changing severity display.

- [ ] **Step 4: Run green frontend tests**

Run: `cd toxic-order-monitor && npm test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx`

Expected: pass.

### Task 5: Full Verification

**Files:**
- No new files.

**Interfaces:**
- Verifies the complete feature.

- [ ] **Step 1: Format Rust**

Run: `cargo fmt --all`

Expected: exit 0.

- [ ] **Step 2: Run backend checks**

Run:

```bash
cargo test --test contract_whale_discord_notifier_tests
cargo test --test contract_whale_monitor_tests
cargo check
```

Expected: exit 0 for each command.

- [ ] **Step 3: Run frontend checks**

Run:

```bash
cd toxic-order-monitor
npm test -- --run src/tests/ContractWhaleApi.test.js src/tests/ContractWhaleMonitor.test.jsx
npm run build
```

Expected: exit 0 for each command.

- [ ] **Step 4: Inspect diff**

Run: `git diff -- src/contract_whale_monitor tests/contract_whale_discord_notifier_tests.rs tests/contract_whale_monitor_tests.rs toxic-order-monitor/src/api/contractWhale.js toxic-order-monitor/src/components/ContractWhaleMonitor.jsx toxic-order-monitor/src/tests/ContractWhaleApi.test.js toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx docs/superpowers`

Expected: only intended CWM impact Discord gate changes.
