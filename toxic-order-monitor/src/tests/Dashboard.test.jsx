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

vi.mock("../api/contractWhale.js", () => ({
  fetchContractWhaleSummary: vi.fn(() =>
    Promise.resolve({
      summary: {
        status: "平静",
        healthStatus: "disabled",
        direction: "neutral",
        latestSeverity: "calm",
        latestPushedAtMs: null,
        signalCount: 0,
        readOnly: true,
        enabled: false,
        dryRun: true,
      },
      error: null,
    }),
  ),
  fetchContractWhaleLatest: vi.fn(() =>
    Promise.resolve({
      summary: {
        status: "平静",
        healthStatus: "disabled",
        direction: "neutral",
        latestSeverity: "calm",
        latestPushedAtMs: null,
        signalCount: 0,
        readOnly: true,
        enabled: false,
        dryRun: true,
      },
      items: [],
      error: null,
    }),
  ),
  fetchContractWhaleHistory: vi.fn(() => Promise.resolve({ summary: null, items: [], error: null })),
  fetchContractWhaleEvents: vi.fn(() => Promise.resolve({ items: [], error: null })),
  normalizePlatformStatus: vi.fn((platform) => ({
    key: platform?.platformEnabled ? "active" : "disabled",
    label: platform?.platformEnabled ? "运行中" : "未启用",
    description: "test platform status",
    tone: platform?.platformEnabled ? "emerald" : "slate",
  })),
  normalizeMarketStatus: vi.fn((market) => ({
    key: market?.enabled ? "active" : "disabled",
    label: market?.enabled ? "运行中" : "未启用",
    detail: "test market status",
    tone: market?.enabled ? "emerald" : "slate",
  })),
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
        "## 5. 合约监控信号怎么解读",
        "",
        "主力拉盘表示合约主动买入成交突然放大。",
        "",
        "## 8. Discord 状态怎么理解",
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

  it("opens the BTC giant trade monitor from the sidebar route", async () => {
    renderDashboard("/contract-whale");

    expect(screen.getByRole("link", { name: "BTC/ETH 合约监控" })).toHaveAttribute("href", "/contract-whale");
    expect(screen.getByRole("link", { name: "BTC/ETH 现货监控" })).toHaveAttribute("href", "/spot-whale");
    expect(screen.getByRole("link", { name: "使用指南" })).toHaveAttribute("href", "/usage-guide");
    expect(await screen.findByText("BTC / ETH 合约监控")).toBeInTheDocument();
    expect(screen.getByText(/只读提醒/)).toBeInTheDocument();
    expect(screen.getByText("主力合约监控未启用")).toBeInTheDocument();
    expect(screen.queryByText("High / Critical Risk Candidates")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Medium Risk Candidates/ })).not.toBeInTheDocument();
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

    expect(await screen.findByText("当前有毒订单判断逻辑")).toBeInTheDocument();
    expect(screen.getByText(/短线有毒订单评分 toxicScore/)).toBeInTheDocument();
    expect(screen.getByText(/structureBias 单独表示方向/)).toBeInTheDocument();
    expect(screen.getByText(/CWM 大行情提醒保留独立 gate/)).toBeInTheDocument();
    expect(screen.getByText(/系统只做盘口\/成交异常提醒/)).toBeInTheDocument();
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
    expect(screen.getByText("5. 合约监控信号怎么解读")).toBeInTheDocument();
    expect(screen.getByText("8. Discord 状态怎么理解")).toBeInTheDocument();
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
