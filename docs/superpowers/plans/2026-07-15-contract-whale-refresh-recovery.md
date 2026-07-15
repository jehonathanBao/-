# Contract Whale Refresh Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `/contract-whale/btc` survive a transient public-path asset or API failure without showing a blank screen or leaving the event feed unavailable after the next successful request.

**Architecture:** Keep the existing read-only React/Vite/nginx stack. Add an HTML-level boot shell and one guarded reload for failed hashed assets, prevent cached SPA HTML from referencing removed build hashes, then make the React data loader retry status and history slices once at a short interval while keeping first-load history requests compact until a usable response arrives.

**Tech Stack:** React 19, Vite 7, Vitest 4, jsdom, nginx.

## Global Constraints

- Preserve read-only monitoring and existing Discord/trading safety gates.
- Do not change backend business rules, thresholds, live execution, auth, or secret handling.
- Do not commit, push, deploy, or restart services without explicit user authorization.
- Keep changes small and limited to frontend boot/loading behavior plus regression tests.

---

### Task 1: Recover from a failed hashed frontend asset

**Files:**
- Modify: `toxic-order-monitor/src/tests/FrontendDeployment.test.js`
- Modify: `toxic-order-monitor/index.html`
- Modify: `toxic-order-monitor/src/main.jsx`
- Modify: `toxic-order-monitor/nginx.conf.template`

**Interfaces:**
- Consumes: Vite's generated `/assets/*` entry and preload URLs.
- Produces: `window.__toxicOrderMonitorMarkBooted()`, a static `[data-bootstrap-shell]`, and a no-store response for `index.html`.

- [ ] **Step 1: Write the failing deployment/boot tests**

```js
it("keeps a visible boot shell and arms one guarded retry for failed build assets", () => {
  const html = readFile("toxic-order-monitor/index.html");
  expect(html).toContain("data-bootstrap-shell");
  expect(html).toContain("data-bootstrap-recovery");
  expect(html).toContain("toxic-order-monitor.boot-retry.v1");
  expect(html).toContain("__toxicOrderMonitorMarkBooted");
});

it("does not cache the SPA entry document across hashed deployments", () => {
  const nginxConfig = readFile("toxic-order-monitor/nginx.conf.template");
  expect(nginxConfig).toContain("location = /index.html");
  expect(nginxConfig).toContain('Cache-Control "no-cache, no-store, must-revalidate"');
});
```

- [ ] **Step 2: Run the test and verify RED**

Run: `npm test -- src/tests/FrontendDeployment.test.js`

Expected: FAIL because the boot recovery markers and no-store index location do not exist.

- [ ] **Step 3: Add the minimal HTML/nginx boot recovery**

Add a static shell inside `#root`. Install a capture-phase `error` listener before the Vite entry script; only react to failed `/assets/` script/link nodes, show a recovery message, store one session retry marker, and schedule one reload. Expose `window.__toxicOrderMonitorMarkBooted()` so `src/main.jsx` cancels the reload and clears the marker once the entry module executes. Add an exact nginx `location = /index.html` with `Cache-Control: no-cache, no-store, must-revalidate`.

- [ ] **Step 4: Run the test and verify GREEN**

Run: `npm test -- src/tests/FrontendDeployment.test.js`

Expected: PASS.

### Task 2: Recover the status and historical slices after transient API failure

**Files:**
- Modify: `toxic-order-monitor/src/tests/ContractWhaleMonitor.test.jsx`
- Modify: `toxic-order-monitor/src/components/ContractWhaleMonitor.jsx`

**Interfaces:**
- Consumes: existing API payload `dataState`, `error`, and `retryAfterMs` fields.
- Produces: one bounded short retry for status/event groups and a compact `limit: 20` first history request for BTC and ETH until the first usable history response.

- [ ] **Step 1: Write failing refresh recovery tests**

```jsx
it("retries an unavailable status slice after two seconds", async () => {
  fetchContractWhaleSummary.mockResolvedValueOnce({ summary: null, meta: null, error: "summary_unavailable" });
  fetchContractWhaleLatest.mockResolvedValueOnce({ items: [], dataState: "unavailable", error: "latest_unavailable" });
  vi.useFakeTimers();
  render(<ContractWhaleMonitor />);
  await vi.advanceTimersByTimeAsync(1_999);
  expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(1);
  await vi.advanceTimersByTimeAsync(1);
  await vi.advanceTimersByTimeAsync(0);
  expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(2);
});

it("keeps the compact BTC history request after a transient first failure", async () => {
  fetchContractEvents.mockResolvedValueOnce({ items: [], dataState: "unavailable", error: "contract_events_unavailable" });
  vi.useFakeTimers();
  render(<ContractWhaleMonitor />);
  await vi.advanceTimersByTimeAsync(2_000);
  expect(fetchContractEvents).toHaveBeenNthCalledWith(1, expect.objectContaining({ symbol: "BTC", limit: 20 }));
  expect(fetchContractEvents).toHaveBeenNthCalledWith(2, expect.objectContaining({ symbol: "BTC", limit: 20 }));
});
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run: `npm test -- src/tests/ContractWhaleMonitor.test.jsx`

Expected: FAIL because status has no two-second recovery timer and BTC first-load history uses 50 rows.

- [ ] **Step 3: Implement the bounded retry and compact-first behavior**

Add a status retry timer to the existing effect lifecycle. Generalize the retry-delay calculation so it is capped by each group's normal interval. Schedule at most one short retry, clear it on success/unmount/visibility changes, use `20` for the first BTC and ETH history request, and set `initialEventViewPending = false` only after `isUsableDataPayload(contractEventsPayload)`.

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run: `npm test -- src/tests/ContractWhaleMonitor.test.jsx src/tests/FrontendDeployment.test.js`

Expected: PASS.

### Task 3: Verify the complete frontend and actual refresh flow

**Files:**
- Verify only; no additional files expected.

**Interfaces:**
- Consumes: built Vite assets and the existing public read-only page/API.
- Produces: fresh test/build/browser evidence.

- [ ] **Step 1: Run frontend validation**

```powershell
Set-Location 'D:\DevWorkspaces\Documents\有毒订单监控-rs\toxic-order-monitor'
npm test
npm run build
```

Expected: all tests pass and the production build exits 0. This package has no lint script, so do not invent one.

- [ ] **Step 2: Inspect the diff and safety boundary**

Run: `git diff --check` and `git status --short`.

Expected: only the plan, boot/loading frontend files, and their tests are modified; no `.env`, tokens, runtime data, backend rules, deploy execution, or generated `dist` output is included.

- [ ] **Step 3: Verify repeated refreshes locally or against the built preview**

Run repeated browser reloads and confirm:

- `#root > *` becomes visible on every attempt;
- `#contract-whale-events` appears and leaves the loading/unavailable state after a successful response;
- no application-origin console error is recorded;
- the static shell is visible rather than an empty dark page if a hashed asset is intentionally made unavailable in a controlled test.

- [ ] **Step 4: Report without deploying**

Report the root cause evidence, exact files changed, test/build counts, refresh evidence, and the fact that server deployment/restart remains pending explicit authorization.
