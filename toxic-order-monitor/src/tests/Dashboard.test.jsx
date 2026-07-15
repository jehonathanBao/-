import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mockSignals } from "../data/mockSignals.js";
import Dashboard from "../pages/Dashboard.jsx";
import { useSignalsStore } from "../store/signalsStore.js";

vi.mock("../api/signals.js", async () => {
  const { mockSignals } = await import("../data/mockSignals.js");
  return {
    fetchSignals: vi.fn(() => Promise.resolve(mockSignals)),
  };
});

vi.mock("../api/scanLogs.js", async () => {
  const actual = await vi.importActual("../api/scanLogs.js");
  return {
    ...actual,
    fetchScanLogs: vi.fn(() => Promise.resolve([])),
  };
});

vi.mock("../api/contractWhale.js", async () => import("./__mocks__/contractWhale.js"));

vi.mock("../api/liquidationCascade.js", () => ({
  fetchLiquidationCascade: vi.fn(() =>
    Promise.resolve({
      data: {
        symbol: "BTCUSDT",
        cascadeProbability: 0.82,
        status: "IMMINENT",
        direction: "DOWN",
        estimatedMove: "2.5% - 5%",
        timeWindow: "5m - 30m",
        riskZone: [65500, 66000],
        signals: ["OI_CLUSTER_HIGH", "LIQUIDITY_VOID"],
        components: {
          leverageConcentration: 0.78,
          liquidityGap: 0.66,
          fundingStress: 0.42,
          triggerProximity: 0.71,
          oiStress: 0.57,
        },
      },
      error: null,
    }),
  ),
  fetchLiquidationLeverageMap: vi.fn(() =>
    Promise.resolve({
      data: {
        symbol: "BTCUSDT",
        heatmap: [
          { price: 65500, side: "long", intensity: 0.84, notionalUsd: 120_000_000, distanceBps: 38 },
        ],
        highRiskZones: [{ low: 65500, high: 66000, strength: 0.82, side: "long" }],
      },
      error: null,
    }),
  ),
  fetchLiquidationLiquidityGap: vi.fn(() =>
    Promise.resolve({
      data: { symbol: "BTCUSDT", belowPrice: 0.68, abovePrice: 0.44, dominantGap: "DOWN", signals: ["THIN_BID"] },
      error: null,
    }),
  ),
  fetchBtcStructure: vi.fn(() =>
    Promise.resolve({
      data: {
        symbol: "BTC",
        regime: "LIQUIDATION",
        bias: "SHORT",
        confidence: 0.76,
        structureScore: 0.64,
        liquidationCascadeProbability: 0.82,
        gammaPressure: 0.33,
        signals: ["BTC_STRUCTURE_ONLY", "LIQUIDATION_IMMINENT"],
        metrics: {
          liquidationPressure: 0.82,
          gammaPressure: 0.33,
        },
      },
      error: null,
    }),
  ),
  fetchMarketRegime: vi.fn(() =>
    Promise.resolve({
      data: {
        symbol: "BTCUSDT",
        regime: "LIQUIDATION",
        confidence: 0.76,
        directionBias: "SHORT",
        signals: ["FUNDING_STRESS"],
      },
      error: null,
    }),
  ),
}));

vi.mock("../api/usageGuide.js", () => ({
  fetchUsageGuide: vi.fn(() =>
    Promise.resolve({
      markdown: [
        "# 有毒订单监控用户使用指南",
        "",
        "## 1. 先记住一句话",
        "",
        "页面里所有 `Candidate` 都表示候选信号。",
        "",
        "## 3. 当前有毒订单判断逻辑",
        "",
        "Candidate only，系统只做盘口 / L2 / 成交异常提醒。",
        "",
        "## 7. 合约监控信号怎么解读",
        "",
        "主力拉盘表示合约主动买入成交突然放大。",
        "",
        "## 9. Discord 状态怎么理解",
        "",
        "`cooldown` 表示同方向短时间内已经推过。",
      ].join("\n"),
      readOnly: true,
      sourcePath: "docs/usage-guide.md",
      title: "有毒订单监控用户使用指南",
    }),
  ),
}));

vi.mock("../hooks/useReconnectingWebSocket.js", () => ({
  useReconnectingWebSocket: vi.fn(() => ({ status: "idle", socket: null })),
}));

vi.mock("echarts/core", () => ({
  use: vi.fn(),
  init: vi.fn(() => ({
    setOption: vi.fn(),
    resize: vi.fn(),
    dispose: vi.fn(),
  })),
}));

vi.mock("echarts/charts", () => ({
  BarChart: {},
  PieChart: {},
}));

vi.mock("echarts/components", () => ({
  GridComponent: {},
  LegendComponent: {},
  TooltipComponent: {},
}));

vi.mock("echarts/renderers", () => ({
  CanvasRenderer: {},
}));

describe("Dashboard interactions", () => {
  beforeEach(() => {
    resetSignalsStore();
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
  });

  it("shows signals by default", async () => {
    renderDashboard();

    expect(await screen.findAllByText("BTCUSDT", { exact: false })).not.toHaveLength(0);
  });

  it("shows high-risk candidates by default and keeps medium risk collapsed", async () => {
    renderDashboard();

    expect(await screen.findByText("主力合约监控")).toBeInTheDocument();
    expect(screen.getByText("短线有毒订单评分")).toBeInTheDocument();
    expect(screen.getByText("现货 + 合约主力结构评分")).toBeInTheDocument();
    expect(await screen.findByTestId("signal-card-sig_001")).toBeInTheDocument();
    expect(screen.queryByTestId("signal-card-sig_003")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Medium Risk Candidates/ })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("expands and hides medium-risk candidates from the foldable section", async () => {
    const user = userEvent.setup();
    renderDashboard();

    const toggle = screen.getByRole("button", { name: /Medium Risk Candidates/ });
    await user.click(toggle);

    expect(await screen.findByTestId("signal-card-sig_003")).toBeInTheDocument();
    expect(toggle).toHaveAttribute("aria-expanded", "true");

    await user.click(toggle);

    expect(screen.queryByTestId("signal-card-sig_003")).not.toBeInTheDocument();
    expect(toggle).toHaveAttribute("aria-expanded", "false");
  });

  it("keeps medium-risk candidates display-only for Discord", async () => {
    const user = userEvent.setup();
    renderDashboard();

    await user.click(screen.getByRole("button", { name: /Medium Risk Candidates/ }));
    const button = await screen.findByRole("button", { name: /推送 sig_003 到 Discord/ });

    expect(button).toHaveTextContent("仅页面展示");
    expect(button).toBeDisabled();
  });

  it("filters high risk signals", async () => {
    const user = userEvent.setup();
    renderDashboard();

    await user.click(screen.getByRole("button", { name: /筛选 high 风险/ }));

    expect(screen.getByText(/当前筛选：high 风险/)).toBeInTheDocument();
  });

  it("keeps all mode as high-risk primary list with medium foldout available", async () => {
    const user = userEvent.setup();
    renderDashboard();

    await user.click(screen.getByRole("button", { name: /筛选 high 风险/ }));
    await user.click(screen.getByRole("button", { name: /筛选 all 风险/ }));

    expect(screen.getByText(/当前筛选：高风险主列表/)).toBeInTheDocument();
    expect(screen.getByTestId("signal-card-sig_001")).toBeInTheDocument();
    expect(screen.queryByTestId("signal-card-sig_003")).not.toBeInTheDocument();
  });

  it("opens the BTC contract monitor from the dedicated sidebar route", async () => {
    renderDashboard("/contract-whale/btc");

    expect(screen.getByTestId("workspace-main")).toHaveClass("contract-workspace-main");
    expect(screen.getByTestId("workspace-sidebar")).toBeInTheDocument();
    expect(screen.queryByText("盘口异常监控大屏")).not.toBeInTheDocument();
    expect(screen.getByRole("navigation", { name: "主导航" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "监控首页" })).toHaveAttribute("href", "/dashboard");
    expect(screen.getByRole("link", { name: "BTC 合约监控" })).toHaveAttribute("href", "/contract-whale/btc");
    expect(screen.getByRole("link", { name: "ETH 合约监控" })).toHaveAttribute("href", "/contract-whale/eth");
    expect(screen.queryByRole("link", { name: "BTC/ETH 合约监控" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "强平瀑布预测" })).not.toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "妖币控盘监控" })).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "山寨合约异常" })).toHaveAttribute("href", "/alt-contract-monitor");
    expect(screen.getByRole("link", { name: "新币合约监控" })).toHaveAttribute("href", "/new-token-watch");
    expect(screen.getByRole("link", { name: "BTC 现货监控" })).toHaveAttribute("href", "/spot-monitor/btc");
    expect(screen.getByRole("link", { name: "ETH 现货监控" })).toHaveAttribute("href", "/spot-monitor/eth");
    expect(screen.queryByRole("link", { name: "BTC/ETH 现货监控" })).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "使用指南" })).toHaveAttribute("href", "/usage-guide");
    expect((await screen.findAllByText("BTC 合约监控")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("BTC CONTRACT WHALE FLOW").length).toBeGreaterThan(0);
    expect(screen.getByText(/只读提醒/)).toBeInTheDocument();
    expect(screen.getAllByText("主力合约监控未启用").length).toBeGreaterThan(0);
    expect(screen.queryByText("High / Critical Risk Candidates")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Medium Risk Candidates/ })).not.toBeInTheDocument();
  });

  it("opens the ETH contract monitor as an isolated mainstream route", async () => {
    renderDashboard("/contract-whale/eth");

    expect((await screen.findAllByText("ETH 合约监控")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("ETH CONTRACT WHALE FLOW").length).toBeGreaterThan(0);
    expect(screen.getByText("币种：ETH（当前页面固定）")).toBeInTheDocument();
    expect(screen.queryByLabelText("币种")).not.toBeInTheDocument();
    expect(screen.queryByText("SOL")).not.toBeInTheDocument();
  });

  it("opens the standalone liquidation cascade predictor route", async () => {
    renderDashboard("/liquidation-cascade");

    expect(screen.queryByRole("link", { name: "强平瀑布预测" })).not.toBeInTheDocument();
    expect((await screen.findAllByText("强平瀑布预测")).length).toBeGreaterThan(0);
    expect(await screen.findByText("IMMINENT")).toBeInTheDocument();
    expect(screen.getAllByText("82%").length).toBeGreaterThan(0);
    expect(screen.getByText("2.5% - 5%")).toBeInTheDocument();
    expect(screen.getAllByText("BTC_STRUCTURE_ONLY").length).toBeGreaterThan(0);
    expect(screen.queryByText("mean_reversion_only")).not.toBeInTheDocument();
    expect(screen.queryByText("High / Critical Risk Candidates")).not.toBeInTheDocument();
  });

  it("does not expose the removed altcoin manipulation page", () => {
    renderDashboard("/dashboard");

    expect(screen.queryByRole("link", { name: "妖币控盘监控" })).not.toBeInTheDocument();
    expect(screen.queryByText("妖币控盘监控")).not.toBeInTheDocument();
  });

  it("keeps dashboard and BTC spot monitor route working", async () => {
    renderDashboard("/dashboard");

    expect(await screen.findByText("High / Critical Risk Candidates")).toBeInTheDocument();
    cleanup();
    resetSignalsStore();

    renderDashboard("/spot-monitor/btc");

    expect((await screen.findAllByText("BTC 现货监控")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("BTC SPOT WHALE FLOW").length).toBeGreaterThan(0);
    expect(screen.getByText("币种：BTC（当前页面固定）")).toBeInTheDocument();
    expect(screen.queryByText("SOL")).not.toBeInTheDocument();
    expect(screen.queryByText("High / Critical Risk Candidates")).not.toBeInTheDocument();
  });

  it("opens the ETH spot monitor as an isolated mainstream route", async () => {
    renderDashboard("/spot-monitor/eth");

    expect((await screen.findAllByText("ETH 现货监控")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("ETH SPOT WHALE FLOW").length).toBeGreaterThan(0);
    expect(screen.getByText("币种：ETH（当前页面固定）")).toBeInTheDocument();
    expect(screen.queryByLabelText("币种")).not.toBeInTheDocument();
    expect(screen.queryByText("SOL")).not.toBeInTheDocument();
  });

  it("places S level candidates in the sidebar signals view", async () => {
    renderDashboard("/signals");

    expect(await screen.findByText("S 级异常信号")).toBeInTheDocument();
    expect(screen.getByText(/当前筛选：异常信号：S 级/)).toBeInTheDocument();
    expect(screen.getByTestId("signal-card-sig_001")).toBeInTheDocument();
    expect(screen.getByTestId("signal-card-sig_007")).toBeInTheDocument();
    expect(screen.queryByTestId("signal-card-sig_002")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Medium Risk Candidates/ })).not.toBeInTheDocument();
  });

  it("places medium-risk candidates in the signal history view", async () => {
    renderDashboard("/history");

    expect(await screen.findByText("信号历史 · 中风险异常")).toBeInTheDocument();
    expect(screen.getByText(/当前筛选：信号历史：中风险异常/)).toBeInTheDocument();
    expect(screen.getByTestId("signal-card-sig_003")).toBeInTheDocument();
    expect(screen.getByTestId("signal-card-sig_005")).toBeInTheDocument();
    expect(screen.queryByTestId("signal-card-sig_001")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Medium Risk Candidates/ })).not.toBeInTheDocument();
  });

  it("shows the current toxic-order decision logic in the warning rules view", async () => {
    renderDashboard("/rules");

    expect(await screen.findByText("判断逻辑已移至使用指南")).toBeInTheDocument();
    expect(screen.getByText(/系统只做盘口 \/ 成交异常提醒/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "打开使用指南" })).toHaveAttribute("href", "/usage-guide");
    expect(screen.queryByText(/短线有毒订单评分 toxicScore/)).not.toBeInTheDocument();
  });

  it("shows the user usage guide from the docs markdown file", async () => {
    renderDashboard("/usage-guide");

    expect(await screen.findByText("用户使用指南")).toBeInTheDocument();
    expect(screen.getByText("1. 先记住一句话")).toBeInTheDocument();
    expect(
      screen.getByText((_, element) =>
        element?.tagName === "P" && element.textContent.includes("页面里所有 Candidate 都表示候选信号。"),
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("3. 当前有毒订单判断逻辑")).toBeInTheDocument();
    expect(screen.getByText("7. 合约监控信号怎么解读")).toBeInTheDocument();
    expect(screen.getByText("9. Discord 状态怎么理解")).toBeInTheDocument();
    expect(screen.queryByText("High / Critical Risk Candidates")).not.toBeInTheDocument();
  });

  it("shows the detail panel after selecting replay", async () => {
    const user = userEvent.setup();
    renderDashboard();

    await user.click(screen.getByRole("button", { name: /查看回放 sig_002/ }));

    await waitFor(() => expect(screen.getAllByText("ETHUSDT", { exact: false })).not.toHaveLength(0));
  });
});

function renderDashboard(path = "/") {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Dashboard />
    </MemoryRouter>,
  );
}

function resetSignalsStore() {
  const firstHighRiskSignal = mockSignals.find((signal) => signal.risk === "high") ?? mockSignals[0];
  useSignalsStore.setState({
    rawInboxSignals: mockSignals,
    signals: mockSignals,
    selectedSignal: firstHighRiskSignal,
    activeRiskFilter: "high",
    pushStatus: {},
    storageWarning: null,
    pushLogs: [],
    discordConnected: false,
    lastPushedAt: null,
    clearedAtMs: 0,
    clearedSignalKeys: [],
  });
}
