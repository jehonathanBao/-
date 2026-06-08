import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Dashboard from "../pages/Dashboard.jsx";
import { useSignalsStore } from "../store/signalsStore.js";

const wsMock = vi.hoisted(() => ({
  optionsByPath: new Map(),
  status: "open",
}));

vi.mock("../api/signals.js", async () => {
  const actual = await vi.importActual("../api/signals.js");
  return {
    ...actual,
    fetchSignals: vi.fn(() => Promise.resolve([])),
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
  useReconnectingWebSocket: vi.fn((path, options) => {
    wsMock.optionsByPath.set(path, options);
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
    wsMock.optionsByPath.clear();
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

    await waitFor(() => expect(wsMock.optionsByPath.get("/ws/signals")?.onMessage).toBeTypeOf("function"));
    wsMock.optionsByPath.get("/ws/signals").onMessage({
      data: JSON.stringify({
        type: "signal_snapshot",
        signals: [
          wsItem({ signalId: "ws-high", severity: "high" }),
          wsItem({ signalId: "ws-medium", severity: "medium" }),
        ],
      }),
    });

    expect(await screen.findByTestId("signal-card-ws-high")).toBeInTheDocument();
    expect(screen.getByText("Risk 91")).toBeInTheDocument();
    expect(screen.getByText("Quality 82")).toBeInTheDocument();
    expect(screen.getByText("TOF 88")).toBeInTheDocument();
    expect(screen.getByText("Perp 87")).toBeInTheDocument();
    expect(screen.getAllByText("Advanced 89").length).toBeGreaterThan(0);
    expect(screen.getByText(/CWM 92/)).toBeInTheDocument();
    expect(screen.getAllByText(/OpenInterestCandidate/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/MarketPressureHeatmapCandidate/).length).toBeGreaterThan(0);
    expect(screen.getByText("Discord：未推送，原因：历史缓存不自动推送")).toBeInTheDocument();
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

    await waitFor(() => expect(wsMock.optionsByPath.get("/ws/signals")?.onMessage).toBeTypeOf("function"));
    wsMock.optionsByPath.get("/ws/signals").onMessage({
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

    await waitFor(() => expect(wsMock.optionsByPath.get("/ws/signals")?.onMessage).toBeTypeOf("function"));
    wsMock.optionsByPath.get("/ws/signals").onMessage({
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

    await waitFor(() => expect(screen.getAllByText("reconnecting").length).toBeGreaterThanOrEqual(1));
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
    finalRiskScore: severity === "medium" ? 72 : 91,
    dataQuality: 82,
    tofScore: 88.4,
    perpScore: 87,
    perpCandidateType: "OpenInterestCandidate",
    advancedScore: 89,
    advancedCandidateType: "MarketPressureHeatmapCandidate",
    cwmContribution: {
      available: true,
      source: "contract_whale_monitor",
      formula: "finalRiskScore = spotRisk*0.35 + TOF-lite*0.25 + perpMetrics*0.25 + CWM*0.15",
      contributionWeight: 0.15,
      score: 92,
      weightedContribution: 13.8,
      signalId: "contract-whale:BTC:15:1700000000000:buy",
      severity: "s",
      signalType: "aggressive_buy",
      direction: "buy",
      windowSec: 15,
      dataQuality: 88,
      dominance: 0.676,
      summary: "多平台主动买入爆发",
      discordGateIndependent: true,
    },
    finalCandidateType: "High Risk Bullish Candidate",
    metricsDirection: "bullish",
    mergedConfidence: 87,
    candidateType: "spoofing_candidate",
    explainTags: ["high_vpin_proxy", "bid_depth_withdrawal"],
    directionLabel: "看跌 / Ask-Sell",
    directionConfidence: 82.5,
    directionSource: "detector+tof_metrics",
    alertStatus: severity === "medium" ? "rejected" : "skipped",
    alertReason: severity === "medium" ? "non_high_risk" : "cached_on_boot",
    discordAlert: {
      autoEligible: false,
      autoSent: false,
      lastDecision: severity === "medium" ? "rejected" : "skipped",
      reason: severity === "medium" ? "non_high_risk" : "cached_on_boot",
      sentAt: null,
      manualSentAt: null,
    },
    tofMetrics: {
      tofScore: 88.4,
      vpinProxy: 89,
      tradeImbalance: -0.43,
      bidDepthWithdrawal: 58,
      askDepthWithdrawal: 12,
      spreadBps: 8.4,
      metricsConfidence: 82,
    },
    perpTofMetrics: {
      oiChange: 150000,
      oiDirection: "long_increase",
      fundingRate: -0.071,
      fundingSide: "short",
      liquidationPressure: 82,
      squeezeSide: "short",
      aggBuyVolume: 1500000,
      aggSellVolume: 420000,
      metricsDirection: "bullish",
      riskScore: 87,
      dataQuality: 88,
      candidateType: "OpenInterestCandidate",
      explainTags: ["OI long increase"],
      confidence: 87,
    },
    advancedTofMetrics: {
      vpinEnhanced: 88,
      largeOrderFlowCluster: 76,
      historicalFundingOiTrend: 84,
      marketPressureHeatmap: 91,
      spotRiskScore: 86,
      spotTofScore: 88.4,
      perpScore: 87,
      finalRiskScore: 89,
      dataQuality: 86,
      metricsCompleteness: 95,
      freshDataCoverage: 92,
      candidateType: "MarketPressureHeatmapCandidate",
      finalCandidateType: "High Risk Bullish Advanced Candidate",
      metricsDirection: "bullish",
      confidence: 90,
      explainTags: ["Market pressure heatmap"],
    },
    qualityBucket: "good",
    readOnly: true,
    runtimeModified: false,
    analysisOnly: true,
    executionEnabled: false,
  };
}
