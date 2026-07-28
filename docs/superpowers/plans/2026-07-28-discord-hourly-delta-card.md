# Discord Hourly Delta Card Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make the Binance BTCUSDT 1H Delta Discord alert direction-first so traders can identify bullish/bearish bias and net flow at a glance.

**Architecture:** Keep the existing calculation, persistence, idempotency, outbox, and dry-run behavior unchanged. Refine only the Discord content and embed builder, adding derived buy/sell shares and a compact Unicode direction bar to the presentation layer.

**Tech Stack:** Rust, serde_json, chrono, existing Discord webhook client, Cargo integration tests.

## Global Constraints

- Preserve `|Delta| > 1000 BTC` and closed-1H-only alert semantics.
- Preserve read-only monitoring; no trading or execution controls.
- Preserve dry-run defaults and webhook validation.
- Do not stage or overwrite unrelated existing worktree changes.
- Use the existing hourly Delta outbox and record key for dedupe.

---

### Task 1: Add direction-first Discord copy tests

**Files:**
- Modify: `tests/hourly_delta_alert_discord_tests.rs`
- Test: `tests/hourly_delta_alert_discord_tests.rs`

**Interfaces:**
- Consumes: `build_hourly_delta_discord_content`, `build_hourly_delta_discord_payload`, `HourlyDeltaResult`.
- Produces: assertions for bearish/bullish hero copy, share values, bar text, metadata, and preserved disclaimers.

- [ ] **Step 1: Add bearish presentation assertions**

Assert the `-2,800 BTC` fixture includes `🔴 偏空`, `净卖出：2,800 BTC`, `卖出占比`, `买入占比`, `方向强度`, and `1H 已收线`.

- [ ] **Step 2: Add bullish presentation assertions**

Assert the `+1,200 BTC` fixture includes `🟢 偏多`, `净买入：1,200 BTC`, and a green presentation payload color.

- [ ] **Step 3: Add payload field-order assertions**

Assert the embed title/content put direction and net delta before the secondary buy/sell and period fields, while retaining the read-only disclaimer and record footer.

- [ ] **Step 4: Run the focused test and confirm it fails**

Run: `cargo test --test hourly_delta_alert_discord_tests -- --nocapture`

Expected: new direction-first assertions fail against the current copy.

### Task 2: Implement direction-first Discord presentation

**Files:**
- Modify: `src/contract_whale_monitor/hourly_delta_alert/discord.rs:65-119`
- Test: `tests/hourly_delta_alert_discord_tests.rs`

**Interfaces:**
- Consumes: existing `HourlyDeltaResult` and `HourlyDeltaDirection`.
- Produces: unchanged `build_hourly_delta_discord_content` and `build_hourly_delta_discord_payload` signatures with updated display output.

- [ ] **Step 1: Add display helpers**

Add private helpers that calculate finite, zero-safe buy/sell shares and render a bounded 20-character Unicode bar. Use `sell_share` for bearish alerts and `buy_share` for bullish alerts; never change the alert decision.

- [ ] **Step 2: Replace the content hero**

Use `🔴 偏空 | BTC 1H 净卖出 ...` or `🟢 偏多 | BTC 1H 净买入 ...` as the first content line, then show share percentages and the direction bar before secondary metadata.

- [ ] **Step 3: Reorder embed fields**

Put `方向`, `净差 Delta`, `主动卖出`, `主动买入`, and shares first; keep cycle, threshold, source, status, disclaimer, and record key afterward.

- [ ] **Step 4: Keep Discord behavior unchanged**

Do not alter dry-run, webhook validation, retry, outbox, threshold, or direction calculation code.

- [ ] **Step 5: Run the focused tests and confirm they pass**

Run: `cargo test --test hourly_delta_alert_discord_tests -- --nocapture`

Expected: all Discord card, dry-run, below-threshold, and outbox idempotency tests pass.

### Task 3: Full verification and server sync

**Files:**
- Verify: `src/contract_whale_monitor/hourly_delta_alert/discord.rs`
- Verify: `tests/hourly_delta_alert_discord_tests.rs`
- Verify: `tests/hourly_delta_alert_calc_tests.rs`

**Interfaces:**
- Consumes: updated card builder and existing runtime.
- Produces: verified commit, pushed branch, deployed service, and restart health evidence.

- [ ] **Step 1: Run focused and full validation**

Run:

```powershell
cargo fmt --check
cargo test --test hourly_delta_alert_discord_tests -- --nocapture
cargo test --test hourly_delta_alert_calc_tests -- --nocapture
cargo test -j 1 --all-targets
```

- [ ] **Step 2: Review the diff and stage only feature files**

Confirm unrelated modified files and `.trellis/scripts/common/__pycache__/` remain unstaged. Stage the Discord builder, focused tests, and this plan only.

- [ ] **Step 3: Commit the presentation change**

Use commit message: `feat: redesign hourly delta discord card`.

- [ ] **Step 4: Push and deploy using the existing repository runbook**

Push the current branch, run the configured server sync/deploy command, and restart only the monitoring service.

- [ ] **Step 5: Verify the deployed runtime**

Check service status and health endpoint, then confirm dry-run/log output contains the new direction-first card text without a real webhook send.
