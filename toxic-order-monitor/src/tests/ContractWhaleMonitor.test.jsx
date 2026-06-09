import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ContractWhaleMonitor from "../components/ContractWhaleMonitor.jsx";
import {
  fetchContractWhaleEvents,
  fetchContractWhaleHistory,
  fetchContractWhaleLatest,
  fetchContractWhaleSummary,
} from "../api/contractWhale.js";

function hasPriceText(text) {
  return typeof text === "string" && text.includes("69,917");
}

vi.mock("../api/contractWhale.js", () => ({
  fetchContractWhaleSummary: vi.fn(() =>
    Promise.resolve({
      summary: {
        status: "strong",
        healthStatus: "healthy",
        healthReason: "primary_sources_recent",
        thresholdProfile: "binance_bitfinex",
        activeExchangeCount: 2,
        enabledExchanges: ["binance", "bitfinex"],
        disabledExchanges: ["okx"],
        activeContractExchanges: ["binance", "bitfinex"],
        direction: "buy",
        latestDirection: "buy",
        latestSeverity: "s",
        latestPushedAtMs: null,
        lastDiscordSentAt: null,
        signalCount: 1,
        readOnly: true,
        enabled: true,
        dryRun: true,
        contractDataQuality: 95,
        spotDataQuality: 78,
        overallDataQuality: 88,
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
          binance: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, reconnectCount: 0, platformEnabled: true, contractEnabled: true, enabledMarkets: ["spot", "perp", "funding", "oi", "liquidation"], marketRoles: { spot: "primary", perp: "primary", funding: "primary", oi: "primary", liquidation: "primary" } },
          okx: { connected: false, status: "disabled", lastTradeAt: null, reconnectCount: 0, platformEnabled: false, contractEnabled: false, enabledMarkets: [], marketRoles: {} },
          bitfinex: { connected: false, status: "reconnecting", lastTradeAt: null, reconnectCount: 3, platformEnabled: true, contractEnabled: true, enabledMarkets: ["spot", "perp"], marketRoles: { spot: "confirmation", perp: "confirmation" } },
          coinbase: { connected: false, status: "spot_only", lastTradeAt: null, reconnectCount: 0, platformEnabled: true, contractEnabled: false, enabledMarkets: ["spot"], marketRoles: { spot: "spot_confirmation" } },
        },
        platforms: {
          binance: {
            platformEnabled: true,
            status: "active",
            markets: {
              spot: { enabled: true, status: "enabled", role: "primary_liquidity" },
              perp: { enabled: true, status: "active", role: "primary_liquidity" },
              funding: { enabled: true, status: "enabled", role: "primary_liquidity" },
              oi: { enabled: true, status: "enabled", role: "primary_liquidity" },
              liquidation: { enabled: true, status: "enabled", role: "primary_liquidity" },
            },
          },
          bitfinex: {
            platformEnabled: true,
            status: "active",
            markets: {
              spot: { enabled: true, status: "enabled", role: "confirmation" },
              perp: { enabled: true, status: "active", role: "confirmation" },
            },
          },
          coinbase: {
            platformEnabled: true,
            status: "spot_only",
            markets: {
              spot: { enabled: true, status: "enabled", role: "spot_confirmation" },
              perp: { enabled: false, status: "disabled", role: "optional" },
            },
          },
          okx: {
            platformEnabled: false,
            status: "disabled",
            markets: {
              perp: { enabled: false, status: "disabled", role: "optional" },
            },
          },
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
        thresholdProfile: "binance_bitfinex",
        activeExchangeCount: 2,
        enabledExchanges: ["binance", "bitfinex"],
        disabledExchanges: ["okx"],
        activeContractExchanges: ["binance", "bitfinex"],
        direction: "buy",
        latestDirection: "buy",
        latestSeverity: "s",
        latestPushedAtMs: null,
        lastDiscordSentAt: null,
        signalCount: 1,
        readOnly: true,
        enabled: true,
        dryRun: true,
        contractDataQuality: 95,
        spotDataQuality: 78,
        overallDataQuality: 88,
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
          binance: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 120, reconnectCount: 0, platformEnabled: true, contractEnabled: true, enabledMarkets: ["spot", "perp", "funding", "oi", "liquidation"], marketRoles: { spot: "primary", perp: "primary", funding: "primary", oi: "primary", liquidation: "primary" } },
          okx: { connected: false, status: "disabled", lastTradeAt: null, latencyMs: null, reconnectCount: 0, platformEnabled: false, contractEnabled: false, enabledMarkets: [], marketRoles: {} },
          bitfinex: { connected: false, status: "reconnecting", lastTradeAt: null, latencyMs: null, reconnectCount: 3, platformEnabled: true, contractEnabled: true, enabledMarkets: ["spot", "perp"], marketRoles: { spot: "confirmation", perp: "confirmation" } },
          coinbase: { connected: false, status: "spot_only", lastTradeAt: null, latencyMs: null, reconnectCount: 0, platformEnabled: true, contractEnabled: false, enabledMarkets: ["spot"], marketRoles: { spot: "spot_confirmation" } },
        },
        platforms: {
          binance: {
            platformEnabled: true,
            status: "active",
            markets: {
              spot: { enabled: true, status: "enabled", role: "primary_liquidity" },
              perp: { enabled: true, status: "active", role: "primary_liquidity" },
              funding: { enabled: true, status: "enabled", role: "primary_liquidity" },
              oi: { enabled: true, status: "enabled", role: "primary_liquidity" },
              liquidation: { enabled: true, status: "enabled", role: "primary_liquidity" },
            },
          },
          bitfinex: {
            platformEnabled: true,
            status: "active",
            markets: {
              spot: { enabled: true, status: "enabled", role: "confirmation" },
              perp: { enabled: true, status: "active", role: "confirmation" },
            },
          },
          coinbase: {
            platformEnabled: true,
            status: "spot_only",
            markets: {
              spot: { enabled: true, status: "enabled", role: "spot_confirmation" },
              perp: { enabled: false, status: "disabled", role: "optional" },
            },
          },
          okx: {
            platformEnabled: false,
            status: "disabled",
            markets: {
              perp: { enabled: false, status: "disabled", role: "optional" },
            },
          },
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
          dominantVenueNetContributionShare: 0.986,
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
          marketType: "perp",
          sourceRole: "primary",
          thresholdProfile: "binance_bitfinex",
          activeSources: {
            contract: [
              { exchange: "binance", marketType: "perp", sourceRole: "primary", enabled: true, status: "active" },
              { exchange: "bitfinex", marketType: "perp", sourceRole: "confirmation", enabled: true, status: "configured" },
              { exchange: "coinbase", marketType: "perp", sourceRole: "optional", enabled: false, status: "spot_only" },
              { exchange: "okx", marketType: "perp", sourceRole: "disabled", enabled: false, status: "disabled" },
            ],
            spot: [
              { exchange: "binance", marketType: "spot", sourceRole: "primary", enabled: true, status: "configured" },
              { exchange: "coinbase", marketType: "spot", sourceRole: "spot_confirmation", enabled: true, status: "spot_only" },
            ],
          },
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
              buyShare: 0.929,
              sellShare: 0.071,
              netVolumeBtc: 2410,
              dominance: 0.858,
              netContributionShare: 0.601,
            },
            {
              exchange: "okx",
              buyVolumeBtc: 1780,
              sellVolumeBtc: 180,
              totalVolumeBtc: 1960,
              buyShare: 0.908,
              sellShare: 0.092,
              netVolumeBtc: 1600,
              dominance: 0.816,
              netContributionShare: 0.399,
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
        thresholdProfile: "binance_bitfinex",
        activeExchangeCount: 2,
        enabledExchanges: ["binance", "bitfinex"],
        disabledExchanges: ["okx"],
        activeContractExchanges: ["binance", "bitfinex"],
        direction: "sell",
        latestDirection: "sell",
        latestSeverity: "critical",
        signalCount: 1,
        readOnly: true,
        enabled: true,
        dryRun: true,
        contractDataQuality: 72,
        spotDataQuality: 84,
        overallDataQuality: 77,
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
        platforms: {},
      },
      items: [],
      error: null,
    }),
  ),
  fetchContractWhaleEvents: vi.fn(() =>
    Promise.resolve({
      items: [
        {
          id: 7,
          symbol: "BTC",
          startedAt: 1_700_000_000_000,
          endedAt: 1_700_000_900_000,
          peakAt: 1_700_000_300_000,
          regimeType: "main_force_long_build",
          severity: "Major",
          peakMainForceScore: 88,
          peakExtremeImpactScore: 61,
          peakStructureBias: 64,
          confidence: 76,
          mainForceConfirmed: true,
          extremeImpactConfirmed: false,
          liquidationDriven: false,
          reasons: {
            finalResult: "高概率主力建多，不是单纯清算推动。",
          },
        },
      ],
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
    expect(screen.getAllByText("Binance+Bitfinex").length).toBeGreaterThan(0);
    expect(screen.getByText("当前统计数据源：Binance · Binance+Bitfinex")).toBeInTheDocument();
    expect(screen.getByText("合约数据质量 95/100 · 现货数据质量 78/100 · 总体 88/100")).toBeInTheDocument();
    expect(screen.getAllByText("Binance").length).toBeGreaterThan(0);
    expect(screen.getAllByText("在线")).toHaveLength(1);
    expect(screen.getAllByText("未启用").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Bitfinex").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Coinbase").length).toBeGreaterThan(0);
    expect(screen.getByText("重连中")).toBeInTheDocument();
    expect(screen.getByText("现货专用")).toBeInTheDocument();
    expect(screen.getByText("延迟 120ms")).toBeInTheDocument();
    expect(screen.getByText("重连 3")).toBeInTheDocument();
    expect(screen.getByText("平台能力")).toBeInTheDocument();
    expect(screen.getByTestId("platform-capability-coinbase")).toBeInTheDocument();
    expect(screen.getByText("当前仅启用现货，用于现货确认和中长线结构评分。")).toBeInTheDocument();
    expect(screen.getByText("现货确认源")).toBeInTheDocument();
    expect(screen.getAllByText("主力拉盘").length).toBeGreaterThan(0);
    expect(screen.getByText("4,820 BTC")).toBeInTheDocument();
    expect(screen.getByText("$337M")).toBeInTheDocument();
    expect(screen.getAllByText((_, element) => hasPriceText(element?.textContent || "")).length).toBeGreaterThan(0);
    expect(screen.getByText("净买入 3,260 BTC")).toBeInTheDocument();
    expect(screen.getByText("67.6%")).toBeInTheDocument();
    expect(screen.getByText("9.4x")).toBeInTheDocument();
    expect(screen.getByText("P99.9")).toBeInTheDocument();
    expect(screen.getByText("+0.31%")).toBeInTheDocument();
    expect(screen.getByText("疑似强平 420 BTC / 8.7%")).toBeInTheDocument();
    expect(screen.getByText("+900 BTC / +1.20% OI上升")).toBeInTheDocument();
    expect(screen.getByText("+0.02% 偏多")).toBeInTheDocument();
    expect(screen.getByText("待推")).toBeInTheDocument();
    expect(screen.getByText("主力结构事件历史")).toBeInTheDocument();
    expect(screen.getByText("让你知道这里发生过什么主力行为")).toBeInTheDocument();
    expect(screen.getByTestId("main-force-event-7")).toBeInTheDocument();
    expect(screen.getByText("主力建多")).toBeInTheDocument();
    expect(screen.getByText("峰值主力评分")).toBeInTheDocument();
    expect(screen.getByText("非清算驱动")).toBeInTheDocument();
    expect(screen.getByText("已结束")).toBeInTheDocument();
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
    expect(fetchContractWhaleEvents).toHaveBeenCalledWith(
      expect.objectContaining({
        symbol: "BTC",
        limit: 12,
      }),
    );
  });

  it("shows a spot-only explanation when coinbase is selected in contract history", async () => {
    const user = userEvent.setup();
    fetchContractWhaleHistory.mockResolvedValueOnce({
      summary: {
        status: "calm",
        healthStatus: "healthy",
        healthReason: "primary_sources_recent",
        thresholdProfile: "binance_bitfinex",
        activeExchangeCount: 2,
        enabledExchanges: ["binance", "bitfinex"],
        disabledExchanges: ["okx"],
        activeContractExchanges: ["binance", "bitfinex"],
        direction: "neutral",
        latestDirection: "neutral",
        latestSeverity: "calm",
        signalCount: 0,
        readOnly: true,
        enabled: true,
        dryRun: true,
        contractDataQuality: 95,
        spotDataQuality: 78,
        overallDataQuality: 88,
        trend60s: {
          buyVolumeBtc: 0,
          sellVolumeBtc: 0,
          totalVolumeBtc: 0,
          netVolumeBtc: 0,
          dominance: 0,
          buyRatio: 0,
          sellRatio: 0,
          updatedAtMs: 1_700_000_000_000,
        },
        exchanges: {},
        platforms: {},
      },
      items: [],
      meta: {
        exchange: "coinbase",
        marketType: "perp",
        exchangeStatus: "spot_only",
        reason: "coinbase_perp_disabled",
      },
      error: null,
    });

    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.selectOptions(screen.getByLabelText("交易所"), "coinbase");

    await waitFor(() =>
      expect(fetchContractWhaleHistory).toHaveBeenLastCalledWith(
        expect.objectContaining({
          symbol: "BTC",
          exchange: "coinbase",
          limit: 50,
        }),
      ),
    );
    expect(screen.getByText("Coinbase 当前仅启用现货，未启用合约；本页只统计 perp 合约成交，因此不会返回 Coinbase 合约信号。")).toBeInTheDocument();
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
    expect(screen.getByText("Active Source Snapshot")).toBeInTheDocument();
    expect(screen.getByText("合约源")).toBeInTheDocument();
    expect(screen.getByText("现货源")).toBeInTheDocument();
    expect(screen.getByText("已参与")).toBeInTheDocument();
    expect(screen.getAllByText("仅现货").length).toBeGreaterThan(0);
    expect(screen.getByText("Coinbase · Perp")).toBeInTheDocument();
    expect(screen.getByText("OKX · Perp")).toBeInTheDocument();
    expect(screen.getAllByText((_, element) => hasPriceText(element?.textContent || "")).length).toBeGreaterThan(0);
    expect(screen.getByText("5s / 15s / 60s 窗口数据")).toBeInTheDocument();
    expect(screen.getByText("平台拆分")).toBeInTheDocument();
    expect(screen.getByText("主动买入：2,610 BTC")).toBeInTheDocument();
    expect(screen.getByText("买/卖占比：92.9% / 7.1%")).toBeInTheDocument();
    expect(screen.getByText("净流贡献：60.1%")).toBeInTheDocument();
    expect(screen.getByText("Dominant Venue Net Flow")).toBeInTheDocument();
    expect(screen.getByText("Raw Scoring Breakdown")).toBeInTheDocument();
    expect(screen.getByText("Volume Strength")).toBeInTheDocument();
    expect(screen.getByText("口径说明")).toBeInTheDocument();
    expect(screen.getByText(/方向强度 = abs/)).toBeInTheDocument();
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
