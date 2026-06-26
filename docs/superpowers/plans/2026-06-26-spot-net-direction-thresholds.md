# Spot Net Direction Thresholds Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `abs50` and `abs100` net-direction filters to the BTC/ETH spot monitor and keep frontend plus backend filter semantics aligned.

**Architecture:** Extend the existing spot monitor filter contract rather than inventing a new parameter shape. The frontend select, frontend history request tests, and backend `net_direction` parser all continue to speak the same `absNNN` vocabulary, while the UI's local absolute-net-direction filter keeps matching the selected threshold.

**Tech Stack:** React, Vitest, Axios, Rust, Axum

## Global Constraints

- Keep the existing `net_direction` query parameter and extend it conservatively.
- Preserve absolute-value semantics: positive and negative net direction both count toward the threshold.
- Do not change unrelated spot monitor filters, routing, or persistence behavior.
- Follow TDD: write failing tests first, verify they fail, then implement minimal code.

---

### Task 1: Add failing API and component tests for `abs50` and `abs100`

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/SpotWhaleApi.test.js`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/tests/SpotWhaleMonitor.test.jsx`

**Interfaces:**
- Consumes: `fetchSpotWhaleHistory(filters)` from `toxic-order-monitor/src/api/spotWhale.js`
- Produces: test coverage proving `abs50` and `abs100` must be accepted by frontend request construction and by the visible net-direction filter UI

- [ ] **Step 1: Write the failing API test**

```js
  it("passes abs50 and abs100 net direction filters to history endpoint", async () => {
    axios.get.mockResolvedValue({
      data: {
        summary: { enabled: true, symbol: "BTC" },
        items: [],
      },
    });

    await fetchSpotWhaleHistory({ symbol: "BTC", net_direction: "abs50", limit: 50 });
    await fetchSpotWhaleHistory({ symbol: "BTC", net_direction: "abs100", limit: 50 });

    expect(axios.get.mock.calls[0][0]).toContain("net_direction=abs50");
    expect(axios.get.mock.calls[1][0]).toContain("net_direction=abs100");
  });
```

- [ ] **Step 2: Write the failing component test**

```jsx
  it("supports abs50 and abs100 net-direction options in the spot filter", async () => {
    const user = userEvent.setup();
    render(<SpotWhaleMonitor />);

    await screen.findByLabelText("净方向");

    fetchSpotWhaleHistory.mockResolvedValueOnce({
      summary: { enabled: true, dryRun: false, symbol: "BTC", exchanges: {} },
      items: [
        {
          id: "spot-whale-negative-100-BTC",
          ts: 1_700_000_000_002,
          symbol: "BTC",
          windowSec: 15,
          signalType: "spot_aggressive_sell",
          direction: "sell",
          severity: "medium",
          score: 80,
          totalVolumeBase: 150,
          netVolumeBase: -100,
          totalNotionalUsd: 6_000_000,
          dominance: 0.67,
          priceMovePct: -0.05,
          coinbasePremiumPct: 0,
          mainExchange: "binance",
          dataQuality: 88,
          discordEligible: false,
          discordSent: false,
          exchanges: [],
          finalResult: "spot sell pressure",
        },
      ],
      error: null,
    });

    await user.selectOptions(screen.getByLabelText("净方向"), "abs100");

    expect(fetchSpotWhaleHistory).toHaveBeenCalledWith(
      expect.objectContaining({ limit: 50, net_direction: "abs100", symbol: "BTC" }),
    );
    expect(await screen.findByTestId("spot-whale-row-spot-whale-negative-100-BTC")).toBeInTheDocument();
    expect(screen.getByText("-100 BTC")).toBeInTheDocument();
  });
```

- [ ] **Step 3: Run tests to verify they fail**

Run:
```bash
npm test -- --run src/tests/SpotWhaleApi.test.js src/tests/SpotWhaleMonitor.test.jsx
```

Expected: FAIL because `abs50` / `abs100` are not yet present in the component options and the new assertions have no implementation support.

### Task 2: Implement the new thresholds in frontend and backend

**Files:**
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/toxic-order-monitor/src/components/SpotWhaleMonitor.jsx`
- Modify: `D:/DevWorkspaces/Documents/有毒订单监控-rs/src/api/spot_whale_routes.rs`

**Interfaces:**
- Consumes: `filters.net_direction` values from the spot monitor UI
- Produces: accepted backend values `abs50`, `abs100`, `abs200`, `abs500` and matching UI options

- [ ] **Step 1: Add the new frontend select options**

```jsx
          <option value="all">全部</option>
          <option value="abs50">大于 50（正负）</option>
          <option value="abs100">大于 100（正负）</option>
          <option value="abs200">大于 200（正负）</option>
          <option value="abs500">大于 500（正负）</option>
```

- [ ] **Step 2: Extend backend parser support**

```rust
    match compact.as_str() {
        "abs50" | "gte50" | "min50" | "50" => Ok(Some(50.0)),
        "abs100" | "gte100" | "min100" | "100" => Ok(Some(100.0)),
        "abs200" | "gte200" | "min200" | "200" => Ok(Some(200.0)),
        "abs500" | "gte500" | "min500" | "500" => Ok(Some(500.0)),
        _ => Err(bad_request(
            "invalid_net_direction",
            "net_direction must be all, abs50, abs100, abs200, or abs500",
        )),
    }
```

- [ ] **Step 3: Run the targeted tests**

Run:
```bash
npm test -- --run src/tests/SpotWhaleApi.test.js src/tests/SpotWhaleMonitor.test.jsx
```

Expected: PASS

### Task 3: Verify, sync, and validate live

**Files:**
- Modify: none expected if tests pass cleanly

**Interfaces:**
- Consumes: git working tree plus existing deploy path `/opt/toxic-order-monitor-rs`
- Produces: synced live server with the updated spot filter options

- [ ] **Step 1: Run focused verification**

Run:
```bash
cargo check
npm run build
```

Expected: both commands exit 0

- [ ] **Step 2: Commit and push**

```bash
git add src/api/spot_whale_routes.rs toxic-order-monitor/src/components/SpotWhaleMonitor.jsx toxic-order-monitor/src/tests/SpotWhaleApi.test.js toxic-order-monitor/src/tests/SpotWhaleMonitor.test.jsx docs/superpowers/plans/2026-06-26-spot-net-direction-thresholds.md
git commit -m "feat: add lower spot net direction thresholds"
git push origin main
```

- [ ] **Step 3: Sync server and verify**

```bash
ssh -b 192.168.1.229 -i C:\Users\byhdo\.ssh\codex_contabo_tokyo_root root@5.104.80.120 "cd /opt/toxic-order-monitor-rs && git pull --ff-only && docker compose up -d --build frontend backend && docker compose ps && curl -fsS http://127.0.0.1:8000/healthz && echo && curl -fsS http://127.0.0.1:8000/readyz"
```

Expected: containers up, `healthz` returns healthy, `readyz` returns ready
