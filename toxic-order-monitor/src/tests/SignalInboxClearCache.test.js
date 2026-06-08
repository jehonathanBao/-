import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mockSignals } from "../data/mockSignals.js";
import Dashboard from "../pages/Dashboard.jsx";
import { SIGNAL_INBOX_STORAGE_KEY, useSignalsStore } from "../store/signalsStore.js";

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
  fetchContractWhaleLatest: vi.fn(() =>
    Promise.resolve({
      summary: {
        status: "平静",
        direction: "neutral",
        latestSeverity: "calm",
        latestPushedAtMs: null,
        signalCount: 0,
        readOnly: true,
      },
      items: [],
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

describe("Signal inbox clear cache", () => {
  beforeEach(() => {
    resetStore();
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    window.localStorage.clear();
  });

  it("clears the page cache after confirmation", async () => {
    const user = userEvent.setup();
    vi.spyOn(window, "confirm").mockReturnValue(true);
    renderDashboard();

    await user.click(screen.getByRole("button", { name: "清除缓存" }));

    await waitFor(() => expect(screen.getByText("暂无缓存的有毒订单候选信号")).toBeInTheDocument());
    expect(screen.getByText("新的候选信号出现后会继续追加")).toBeInTheDocument();
    expect(useSignalsStore.getState().rawInboxSignals).toHaveLength(0);
  });

  it("does not re-add cleared backend signals with the same dedupe keys", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    useSignalsStore.getState().clearSignalInbox();

    useSignalsStore.getState().setSignals(mockSignals);

    expect(useSignalsStore.getState().rawInboxSignals).toHaveLength(0);
  });

  it("blocks cleared old keys while allowing new candidate keys after clear", () => {
    const newSignal = {
      ...mockSignals[1],
      id: "sig_after_clear_new_key",
      dedupeKey: "okx:ETHUSDT:after-clear:new-key",
    };

    useSignalsStore.getState().clearSignalInbox();
    useSignalsStore.getState().setSignals([mockSignals[0]]);

    expect(useSignalsStore.getState().rawInboxSignals).toHaveLength(0);

    useSignalsStore.getState().setSignals([newSignal]);

    const state = useSignalsStore.getState();
    expect(state.rawInboxSignals).toHaveLength(1);
    expect(state.rawInboxSignals[0].id).toBe("sig_after_clear_new_key");
  });

  it("keeps the inbox empty after clear, reload, and old backend response", async () => {
    useSignalsStore.getState().clearSignalInbox();

    const saved = JSON.parse(window.localStorage.getItem(SIGNAL_INBOX_STORAGE_KEY));
    expect(saved.rawInboxSignals).toEqual([]);
    expect(saved.clearedSignalKeys.length).toBeGreaterThan(0);

    vi.resetModules();
    const freshStore = await import("../store/signalsStore.js");

    expect(freshStore.useSignalsStore.getState().rawInboxSignals).toHaveLength(0);
    expect(freshStore.useSignalsStore.getState().clearedSignalKeys.length).toBeGreaterThan(0);

    freshStore.useSignalsStore.getState().setSignals(mockSignals);

    expect(freshStore.useSignalsStore.getState().rawInboxSignals).toHaveLength(0);
    expect(freshStore.useSignalsStore.getState().clearedSignalKeys.length).toBeGreaterThan(0);
  });

  it("restores cached inbox signals from localStorage on module reload", async () => {
    window.localStorage.setItem(
      SIGNAL_INBOX_STORAGE_KEY,
      JSON.stringify({
        rawInboxSignals: [mockSignals[1]],
        clearedAtMs: 0,
        clearedSignalKeys: [],
      }),
    );

    vi.resetModules();
    const freshStore = await import("../store/signalsStore.js");

    expect(freshStore.useSignalsStore.getState().rawInboxSignals).toHaveLength(1);
    expect(freshStore.useSignalsStore.getState().rawInboxSignals[0].id).toBe("sig_002");
  });

  it("keeps memory state when localStorage writes fail", () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("quota exceeded");
    });

    expect(() => useSignalsStore.getState().clearSignalInbox()).not.toThrow();

    const state = useSignalsStore.getState();
    expect(state.rawInboxSignals).toHaveLength(0);
    expect(state.storageWarning).toBe("LOCAL_STORAGE_WRITE_FAILED");
  });
});

function renderDashboard() {
  return render(
    React.createElement(
      MemoryRouter,
      null,
      React.createElement(Dashboard),
    ),
  );
}

function resetStore() {
  useSignalsStore.setState({
    rawInboxSignals: mockSignals,
    signals: mockSignals,
    selectedSignal: mockSignals[0],
    activeRiskFilter: "all",
    pushStatus: {},
    storageWarning: null,
    pushLogs: [],
    discordConnected: false,
    lastPushedAt: null,
    clearedAtMs: 0,
    clearedSignalKeys: [],
  });
}
