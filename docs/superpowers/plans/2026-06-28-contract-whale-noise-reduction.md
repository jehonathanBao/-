# Contract Whale Noise Reduction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce noisy Medium contract-whale events without changing the detector’s core architecture or weakening High/Critical/S coverage.

**Architecture:** Tighten Medium entry gating in the detector, change cross-window merge from additive inflation to representative snapshots, and make lifecycle updates use peak snapshots plus a short dedup suppression window instead of cumulative volume growth. Keep the existing detector, merge, lifecycle, and projection pipeline intact so data remains real and traceable.

**Tech Stack:** Rust backend, Axum API projection, existing Rust integration tests, React frontend consuming existing event APIs.

## Global Constraints

- Do not change High, Critical, or S trigger thresholds.
- Do not remove 5s / 15s / 60s windows.
- Do not remove merge or lifecycle stages.
- Do not fabricate history or fake events.
- Prefer narrowly scoped backend behavior changes with tests before implementation.

---

### Task 1: Lock in merge and lifecycle anti-inflation behavior with failing tests

**Files:**
- Modify: `D:\DevWorkspaces\Documents\有毒订单监控-rs\tests\contract_whale_routes_tests.rs`

**Interfaces:**
- Consumes: `build_contract_whale_history_response(...)`, `build_final_events_from_contract_whale_signals(...)`
- Produces: regression coverage for cross-window merge and lifecycle snapshot semantics

- [ ] **Step 1: Add failing assertions for max-window merge instead of summed volume**
- [ ] **Step 2: Add failing assertions for lifecycle snapshot volume instead of cumulative volume**
- [ ] **Step 3: Add failing assertions for 30s duplicate Medium suppression**
- [ ] **Step 4: Run targeted Rust tests and confirm RED**

### Task 2: Lock in detector Medium gating with failing tests

**Files:**
- Modify: `D:\DevWorkspaces\Documents\有毒订单监控-rs\tests\contract_whale_monitor_tests.rs`

**Interfaces:**
- Consumes: `detect_contract_whale_signal(...)`, `detect_contract_whale_signal_with_config(...)`
- Produces: regression coverage for price-response and dominance gating on Medium only

- [ ] **Step 1: Add failing test showing low-price-response noise no longer becomes Medium**
- [ ] **Step 2: Add failing test showing strong directional follow-through still produces Medium**
- [ ] **Step 3: Run targeted Rust tests and confirm RED**

### Task 3: Implement anti-inflation merge and lifecycle behavior

**Files:**
- Modify: `D:\DevWorkspaces\Documents\有毒订单监控-rs\src\contract_whale_monitor\merge.rs`
- Modify: `D:\DevWorkspaces\Documents\有毒订单监控-rs\src\contract_whale_monitor\event_lifecycle.rs`

**Interfaces:**
- Consumes: merged signal snapshots, lifecycle updates
- Produces: max-window merge semantics, snapshot-based lifecycle state, 30s Medium suppression

- [ ] **Step 1: Change merge volume/notional/net aggregation to representative max snapshot semantics**
- [ ] **Step 2: Preserve metadata union (`merged_from`, exchanges, merged windows) without summed volume inflation**
- [ ] **Step 3: Change lifecycle updates to keep peak snapshot volume and bounded accumulated OI**
- [ ] **Step 4: Add same-symbol/same-direction/same-type Medium suppression inside the lifecycle update window**
- [ ] **Step 5: Run targeted Rust tests and confirm GREEN**

### Task 4: Implement Medium detector tightening only

**Files:**
- Modify: `D:\DevWorkspaces\Documents\有毒订单监控-rs\src\contract_whale_monitor\detector.rs`

**Interfaces:**
- Consumes: `ContractWhaleWindowStats`
- Produces: stricter `ContractWhaleSeverity::Medium` and clearer calm reject reasons

- [ ] **Step 1: Require price-response or stronger dynamic/dominance confirmation before Medium**
- [ ] **Step 2: Keep existing High/Critical/S branches unchanged**
- [ ] **Step 3: Update calm reject reasons to reflect new Medium gating**
- [ ] **Step 4: Run targeted Rust tests and confirm GREEN**

### Task 5: Full verification and behavioral sanity check

**Files:**
- No new code; verify changed backend behavior only

**Interfaces:**
- Consumes: full route/history/event pipeline
- Produces: confidence that noise falls while meaningful events remain

- [ ] **Step 1: Run `cargo test --test contract_whale_routes_tests`**
- [ ] **Step 2: Run `cargo test --test contract_whale_monitor_tests`**
- [ ] **Step 3: Run `cargo check`**
- [ ] **Step 4: Review changed expectations for historical stream / ACTIVE / CLOSED semantics**

## Self-Review

- Spec coverage: merge inflation, lifecycle inflation, Medium gating, and duplicate suppression each have a task.
- Placeholder scan: no TBD/TODO placeholders remain.
- Type consistency: all touched interfaces already exist in the codebase and remain local to the contract-whale pipeline.

## Execution Handoff

The user already asked for implementation now, so execute inline in this session using TDD and verify before claiming completion.
