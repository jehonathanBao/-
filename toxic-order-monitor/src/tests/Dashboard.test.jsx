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

  it("shows the detail panel after selecting replay", async () => {
    const user = userEvent.setup();
    renderDashboard();

    await user.click(screen.getByRole("button", { name: /查看回放 sig_002/ }));

    await waitFor(() => expect(screen.getAllByText("ETHUSDT", { exact: false })).not.toHaveLength(0));
  });
});

function renderDashboard() {
  return render(
    <MemoryRouter>
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
