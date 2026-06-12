import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchBinanceAltContractHistory,
  fetchBinanceAltContractLatest,
  fetchBinanceAltContractSummary,
  normalizeAltContractSignal,
} from "../api/binanceAltContract.js";

vi.mock("axios", () => ({
  default: {
    get: vi.fn(),
  },
}));

describe("binance alt contract api", () => {
  beforeEach(() => {
    axios.get.mockReset();
    vi.stubEnv("VITE_API_BASE_URL", "");
  });

  it("maps latest alt contract response into frontend shape", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: {
          status: "active",
          healthStatus: "healthy",
          latestDirection: "buy",
          latestSeverity: "s",
          monitoredSymbols: ["SOLUSDT", "DOGEUSDT"],
          displayMinNotionalUsd: 500_000,
          activeAnomalyCount: 2,
          recentCriticalOrSCount: 1,
          dryRunWouldSendCount: 1,
          enabled: true,
          dryRun: true,
          readOnly: true,
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
          trend60s: {
            buyVolumeBase: 120_000,
            sellVolumeBase: 30_000,
            totalVolumeBase: 150_000,
            netVolumeBase: 90_000,
            totalNotionalUsd: 12_000_000,
            buyRatio: 0.8,
            sellRatio: 0.2,
          },
          exchanges: {
            binance: {
              connected: true,
              status: "connected",
              lastTradeAt: 1_700_000_000_000,
              latencyMs: 90,
              reconnectCount: 0,
            },
          },
        },
        items: [altSignal(), lowNotionalSignal()],
      },
    });

    const payload = await fetchBinanceAltContractLatest(25, "SOL");

    expect(axios.get).toHaveBeenCalledWith("/api/binance-alt-contract/latest?limit=25&symbol=SOL");
    expect(payload.summary).toMatchObject({
      status: "active",
      healthStatus: "healthy",
      latestDirection: "buy",
      latestSeverity: "s",
      monitoredSymbols: ["SOLUSDT", "DOGEUSDT"],
      displayMinNotionalUsd: 500_000,
      activeAnomalyCount: 2,
      recentCriticalOrSCount: 1,
      dryRunWouldSendCount: 1,
      enabled: true,
      dryRun: true,
      readOnly: true,
      dryRunStats: {
        signals1h: 3,
        wouldSend1h: 2,
        liquidationDriven1h: 1,
        signals24h: 14,
        critical24h: 4,
        s24h: 2,
        wouldSend24h: 5,
        liquidationDriven24h: 4,
      },
      trend60s: {
        totalNotionalUsd: 12_000_000,
        buyRatio: 0.8,
        sellRatio: 0.2,
      },
      exchanges: {
        binance: {
          connected: true,
          status: "connected",
          latencyMs: 90,
        },
      },
    });
    expect(payload.items).toHaveLength(1);
    expect(payload.items[0]).toMatchObject({
      id: "bacm-sol-s",
      symbol: "SOL",
      productId: "SOLUSDT",
      signalType: "main_force_long_build",
      severity: "s",
      abnormalScore: 91,
      buildScore: 87,
      triggerPriceUsd: 175.5,
      discordWouldSend: true,
      discordSent: false,
      activeSources: [
        {
          exchange: "binance",
          marketType: "perp",
          role: "primary",
          status: "active",
        },
      ],
    });
  });

  it("passes history filters including liquidationDriven and build threshold", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: {},
        items: [],
      },
    });

    const payload = await fetchBinanceAltContractHistory({
      symbol: "SOL",
      severity: "critical",
      signal_type: "main_force_long_build",
      direction: "buy",
      would_send: true,
      liquidationDriven: false,
      tier: "b",
      min_build_score: 85,
      limit: 25,
    });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/binance-alt-contract/history?symbol=SOL&severity=critical&signal_type=main_force_long_build&direction=buy&would_send=true&liquidationDriven=false&tier=b&min_build_score=85&limit=25",
    );
    expect(payload.items).toEqual([]);
    expect(payload.error).toBeNull();
  });

  it("computes trigger price from notional and volume and drops sensitive fields", () => {
    const signal = normalizeAltContractSignal({
      id: "fallback-price",
      symbol: "DOGE",
      totalVolumeBase: 10_000,
      totalNotionalUsd: 1_500,
      rawPayload: "must not render",
      webhook: "must not render",
      token: "must not render",
    });

    expect(signal.triggerPriceUsd).toBe(0.15);
    expect(signal.rawPayload).toBeUndefined();
    expect(signal.webhook).toBeUndefined();
    expect(signal.token).toBeUndefined();
  });

  it("falls back to disabled read-only summary on request failure", async () => {
    axios.get.mockRejectedValueOnce(new Error("network"));

    const payload = await fetchBinanceAltContractSummary();

    expect(payload.error).toBe("summary_unavailable");
    expect(payload.summary).toMatchObject({
      enabled: false,
      dryRun: true,
      readOnly: true,
      healthStatus: "disabled",
    });
  });
});

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

function lowNotionalSignal() {
  return {
    ...altSignal(),
    id: "bacm-doge-small",
    symbol: "DOGE",
    productId: "DOGEUSDT",
    totalVolumeBase: 1_000,
    totalNotionalUsd: 499_999,
    triggerPriceUsd: 0.2,
  };
}
