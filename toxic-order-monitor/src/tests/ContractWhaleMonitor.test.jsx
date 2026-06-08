import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ContractWhaleMonitor from "../components/ContractWhaleMonitor.jsx";
import {
  fetchContractWhaleHistory,
  fetchContractWhaleLatest,
  fetchContractWhaleSummary,
} from "../api/contractWhale.js";

vi.mock("../api/contractWhale.js", () => ({
  fetchContractWhaleSummary: vi.fn(() =>
    Promise.resolve({
      summary: {
        status: "strong",
        healthStatus: "healthy",
        healthReason: "primary_sources_recent",
        direction: "buy",
        latestDirection: "buy",
        latestSeverity: "s",
        latestPushedAtMs: null,
        lastDiscordSentAt: null,
        signalCount: 1,
        readOnly: true,
        enabled: true,
        dryRun: true,
        trend60s: {
          buyVolumeBtc: 6200,
          sellVolumeBtc: 3800,
          totalVolumeBtc: 10000,
          netVolumeBtc: 2400,
          dominance: 0.24,
          buyRatio: 0.62,
          sellRatio: 0.38,
          updatedAtMs: 1_700_000_000_000,
        },
        exchanges: {
          binance: { connected: true, lastTradeAt: 1_700_000_000_000, reconnectCount: 0 },
          okx: { connected: true, lastTradeAt: 1_700_000_000_000, reconnectCount: 1 },
          bitfinex: { connected: false, lastTradeAt: null, reconnectCount: 3 },
        },
      },
      error: null,
    }),
  ),
  fetchContractWhaleLatest: vi.fn(() =>
    Promise.resolve({
      summary: {
        status: "strong",
        healthStatus: "healthy",
        healthReason: "primary_sources_recent",
        direction: "buy",
        latestDirection: "buy",
        latestSeverity: "s",
        latestPushedAtMs: null,
        lastDiscordSentAt: null,
        signalCount: 1,
        readOnly: true,
        enabled: true,
        dryRun: true,
        trend60s: {
          buyVolumeBtc: 6200,
          sellVolumeBtc: 3800,
          totalVolumeBtc: 10000,
          netVolumeBtc: 2400,
          dominance: 0.24,
          buyRatio: 0.62,
          sellRatio: 0.38,
          updatedAtMs: 1_700_000_000_000,
        },
        exchanges: {
          binance: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 120, reconnectCount: 0 },
          okx: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 1000, reconnectCount: 1 },
          bitfinex: { connected: false, status: "reconnecting", lastTradeAt: null, latencyMs: null, reconnectCount: 3 },
        },
      },
      items: [
        {
          id: "contract-whale-row",
          ts: 1_700_000_000_000,
          symbol: "BTC",
          windowSec: 15,
          signalType: "aggressive_buy",
          direction: "buy",
          severity: "s",
          score: 94,
          totalVolumeBtc: 4820,
          netVolumeBtc: 3260,
          totalNotionalUsd: 337_000_000,
          dominance: 0.676,
          priceMovePct: 0.31,
          mainExchange: "binance",
          dynamicMultiple: 9.4,
          percentileLevel: 99.9,
          multiExchangeConfirmed: true,
          liquidationSuspected: true,
          liquidationLongBtc: 420,
          liquidationShortBtc: 0,
          liquidationRatio: 0.087,
          oiChange5mBtc: 900,
          oiChangePct: 1.2,
          oiBias: "rising",
          fundingRate: 0.00018,
          fundingBias: "long",
          dataQuality: 91,
          discordEligible: true,
          discordSent: false,
          discordReason: "critical_or_s_gate",
          mergedFrom: ["contract-whale:BTC:5:1700000000000:buy"],
          exchanges: [
            {
              exchange: "binance",
              buyVolumeBtc: 2610,
              sellVolumeBtc: 200,
              totalVolumeBtc: 2810,
              netVolumeBtc: 2410,
              dominance: 0.858,
            },
            {
              exchange: "okx",
              buyVolumeBtc: 1780,
              sellVolumeBtc: 180,
              totalVolumeBtc: 1960,
              netVolumeBtc: 1600,
              dominance: 0.816,
            },
          ],
          finalResult: "多平台主动买入爆发，疑似主力合约拉盘",
        },
      ],
      error: null,
    }),
  ),
  fetchContractWhaleHistory: vi.fn(() =>
    Promise.resolve({
      summary: {
        status: "active",
        healthStatus: "healthy",
        healthReason: "primary_sources_recent",
        direction: "sell",
        latestDirection: "sell",
        latestSeverity: "critical",
        signalCount: 1,
        readOnly: true,
        enabled: true,
        dryRun: true,
        trend60s: {
          buyVolumeBtc: 1000,
          sellVolumeBtc: 4000,
          totalVolumeBtc: 5000,
          netVolumeBtc: -3000,
          dominance: 0.6,
          buyRatio: 0.2,
          sellRatio: 0.8,
          updatedAtMs: 1_700_000_010_000,
        },
        exchanges: {},
      },
      items: [],
      error: null,
    }),
  ),
}));

describe("ContractWhaleMonitor", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("renders summary cards and latest contract whale signals", async () => {
    render(<ContractWhaleMonitor />);

    expect(await screen.findByText("主力合约监控")).toBeInTheDocument();
    expect(screen.getByText("强异动")).toBeInTheDocument();
    expect(screen.getByText("健康")).toBeInTheDocument();
    expect(screen.getByText("Dry-run")).toBeInTheDocument();
    expect(screen.getByText("Buy 62.0% / Sell 38.0%")).toBeInTheDocument();
    expect(screen.getByText("总量 10,000 BTC · dominance 24.0%")).toBeInTheDocument();
    expect(screen.getAllByText("Binance").length).toBeGreaterThan(0);
    expect(screen.getAllByText("在线")).toHaveLength(2);
    expect(screen.getAllByText("Bitfinex").length).toBeGreaterThan(0);
    expect(screen.getByText("重连中")).toBeInTheDocument();
    expect(screen.getByText("延迟 120ms")).toBeInTheDocument();
    expect(screen.getByText("重连 3")).toBeInTheDocument();
    expect(screen.getAllByText("主力拉盘").length).toBeGreaterThan(0);
    expect(screen.getByText("4,820 BTC")).toBeInTheDocument();
    expect(screen.getByText("$337M")).toBeInTheDocument();
    expect(screen.getByText("净买入 3,260 BTC")).toBeInTheDocument();
    expect(screen.getByText("67.6%")).toBeInTheDocument();
    expect(screen.getByText("9.4x")).toBeInTheDocument();
    expect(screen.getByText("P99.9")).toBeInTheDocument();
    expect(screen.getByText("+0.31%")).toBeInTheDocument();
    expect(screen.getByText("疑似强平 420 BTC / 8.7%")).toBeInTheDocument();
    expect(screen.getByText("+900 BTC / +1.20% OI上升")).toBeInTheDocument();
    expect(screen.getByText("+0.02% 偏多")).toBeInTheDocument();
    expect(screen.getByText("待推")).toBeInTheDocument();
  });

  it("syncs filters to the history API", async () => {
    const user = userEvent.setup();
    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.selectOptions(screen.getByLabelText("等级"), "critical");

    await waitFor(() =>
      expect(fetchContractWhaleHistory).toHaveBeenCalledWith(
        expect.objectContaining({
          symbol: "BTC",
          severity: "critical",
          limit: 50,
        }),
      ),
    );
  });

  it("opens a read-only detail modal from the signal row", async () => {
    const user = userEvent.setup();
    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.click(screen.getByRole("button", { name: /查看主力合约信号详情 contract-whale-row/ }));

    expect(screen.getByRole("dialog", { name: "主力合约信号详情" })).toBeInTheDocument();
    expect(screen.getByText("Contract Whale Detail")).toBeInTheDocument();
    expect(screen.getByText("Discord Gate")).toBeInTheDocument();
    expect(screen.getByText("可进入推送判断")).toBeInTheDocument();
    expect(screen.getByText("critical_or_s_gate")).toBeInTheDocument();
    expect(screen.getByText("5s / 15s / 60s 窗口数据")).toBeInTheDocument();
    expect(screen.getByText("平台拆分")).toBeInTheDocument();
    expect(screen.getByText("主动买入：2,610 BTC")).toBeInTheDocument();
    expect(screen.getByText("Raw Scoring Breakdown")).toBeInTheDocument();
    expect(screen.getByText("Volume Strength")).toBeInTheDocument();
    expect(screen.getByText("contract-whale:BTC:5:1700000000000:buy")).toBeInTheDocument();
    expect(screen.queryByText(/rawPayload/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/webhook/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/token/i)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "关闭主力合约信号详情" }));
    expect(screen.queryByRole("dialog", { name: "主力合约信号详情" })).not.toBeInTheDocument();
  });

  it("keeps the panel visible and shows a light error when polling fails", async () => {
    fetchContractWhaleLatest.mockResolvedValueOnce({
      summary: {
        status: "calm",
        healthStatus: "healthy",
        direction: "neutral",
        latestDirection: "neutral",
        latestSeverity: "calm",
        signalCount: 0,
        readOnly: true,
        enabled: true,
        dryRun: true,
        exchanges: {},
      },
      items: [],
      error: "latest_unavailable",
    });

    render(<ContractWhaleMonitor />);

    expect(await screen.findByText("主力合约监控")).toBeInTheDocument();
    expect(screen.getByText("主力合约监控数据暂时不可用，已保留上一次结果。")).toBeInTheDocument();
  });

  it("polls summary every 5s and latest signals every 10s while visible", async () => {
    vi.useFakeTimers();

    render(<ContractWhaleMonitor />);

    expect(screen.getByText("主力合约监控")).toBeInTheDocument();
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(1);
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(0);

    await vi.advanceTimersByTimeAsync(5_000);
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(1);
    expect(fetchContractWhaleSummary).toHaveBeenLastCalledWith("BTC");
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(5_000);
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(2);
    expect(fetchContractWhaleSummary).toHaveBeenLastCalledWith("BTC");
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(2);
  });

  it("keeps latest requests scoped to ETH after symbol switch", async () => {
    const user = userEvent.setup();

    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.selectOptions(screen.getByLabelText("币种"), "ETH");

    await waitFor(() => expect(fetchContractWhaleLatest).toHaveBeenLastCalledWith(50, "ETH"));
  });
});
