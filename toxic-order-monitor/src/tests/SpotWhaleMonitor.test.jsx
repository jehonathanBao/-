import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import SpotWhaleMonitor from "../components/SpotWhaleMonitor.jsx";
import {
  fetchSpotWhaleLatest,
  fetchSpotWhaleSummary,
} from "../api/spotWhale.js";

vi.mock("../api/spotWhale.js", () => ({
  fetchSpotWhaleSummary: vi.fn((symbol = "BTC") =>
    Promise.resolve({
      summary: {
        status: "strong",
        healthStatus: "healthy",
        latestDirection: "buy",
        latestSeverity: "critical",
        signalCount: 1,
        enabled: true,
        dryRun: false,
        symbol,
        trend60s: {
          buyVolumeBase: symbol === "ETH" ? 4000 : 820,
          sellVolumeBase: 120,
          totalVolumeBase: symbol === "ETH" ? 4120 : 940,
          netVolumeBase: symbol === "ETH" ? 3880 : 700,
          dominance: 0.74,
          buyRatio: 0.87,
          sellRatio: 0.13,
          updatedAtMs: 1_700_000_000_000,
        },
        exchanges: {
          binance: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 50 },
          coinbase: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 80 },
          bitfinex: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 120 },
        },
      },
      error: null,
    }),
  ),
  fetchSpotWhaleLatest: vi.fn((limit = 50, symbol = "BTC") =>
    Promise.resolve({
      summary: { enabled: true, dryRun: false, symbol, exchanges: {} },
      items: [
        {
          id: `spot-whale-${symbol}`,
          ts: 1_700_000_000_000,
          symbol,
          windowSec: 15,
          signalType: "spot_aggressive_buy",
          direction: "buy",
          severity: "critical",
          score: 88,
          totalVolumeBase: symbol === "ETH" ? 4200 : 820,
          netVolumeBase: symbol === "ETH" ? 3900 : 700,
          totalNotionalUsd: 62_000_000,
          dominance: 0.74,
          priceMovePct: 0.21,
          coinbasePremiumPct: 0.04,
          mainExchange: "binance",
          dataQuality: 91,
          discordEligible: true,
          discordSent: false,
          discordReason: "critical_or_s_gate",
          exchanges: [
            { exchange: "binance", buyVolumeBase: 520, sellVolumeBase: 80, dominance: 0.73 },
            { exchange: "coinbase", buyVolumeBase: 300, sellVolumeBase: 40, dominance: 0.76 },
            { exchange: "bitfinex", buyVolumeBase: 40, sellVolumeBase: 0, dominance: 1 },
          ],
          finalResult: "Binance / Coinbase / Bitfinex 现货主动买入同步放大",
        },
      ],
      error: null,
    }),
  ),
  fetchSpotWhaleHistory: vi.fn(() => Promise.resolve({ summary: {}, items: [], error: null })),
}));

describe("SpotWhaleMonitor", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders BTC/ETH spot monitor with exchange health and signals", async () => {
    render(<SpotWhaleMonitor />);

    expect(await screen.findByText("BTC / ETH 现货监控")).toBeInTheDocument();
    expect(screen.getAllByText("Binance").length).toBeGreaterThan(0);
    expect(screen.getByText("Coinbase")).toBeInTheDocument();
    expect(screen.getByText("Bitfinex")).toBeInTheDocument();
    expect(await screen.findByText("现货主动买入")).toBeInTheDocument();
    expect(screen.getByLabelText("净方向")).toBeInTheDocument();
    expect(
      screen.getAllByText((_, element) => {
        const text = element?.textContent || "";
        return text.includes("BTC") && text.includes("$75,610");
      }).length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText("$75,610").length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("符合 gate")).toBeInTheDocument();
  });

  it("filters visible spot signals by signed net direction threshold", async () => {
    const user = userEvent.setup();
    render(<SpotWhaleMonitor />);

    expect(await screen.findByTestId("spot-whale-row-spot-whale-BTC")).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("净方向"), "pos100");
    expect(await screen.findByTestId("spot-whale-row-spot-whale-BTC")).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("净方向"), "neg50");
    await waitFor(() => {
      expect(screen.queryByTestId("spot-whale-row-spot-whale-BTC")).not.toBeInTheDocument();
    });
    expect(screen.getByText("暂无匹配净方向阈值的现货异动")).toBeInTheDocument();
  });

  it("refreshes summary and latest when switching to ETH", async () => {
    const user = userEvent.setup();
    render(<SpotWhaleMonitor />);

    const symbolSelect = await screen.findByLabelText("币种");
    await user.selectOptions(symbolSelect, "ETH");

    await waitFor(() => {
      expect(fetchSpotWhaleSummary).toHaveBeenCalledWith("ETH");
      expect(fetchSpotWhaleLatest).toHaveBeenCalledWith(50, "ETH");
    });
  });

  it("shows stale spot exchanges instead of reporting them as online", async () => {
    const staleSummary = {
      status: "calm",
      healthStatus: "unhealthy",
      latestDirection: "neutral",
      latestSeverity: "calm",
      signalCount: 0,
      enabled: true,
      dryRun: false,
      symbol: "BTC",
      trend60s: {
        buyVolumeBase: 0,
        sellVolumeBase: 0,
        totalVolumeBase: 0,
        netVolumeBase: 0,
        dominance: 0,
        buyRatio: 0,
        sellRatio: 0,
        updatedAtMs: null,
      },
        exchanges: {
          binance: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 50 },
          coinbase: { connected: false, status: "stale", lastTradeAt: 1_699_999_000_000, latencyMs: 1_000_000 },
          bitfinex: { connected: false, status: "stale", lastTradeAt: 1_699_999_000_000, latencyMs: 1_000_000 },
        },
    };
    fetchSpotWhaleSummary.mockResolvedValueOnce({ summary: staleSummary, error: null });
    fetchSpotWhaleLatest.mockResolvedValueOnce({ summary: staleSummary, items: [], error: null });

    render(<SpotWhaleMonitor />);

    expect((await screen.findAllByText("无近期成交")).length).toBeGreaterThanOrEqual(2);
    expect(screen.getByText("健康状态")).toBeInTheDocument();
    expect(screen.getByText("异常")).toBeInTheDocument();
    expect(screen.getByText(/60s 总成交 0 BTC/)).toBeInTheDocument();
  });
});
