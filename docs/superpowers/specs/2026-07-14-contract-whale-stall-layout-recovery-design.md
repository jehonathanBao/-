# Contract Whale Stall and Layout Recovery Design

**Status:** Implemented and locally verified on 2026-07-14. Production release remains a separately authorized operation.

## Problem

The production ETH contract monitor intermittently renders an unavailable event feed, duplicated warning banners, stale analysis presented as current, and a large empty panel. During the captured failure window, the public Nginx logs recorded simultaneous `499` responses for summary, latest, contract events, final events, and intelligence requests, followed by recovery to `200` without a restart.

The failure is not missing ETH data. The heavy Axum handlers call synchronous SQLite queries and lifecycle projection work directly from async request handlers. The frontend polls multiple heavy views, aborts requests after 5, 6, or 12 seconds, and converts those aborts into unavailable states. The event panel then keeps an unconditional `min-h-[50vh]`, which amplifies a transient backend stall into a large blank area.

## Goals

- Prevent heavy contract-event projection from blocking health, summary, and latest responses.
- Deduplicate concurrent equivalent heavy queries and bound their concurrency.
- Add a specialized SQLite query path for the production symbol, time-range, and notional filter shape.
- Preserve and clearly label the last successful data during partial outages.
- Make the historical event stream the first substantial page content.
- Remove large empty error states and avoid cramped two-column layouts.
- Preserve all detector, threshold, lifecycle, Discord, read-only, and dry-run semantics.

## Non-goals

- No detector, score, severity, visibility, retention, or Discord-rule changes.
- No trading, execution, signing, payment, deletion, or administrative actions.
- No new frontend action buttons.
- No broad rewrite of the contract-whale domain or the dashboard component tree.
- No automatic commit, deployment, or server restart.

## Chosen Approach

Use bounded blocking isolation, keyed single-flight, a verified SQLite query fast path, and fault-tolerant frontend presentation. This is narrower than introducing a continuously materialized event projection and fixes the root cause instead of only increasing client timeouts.

## Backend Architecture

### Heavy projection runtime

Add a contract-event projection runtime owned by `AppState` with these responsibilities:

- A global `tokio::sync::Semaphore` limits heavy projection work to two concurrent jobs.
- A normalized query key identifies equivalent contract-event projection requests.
- A keyed single-flight lock ensures one active calculation per query key.
- A bounded successful-result cache stores at most 64 query entries.
- Inserting a 65th entry evicts the entry with the oldest successful-cache timestamp; completed single-flight state must not grow independently of this bound.
- Fresh cache lifetime is 10 seconds. A successful cache entry may be served as stale for at most 5 minutes while a refresh is in progress or capacity is exhausted.
- Cache lookup and single-flight coordination remain asynchronous and must not perform SQLite work while holding an async lock.

`contract-events` and `final-events-v2` keep their public response shapes. Their synchronous database and projection work runs under `spawn_blocking`. Prepared projection data may be reused between endpoints only when the cached source query covers the requested filters and limit; reuse must never increase the ETH initial request beyond its current 20-row limit.

### Time budgets and degraded responses

- A request may wait up to 4 seconds for an in-flight or newly started heavy calculation.
- If a fresh result completes, return it normally.
- If the budget expires and a cache entry no older than 5 minutes exists, return it with `dataState=stale`, `degraded=true`, cache age, and a stable reason code.
- If no usable cache exists, return a structured `503 Service Unavailable` response before the browser timeout. Include a stable error code and retry delay; never fabricate an empty successful result.
- The heavy calculation may finish under its semaphore permit and populate the cache after an individual waiter times out. Client cancellation must not start duplicate work.

Health, summary, and latest routes do not wait on the heavy semaphore or single-flight map.

### SQLite query path

The current frontend request supplies symbol, 24-hour range, and minimum notional, causing the repository to use a nullable-OR general query. Add a dedicated parameterized query branch for this concrete filter shape. Before adding or changing an index:

1. Capture `EXPLAIN QUERY PLAN` for the existing query.
2. Prefer an existing index if it satisfies symbol filtering and timestamp ordering.
3. Add one migration-backed composite index only if the query plan proves it is needed.
4. Verify the final plan avoids a full table scan and avoid an unnecessary temporary sort.

The fast path must return the same rows and ordering as the general path.

## Frontend Data Model and Error Handling

Replace the shared page-level failure state with independent status for:

- summary/latest;
- historical contract events;
- lifecycle final events;
- intelligence analysis.

Each slice records its last successful refresh time and retains its last successful payload. A failure in one slice cannot mark unrelated panels unavailable.

Display rules:

- One consolidated data-state banner replaces duplicated top-level and event-feed warnings.
- Fresh data is presented normally.
- Retained data is labeled `STALE` with the last successful time.
- Current risk becomes `UNKNOWN` when its source is stale; the last value is shown secondarily, for example `上次 LOW RISK（时间）`.
- If no successful data exists, show a compact degraded state with the next automatic retry time.
- Do not add a manual retry button; existing automatic polling remains the recovery mechanism.

## Frontend Layout

The first-screen order becomes:

1. compact page status and data-state banner;
2. filters and visibility explanation;
3. historical event stream;
4. Pro Desk overview and secondary analysis panels;
5. lifecycle and system diagnostics.

The historical event panel keeps `min-h-[50vh]` only when it contains visible event content. Loading, empty, filtered-empty, and error states use content-driven height.

The historical-event and market-structure columns switch from `xl` to `2xl`. Below `2xl`, they stack with the historical event stream first. The same rule applies where a fixed secondary column would make the primary monitoring content cramped.

## Testing Strategy

Implementation follows red-green-refactor.

Backend tests:

- A deliberately slow heavy projection does not delay health, summary, or latest.
- Equivalent concurrent requests execute one underlying calculation.
- Heavy jobs never exceed the concurrency limit.
- Fresh, stale, no-cache, timeout, and failed-calculation paths return the specified state and status code.
- Specialized and general SQLite queries return equivalent rows and ordering.
- `EXPLAIN QUERY PLAN` confirms the selected production query path uses the intended index and avoids a full table scan.
- Existing contract-event and lifecycle semantics tests remain unchanged and green.

Frontend tests:

- A failed event refresh retains previous events and labels only that slice stale.
- Stale intelligence renders current risk as unknown and the previous risk as secondary history.
- Only one consolidated warning is visible.
- Error and empty event states do not use the half-viewport minimum height.
- Populated events retain the large primary workspace.
- Historical events precede Pro Desk analysis in document order.
- Existing polling recovery replaces stale state after a successful refresh.

Responsive verification:

- Verify 1223, 1280, and 1536 pixel viewports with browser automation.
- Confirm no document-level horizontal overflow or clipped secondary column.
- Confirm the event stream is the first substantial content region.

## Acceptance Criteria

- Under a stalled event projection, health, summary, and latest continue responding.
- Multi-tab equivalent polling produces one heavy calculation per query key.
- The local concurrent integration probe keeps summary/latest p95 below 2 seconds and contract-events p95 below 3 seconds with no request above 5 seconds.
- After a separately approved production deployment, the public operational gate is summary/latest p95 below 2 seconds and contract-events p95 below 3 seconds with p99 below 5 seconds.
- No duplicated failure banner or half-screen blank error panel remains.
- Stale analysis cannot be mistaken for a current risk conclusion.
- The three required desktop viewport widths have no clipping.
- Relevant Rust tests run sequentially in this repository, frontend focused tests pass, and the production frontend build succeeds.
- Work remains read-only/dry-run and does not alter alert eligibility or trading behavior.

## Delivery Boundary

The implementation is completed and verified locally first. Git commit, push, production deployment, and service restart require separate explicit authorization after local evidence is presented. Public latency targets remain an unverified release gate until that authorization is granted.
