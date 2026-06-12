import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import BinanceAltContractMonitor from "../components/BinanceAltContractMonitor.jsx";
import {
  fetchBinanceAltContractHistory,
  fetchBinanceAltContractLatest,
  fetchBinanceAltContractSummary,
} from "../api/binanceAltContract.js";

vi.mock("../api/binanceAltContract.js", () => ({
  fetchBinanceAltContractSummary: vi.fn(() =>
    Promise.resolve({
      summary: altSummary(),
      error: null,
    }),
  ),
  fetchBinanceAltContractLatest: vi.fn(() =>
    Promise.resolve({
      summary: altSummary(),
      items: [altSignal()],
      error: null,
    }),
  ),
  fetchBinanceAltContractHistory: vi.fn(() =>
    Promise.resolve({
      summary: altSummary(),
      items: [],
      error: null,
    }),
  ),
}));

describe("BinanceAltContractMonitor", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("renders summary, prices, dry-run state and latest alt contract signals", async () => {
    render(<BinanceAltContractMonitor />);

    expect(await screen.findByText("山寨合约异常监控")).toBeInTheDocument();
    expect(screen.getByText(/全量监控 Binance USDT 永续山寨合约/)).toBeInTheDocument();
    expect(screen.getAllByText(/全 Binance USDT 永续/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Tier A1 \/ B1 \/ C0 \/ D0 \/ E0/).length).toBeGreaterThan(0);
    expect(screen.getByText("在线")).toBeInTheDocument();
    expect(screen.getAllByText("主力建多").length).toBeGreaterThan(0);
    expect(screen.getByText("$175.50")).toBeInTheDocument();
    expect(screen.getByText("91/100")).toBeInTheDocument();
    expect(screen.getByText("87/100")).toBeInTheDocument();
    expect(screen.getByText("$143.9M")).toBeInTheDocument();
    expect(screen.getByText("10.2x")).toBeInTheDocument();
    expect(screen.getByText("+210,000 SOL")).toBeInTheDocument();
    expect(screen.getByText("+2.400%")).toBeInTheDocument();
    expect(screen.getByText("dry-run would_send")).toBeInTheDocument();
    expect(screen.getByText("Dry-run 24h")).toBeInTheDocument();
    expect(screen.getByText(/signals 14/)).toBeInTheDocument();
    expect(screen.getAllByText(/Candidate/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Hot OI/).length).toBeGreaterThan(0);
    expect(screen.getByText(/markPrice/)).toBeInTheDocument();
    expect(screen.getByText(/ticker/)).toBeInTheDocument();
  });

  it("uses history when severity filter is selected", async () => {
    const user = userEvent.setup();
    render(<BinanceAltContractMonitor />);

    await screen.findByText("山寨合约异常监控");
    await user.selectOptions(screen.getByLabelText("等级"), "critical");

    await waitFor(() =>
      expect(fetchBinanceAltContractHistory).toHaveBeenCalledWith(
        expect.objectContaining({
          symbol: "all",
          severity: "critical",
          signal_type: "all",
          limit: 50,
        }),
      ),
    );
  });

  it("uses history when liquidationDriven and tier filters are selected", async () => {
    const user = userEvent.setup();
    render(<BinanceAltContractMonitor />);

    await screen.findByText("山寨合约异常监控");
    await user.selectOptions(screen.getByLabelText("清算"), "true");
    await user.selectOptions(screen.getByLabelText("流动性 Tier"), "b");

    await waitFor(() =>
      expect(fetchBinanceAltContractHistory).toHaveBeenCalledWith(
        expect.objectContaining({
          liquidationDriven: "true",
          tier: "b",
          limit: 50,
        }),
      ),
    );
  });

  it("opens a read-only detail modal with score and source snapshot", async () => {
    const user = userEvent.setup();
    render(<BinanceAltContractMonitor />);

    await screen.findByText("山寨合约异常监控");
    await user.click(screen.getByTestId("alt-contract-row-bacm-sol-s"));

    expect(screen.getByText("Alt Contract Review")).toBeInTheDocument();
    expect(screen.getByText("SOL · 主力建多")).toBeInTheDocument();
    expect(screen.getAllByText("$175.50").length).toBeGreaterThan(0);
    expect(screen.getByText("Discord dry-run")).toBeInTheDocument();
    expect(screen.getAllByText("dry-run would_send").length).toBeGreaterThan(0);
    expect(screen.getByText("主力置信度")).toBeInTheDocument();
    expect(screen.getByText("证据数量")).toBeInTheDocument();
    expect(screen.getByText("Window Confirmations")).toBeInTheDocument();
    expect(screen.getByText("主动买入占优")).toBeInTheDocument();
    expect(screen.getByText("多窗口确认")).toBeInTheDocument();
    expect(screen.getByText("Score Breakdown")).toBeInTheDocument();
    expect(screen.getByText("Active Source Snapshot")).toBeInTheDocument();
    expect(screen.getByText("binance · perp · primary · active")).toBeInTheDocument();
    expect(screen.getByText("山寨合约主动买入爆发，OI 同步上升，疑似主力建多。")).toBeInTheDocument();
    expect(screen.getByText("Abnormal Score")).toBeInTheDocument();
    expect(screen.getByText("Build Score")).toBeInTheDocument();
    expect(screen.getAllByText("OI").length).toBeGreaterThan(0);
    expect(screen.getByText("Price Move")).toBeInTheDocument();
    expect(screen.getByText(/有强平快照/)).toBeInTheDocument();
    expect(screen.queryByText(/rawPayload/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/webhook/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/token/i)).not.toBeInTheDocument();
  });
});

function altSummary() {
  return {
    status: "active",
    healthStatus: "healthy",
    latestDirection: "buy",
    latestSeverity: "s",
    latestSignalAt: 1_700_000_000_000,
    signalCount: 1,
    monitoredSymbols: ["SOLUSDT", "DOGEUSDT"],
    activeAnomalyCount: 1,
    recentCriticalOrSCount: 1,
    dryRunWouldSendCount: 1,
    enabled: true,
    dryRun: true,
    readOnly: true,
    trend60s: {
      buyVolumeBase: 820_000,
      sellVolumeBase: 210_000,
      totalVolumeBase: 1_030_000,
      netVolumeBase: 610_000,
      totalNotionalUsd: 143_910_000,
      dominance: 0.59,
      buyRatio: 0.8,
      sellRatio: 0.2,
      updatedAtMs: 1_700_000_000_000,
    },
    exchanges: {
      binance: {
        connected: true,
        status: "connected",
        lastTradeAt: Date.now(),
        latencyMs: 92,
        reconnectCount: 0,
      },
    },
    dryRunStats: {
      signals1h: 3,
      high1h: 1,
      critical1h: 1,
      s1h: 1,
      wouldSend1h: 2,
      skippedLowScore1h: 1,
      skippedCooldown1h: 0,
      skippedDataQuality1h: 0,
      liquidationDriven1h: 1,
      signals24h: 14,
      high24h: 8,
      critical24h: 4,
      s24h: 2,
      wouldSend24h: 5,
      skippedLowScore24h: 3,
      skippedCooldown24h: 2,
      skippedDataQuality24h: 1,
      liquidationDriven24h: 4,
    },
    symbolUniverse: {
      mode: "all_binance_usdt_perp",
      limit: 0,
      monitoredCount: 2,
      tierCounts: { A: 1, B: 1, C: 0, D: 0, E: 0 },
      whitelist: [],
      blacklist: [],
      excludedSymbols: ["BTCUSDT", "ETHUSDT"],
      min24hQuoteVolumeUsd: 0,
    },
    allMarketContext: {
      markPriceConnected: true,
      tickerConnected: true,
      forceOrderConnected: true,
      lastMarkPriceAt: Date.now(),
      lastTickerAt: Date.now(),
      lastForceOrderAt: Date.now(),
      candidateSymbols: ["SOLUSDT"],
      hotOiSymbols: ["SOLUSDT"],
    },
  };
}

function altSignal() {
  return {
    id: "bacm-sol-s",
    ts: 1_700_000_000_000,
    symbol: "SOL",
    productId: "SOLUSDT",
    tier: "b",
    windowSec: 60,
    signalType: "main_force_long_build",
    direction: "buy",
    severity: "s",
    abnormalScore: 91,
    buildScore: 87,
    mainForceConfidence: 84,
    evidenceCount: 5,
    evidenceTags: [
      "aggressive_buy_dominant",
      "oi_expanding",
      "dynamic_multiple_critical",
      "price_follow_through",
      "multi_window_confirmed",
    ],
    windowConfirmations: [
      {
        windowSec: 15,
        notionalUsd: 42_000_000,
        dynamicMultiple: 8.2,
        directionalStrength: 0.72,
        confirmed: true,
      },
      {
        windowSec: 60,
        notionalUsd: 143_910_000,
        dynamicMultiple: 10.2,
        directionalStrength: 0.74,
        confirmed: true,
      },
    ],
    marketWideMove: false,
    marketImpulseRatio: 0.04,
    relativeStrengthRank: 2,
    postSignalStatus: "pending",
    signalVwap: 175.5,
    retestStatus: "unknown",
    oiFreshnessSec: 14,
    oiQuality: "fresh",
    fundingCrowding: "neutral",
    fundingPenalty: 0,
    directionBias: 76,
    dataQuality: 92,
    totalVolumeBase: 820_000,
    netVolumeBase: 610_000,
    totalNotionalUsd: 143_910_000,
    triggerPriceUsd: 175.5,
    dominance: 0.74,
    priceMovePct: 2.4,
    dynamicMultiple: 10.2,
    oiChange1mBase: 210_000,
    oiChangePct: 1.8,
    fundingRate: 0.00021,
    liquidationSuspected: false,
    forceOrderSnapshot: true,
    mainExchange: "binance",
    exchanges: [
      {
        exchange: "binance",
        totalVolumeBase: 820_000,
        netVolumeBase: 610_000,
        totalNotionalUsd: 143_910_000,
        dominance: 0.74,
      },
    ],
    scoreBreakdown: {
      volumeScore: 25,
      dynamicScore: 18,
      directionalScore: 13,
      oiScore: 12,
      priceScore: 10,
      liquidationScore: 0,
      persistenceScore: 6,
      fundingScore: 3,
      dataQualityScore: 5,
      penaltyScore: 0,
    },
    activeSources: [
      {
        exchange: "binance",
        marketType: "perp",
        role: "primary",
        status: "active",
      },
    ],
    discordEligible: true,
    discordWouldSend: true,
    discordSent: false,
    discordReason: "dry_run",
    finalResult: "山寨合约主动买入爆发，OI 同步上升，疑似主力建多。",
  };
}
