import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Dashboard from "../pages/Dashboard.jsx";
import { useSignalsStore } from "../store/signalsStore.js";

const wsMock = vi.hoisted(() => ({
  options: null,
  status: "open",
}));

vi.mock("../api/signals.js", async () => {
  const actual = await vi.importActual("../api/signals.js");
  return {
    ...actual,
    fetchSignals: vi.fn(() => Promise.resolve([])),
  };
});

vi.mock("../hooks/useReconnectingWebSocket.js", () => ({
  useReconnectingWebSocket: vi.fn((_path, options) => {
    wsMock.options = options;
    return { status: wsMock.status, socket: null };
  }),
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

describe("Dashboard websocket signal stream", () => {
  beforeEach(() => {
    resetSignalsStore();
    wsMock.options = null;
    wsMock.status = "open";
  });

  afterEach(() => {
    cleanup();
    window.localStorage.clear();
  });

  it("merges redacted websocket snapshots into the persistent inbox", async () => {
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );

    await waitFor(() => expect(wsMock.options?.onMessage).toBeTypeOf("function"));
    wsMock.options.onMessage({
      data: JSON.stringify({
        type: "signal_snapshot",
        signals: [
          wsItem({ signalId: "ws-high", severity: "high" }),
          wsItem({ signalId: "ws-medium", severity: "medium" }),
        ],
      }),
    });

    expect(await screen.findByTestId("signal-card-ws-high")).toBeInTheDocument();
    expect(screen.queryByTestId("signal-card-ws-medium")).not.toBeInTheDocument();
    expect(useSignalsStore.getState().rawInboxSignals.map((signal) => signal.id)).toEqual([
      "ws-high",
      "ws-medium",
    ]);
  });

  it("dedupes repeated websocket ids and keeps medium folded", async () => {
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );

    await waitFor(() => expect(wsMock.options?.onMessage).toBeTypeOf("function"));
    wsMock.options.onMessage({
      data: JSON.stringify({
        type: "signal_snapshot",
        signals: [
          wsItem({ signalId: "ws-duplicate", severity: "high" }),
          wsItem({ signalId: "ws-duplicate", severity: "high" }),
          wsItem({ signalId: "ws-medium", severity: "medium" }),
        ],
      }),
    });

    await waitFor(() =>
      expect(useSignalsStore.getState().rawInboxSignals.map((signal) => signal.id)).toEqual([
        "ws-duplicate",
        "ws-medium",
      ]),
    );
    expect(screen.getAllByTestId("signal-card-ws-duplicate")).toHaveLength(1);
    expect(screen.queryByTestId("signal-card-ws-medium")).not.toBeInTheDocument();
  });

  it("does not render forbidden websocket payload fields", async () => {
    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );

    await waitFor(() => expect(wsMock.options?.onMessage).toBeTypeOf("function"));
    wsMock.options.onMessage({
      data: JSON.stringify({
        type: "signal_snapshot",
        signals: [
          {
            ...wsItem({ signalId: "ws-redacted", severity: "high" }),
            markout: "forbidden-markout-value",
            evidence: "forbidden-evidence-value",
            stale: "forbidden-stale-value",
            token: "forbidden-token-value",
            webhook: "forbidden-webhook-value",
            rawPayload: "forbidden-raw-payload-value",
            apiKey: "forbidden-api-key-value",
            authorization: "forbidden-authorization-value",
          },
        ],
      }),
    });

    expect(await screen.findByTestId("signal-card-ws-redacted")).toBeInTheDocument();
    for (const forbidden of [
      "forbidden-markout-value",
      "forbidden-evidence-value",
      "forbidden-stale-value",
      "forbidden-token-value",
      "forbidden-webhook-value",
      "forbidden-raw-payload-value",
      "forbidden-api-key-value",
      "forbidden-authorization-value",
    ]) {
      expect(screen.queryByText(forbidden)).not.toBeInTheDocument();
    }
  });

  it("shows reconnecting status without clearing existing signals", async () => {
    wsMock.status = "reconnecting";
    useSignalsStore.getState().setSignals([wsItem({ signalId: "ws-existing", severity: "high" })].map((item) => ({
      id: item.id,
      dedupeKey: item.id,
      time: "2023-11-14 22:13:20",
      exchange: "Runtime",
      symbol: item.symbol,
      type: item.detector,
      side: "Ask/Sell",
      reason: item.coreReason,
      finalResult: item.finalResult,
      level: "A",
      risk: "high",
      score: item.riskScore,
      confidence: 82,
      dataQuality: item.dataQuality,
      status: "unhandled",
      pushedAt: null,
      isLive: true,
    })));

    render(
      <MemoryRouter>
        <Dashboard />
      </MemoryRouter>,
    );

    expect(await screen.findByText("reconnecting")).toBeInTheDocument();
    expect(screen.getByTestId("signal-card-ws-existing")).toBeInTheDocument();
  });
});

function resetSignalsStore() {
  useSignalsStore.setState({
    rawInboxSignals: [],
    signals: [],
    selectedSignal: null,
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

function wsItem({ signalId, severity }) {
  return {
    id: signalId,
    symbol: "BTC-PERP",
    detector: "spoofing_candidate",
    direction: "short",
    severity,
    confidence: 0.82,
    createdAt: "2023-11-14T22:13:20.000Z",
    finalResult: "Ask/Sell · large ask wall removed",
    coreReason: "large ask wall removed",
    riskScore: severity === "medium" ? 72 : 85,
    dataQuality: 82,
    qualityBucket: "good",
    readOnly: true,
    runtimeModified: false,
    analysisOnly: true,
    executionEnabled: false,
  };
}
