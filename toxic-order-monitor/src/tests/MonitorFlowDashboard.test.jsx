import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchMonitorFlowSnapshot } from "../api/monitorFlow.js";
import MonitorFlowDashboard, { buildMonitorFlowEvents } from "../components/MonitorFlowDashboard.jsx";

vi.mock("../api/monitorFlow.js", () => ({
  fetchMonitorFlowSnapshot: vi.fn(),
}));

describe("MonitorFlowDashboard", () => {
  beforeEach(() => {
    fetchMonitorFlowSnapshot.mockResolvedValue(mockSnapshot());
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders one unified stream without duplicating the contract trading desk", async () => {
    renderDashboard();

    expect(await screen.findByText("全市场实时监控流")).toBeInTheDocument();
    expect(screen.getByText("实时监控事件流")).toBeInTheDocument();
    expect(screen.getByText("BTC 当前脉冲")).toBeInTheDocument();
    expect(screen.getByText("监控链路")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "BTC 价格走势" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "重点观察" })).toBeInTheDocument();
    expect(screen.queryByTestId("ai-neural-field")).not.toBeInTheDocument();
    expect(screen.queryByText("MARKET CONSENSUS")).not.toBeInTheDocument();
    expect(screen.queryByText("合约市场事件")).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: /合约事件带/ })).toHaveAttribute("href", "/contract-whale/btc");
  });

  it("filters the tape and can pause automatic refresh", async () => {
    const user = userEvent.setup();
    renderDashboard();

    expect(await screen.findByText(/现货主动买入/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /合约/ }));

    expect(screen.getByText("主动买压")).toBeInTheDocument();
    expect(screen.queryByText(/现货主动买入/)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "暂停流" }));
    expect(screen.getByText("PAUSED")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "继续流" })).toHaveAttribute("aria-pressed", "true");
  });

  it("normalizes contract, spot, Delta, VPIN/TOF, risk and system events by time", () => {
    const events = buildMonitorFlowEvents(mockSnapshot(), [mockRiskSignal()]);

    expect(events.map(event => event.channel)).toEqual(
      expect.arrayContaining(["contract", "spot", "micro", "signal", "system"]),
    );
    expect(events).toEqual([...events].sort((left, right) => right.ts - left.ts));
    expect(events.length).toBeLessThan(20);
  });

  it("keeps the newest successful snapshot when a refresh fails", async () => {
    fetchMonitorFlowSnapshot
      .mockResolvedValueOnce(mockSnapshot())
      .mockRejectedValueOnce(new Error("temporary network failure"));
    const user = userEvent.setup();
    renderDashboard();

    expect(await screen.findByText("主动买压")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "立即同步" }));

    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("保留最近一次成功快照"));
    expect(screen.getByText("主动买压")).toBeInTheDocument();
  });

  it("does not present missing trade-flow data as zero flow or a normal TOF reading", async () => {
    fetchMonitorFlowSnapshot.mockResolvedValue({
      fetchedAtMs: Date.now(), contract: { BTC: { summary: { enabled: false, trend60s: { netVolumeBtc: 0 } }, error: "summary_unavailable" }, events: [] },
      spot: { BTC: { summary: { enabled: false, trend60s: { netVolumeBase: 0, dominance: 0 } }, error: "latest_unavailable" } },
      orderflow: { source: "binance_futures_kline_public_fallback", monitorFlowCandles: 0, candles: [{ time: Date.now(), close: 60_000, buyBase: 0, sellBase: 0, deltaBase: 0, vpin: null, tofVolumeBtc: null }] },
    });
    renderDashboard();
    expect(await screen.findByText("WAITING")).toBeInTheDocument();
    expect(screen.getAllByText("等待数据").length).toBeGreaterThanOrEqual(4);
    expect(screen.queryByText("NORMAL")).not.toBeInTheDocument();
    expect(screen.queryByText("0 BTC")).not.toBeInTheDocument();
  });
});

function renderDashboard() {
  return render(
    <MemoryRouter>
      <MonitorFlowDashboard
        discordConnected
        rawInboxSignals={[mockRiskSignal()]}
        runtimeBoundary={{ phase: "confirmed", readOnly: true, monitoringStarted: true }}
        wsStatus="open"
      />
    </MemoryRouter>,
  );
}

function mockSnapshot() {
  const now = Date.now();
  return {
    fetchedAtMs: now,
    contract: {
      BTC: {
        summary: {
          activeExchangeCount: 2,
          latestDirection: "buy",
          overallDataQuality: 91,
          trend60s: { netVolumeBtc: 345 },
        },
      },
      ETH: { summary: null },
      events: [{
        id: "c-1",
        eventId: "c-1",
        ts: now - 2_000,
        symbol: "BTC",
        signalType: "active_buy_pressure",
        displaySignalType: "主动买压",
        direction: "buy",
        netVolumeBtc: 345,
        totalVolumeBtc: 507,
        displayVolumeBtc: 507,
        totalNotionalUsd: 40_000_000,
        mainExchange: "binance",
        impactGrade: "A",
        severity: "high",
        discordEligible: true,
        discordSent: false,
        windowSec: 60,
      }],
    },
    spot: {
      BTC: {
        summary: {
          enabled: true,
          latestDirection: "buy",
          trend60s: { netVolumeBase: 230, dominance: 0.68 },
          exchanges: {
            binance: { connected: true },
            coinbase: { connected: true },
            bitfinex: { connected: true },
          },
        },
        items: [{
          id: "s-1",
          ts: now - 1_000,
          symbol: "BTC",
          signalType: "spot_aggressive_buy",
          finalResult: "现货主动买入达到净方向阈值",
          direction: "buy",
          netVolumeBase: 230,
          totalNotionalUsd: 18_000_000,
          mainExchange: "binance",
          dominance: 0.68,
          dataQuality: 96,
          severity: "high",
          discordEligible: true,
          windowSec: 60,
        }],
      },
      ETH: { summary: null, items: [] },
    },
    orderflow: {
      interval: "1h",
      candles: [{
        time: now - 3_600_000,
        closeTime: now - 500,
        volumeBase: 4_000,
        buyBase: 2_600,
        sellBase: 1_400,
        deltaBase: 1_200,
        deltaPct: 30,
        tradeCount: 80_000,
        closed: false,
        vpin: 0.81,
        vpinZscore: 2.3,
        vpinPercentile: 0.97,
        vpinSpike: true,
        vpinHigh: true,
        vpinExtreme: false,
        tofVolumeBtc: 520,
        tofSeverity: "high",
        tofAlert: true,
        tofReasons: ["VPIN 高位", "订单流毒性放大"],
      }],
    },
    scanLogs: [{
      id: "log-1",
      tsMs: now - 100,
      level: "warn",
      kind: "discord_auto_push_skipped",
      message: "Discord auto push skipped: rating unavailable",
      symbol: "BTC-PERP",
      candidateId: "candidate-1",
    }],
    system: {
      storage: { enabled: true, status: "healthy", lastWriteTs: now - 1_000, lastError: null },
      marketDataQuality: { status: "healthy", latestTradeTs: now - 50, lastMessageTs: now - 20, recentLaggedEvents: 0 },
      venues: {
        binance: { enabled: true, status: "connected", tradeActive: true, lastMessageTs: now - 20 },
        bitfinex: { enabled: true, status: "connected", tradeActive: true, lastMessageTs: now - 80 },
      },
    },
  };
}

function mockRiskSignal() {
  return {
    id: "risk-1",
    ts: Date.now() - 3_000,
    symbol: "BTCUSDT",
    type: "SpoofingCandidate",
    side: "Ask/Sell",
    reason: "盘口撤单异常",
    level: "S",
    risk: "high",
    score: 92,
    confidence: 88,
    dataQuality: 91,
  };
}
