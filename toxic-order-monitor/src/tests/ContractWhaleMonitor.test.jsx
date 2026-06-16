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
  CWM_MAX_PRICE_DEVIATION_PCT: 5,
  fetchContractWhaleSummary: vi.fn(() =>
    Promise.resolve({
      summary: {
        status: "strong",
        healthStatus: "healthy",
        healthReason: "primary_sources_recent",
        thresholdProfile: "binance_bitfinex",
        thresholdProfileReason: "active_contract_sources=binance,bitfinex",
        configuredContractSources: ["binance", "bitfinex"],
        eligibleContractSources: ["binance", "bitfinex"],
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
        discordDryRunStats: {
          signals1h: 4,
          high1h: 1,
          critical1h: 2,
          s1h: 1,
          wouldSend1h: 3,
          skippedLowScore1h: 1,
        },
        marketStructureLite: {
          status: "confirmed",
          regimeType: "main_force_long_build",
          mainForceScore: 84,
          extremeImpactScore: 62,
          structureBias: 64,
          confidence: 76,
          dataQuality: 88,
          spotScore: 71,
          contractScore: 94,
          crossConfirmScore: 75,
          mainForceConfirmed: true,
          extremeImpactConfirmed: false,
          reason: "合约主动买入与现货方向确认，主力建多概率提高。",
        },
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
        thresholdProfileReason: "active_contract_sources=binance,bitfinex",
        configuredContractSources: ["binance", "bitfinex"],
        eligibleContractSources: ["binance", "bitfinex"],
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
        discordDryRunStats: {
          signals1h: 4,
          high1h: 1,
          critical1h: 2,
          s1h: 1,
          wouldSend1h: 3,
          skippedLowScore1h: 1,
        },
        marketStructureLite: {
          status: "confirmed",
          regimeType: "main_force_long_build",
          mainForceScore: 84,
          extremeImpactScore: 62,
          structureBias: 64,
          confidence: 76,
          dataQuality: 88,
          spotScore: 71,
          contractScore: 94,
          crossConfirmScore: 75,
          mainForceConfirmed: true,
          extremeImpactConfirmed: false,
          reason: "合约主动买入与现货方向确认，主力建多概率提高。",
        },
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
          mainForceScore: 87,
          spotScore: 81,
          contractScore: 94,
          totalVolumeBtc: 4820,
          netVolumeBtc: 3260,
          totalNotionalUsd: 337_000_000,
          dominance: 0.676,
          orderPriceUsd: 69_917,
          currentMarketPriceUsd: 70_000,
          priceDeviationPct: 0.1186,
          priceDeviationFiltered: false,
          priceMovePct: 0.31,
          priceMove15sPct: 0.31,
          priceResponseType: "trend_follow_up",
          mainExchange: "binance",
          dominantVenueNetContributionShare: 0.986,
          dynamicMultiple: 9.4,
          dynamicBaselineBtc: 512,
          dynamicThresholdLevel: "critical",
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
          thresholdProfileReason: "active_contract_sources=binance,bitfinex",
          configuredContractSources: ["binance", "bitfinex"],
          eligibleContractSources: ["binance", "bitfinex"],
          activeContractSources: ["binance", "bitfinex"],
          activeSources: {
            contract: [
              { exchange: "binance", marketType: "perp", sourceRole: "primary", enabled: true, status: "active" },
              { exchange: "bitfinex", marketType: "perp", sourceRole: "confirmation", enabled: true, status: "configured" },
            ],
            spot: [
              { exchange: "binance", marketType: "spot", sourceRole: "primary", enabled: true, status: "configured" },
              { exchange: "coinbase", marketType: "spot", sourceRole: "spot_confirmation", enabled: true, status: "spot_only" },
            ],
          },
          spotConfirmation: {
            status: "confirmed",
            confirmationType: "confirms_contract_direction",
            direction: "buy",
            score: 81,
            latestSignalId: "spot-whale:BTC:15:1700000000000:buy",
            latestSignalAt: 1_700_000_000_000,
            signalType: "spot_aggressive_buy",
            severity: "high",
            totalVolumeBtc: 820,
            netVolumeBtc: 610,
            dominance: 0.744,
            coinbasePremiumPct: 0.018,
            finalResult: "现货主动买入跟随合约方向",
          },
          dataQuality: 91,
          scoreBreakdown: {
            volumeScore: 23.6,
            notionalScore: 10.5,
            dynamicAnomalyScore: 18.8,
            directionalStrengthScore: 10.6,
            priceResponseScore: 15,
            multiSourceScore: 8,
            dataQualityScore: 4.6,
            dominantVenueScore: 4.8,
            oiContextScore: 4,
            penaltyScore: -10,
            finalScore: 89.9,
          },
          discordEligible: true,
          discordSent: false,
          discordReason: "critical_or_s_gate",
          discordWouldSend: true,
          mergedFrom: ["contract-whale:BTC:5:1700000000000:buy"],
          cluster: {
            clusterId: "cwm-cluster:BTC:buy:14166666",
            signalCount: 3,
            dominantIntent: "liquidity_probe_buy",
            startedAt: 1_700_000_000_000,
            updatedAt: 1_700_000_090_000,
            durationMs: 90_000,
            intensity: 0.91,
            priceRangePct: 0.18,
          },
          persistence: {
            persistenceScore: 0.82,
            signalHalfLifeMs: 60_000,
            regimeStability: 0.67,
            redundantWithPrevious: true,
            redundantReason: "same_intent_within_60s",
          },
          whaleAction: {
            ts: 1_700_000_000_000,
            symbol: "BTC",
            actionType: "aggressive_buy",
            volume: 3260,
            priceImpact: 0.31,
            exchange: "binance",
          },
          trajectory: {
            trajectoryId: "whale-trajectory:cwm-cluster:BTC:buy:14166666",
            startTs: 1_700_000_000_000,
            endTs: 1_700_000_090_000,
            durationMs: 90_000,
            actions: [
              { ts: 1_700_000_000_000, symbol: "BTC", actionType: "liquidity_probe", volume: 1000, priceImpact: 0.08, exchange: "binance" },
              { ts: 1_700_000_090_000, symbol: "BTC", actionType: "aggressive_buy", volume: 3260, priceImpact: 0.31, exchange: "bitfinex" },
            ],
            intent: "accumulation",
            regimePath: ["manipulation", "accumulation"],
            stealthProfile: {
              gamma: 0.73,
              fragmentation: 0.66,
              entropy: 0.82,
              crossExchangeDispersion: 0.33,
            },
            aggressivenessCurve: [0.41, 0.94],
            conclusion: "连续买方压力和承接行为占优，疑似主力分批吸筹。",
          },
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
  normalizePlatformStatus: vi.fn((platform) => {
    const status = String(platform?.status || "disabled").toLowerCase();
    const enabled = Boolean(platform?.platformEnabled ?? platform?.enabled);
    if (!enabled || status === "disabled") {
      return {
        key: "disabled",
        label: "未启用",
        description: "当前平台未启用，不参与合约监控、现货确认或 Discord gate。",
        tone: "slate",
      };
    }
    if (status === "spot_only") {
      return {
        key: "spot_only",
        label: "现货专用",
        description: "当前仅启用现货确认，不参与 CWM 合约成交量、阈值和 Discord gate。",
        tone: "cyan",
      };
    }
    return {
      key: "active",
      label: "运行中",
      description: "平台能力已配置，按启用 market role 参与对应统计。",
      tone: "emerald",
    };
  }),
  normalizeMarketStatus: vi.fn((market, marketType) => {
    const status = String(market?.status || (market?.enabled ? "enabled" : "disabled")).toLowerCase();
    const role = String(market?.role || "").toLowerCase();
    const enabled = Boolean(market?.enabled);
    const hasRecentTrade = Number.isFinite(Number(market?.lastTradeAt)) && Number(market?.lastTradeAt) > 0;
    if (!enabled || status === "disabled") {
      return { key: "disabled", label: "未启用", detail: "不参与当前合约监控", tone: "slate" };
    }
    if (status === "spot_only" || role === "spot_confirmation") {
      return {
        key: "spot_only",
        label: marketType === "spot" ? "现货确认源" : "现货专用",
        detail: "只用于现货确认，不进入合约成交量统计。",
        tone: "cyan",
      };
    }
    if ((status === "active" || status === "connected") && hasRecentTrade) {
      return { key: "active", label: "运行中", detail: "该 market 已参与对应统计。", tone: "emerald" };
    }
    return { key: "waiting_for_data", label: "已启用 / 等待数据", detail: "配置已启用，等待 collector 或下一笔成交更新。", tone: "cyan" };
  }),
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
    expect(screen.getByText("最近 60 秒主动成交流只表示 flow，不用于判断平台在线 / 离线状态。")).toBeInTheDocument();
    expect(screen.getByText("合约数据质量 95/100 · 现货数据质量 78/100 · 总体 88/100 · active_contract_sources=binance,bitfinex")).toBeInTheDocument();
    expect(screen.getByTestId("platform-status-strip")).toBeInTheDocument();
    expect(screen.getByText("平台状态")).toBeInTheDocument();
    expect(screen.getByText(/合约源 Binance Perp, Bitfinex Perp/)).toBeInTheDocument();
    expect(screen.getByText(/现货确认 Coinbase Spot, Binance Spot, Bitfinex Spot/)).toBeInTheDocument();
    expect(screen.getByText("Coinbase 仅现货确认，不参与 CWM 合约成交量、阈值和 Discord gate。")).toBeInTheDocument();
    expect(screen.getAllByText("Binance").length).toBeGreaterThan(0);
    expect(screen.getAllByText("运行中").length).toBeGreaterThan(0);
    expect(screen.getAllByText("未启用").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Bitfinex").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Coinbase").length).toBeGreaterThan(0);
    expect(screen.getByTestId("platform-status-chip-binance")).toHaveTextContent("Binance");
    expect(screen.getByTestId("platform-status-chip-bitfinex")).toHaveTextContent("重连中");
    expect(screen.getByTestId("platform-status-chip-coinbase")).toHaveTextContent("仅现货");
    expect(screen.getByTestId("platform-status-chip-okx")).toHaveTextContent("未启用");
    expect(screen.queryByTestId("platform-capability-coinbase")).not.toBeInTheDocument();
    expect(screen.queryByText("SPOT")).not.toBeInTheDocument();
    expect(screen.queryByText("FUNDING")).not.toBeInTheDocument();
    expect(screen.getByText("Whale Behavior Timeline")).toBeInTheDocument();
    expect(screen.getByText("主力行为轨迹（辅助）")).toBeInTheDocument();
    expect(screen.getByText("Whale Entity List")).toBeInTheDocument();
    expect(screen.getByText("Trajectory Timeline")).toBeInTheDocument();
    expect(screen.getByText("Stealth Curve (gamma)")).toBeInTheDocument();
    expect(screen.getByText("Hazard Curve (lambda proxy)")).toBeInTheDocument();
    expect(screen.getByText("逐条合约信号")).toBeInTheDocument();
    expect(screen.getByText(/每一次 CWM 检测到的合约信号都会在这里展示/)).toBeInTheDocument();
    expect(screen.getByTestId("raw-contract-whale-signals")).toBeInTheDocument();
    expect(screen.getAllByText("主力拉盘").length).toBeGreaterThan(0);
    expect(
      screen.getAllByText((_, element) => {
        const text = element?.textContent || "";
        return text.includes("BTC") && text.includes("$69,917");
      }).length,
    ).toBeGreaterThan(0);
    expect(screen.getByText("4,820 BTC")).toBeInTheDocument();
    expect(screen.getByText("$337M")).toBeInTheDocument();
    expect(screen.getAllByText((_, element) => hasPriceText(element?.textContent || "")).length).toBeGreaterThan(0);
    expect(screen.getByText("0.12%")).toBeInTheDocument();
    expect(screen.getAllByText("87/100").length).toBeGreaterThan(0);
    expect(screen.getByText("S 81 / C 94")).toBeInTheDocument();
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
    expect(screen.getByText("结构判断")).toBeInTheDocument();
    expect(screen.getAllByText("主力评分").length).toBeGreaterThan(0);
    expect(screen.getByText("现货确认")).toBeInTheDocument();
    expect(screen.getByText("Dry-run 1h")).toBeInTheDocument();
    expect(screen.getByText("would-send 3")).toBeInTheDocument();
    expect(screen.getByTestId("main-force-event-7")).toBeInTheDocument();
    expect(screen.getByText("主力建多")).toBeInTheDocument();
    expect(screen.getByText("峰值主力评分")).toBeInTheDocument();
    expect(screen.getByText("非清算驱动")).toBeInTheDocument();
    expect(screen.getByText("已结束")).toBeInTheDocument();
  });

  it("uses the signal symbol as the base unit for ETH contract flow values", async () => {
    fetchContractWhaleLatest.mockResolvedValueOnce({
      summary: {
        status: "active",
        healthStatus: "healthy",
        thresholdProfile: "binance_bitfinex",
        thresholdProfileReason: "active_contract_sources=binance,bitfinex",
        latestDirection: "buy",
        latestSeverity: "medium",
        signalCount: 1,
        readOnly: true,
        enabled: true,
        dryRun: true,
        contractDataQuality: 78,
        spotDataQuality: 60,
        overallDataQuality: 71,
        discordDryRunStats: {},
        marketStructureLite: {},
        trend60s: {
          symbol: "ETH",
          buyVolumeBtc: 688,
          sellVolumeBtc: 73,
          totalVolumeBtc: 761,
          netVolumeBtc: 614,
          dominance: 0.807,
          buyRatio: 0.904,
          sellRatio: 0.096,
        },
        exchanges: {},
        platforms: {},
      },
      items: [
        {
          id: "eth-contract-whale-row",
          ts: 1_700_000_100_000,
          symbol: "ETH",
          windowSec: 60,
          signalType: "aggressive_buy",
          direction: "buy",
          severity: "medium",
          score: 34,
          mainForceScore: 34,
          spotScore: 27,
          contractScore: 21,
          totalVolumeBtc: 16869,
          netVolumeBtc: 610,
          totalNotionalUsd: 28_000_000,
          dominance: 0.036,
          triggerPriceUsd: 1675,
          priceDeviationPct: 0.04,
          priceMovePct: 0.03,
          mainExchange: "binance",
          liquidationSuspected: false,
          oiBias: "unknown",
          fundingBias: "unknown",
          exchanges: [],
          finalResult: "ETH 主动买入放大",
        },
      ],
      error: null,
    });

    render(<ContractWhaleMonitor />);

    expect(await screen.findByText("16,869 ETH")).toBeInTheDocument();
    expect(screen.getByText("净买入 610 ETH")).toBeInTheDocument();
    expect(screen.getByText("总量 761 ETH · dominance 80.7%")).toBeInTheDocument();
    expect(screen.queryByText("净买入 610 BTC")).not.toBeInTheDocument();
  });

  it("uses the selected symbol as the trend unit when ETH summary omits trend symbol", async () => {
    const user = userEvent.setup();
    fetchContractWhaleLatest
      .mockResolvedValueOnce({
        summary: {
          status: "calm",
          healthStatus: "healthy",
          latestDirection: "neutral",
          latestSeverity: "calm",
          signalCount: 0,
          readOnly: true,
          enabled: true,
          dryRun: true,
          contractDataQuality: 90,
          spotDataQuality: 80,
          overallDataQuality: 85,
          discordDryRunStats: {},
          marketStructureLite: {},
          trend60s: {
            symbol: "BTC",
            buyVolumeBtc: 0,
            sellVolumeBtc: 0,
            totalVolumeBtc: 0,
            netVolumeBtc: 0,
            dominance: 0,
            buyRatio: 0,
            sellRatio: 0,
          },
          exchanges: {},
          platforms: {},
        },
        items: [],
        error: null,
      })
      .mockResolvedValueOnce({
        summary: {
          status: "active",
          healthStatus: "healthy",
          latestDirection: "buy",
          latestSeverity: "medium",
          signalCount: 1,
          readOnly: true,
          enabled: true,
          dryRun: true,
          contractDataQuality: 78,
          spotDataQuality: 60,
          overallDataQuality: 71,
          discordDryRunStats: {},
          marketStructureLite: {},
          trend60s: {
            buyVolumeBtc: 688,
            sellVolumeBtc: 73,
            totalVolumeBtc: 761,
            netVolumeBtc: 614,
            dominance: 0.807,
            buyRatio: 0.904,
            sellRatio: 0.096,
          },
          exchanges: {},
          platforms: {},
        },
        items: [
          {
            id: "eth-selected-contract-whale-row",
            ts: 1_700_000_100_000,
            symbol: "ETH",
            windowSec: 60,
            signalType: "aggressive_buy",
            direction: "buy",
            severity: "medium",
            score: 34,
            mainForceScore: 34,
            spotScore: 27,
            contractScore: 21,
            totalVolumeBtc: 16869,
            netVolumeBtc: 610,
            totalNotionalUsd: 28_000_000,
            dominance: 0.036,
            triggerPriceUsd: 1675,
            priceDeviationPct: 0.04,
            priceMovePct: 0.03,
            mainExchange: "binance",
            exchanges: [],
            finalResult: "ETH 主动买入放大",
          },
        ],
        error: null,
      });

    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.selectOptions(screen.getByLabelText("币种"), "ETH");

    await waitFor(() => expect(fetchContractWhaleLatest).toHaveBeenLastCalledWith(50, "ETH"));
    expect(await screen.findByText("总量 761 ETH · dominance 80.7%")).toBeInTheDocument();
    expect(screen.getByText("净买入 610 ETH")).toBeInTheDocument();
    expect(screen.queryByText("总量 761 BTC · dominance 80.7%")).not.toBeInTheDocument();
    expect(screen.queryByText("净买入 610 BTC")).not.toBeInTheDocument();
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
    expect(screen.getByText("dry-run 会推送")).toBeInTheDocument();
    expect(screen.getAllByText("现货确认").length).toBeGreaterThan(0);
    expect(screen.getByText("现货与合约同向")).toBeInTheDocument();
    expect(screen.getByText("现货主动买入跟随合约方向")).toBeInTheDocument();
    expect(screen.getByText("Active Source Snapshot")).toBeInTheDocument();
    expect(screen.getByText("合约源")).toBeInTheDocument();
    expect(screen.getByText("现货源")).toBeInTheDocument();
    expect(screen.getByText("已参与")).toBeInTheDocument();
    expect(screen.getAllByText("仅现货").length).toBeGreaterThan(0);
    expect(screen.getByText("Coinbase · Spot")).toBeInTheDocument();
    expect(screen.queryByText("Coinbase · Perp")).not.toBeInTheDocument();
    expect(screen.queryByText("OKX · Perp")).not.toBeInTheDocument();
    expect(screen.getAllByText((_, element) => hasPriceText(element?.textContent || "")).length).toBeGreaterThan(0);
    expect(screen.getByText("当前价格")).toBeInTheDocument();
    expect(screen.getByText("$70,000")).toBeInTheDocument();
    expect(screen.getByText("信号价格")).toBeInTheDocument();
    expect(screen.getAllByText("价格偏离").length).toBeGreaterThan(0);
    expect(screen.getByText("未过滤（阈值 5%）")).toBeInTheDocument();
    expect(screen.getByText("Main Force Score")).toBeInTheDocument();
    expect(screen.getByText("Spot Score")).toBeInTheDocument();
    expect(screen.getByText("Contract Score")).toBeInTheDocument();
    expect(screen.getByText("5s / 15s / 60s 窗口数据")).toBeInTheDocument();
    expect(screen.getByText("平台拆分")).toBeInTheDocument();
    expect(screen.getByText("主动买入：2,610 BTC")).toBeInTheDocument();
    expect(screen.getByText("买/卖占比：92.9% / 7.1%")).toBeInTheDocument();
    expect(screen.getByText("净流贡献：60.1%")).toBeInTheDocument();
    expect(screen.getByText("Dominant Venue")).toBeInTheDocument();
    expect(screen.getByText("Score Breakdown")).toBeInTheDocument();
    expect(screen.getByText("Signal Cluster / Persistence")).toBeInTheDocument();
    expect(screen.getByText("cwm-cluster:BTC:buy:14166666")).toBeInTheDocument();
    expect(screen.getAllByText("买方流动性测试").length).toBeGreaterThan(0);
    expect(screen.getAllByText("3").length).toBeGreaterThan(0);
    expect(screen.getAllByText("1m 30s").length).toBeGreaterThan(0);
    expect(screen.getAllByText("82.0%").length).toBeGreaterThan(0);
    expect(screen.getByText("是：60 秒内同意图重复投影")).toBeInTheDocument();
    expect(screen.getByText(/同一主力意图轨迹/)).toBeInTheDocument();
    expect(screen.getByText("Whale Trajectory")).toBeInTheDocument();
    expect(screen.getByText("whale-trajectory:cwm-cluster:BTC:buy:14166666")).toBeInTheDocument();
    expect(screen.getAllByText("隐蔽吸筹").length).toBeGreaterThan(0);
    expect(screen.getAllByText("操控 -> 吸筹").length).toBeGreaterThan(0);
    expect(screen.getAllByText("连续买方压力和承接行为占优，疑似主力分批吸筹。").length).toBeGreaterThan(0);
    expect(screen.getAllByText("1. 流动性测试").length).toBeGreaterThan(0);
    expect(screen.getAllByText("2. 主动买入").length).toBeGreaterThan(0);
    expect(screen.getByText("Volume Strength")).toBeInTheDocument();
    expect(screen.getByText("Dynamic Baseline")).toBeInTheDocument();
    expect(screen.getByText("512 BTC")).toBeInTheDocument();
    expect(screen.getByText("Critical 动态异常")).toBeInTheDocument();
    expect(screen.getByText("价格响应")).toBeInTheDocument();
    expect(screen.getAllByText("买盘推动上涨").length).toBeGreaterThan(0);
    expect(screen.getByText(/成交流和价格方向一致/)).toBeInTheDocument();
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
