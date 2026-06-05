import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { pushDiscordAlert, sendDiscordTestMessage } from "../api/discord.js";
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

vi.mock("../api/discord.js", () => ({
  pushDiscordAlert: vi.fn(),
  sendDiscordTestMessage: vi.fn(),
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

describe("Discord push safety", () => {
  beforeEach(() => {
    resetSignalsStore();
    pushDiscordAlert.mockReset();
    sendDiscordTestMessage.mockReset();
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
    vi.restoreAllMocks();
  });

  it("sends test push through an isolated test payload", async () => {
    const user = userEvent.setup();
    sendDiscordTestMessage.mockResolvedValueOnce({ ok: true });
    renderDashboard();

    await user.click(screen.getByRole("button", { name: /测试 Discord 推送/ }));

    expect(sendDiscordTestMessage).toHaveBeenCalledTimes(1);
    expect(pushDiscordAlert).not.toHaveBeenCalled();
    expect(useSignalsStore.getState().rawInboxSignals.find((signal) => signal.id === "sig_001").status).toBe(
      "unhandled",
    );
    expect(await screen.findByRole("status")).toHaveTextContent("Discord 测试消息发送成功");
  });

  it("requires confirmation before manual signal push", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "confirm").mockReturnValue(false);
    renderDashboard();

    await user.click(await screen.findByRole("button", { name: /推送 sig_001 到 Discord/ }));

    expect(window.confirm).toHaveBeenCalledWith(
      "确认推送该高风险候选信号到 Discord？\n该操作会真实发送到告警频道。",
    );
    expect(pushDiscordAlert).not.toHaveBeenCalled();
  });

  it("disables pending manual push and avoids duplicate clicks", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "confirm").mockReturnValue(true);
    let resolvePush;
    pushDiscordAlert.mockReturnValueOnce(
      new Promise((resolve) => {
        resolvePush = resolve;
      }),
    );
    renderDashboard();

    const button = await screen.findByRole("button", { name: /推送 sig_001 到 Discord/ });
    await user.click(button);

    expect(pushDiscordAlert).toHaveBeenCalledTimes(1);
    expect(button).toBeDisabled();
    expect(button).toHaveTextContent("推送中");

    await user.click(button);
    expect(pushDiscordAlert).toHaveBeenCalledTimes(1);

    resolvePush({ ok: true });
    await waitFor(() => expect(useSignalsStore.getState().lastPushedAt).toBeTruthy());
  });

  it("keeps medium signals display-only and lets high signals push", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "confirm").mockReturnValue(true);
    pushDiscordAlert.mockResolvedValue({ ok: true });
    renderDashboard();

    await user.click(screen.getByRole("button", { name: /Medium Risk Candidates/ }));
    const mediumButton = await screen.findByRole("button", { name: /推送 sig_003 到 Discord/ });
    expect(mediumButton).toHaveTextContent("仅页面展示");
    expect(mediumButton).toBeDisabled();

    await user.click(screen.getByRole("button", { name: /推送 sig_001 到 Discord/ }));
    await waitFor(() => expect(pushDiscordAlert).toHaveBeenCalledTimes(1));
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
