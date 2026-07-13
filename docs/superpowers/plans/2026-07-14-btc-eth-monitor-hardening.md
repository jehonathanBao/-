# BTC / ETH Monitor Hardening Implementation Plan

> **Scope:** Harden the existing read-only BTC/ETH contract and spot monitors. Do not add execution, account, order, or private-data capabilities.

**Goal:** Make quantity units, source health, evidence freshness, signal gates, history semantics, and retention behavior trustworthy per asset while preserving existing routes and compatibility fields.

## 1. Establish Release Gates

- Add/repair compile fixtures so `cargo check --all-targets` is a reliable gate.
- Correct lifecycle-volume tests to enforce peak-window semantics instead of cumulative semantics.
- Keep the existing public API fields compatible while adding explicit semantic/evidence fields.

## 2. Contract Data Integrity

- Normalize OKX contract size with instrument `ctVal`; fail closed when metadata is unavailable.
- Track venue health by canonical symbol, not only by exchange.
- Bound OI and funding evidence by configured freshness before detection and persistence.
- Expose liquidation evidence as `live`, `inferred`, or `unavailable`; never present an unstarted collector as live data.
- Tighten Medium and multi-venue confirmation with dominance, quality, notional, and meaningful venue-contribution gates.

## 3. Spot Signal Integrity

- Replace cross-venue first/last price response with same-venue return aggregation.
- Require material venue contribution for multi-venue confirmation.
- Tighten Medium gates without changing High/Critical thresholds.
- Add stale-current diagnostics and deterministic cursor pagination while preserving offset compatibility.
- Schedule spot-history retention and log each run.

## 4. API and UI Semantics

- Render quantity units from the selected asset (`BTC` or `ETH`) while retaining legacy numeric field names.
- Make lifecycle peak-window semantics explicit and stop claiming cumulative volume where none exists.
- Show OI/funding/liquidation availability and stale reasons in read-only views.
- Avoid public calls to operator/debug endpoints and consolidate refresh timing around the canonical event timeline.

## 5. Verification and Rollout

- Run targeted Rust and React tests after each behavior slice.
- Run `cargo fmt --check`, `cargo check --all-targets`, relevant/full tests, frontend tests, and production build.
- Review the diff for secrets, execution-boundary drift, and unrelated changes.
- Commit only this task, push `main`, pull on production, rebuild/restart backend and frontend, then verify health and BTC/ETH contract/spot routes.
