import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";
import Dashboard from "../pages/Dashboard.jsx";
import App from "../App.jsx";

vi.mock("../api/signals.js", () => ({
  fetchSignals: vi.fn(() => Promise.resolve([])),
  mapInboxItemToSignal: vi.fn((item) => item),
}));

vi.mock("../api/discord.js", () => ({
  pushDiscordAlert: vi.fn(),
  sendDiscordTestMessage: vi.fn(),
}));

vi.mock("../hooks/useReconnectingWebSocket.js", () => ({
  useReconnectingWebSocket: vi.fn(() => ({ status: "idle", socket: null })),
}));

vi.mock("../components/BinanceAltContractMonitor.jsx", () => ({
  default: () => <div data-testid="alt-contract-monitor-probe">Alt contract monitor</div>,
}));

vi.mock("../components/ContractWhaleMonitor.jsx", () => ({
  default: () => <div data-testid="contract-monitor-probe">Contract monitor</div>,
}));

vi.mock("../components/LiquidationCascadeDashboard.jsx", () => ({
  default: () => <div data-testid="liquidation-monitor-probe">Liquidation monitor</div>,
}));

vi.mock("../components/NewTokenWatch.jsx", () => ({
  default: () => <div data-testid="new-token-monitor-probe">New token monitor</div>,
}));

vi.mock("../components/PushLog.jsx", () => ({
  default: () => <div data-testid="push-log-probe">Push log</div>,
}));

vi.mock("../components/RiskCard.jsx", () => ({
  default: ({ risk }) => <button type="button">Risk {risk}</button>,
}));

vi.mock("../components/RiskCharts.jsx", () => ({
  default: () => <div data-testid="risk-charts-probe">Risk charts</div>,
}));

vi.mock("../components/RiskSystemSummaryCards.jsx", () => ({
  default: () => <div data-testid="risk-summary-probe">Risk summary</div>,
}));

vi.mock("../components/RuleStatus.jsx", () => ({
  default: () => <div data-testid="rule-status-probe">Rule status</div>,
}));

vi.mock("../components/ScanLogPanel.jsx", () => ({
  default: () => <div data-testid="scan-log-probe">Scan log</div>,
}));

vi.mock("../components/SignalDetail.jsx", () => ({
  default: () => <div data-testid="signal-detail-probe">Signal detail</div>,
}));

vi.mock("../components/SignalTable.jsx", () => ({
  default: ({ title }) => <div data-testid="signal-table-probe">{title}</div>,
}));

vi.mock("../components/SpotWhaleMonitor.jsx", () => ({
  default: () => <div data-testid="spot-monitor-probe">Spot monitor</div>,
}));

vi.mock("../components/UsageGuide.jsx", () => ({
  default: () => <div data-testid="usage-guide-probe">Usage guide</div>,
}));

const ROUTES = [
  ["/dashboard", "监控首页", false],
  ["/contract-whale/btc", "BTC 合约监控", true],
  ["/contract-whale/eth", "ETH 合约监控", true],
  ["/spot-monitor/btc", "BTC 现货监控", false],
  ["/spot-monitor/eth", "ETH 现货监控", false],
  ["/liquidation-cascade", null, false],
  ["/alt-contract-monitor", "山寨合约异常", false],
  ["/new-token-watch", "新币合约监控", false],
  ["/signals", "异常信号", false],
  ["/history", "信号历史", false],
  ["/rules", "告警规则", false],
  ["/usage-guide", "使用指南", false],
  ["/discord", "Discord 设置", false],
  ["/settings", "系统设置", false],
];

const PAGE_INTROS = [
  ["/spot-monitor/btc", "BTC 现货监控"],
  ["/spot-monitor/eth", "ETH 现货监控"],
  ["/liquidation-cascade", "强平瀑布预测"],
  ["/alt-contract-monitor", "山寨合约异常监控"],
  ["/new-token-watch", "新币合约监控"],
  ["/usage-guide", "用户使用指南"],
];

describe("Unified workspace shell", () => {
  afterEach(() => {
    cleanup();
    window.localStorage.clear();
  });

  it.each(ROUTES)("uses the unified workspace on %s", (path, activeLabel, contractRoute) => {
    renderDashboard(path);

    expect(screen.getByTestId("workspace-shell")).toHaveClass("workspace-shell");
    expect(screen.getByTestId("workspace-main")).toHaveClass("workspace-main");
    expect(screen.getByTestId("workspace-sidebar")).toBeInTheDocument();
    expect(screen.getByText("READ ONLY")).toBeInTheDocument();

    if (activeLabel) {
      expect(screen.getByRole("link", { name: activeLabel })).toHaveAttribute("aria-current", "page");
    }

    if (contractRoute) {
      expect(screen.queryByTestId("workspace-command-header")).not.toBeInTheDocument();
      expect(screen.getByTestId("contract-monitor-probe")).toBeInTheDocument();
    } else {
      expect(screen.getByTestId("workspace-command-header")).toBeInTheDocument();
    }
  });

  it.each(PAGE_INTROS)("uses the compact page intro on %s", (path, title) => {
    renderDashboard(path);

    expect(screen.getByTestId("workspace-page-header")).toHaveTextContent(title);
  });

  it.each([
    ["/", null, false],
    ["/contract-whale", "BTC 合约监控", true],
    ["/spot-monitor", "BTC 现货监控", false],
    ["/spot-whale", "BTC 现货监控", false],
  ])("keeps the unified workspace after resolving alias %s", async (path, activeLabel, contractRoute) => {
    renderApp(path);

    expect(await screen.findByTestId("workspace-shell")).toBeInTheDocument();
    expect(screen.getByTestId("workspace-main")).toBeInTheDocument();
    expect(screen.getByTestId("workspace-sidebar")).toBeInTheDocument();

    if (activeLabel) {
      expect(screen.getByRole("link", { name: activeLabel })).toHaveAttribute("aria-current", "page");
    }

    if (contractRoute) {
      expect(screen.queryByTestId("workspace-command-header")).not.toBeInTheDocument();
    } else {
      expect(screen.getByTestId("workspace-command-header")).toBeInTheDocument();
    }
  });
});

function renderDashboard(path) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <Dashboard />
    </MemoryRouter>,
  );
}

function renderApp(path) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <App />
    </MemoryRouter>,
  );
}
