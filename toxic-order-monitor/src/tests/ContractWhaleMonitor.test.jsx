import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ContractWhaleMonitor, { signalDisplayType } from "../components/ContractWhaleMonitor.jsx";
import {
  fetchContractEventDebugCounts,
  fetchContractEvents,
  fetchContractRetentionStatus,
  fetchContractWhaleLatencyDebug,
  fetchContractWhaleEvents,
  fetchContractWhaleHistory,
  fetchContractWhaleLatest,
  fetchContractWhaleRawFlowDebug,
  fetchContractWhaleSummary,
  fetchContractWhaleIntelligenceTerminal,
  fetchContractWhaleTradingDecisions,
  fetchFinalEvents,
  fetchFinalEventsV2,
} from "../api/contractWhale.js";

function hasPriceText(text) {
  return typeof text === "string" && text.includes("69,917");
}

describe("contract whale behavior labels", () => {
  it("does not call unconfirmed aggressive flow main force", () => {
    expect(
      signalDisplayType({
        signalType: "aggressive_buy",
        displaySignalType: "主力拉盘",
        behaviorState: "insufficient",
        behaviorType: "insufficient_evidence",
      }),
    ).toBe("主动买压");
    expect(
      signalDisplayType({
        signalType: "aggressive_sell",
        behaviorState: "confirmed",
        behaviorType: "new_short_build",
      }),
    ).toBe("主力建空");
  });
});

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
        noiseSuppression: {
          rawCandidates: 6,
          mergedEvents: 3,
          lifecycleEvents: 2,
          filteredEvents: 2,
          tradeableSetups: 2,
          suppressedDuplicates: 4,
          noiseReductionPct: 67,
        },
        tradeOpportunities: [
          {
            signalId: "contract-whale-row",
            rank: 1,
            setupType: "主力拉盘",
            action: "LONG",
            directionBias: "buy",
            tradeScore: 87,
            confidence: 79,
            severity: "s",
            windowSec: 15,
            regimeContext: "main_force_long_build",
            rationale: "多窗口主买一致，价格顺势跟随，属于可交易级主力拉盘。",
          },
          {
            signalId: "contract-whale-row-2",
            rank: 2,
            setupType: "下方吸收",
            action: "LONG",
            directionBias: "absorption",
            tradeScore: 73,
            confidence: 68,
            severity: "high",
            windowSec: 5,
            regimeContext: "downside_absorption",
            rationale: "卖压未能继续压低价格，吸收结构保持有效。",
          },
        ],
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
  fetchContractWhaleTradingDecisions: vi.fn(() =>
    Promise.resolve({
      symbol: "BTC",
      timestamp: 1_700_000_000_000,
      marketBias: "BULLISH",
      biasConfidence: 82,
      biasReason: "多头高分 setup 明显占优，且结构上下文为主力建多。",
      noiseSuppression: {
        rawCandidates: 6,
        mergedEvents: 3,
        lifecycleEvents: 2,
        filteredEvents: 2,
        tradeableSetups: 2,
        suppressedDuplicates: 4,
        noiseReductionPct: 67,
      },
      topSetups: [
        {
          semanticType: "decision_support",
          riskState: "low",
          signalId: "contract-whale-row",
          rank: 1,
          directionBias: "BULLISH_BIAS",
          setupType: "主力拉盘",
          score: 87,
          confidence: 79,
          confidenceLabel: "HIGH",
          regimeContext: "main_force_long_build",
          windowSec: 15,
          pressureZone: {
            lowPrice: 69810,
            highPrice: 69950,
            label: "69,810 - 69,950",
          },
          riskBoundary: {
            priceLevel: 69640,
            reason: "跌破主力吸收参考位，说明顺势跟随结构减弱。",
          },
          reasons: ["多窗口主买一致", "价格顺势跟随", "双交易所确认"],
        },
      ],
      noTradeZones: [
        {
          reason: "价格响应不足，当前更像低分震荡 chop。",
          rangeLabel: "69,900 - 70,040",
          lowPrice: 69900,
          highPrice: 70040,
        },
      ],
    }),
  ),
  fetchContractWhaleIntelligenceTerminal: vi.fn(() =>
    Promise.resolve({
      symbol: "BTC",
      timestamp: 1_700_000_000_000,
      marketRegime: {
        regime: "RANGING",
        confidence: 78,
        reason: "成交量活跃但价格延续性一般，结构更接近区间整理。",
      },
      liquidityBehaviors: [
        {
          behavior: "absorption",
          label: "Absorption",
          strengthScore: 84,
          confidence: 80,
          reason: "买方承接稳定，价格没有继续下破。",
          rangeLabel: "69,760 - 69,890",
        },
        {
          behavior: "fake_breakout",
          label: "Fake Breakout",
          strengthScore: 68,
          confidence: 61,
          reason: "冲高成交放大，但价格跟随不足。",
          rangeLabel: "69,980 - 70,040",
        },
      ],
      rankedEvents: [
        {
          signalId: "contract-whale-row",
          rank: 1,
          eventType: "主力拉盘",
          directionBias: "BUY",
          strengthScore: 87,
          strengthLabel: "HIGH",
          regimeAlignment: "aligned",
          liquidityBehavior: "breakout_pressure",
          windowSec: 15,
          rationale: "多窗口主买一致，且价格顺势跟随。",
        },
        {
          signalId: "contract-whale-row-2",
          rank: 2,
          eventType: "下方吸收",
          directionBias: "ABSORPTION",
          strengthScore: 81,
          strengthLabel: "HIGH",
          regimeAlignment: "supportive",
          liquidityBehavior: "absorption",
          windowSec: 5,
          rationale: "卖压释放后价格守住关键区间。",
        },
      ],
      opportunityMap: [
        {
          zoneType: "absorption_zone",
          label: "Absorption Zone",
          lowPrice: 69760,
          highPrice: 69890,
          rangeLabel: "69,760 - 69,890",
          strengthScore: 84,
          description: "该区间出现稳定承接，适合作为结构观察点。",
        },
        {
          zoneType: "fake_breakout_risk_zone",
          label: "Fake Breakout Risk",
          lowPrice: 69980,
          highPrice: 70040,
          rangeLabel: "69,980 - 70,040",
          strengthScore: 68,
          description: "冲高但价格未能延续，警惕假突破。",
        },
      ],
      noiseSuppression: {
        rawCandidates: 6,
        mergedEvents: 3,
        lifecycleEvents: 2,
        filteredEvents: 2,
        tradeableSetups: 2,
        suppressedDuplicates: 4,
        noiseReductionPct: 67,
      },
      signalCompression: {
        qualityScore: 82,
        topSignalCount: 2,
        discardedCount: 4,
        compressionReason: "cross-window dedup + quality gating",
      },
      tradeIdeas: [
        {
          semanticType: "decision_support",
          riskState: "low",
          signalId: "contract-whale-row",
          rank: 1,
          setupType: "Absorption continuation",
          directionBias: "BULLISH_BIAS",
          score: 87,
          confidence: 84,
          confidenceLabel: "HIGH",
          pressureZone: {
            lowPrice: 69810,
            highPrice: 69950,
            label: "69,810 - 69,950",
          },
          riskBoundary: {
            priceLevel: 69640,
            reason: "跌破主力吸收参考位，说明当前结构支持减弱。",
          },
          structureContext: "absorption + dominance + sweep",
          regimeContext: "TRENDING_UP",
          windowSec: 15,
        },
      ],
      riskContext: {
        fakeBreakoutRisk: "HIGH",
        summary: "当前存在较强假突破风险，交易参考需要让位于风险抑制。",
        noTradeZones: [
          {
            reason: "价格响应不足，当前更像低分震荡 chop。",
            rangeLabel: "69,900 - 70,040",
            lowPrice: 69900,
            highPrice: 70040,
          },
        ],
      },
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
        noiseSuppression: {
          rawCandidates: 6,
          mergedEvents: 3,
          lifecycleEvents: 2,
          filteredEvents: 2,
          tradeableSetups: 2,
          suppressedDuplicates: 4,
          noiseReductionPct: 67,
        },
        tradeOpportunities: [
          {
            signalId: "contract-whale-row",
            rank: 1,
            setupType: "主力拉盘",
            action: "LONG",
            directionBias: "buy",
            tradeScore: 87,
            confidence: 79,
            severity: "s",
            windowSec: 15,
            regimeContext: "main_force_long_build",
            rationale: "多窗口主买一致，价格顺势跟随，属于可交易级主力拉盘。",
          },
          {
            signalId: "contract-whale-row-2",
            rank: 2,
            setupType: "下方吸收",
            action: "LONG",
            directionBias: "absorption",
            tradeScore: 73,
            confidence: 68,
            severity: "high",
            windowSec: 5,
            regimeContext: "downside_absorption",
            rationale: "卖压未能继续压低价格，吸收结构保持有效。",
          },
        ],
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
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_buy:1700000000000",
            status: "active",
            startTime: 1_700_000_000_000,
            lastUpdateTime: 1_700_000_000_000,
            volumeAccumulated: 4820,
            oiAccumulated: 900,
            updateCount: 2,
          },
          eventQuality: {
            qualityScore: 0.86,
            mergeSimilarityScore: 0.91,
            valid: true,
            falseEventFlags: [],
          },
        },
        {
          id: "contract-whale-closed-row",
          ts: 1_699_999_820_000,
          symbol: "BTC",
          windowSec: 15,
          signalType: "aggressive_sell",
          direction: "sell",
          severity: "medium",
          score: 44,
          mainForceScore: 44,
          spotScore: 30,
          contractScore: 44,
          totalVolumeBtc: 760,
          netVolumeBtc: -520,
          totalNotionalUsd: 53_000_000,
          dominance: 0.684,
          orderPriceUsd: 69_800,
          currentMarketPriceUsd: 70_000,
          priceDeviationPct: 0.286,
          priceDeviationFiltered: false,
          priceMovePct: -0.12,
          priceResponseType: "no_clear_response",
          mainExchange: "binance",
          marketType: "perp",
          sourceRole: "primary",
          dataQuality: 86,
          discordEligible: false,
          discordSent: false,
          discordReason: "display_only",
          discordWouldSend: false,
          mergedFrom: [],
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_sell:1699999820000",
            status: "closed",
            startTime: 1_699_999_820_000,
            lastUpdateTime: 1_699_999_820_000,
            volumeAccumulated: 760,
            oiAccumulated: 0,
            updateCount: 1,
          },
          eventQuality: {
            qualityScore: 0.67,
            mergeSimilarityScore: 0.5,
            valid: true,
            falseEventFlags: [],
          },
          finalResult: "上一段主动卖出事件已结束",
        },
      ],
      error: null,
    }),
  ),
  fetchContractWhaleRawFlowDebug: vi.fn(() =>
    Promise.resolve({
      symbol: "BTC",
      range: "24h",
      config: {
        appRequestedSymbol: "BTC-PERP",
        querySymbol: "BTC",
      },
      normalizer: {
        connectorSymbolMismatch: false,
      },
      contractFlow1s: {
        exactSymbolRows: 12,
      },
      diagnosis: {
        status: "raw_flow_available",
        primaryReason: "raw_flow_present",
        details: [],
      },
      error: null,
    }),
  ),
  fetchContractWhaleLatencyDebug: vi.fn(() =>
    Promise.resolve({
      symbol: "BTC",
      range: "24h",
      serverTime: 1_700_000_000_000,
      latest: {
        count: 1,
        maxTs: 1_700_000_000_000,
        ageSec: 0,
        staleCount: 0,
      },
      contractEvents: {
        count: 1,
        maxEventTs: 1_700_000_000_000,
        lagSec: 0,
        lagVsLatestSec: 0,
        cacheAgeSec: 0,
        cacheTtlSec: 5,
      },
      finalEventsV2: {
        activeCount: 1,
        closedCount: 0,
        maxEventTs: 1_700_000_000_000,
        projectionLagSec: 0,
        cacheAgeSec: 0,
        cacheTtlSec: 10,
        generatedAt: 1_700_000_000_000,
      },
      flow: {
        updatedAt: 1_700_000_000_000,
        flowLagSec: 0,
      },
      diagnosis: {
        layer: "ok",
        reason: "in_sync",
      },
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
  fetchContractEvents: vi.fn(() =>
    Promise.resolve({
      items: [
        {
          id: "contract-event-row",
          eventId: "cwm-event:BTC:aggressive_buy:1700000000000",
          sourceSignalId: "contract-whale-row",
          ts: 1_700_000_000_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
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
          triggerPriceUsd: 69_917,
          orderPriceUsd: 69_917,
          currentMarketPriceUsd: 70_000,
          priceDeviationPct: 0.1186,
          priceDeviationFiltered: false,
          priceMovePct: 0.31,
          mainExchange: "binance",
          dynamicMultiple: 9.4,
          percentileLevel: 99.9,
          liquidationSuspected: true,
          liquidationLongBtc: 420,
          liquidationRatio: 0.087,
          oiChange5mBtc: 900,
          oiChangePct: 1.2,
          oiBias: "rising",
          fundingRate: 0.00018,
          fundingBias: "long",
          discordEligible: true,
          discordSent: false,
          discordReason: "critical_or_s_gate",
          discordWouldSend: true,
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_buy:1700000000000",
            status: "active",
            startTime: 1_700_000_000_000,
            lastUpdateTime: 1_700_000_000_000,
            volumeAccumulated: 4820,
            updateCount: 2,
          },
          eventQuality: {
            qualityScore: 0.86,
            mergeSimilarityScore: 0.91,
            valid: true,
            falseEventFlags: [],
          },
          status: "active",
          source: "contract_whale_signals",
          isRetentionProtected: false,
          retentionReason: null,
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      error: null,
    }),
  ),
  fetchContractEventDebugCounts: vi.fn(() =>
    Promise.resolve({
      symbol: "BTC",
      range: "24h",
      generatedAt: "2026-06-27T00:00:00Z",
      db: {
        contractWhaleSignalsTotal24h: 1,
        contractWhaleSignalsBtc24h: 1,
        oldestTs: 1_700_000_000_000,
        newestTs: 1_700_000_000_000,
      },
      apiQuery: {
        matchedBeforeFilter: 1,
        matchedAfterSymbolFilter: 1,
        matchedAfterRangeFilter: 1,
        matchedAfterSeverityFilter: null,
        matchedAfterWindowFilter: null,
        matchedAfterDirectionFilter: null,
        returnedItems: 1,
        limit: 100,
      },
      visibility: {
        visibleCount: 1,
        hiddenCount: 0,
        hiddenReasons: {
          priceDeviationGt5pct: 0,
          missingPrice: 0,
          badQuality: 0,
          disabledMonitor: 0,
          unknown: 0,
        },
      },
      latest: {
        latestCount: 2,
        latestSymbols: ["BTC", "BTC"],
      },
      finalEventsV2: {
        activeCount: 1,
        closedCount: 0,
      },
      latestVsHistory: [
        {
          latestEventId: "latest-visible",
          symbol: "BTC",
          ts: 1_700_000_000_000,
          existsInHistory: true,
          historyEventId: "cwm-event:BTC:aggressive_buy:1700000000000",
          notInHistoryReason: null,
        },
        {
          latestEventId: "latest-pending",
          symbol: "BTC",
          ts: 1_700_000_050_000,
          existsInHistory: false,
          historyEventId: null,
          notInHistoryReason: "latest_snapshot_not_persisted_yet",
        },
      ],
      finalEventsProjection: {
        source: "contract_whale_signals",
        rawSignals: 1,
        afterFilter: 1,
        mergedEvents: 1,
        active: 1,
        closed: 0,
        range: "24h",
      },
      error: null,
    }),
  ),
  fetchFinalEvents: vi.fn(() =>
    Promise.resolve({
      count: 2,
      items: [
        {
          id: "cwm-event:BTC:aggressive_buy:1700000000000",
          finalEventId: "cwm-event:BTC:aggressive_buy:1700000000000",
          sourceSignalId: "contract-whale-row",
          ts: 1_700_000_000_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
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
          triggerPriceUsd: 69_917,
          orderPriceUsd: 69_917,
          currentMarketPriceUsd: 70_000,
          priceDeviationPct: 0.1186,
          priceDeviationFiltered: false,
          priceMovePct: 0.31,
          mainExchange: "binance",
          dynamicMultiple: 9.4,
          percentileLevel: 99.9,
          liquidationSuspected: true,
          liquidationLongBtc: 420,
          liquidationRatio: 0.087,
          oiChange5mBtc: 900,
          oiChangePct: 1.2,
          oiBias: "rising",
          fundingRate: 0.00018,
          fundingBias: "long",
          discordEligible: true,
          discordSent: false,
          discordReason: "critical_or_s_gate",
          discordWouldSend: true,
          mergedFrom: ["contract-whale:BTC:5:1700000000000:buy"],
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_buy:1700000000000",
            status: "active",
            startTime: 1_700_000_000_000,
            lastUpdateTime: 1_700_000_000_000,
            volumeAccumulated: 4820,
            oiAccumulated: 900,
            updateCount: 2,
          },
          eventQuality: {
            qualityScore: 0.86,
            mergeSimilarityScore: 0.91,
            valid: true,
            falseEventFlags: [],
          },
          marketDriver: {
            primaryDriver: "whale_intent",
          },
          liquidationForce: {
            primaryDriver: "whale_initiated_flow",
          },
        },
        {
          id: "cwm-event:BTC:aggressive_sell:1699999820000",
          finalEventId: "cwm-event:BTC:aggressive_sell:1699999820000",
          sourceSignalId: "contract-whale-closed-row",
          ts: 1_699_999_820_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
          windowSec: 15,
          signalType: "aggressive_sell",
          direction: "sell",
          severity: "medium",
          score: 44,
          mainForceScore: 44,
          spotScore: 30,
          contractScore: 44,
          totalVolumeBtc: 760,
          netVolumeBtc: -520,
          totalNotionalUsd: 53_000_000,
          dominance: 0.684,
          triggerPriceUsd: 69_800,
          orderPriceUsd: 69_800,
          currentMarketPriceUsd: 70_000,
          priceDeviationPct: 0.286,
          priceDeviationFiltered: false,
          priceMovePct: -0.12,
          mainExchange: "binance",
          discordEligible: false,
          discordSent: false,
          discordReason: "display_only",
          discordWouldSend: false,
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_sell:1699999820000",
            status: "closed",
            startTime: 1_699_999_820_000,
            lastUpdateTime: 1_699_999_820_000,
            volumeAccumulated: 760,
            oiAccumulated: 0,
            updateCount: 1,
          },
          eventQuality: {
            qualityScore: 0.67,
            mergeSimilarityScore: 0.5,
            valid: true,
            falseEventFlags: [],
          },
        },
      ],
      error: null,
    }),
  ),
  fetchFinalEventsV2: vi.fn(() =>
    Promise.resolve({
      active: [
        {
          id: "cwm-event:BTC:aggressive_buy:1700000000000",
          finalEventId: "cwm-event:BTC:aggressive_buy:1700000000000",
          sourceSignalId: "contract-whale-row",
          ts: 1_700_000_000_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
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
          triggerPriceUsd: 69_917,
          orderPriceUsd: 69_917,
          currentMarketPriceUsd: 70_000,
          priceDeviationPct: 0.1186,
          priceDeviationFiltered: false,
          priceMovePct: 0.31,
          mainExchange: "binance",
          dynamicMultiple: 9.4,
          percentileLevel: 99.9,
          liquidationSuspected: true,
          liquidationLongBtc: 420,
          liquidationRatio: 0.087,
          oiChange5mBtc: 900,
          oiChangePct: 1.2,
          oiBias: "rising",
          fundingRate: 0.00018,
          fundingBias: "long",
          discordEligible: true,
          discordSent: false,
          discordReason: "critical_or_s_gate",
          discordWouldSend: true,
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_buy:1700000000000",
            status: "active",
            startTime: 1_700_000_000_000,
            lastUpdateTime: 1_700_000_000_000,
            volumeAccumulated: 4820,
            updateCount: 2,
          },
          eventQuality: {
            qualityScore: 0.86,
            mergeSimilarityScore: 0.91,
            valid: true,
            falseEventFlags: [],
          },
        },
      ],
      closed: [],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      error: null,
    }),
  ),
  fetchContractRetentionStatus: vi.fn(() =>
    Promise.resolve({
      flowRetentionDays: 14,
      signalRetentionDays: 365,
      signalProtectSeverityS: true,
      signalProtectNetVolumeBtc: 500,
      cleanupIntervalHours: 1,
      tables: {
        contractFlow1s: { rowCount: 10, rowsOlderThanRetention: 0 },
        contractWhaleSignals: {
          rowCount: 5,
          rowsOlderThanRetention: 0,
          protectedSCount: 1,
          protectedNetVolumeCount: 1,
        },
        mainForceEvents: { rowCount: 3, hasRetentionCleanup: false },
      },
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
    window.sessionStorage.clear();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("does not show enabled contract platforms as offline while initial requests are pending", () => {
    fetchContractWhaleLatest.mockReturnValueOnce(new Promise(() => {}));
    fetchFinalEventsV2.mockReturnValueOnce(new Promise(() => {}));
    fetchContractEvents.mockReturnValueOnce(new Promise(() => {}));
    fetchContractEventDebugCounts.mockReturnValueOnce(new Promise(() => {}));
    fetchContractRetentionStatus.mockReturnValueOnce(new Promise(() => {}));

    render(<ContractWhaleMonitor lockedSymbol="ETH" />);

    expect(screen.getByTestId("platform-status-chip-binance")).toHaveTextContent("等待数据");
    expect(screen.getByTestId("platform-status-chip-bitfinex")).toHaveTextContent("等待数据");
    expect(screen.getByTestId("platform-status-chip-binance")).not.toHaveTextContent("离线");
    expect(screen.getByTestId("platform-status-chip-bitfinex")).not.toHaveTextContent("离线");
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(1);
    expect(fetchContractEventDebugCounts).toHaveBeenCalledTimes(1);
  });

  it("keeps the event feed loading until its own request finishes", async () => {
    fetchContractEvents.mockReturnValueOnce(new Promise(() => {}));

    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    expect(screen.getByText("主力合约监控载入中...")).toBeInTheDocument();
    expect(screen.queryByText("暂无主力合约异动")).not.toBeInTheDocument();
  });

  it("renders the summary as soon as it resolves even when latest is still pending", async () => {
    fetchContractWhaleLatest.mockReturnValueOnce(new Promise(() => {}));

    render(<ContractWhaleMonitor />);

    await waitFor(() => {
      expect(screen.getByTestId("contract-workspace-status-ribbon")).toHaveTextContent("健康");
    });
  });

  it("paints the last status snapshot from session cache while the refresh is pending", async () => {
    window.sessionStorage.setItem(
      "contract-whale:status:v1:btc",
      JSON.stringify({
        version: 1,
        savedAt: Date.now(),
        summary: {
          status: "strong",
          healthStatus: "healthy",
          healthReason: "cached_snapshot",
          direction: "buy",
          latestDirection: "buy",
          latestSeverity: "s",
          enabled: true,
          dryRun: true,
          readOnly: true,
        },
        items: [],
      }),
    );
    fetchContractWhaleLatest.mockReturnValueOnce(new Promise(() => {}));

    render(<ContractWhaleMonitor />);

    expect(screen.getByTestId("contract-workspace-status-ribbon")).toHaveTextContent("健康");
  });

  it("restores the last successful event feed immediately across a page refresh", async () => {
    window.sessionStorage.clear();
    const firstRender = render(<ContractWhaleMonitor />);

    expect(await screen.findByTestId("raw-contract-whale-signals")).toHaveTextContent("69,917");
    firstRender.unmount();

    fetchContractEvents.mockReturnValueOnce(new Promise(() => {}));
    render(<ContractWhaleMonitor />);

    expect(screen.getByTestId("raw-contract-whale-signals")).toHaveTextContent("69,917");
    expect(screen.queryByText("暂无可用的历史事件缓存。")).not.toBeInTheDocument();
  });

  it("prioritizes a compact ETH event page before secondary event views", async () => {
    const initialPayload = await fetchContractEvents.getMockImplementation()();
    let resolveContractEvents;
    fetchContractEvents.mockReturnValueOnce(new Promise((resolve) => {
      resolveContractEvents = resolve;
    }));

    render(<ContractWhaleMonitor lockedSymbol="ETH" />);

    await waitFor(() =>
      expect(fetchContractEvents).toHaveBeenCalledWith(
        expect.objectContaining({
          symbol: "ETH",
          range: "7d",
          limit: 20,
        }),
      ),
    );
    expect(fetchFinalEventsV2).not.toHaveBeenCalled();
    expect(fetchContractWhaleIntelligenceTerminal).not.toHaveBeenCalled();

    resolveContractEvents(initialPayload);

    await waitFor(() => expect(fetchFinalEventsV2).toHaveBeenCalledTimes(1));
    expect(fetchContractWhaleIntelligenceTerminal).toHaveBeenCalledTimes(1);
  });

  it("keeps the compact BTC history request after a transient first failure", async () => {
    fetchContractEvents.mockResolvedValueOnce({
      items: [],
      dataState: "unavailable",
      errorCode: "contract_events_unavailable",
      lastKnownDataAvailable: false,
      retryAfterMs: 2_000,
      error: "contract_events_unavailable",
    });
    vi.useFakeTimers();

    render(<ContractWhaleMonitor />);
    await vi.advanceTimersByTimeAsync(0);

    expect(fetchContractEvents).toHaveBeenNthCalledWith(
      1,
      expect.objectContaining({ symbol: "BTC", range: "7d", limit: 20 }),
    );

    await vi.advanceTimersByTimeAsync(2_000);
    await vi.advanceTimersByTimeAsync(0);

    expect(fetchContractEvents).toHaveBeenNthCalledWith(
      2,
      expect.objectContaining({ symbol: "BTC", range: "7d", limit: 20 }),
    );
  });

  it("retries an unavailable status slice after two seconds", async () => {
    fetchContractWhaleSummary.mockResolvedValueOnce({
      summary: null,
      meta: null,
      error: "summary_unavailable",
    });
    fetchContractWhaleLatest.mockResolvedValueOnce({
      summary: null,
      items: [],
      dataState: "unavailable",
      errorCode: "latest_unavailable",
      lastKnownDataAvailable: false,
      retryAfterMs: 2_000,
      error: "latest_unavailable",
    });
    vi.useFakeTimers();

    render(<ContractWhaleMonitor />);
    await vi.advanceTimersByTimeAsync(0);

    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(1_999);
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);
    await vi.advanceTimersByTimeAsync(0);

    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(2);
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(2);
  });

  it("replaces the loading state with a recoverable event-feed error", async () => {
    fetchContractEvents.mockResolvedValueOnce({
      items: [],
      dataState: "degraded",
      error: "contract_events_unavailable",
    });

    render(<ContractWhaleMonitor />);

    expect(await screen.findByText("事件流暂时不可用，系统将在下一轮自动重试。"))
      .toBeInTheDocument();
    expect(screen.queryByText("主力合约监控载入中...")).not.toBeInTheDocument();
  });

  it("keeps historical rows while only the historical slice turns stale, then clears on success", async () => {
    const initialPayload = await fetchContractEvents.getMockImplementation()();
    fetchContractEvents
      .mockResolvedValueOnce(initialPayload)
      .mockResolvedValueOnce({
        items: [],
        dataState: "degraded",
        degraded: true,
        errorCode: "contract_projection_timeout",
        lastKnownDataAvailable: false,
        retryAfterMs: 2_000,
        error: "contract_projection_timeout",
      })
      .mockResolvedValueOnce(initialPayload);
    vi.useFakeTimers();

    render(<ContractWhaleMonitor />);
    await vi.advanceTimersByTimeAsync(0);
    expect(within(screen.getByTestId("raw-contract-whale-signals")).getAllByRole("row").length).toBeGreaterThan(1);

    await vi.advanceTimersByTimeAsync(15_000);
    const staleBanner = screen.getByTestId("data-health-banner");
    expect(staleBanner).toHaveTextContent("历史事件");
    expect(staleBanner).not.toHaveTextContent("生命周期（陈旧）");
    expect(staleBanner).not.toHaveTextContent("智能分析（陈旧）");
    expect(within(screen.getByTestId("raw-contract-whale-signals")).getAllByRole("row").length).toBeGreaterThan(1);

    await vi.advanceTimersByTimeAsync(1_999);
    expect(screen.getByTestId("data-health-banner")).toHaveTextContent("历史事件");

    await vi.advanceTimersByTimeAsync(1);
    await vi.advanceTimersByTimeAsync(0);
    expect(fetchContractEvents).toHaveBeenCalledTimes(3);
    expect(screen.queryByTestId("data-health-banner")).not.toBeInTheDocument();
  });

  it("does not flash the recovery banner while usable projections refresh in the background", async () => {
    const historicalPayload = await fetchContractEvents.getMockImplementation()();
    const lifecyclePayload = await fetchFinalEventsV2.getMockImplementation()();
    fetchContractEvents.mockResolvedValueOnce({
      ...historicalPayload,
      dataState: "stale",
      degraded: true,
      errorCode: "contract_projection_refresh_in_progress",
      lastKnownDataAvailable: true,
      error: null,
    });
    fetchFinalEventsV2.mockResolvedValueOnce({
      ...lifecyclePayload,
      dataState: "stale",
      degraded: true,
      errorCode: "contract_projection_refresh_in_progress",
      lastKnownDataAvailable: true,
      error: null,
    });

    render(<ContractWhaleMonitor />);

    await waitFor(() => expect(fetchFinalEventsV2).toHaveBeenCalledTimes(1));
    expect(within(screen.getByTestId("raw-contract-whale-signals")).getAllByRole("row").length)
      .toBeGreaterThan(1);
    expect(screen.queryByTestId("data-health-banner")).not.toBeInTheDocument();
  });

  it("masks stale intelligence as UNKNOWN while keeping the previous context secondary", async () => {
    fetchContractWhaleIntelligenceTerminal.mockResolvedValueOnce({
      symbol: "ETH",
      marketRegime: { regime: "TRENDING_UP", confidence: 82, reason: "prior structure" },
      riskContext: {
        riskState: "high",
        fakeBreakoutRisk: "HIGH",
        summary: "prior risk",
        noTradeZones: [],
      },
      dataState: "stale",
      degraded: true,
      errorCode: "contract_projection_refresh_in_progress",
      lastKnownDataAvailable: true,
      cacheAgeSec: 18,
      cacheTtlSec: 300,
      error: null,
    });

    render(<ContractWhaleMonitor lockedSymbol="ETH" />);

    expect(await screen.findByTestId("intelligence-freshness")).toHaveTextContent("STALE");
    expect(screen.getByTestId("current-market-regime")).toHaveTextContent("UNKNOWN");
    expect(screen.getByTestId("current-risk-state")).toHaveTextContent("UNKNOWN");
    expect(screen.getByTestId("previous-intelligence-context")).toHaveTextContent("TRENDING_UP");
    expect(screen.getByTestId("previous-intelligence-context")).toHaveTextContent("HIGH RISK");
  });

  it("renders one consolidated recovery banner without an action button", async () => {
    fetchContractWhaleSummary.mockResolvedValueOnce({ summary: null, meta: null, error: "summary_unavailable" });
    fetchContractWhaleLatest.mockResolvedValueOnce({
      summary: null,
      items: [],
      dataState: "degraded",
      errorCode: "latest_unavailable",
      lastKnownDataAvailable: false,
      error: "latest_unavailable",
    });
    fetchContractEvents.mockResolvedValueOnce({
      items: [],
      dataState: "degraded",
      errorCode: "contract_events_unavailable",
      lastKnownDataAvailable: false,
      error: "contract_events_unavailable",
    });
    fetchFinalEventsV2.mockResolvedValueOnce({
      active: [],
      closed: [],
      dataState: "degraded",
      errorCode: "final_events_v2_unavailable",
      lastKnownDataAvailable: false,
      error: "final_events_v2_unavailable",
    });
    fetchContractWhaleIntelligenceTerminal.mockResolvedValueOnce({
      dataState: "degraded",
      errorCode: "intelligence_terminal_unavailable",
      lastKnownDataAvailable: false,
      error: "intelligence_terminal_unavailable",
    });

    render(<ContractWhaleMonitor lockedSymbol="ETH" />);

    const banner = await screen.findByTestId("data-health-banner");
    expect(screen.getAllByTestId("data-health-banner")).toHaveLength(1);
    expect(banner).toHaveTextContent("状态快照");
    expect(banner).toHaveTextContent("历史事件");
    expect(banner).toHaveTextContent("生命周期");
    expect(banner).toHaveTextContent("智能分析");
    expect(within(banner).queryByRole("button")).not.toBeInTheDocument();
  });

  it("keeps the core contract-whale content visible while retention stays deferred", async () => {
    fetchContractRetentionStatus.mockReturnValueOnce(new Promise(() => {}));

    render(<ContractWhaleMonitor />);

    expect(screen.getByText("主力合约监控")).toBeInTheDocument();
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(1);
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(1);
    expect(fetchContractEvents).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(fetchFinalEventsV2).toHaveBeenCalledTimes(1));
    expect(fetchContractEventDebugCounts).toHaveBeenCalledTimes(1);
    expect(fetchContractWhaleEvents).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.getByText("强异动")).toBeInTheDocument());
    expect(fetchContractRetentionStatus).not.toHaveBeenCalled();
    expect(screen.getByText("Buy 62.0% / Sell 38.0%")).toBeInTheDocument();
    expect(screen.getByText("ACTIVE EVENTS (updated)")).toBeInTheDocument();
    expect(screen.getByText("CLOSED EVENTS (finalized)")).toBeInTheDocument();
  });

  it("never starts retention table scans from a normal page mount", async () => {
    vi.useFakeTimers();

    render(<ContractWhaleMonitor />);
    await vi.advanceTimersByTimeAsync(3_001);

    expect(fetchContractRetentionStatus).not.toHaveBeenCalled();
    expect(screen.getByText(/详细统计不在页面加载链路执行/)).toBeInTheDocument();
  });

  it("promotes historical events into the pro desk primary view", async () => {
    render(<ContractWhaleMonitor />);

    const historical = await screen.findByText("HISTORICAL EVENTS (7d stream)");
    const proDesk = screen.getByText("事件驱动交易台总览");
    const structure = screen.getByText("Market Structure");
    const setups = screen.getByText("Structure Setups");
    const systemStatus = screen.getByText("System Status / Latency / Retention");
    const historicalPanel = screen.getByTestId("historical-events-primary");
    const monitorPanel = screen.getByText("主力合约监控").closest("section");
    const commandBar = screen.getByTestId("contract-workspace-command-bar");
    const statusRibbon = screen.getByTestId("contract-workspace-status-ribbon");
    const eventTape = screen.getByTestId("contract-event-tape");
    const insightRail = screen.getByTestId("contract-insight-rail");

    expect(commandBar).toHaveTextContent("BTC / PERP");
    expect(commandBar).toHaveTextContent("只读监控");
    expect(statusRibbon).toHaveTextContent("REGIME");
    expect(eventTape).toContainElement(screen.getByTestId("raw-contract-whale-signals"));
    expect(insightRail).toHaveTextContent("市场结构");
    expect(insightRail).toHaveTextContent("流动性与 OI");
    expect(insightRail).toHaveTextContent("交易机会 / 风险");
    expect(historical.compareDocumentPosition(proDesk) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(proDesk.compareDocumentPosition(structure) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(historical.compareDocumentPosition(structure) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(structure.compareDocumentPosition(setups) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(setups.compareDocumentPosition(systemStatus) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(monitorPanel).toHaveClass("overflow-x-hidden");
    expect(historicalPanel).toHaveClass("min-h-[50vh]");
    expect(screen.getByTestId("primary-analysis-grid")).toHaveClass("contract-primary-grid");
    expect(screen.getByTestId("secondary-analysis-grid").className).toContain("2xl:grid-cols-");
    expect(screen.getByTestId("lifecycle-risk-grid").className).toContain("2xl:grid-cols-");
    expect(screen.queryByText("Institutional Analysis Terminal")).not.toBeInTheDocument();
  });

  it("does not reserve a half-screen canvas for an empty event feed", async () => {
    fetchContractEvents.mockResolvedValueOnce({
      items: [],
      dataState: "fresh",
      degraded: false,
      error: null,
    });

    render(<ContractWhaleMonitor />);

    const historicalPanel = await screen.findByTestId("historical-events-primary");
    await screen.findByText("暂无主力合约异动");
    expect(historicalPanel).not.toHaveClass("min-h-[50vh]");
  });

  it("shows contract classification v2 semantics in the event type column", async () => {
    fetchContractEvents.mockResolvedValueOnce({
      items: [
        {
          id: "contract-event-v2-classification",
          eventId: "cwm-event:BTC:aggressive_sell:v2",
          sourceSignalId: "contract-whale-v2-classification",
          ts: 1_700_000_020_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
          windowSec: 15,
          signalType: "aggressive_sell",
          displaySignalType: "主动卖压",
          flowDirection: "sell_dominant",
          priceResponseTypeV2: "no_clear_response",
          oiContext: "oi_not_confirmed",
          classificationReasons: ["sell_dominant", "price_follow_through_not_confirmed"],
          direction: "sell",
          severity: "medium",
          score: 62,
          totalVolumeBtc: 577,
          netVolumeBtc: -182,
          totalNotionalUsd: 22_000_000,
          dominance: 0.61,
          triggerPriceUsd: 59_500,
          orderPriceUsd: 59_500,
          currentMarketPriceUsd: 59_500,
          priceDeviationPct: 0.1,
          priceDeviationFiltered: false,
          mainExchange: "binance",
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_sell:v2",
            status: "active",
            startTime: 1_700_000_020_000,
            lastUpdateTime: 1_700_000_020_000,
            latestWindowVolumeBtc: 400,
            peakWindowVolumeBtc: 577,
            uniqueTurnoverBtc: 510,
            uniqueTurnoverAvailable: true,
            netOiDeltaBtc: -12,
            peakAbsOiDeltaBtc: 18,
            volumeAccumulated: 577,
            updateCount: 1,
          },
          eventQuality: {
            qualityScore: 0.81,
            mergeSimilarityScore: 0.86,
            valid: true,
            falseEventFlags: [],
          },
          status: "active",
          source: "contract_whale_signals",
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      error: null,
    });

    render(<ContractWhaleMonitor />);

    expect(await screen.findByText("主动卖压")).toBeInTheDocument();
    expect(screen.getByText("主动流：主动卖占优 · 价格：价格响应不明确 · OI：OI 不确认")).toBeInTheDocument();
    expect(screen.getAllByTitle(/主力拉盘\/砸盘仅在主动流方向、价格跟随、多窗口确认同时满足时显示/).length).toBeGreaterThan(0);
  });

  it("renders resolved oi context labels and tooltip text in the historical events stream", async () => {
    fetchContractEvents.mockResolvedValueOnce({
      items: [
        {
          id: "contract-event-oi-semantic",
          eventId: "cwm-event:BTC:aggressive_sell:oi-semantic",
          sourceSignalId: "contract-whale-oi-semantic",
          ts: 1_700_000_020_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
          windowSec: 60,
          signalType: "aggressive_sell",
          displaySignalType: "主动卖压",
          flowDirection: "sell_dominant",
          priceResponseTypeV2: "trend_follow_down",
          oiContext: "new_short_build",
          oiContextLabel: "新空开仓",
          oiDeltaPct: 0.42,
          oiAvailable: true,
          oiReason: "oi_increased_with_sell_pressure",
          classificationReasons: ["sell_dominant", "price_follow_through"],
          direction: "sell",
          severity: "medium",
          score: 62,
          totalVolumeBtc: 577,
          netVolumeBtc: -182,
          totalNotionalUsd: 22_000_000,
          dominance: 0.61,
          triggerPriceUsd: 59_500,
          orderPriceUsd: 59_500,
          currentMarketPriceUsd: 59_500,
          priceDeviationPct: 0.1,
          priceDeviationFiltered: false,
          mainExchange: "binance",
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_sell:oi-semantic",
            status: "active",
            startTime: 1_700_000_020_000,
            lastUpdateTime: 1_700_000_020_000,
            latestWindowVolumeBtc: 400,
            peakWindowVolumeBtc: 577,
            uniqueTurnoverBtc: 510,
            uniqueTurnoverAvailable: true,
            netOiDeltaBtc: -12,
            peakAbsOiDeltaBtc: 18,
            volumeAccumulated: 577,
            updateCount: 1,
          },
          eventQuality: {
            qualityScore: 0.81,
            mergeSimilarityScore: 0.86,
            valid: true,
            falseEventFlags: [],
          },
          status: "active",
          source: "contract_whale_signals",
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      error: null,
    });

    render(<ContractWhaleMonitor />);

    expect(
      await screen.findByText("主动流：主动卖占优 · 价格：卖盘推动下跌 · OI：新空开仓 +0.42%"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/\bundefined\b/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/\bNaN\b/i)).not.toBeInTheDocument();
    expect(
      screen.getAllByTitle(/OI 标签用于解释该窗口内未平仓量变化：/).length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText(/峰值 OI/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/净 OI/).length).toBeGreaterThan(0);
  });

  it("renders notional inline after the symbol in the historical events market column", async () => {
    fetchContractEvents.mockResolvedValueOnce({
      items: [
        {
          id: "contract-event-notional-inline",
          eventId: "cwm-event:BTC:aggressive_sell:notional-inline",
          sourceSignalId: "contract-whale-notional-inline",
          ts: 1_700_000_020_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
          windowSec: 60,
          signalType: "aggressive_sell",
          displaySignalType: "主动卖压",
          flowDirection: "sell_dominant",
          priceResponseTypeV2: "price_follow_through",
          oiContext: "oi_not_confirmed",
          classificationReasons: ["sell_dominant", "price_follow_through"],
          direction: "sell",
          severity: "medium",
          score: 62,
          totalVolumeBtc: 577,
          netVolumeBtc: -182,
          totalNotionalUsd: 22_000_000,
          dominance: 0.61,
          triggerPriceUsd: 59_500,
          orderPriceUsd: 59_500,
          currentMarketPriceUsd: 59_500,
          priceDeviationPct: 0.1,
          priceDeviationFiltered: false,
          mainExchange: "binance",
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_sell:notional-inline",
            status: "active",
            startTime: 1_700_000_020_000,
            lastUpdateTime: 1_700_000_020_000,
            volumeAccumulated: 577,
            updateCount: 1,
          },
          eventQuality: {
            qualityScore: 0.81,
            mergeSimilarityScore: 0.86,
            valid: true,
            falseEventFlags: [],
          },
          status: "active",
          source: "contract_whale_signals",
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      error: null,
    });

    render(<ContractWhaleMonitor />);

    const table = await screen.findByTestId("raw-contract-whale-signals");
    const headers = Array.from(table.querySelectorAll("thead th")).map((node) => node.textContent?.trim());
    expect(headers).not.toContain("名义金额");

    const row = screen.getByTestId("contract-whale-row-cwm-event:BTC:aggressive_sell:notional-inline");
    const marketCell = row.querySelectorAll("td")[1];
    expect(marketCell).toBeTruthy();
    expect(within(marketCell).getByText("BTC")).toBeInTheDocument();
    expect(within(marketCell).getByText("$22M")).toBeInTheDocument();
    expect(within(marketCell).getByText("$59,500")).toBeInTheDocument();
    const marketText = marketCell.textContent || "";
    expect(marketText.indexOf("BTC")).toBeLessThan(marketText.indexOf("$22M"));
    expect(marketText.indexOf("$22M")).toBeLessThan(marketText.indexOf("$59,500"));

    await userEvent.click(within(row).getByRole("button", { name: /查看主力合约信号详情 contract-whale-notional-inline/ }));
    expect(await screen.findByRole("dialog")).toHaveTextContent("BTC 主力合约信号详情");
  });

  it("hides sub-500 BTC contract events and linked setups from the default desk view", async () => {
    fetchContractEvents.mockResolvedValueOnce({
      items: [
        {
          id: "contract-event-low-notional",
          eventId: "cwm-event:BTC:aggressive_buy:low-notional",
          sourceSignalId: "contract-whale-low-notional",
          ts: 1_700_000_000_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
          windowSec: 15,
          signalType: "aggressive_buy",
          direction: "buy",
          severity: "medium",
          score: 71,
          totalVolumeBtc: 450,
          netVolumeBtc: 255,
          totalNotionalUsd: 6_000_000,
          dominance: 0.58,
          priceDeviationPct: 0.2,
          priceDeviationFiltered: false,
          mainExchange: "binance",
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_buy:low-notional",
            status: "active",
            startTime: 1_700_000_000_000,
            lastUpdateTime: 1_700_000_000_000,
            volumeAccumulated: 450,
            updateCount: 1,
          },
          eventQuality: {
            qualityScore: 0.77,
            mergeSimilarityScore: 0.83,
            valid: true,
            falseEventFlags: [],
          },
          status: "active",
          source: "contract_whale_signals",
        },
        {
          id: "contract-event-high-notional",
          eventId: "cwm-event:BTC:aggressive_sell:high-notional",
          sourceSignalId: "contract-whale-high-notional",
          ts: 1_700_000_010_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
          windowSec: 15,
          signalType: "aggressive_sell",
          direction: "sell",
          severity: "high",
          score: 86,
          totalVolumeBtc: 620,
          netVolumeBtc: -410,
          totalNotionalUsd: 11_000_000,
          dominance: 0.61,
          priceDeviationPct: 0.3,
          priceDeviationFiltered: false,
          mainExchange: "binance",
          eventLifecycle: {
            eventId: "cwm-event:BTC:aggressive_sell:high-notional",
            status: "active",
            startTime: 1_700_000_010_000,
            lastUpdateTime: 1_700_000_010_000,
            volumeAccumulated: 620,
            updateCount: 1,
          },
          eventQuality: {
            qualityScore: 0.85,
            mergeSimilarityScore: 0.88,
            valid: true,
            falseEventFlags: [],
          },
          status: "active",
          source: "contract_whale_signals",
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      error: null,
    });
    fetchContractWhaleIntelligenceTerminal.mockResolvedValueOnce({
      symbol: "BTC",
      timestamp: 1_700_000_020_000,
      marketRegime: {
        regime: "RANGING",
        confidence: 78,
        reason: "成交量活跃但价格延续性一般，结构更接近区间整理。",
      },
      liquidityBehaviors: [],
      opportunityMap: [],
      rankedEvents: [
        {
          signalId: "contract-whale-low-notional",
          rank: 1,
          eventType: "Low Notional Rank",
          directionBias: "BUY",
          strengthScore: 71,
          strengthLabel: "MEDIUM",
          regimeAlignment: "supportive",
          liquidityBehavior: "absorption",
          windowSec: 15,
          rationale: "this should stay hidden",
        },
        {
          signalId: "contract-whale-high-notional",
          rank: 2,
          eventType: "High Notional Rank",
          directionBias: "SELL",
          strengthScore: 86,
          strengthLabel: "HIGH",
          regimeAlignment: "aligned",
          liquidityBehavior: "breakout_pressure",
          windowSec: 15,
          rationale: "this should stay visible",
        },
      ],
      noiseSuppression: {
        rawCandidates: 2,
        mergedEvents: 2,
        lifecycleEvents: 2,
        filteredEvents: 2,
        tradeableSetups: 2,
        suppressedDuplicates: 0,
        noiseReductionPct: 0,
      },
      signalCompression: {
        qualityScore: 80,
        topSignalCount: 2,
        discardedCount: 0,
        compressionReason: "test fixture",
      },
      tradeIdeas: [
        {
          signalId: "contract-whale-low-notional",
          rank: 1,
          setupType: "Low Notional Setup",
          directionBias: "BULLISH_BIAS",
          score: 71,
          confidence: 66,
          confidenceLabel: "MEDIUM",
          regimeContext: "RANGING",
          pressureZone: { label: "69,800 - 69,900" },
          riskBoundary: { reason: "hidden low notional" },
          structureContext: "low notional setup should not render",
          windowSec: 15,
        },
        {
          signalId: "contract-whale-high-notional",
          rank: 2,
          setupType: "High Notional Setup",
          directionBias: "BEARISH_BIAS",
          score: 86,
          confidence: 79,
          confidenceLabel: "HIGH",
          regimeContext: "TRENDING_DOWN",
          pressureZone: { label: "69,700 - 69,760" },
          riskBoundary: { reason: "visible high notional" },
          structureContext: "high notional setup should render",
          windowSec: 15,
        },
      ],
      riskContext: {
        fakeBreakoutRisk: "LOW",
        summary: "test fixture",
        noTradeZones: [],
      },
    });

    render(<ContractWhaleMonitor />);

    await screen.findByText("HISTORICAL EVENTS (7d stream)");

    expect(screen.getAllByText(/当前过滤：窗口总流量 ≥ 500 BTC/).length).toBeGreaterThan(0);
    expect(screen.queryByTestId("contract-whale-row-cwm-event:BTC:aggressive_buy:low-notional")).not.toBeInTheDocument();
    expect(screen.getByTestId("contract-whale-row-cwm-event:BTC:aggressive_sell:high-notional")).toBeInTheDocument();
    expect(screen.queryByText("Low Notional Setup")).not.toBeInTheDocument();
    expect(screen.getByText("High Notional Setup")).toBeInTheDocument();
    expect(screen.queryByText("Low Notional Rank")).not.toBeInTheDocument();
    expect(screen.getByText("High Notional Rank")).toBeInTheDocument();
  });

  it("renders jump navigation links for the pro desk sections", async () => {
    render(<ContractWhaleMonitor />);

    await screen.findByText("HISTORICAL EVENTS (7d stream)");

    expect(screen.getByRole("link", { name: "Events" })).toHaveAttribute("href", "#contract-whale-events");
    expect(screen.getByRole("link", { name: "Structure" })).toHaveAttribute("href", "#contract-whale-structure");
    expect(screen.getByRole("link", { name: "Liquidity" })).toHaveAttribute("href", "#contract-whale-liquidity");
    expect(screen.getByRole("link", { name: "Setups" })).toHaveAttribute("href", "#contract-whale-setups");
    expect(screen.getByRole("link", { name: "Risk" })).toHaveAttribute("href", "#contract-whale-risk");
    expect(screen.getByRole("link", { name: "Status" })).toHaveAttribute("href", "#contract-whale-status");
  });

  it("warns when BTC latest only contains stale snapshots and 24h history has no new signals", async () => {
    fetchContractWhaleLatest.mockResolvedValueOnce({
      summary: {
        status: "calm",
        healthStatus: "healthy",
        healthReason: "enabled_sources_recent",
        thresholdProfile: "binance_bitfinex",
        latestDirection: "neutral",
        latestSeverity: "medium",
        signalCount: 1,
        enabled: true,
        dryRun: true,
        contractDataQuality: 95,
        spotDataQuality: 78,
        overallDataQuality: 88,
        trend60s: {
          buyVolumeBtc: 120,
          sellVolumeBtc: 80,
          totalVolumeBtc: 200,
          netVolumeBtc: 40,
          dominance: 0.2,
          buyRatio: 0.6,
          sellRatio: 0.4,
          updatedAtMs: 1_700_000_000_000,
        },
        exchanges: {},
        platforms: {},
      },
      items: [
        {
          id: "stale-latest-btc",
          ts: 1_699_900_000_000,
          symbol: "BTC",
          windowSec: 15,
          signalType: "aggressive_buy",
          direction: "buy",
          severity: "medium",
          score: 40,
          totalVolumeBtc: 300,
          netVolumeBtc: 100,
          totalNotionalUsd: 18_000_000,
          dominance: 0.33,
          mainExchange: "binance",
          dataQuality: 82,
          ageSec: 90_500,
          isStale: true,
          staleReason: "older_than_24h",
        },
      ],
      error: null,
    });
    fetchContractEventDebugCounts.mockResolvedValueOnce({
      symbol: "BTC",
      range: "24h",
      generatedAt: "2026-06-27T00:00:00Z",
      db: {
        contractWhaleSignalsTotal24h: 8,
        contractWhaleSignalsBtc24h: 0,
        oldestTs: null,
        newestTs: null,
      },
      apiQuery: {
        matchedBeforeFilter: 0,
        matchedAfterSymbolFilter: 0,
        matchedAfterRangeFilter: 0,
        matchedAfterSeverityFilter: null,
        matchedAfterWindowFilter: null,
        matchedAfterDirectionFilter: null,
        returnedItems: 0,
        limit: 100,
      },
      visibility: {
        visibleCount: 0,
        hiddenCount: 0,
        hiddenReasons: {
          priceDeviationGt5pct: 0,
          missingPrice: 0,
          badQuality: 0,
          disabledMonitor: 0,
          unknown: 0,
        },
      },
      latest: {
        latestCount: 1,
        staleCount: 1,
        latestSymbols: ["BTC"],
        items: [
          {
            eventId: "stale-latest-btc",
            ts: 1_699_900_000_000,
            ageSec: 90_500,
            isStale: true,
            staleReason: "older_than_24h",
          },
        ],
      },
      finalEventsV2: {
        activeCount: 0,
        closedCount: 0,
      },
      latestVsHistory: [
        {
          latestEventId: "stale-latest-btc",
          symbol: "BTC",
          ts: 1_699_900_000_000,
          existsInHistory: false,
          historyEventId: null,
          notInHistoryReason: "outside_requested_range",
        },
      ],
      finalEventsProjection: {
        source: "contract_whale_signals",
        rawSignals: 0,
        afterFilter: 0,
        mergedEvents: 0,
        active: 0,
        closed: 0,
        range: "24h",
      },
      error: null,
    });
    fetchContractWhaleRawFlowDebug.mockResolvedValueOnce({
      symbol: "BTC",
      range: "24h",
      config: {
        appRequestedSymbol: "ETH-PERP",
        querySymbol: "BTC",
      },
      normalizer: {
        connectorSymbolMismatch: true,
      },
      contractFlow1s: {
        exactSymbolRows: 0,
      },
      diagnosis: {
        status: "upstream_no_raw_flow",
        primaryReason: "connector_requested_symbol_mismatch",
        details: ["connector requested ETH-PERP while query symbol is BTC"],
      },
      error: null,
    });

    render(<ContractWhaleMonitor />);

    expect(
      await screen.findByText("BTC latest 为旧快照，最近 24h 没有新的 BTC 主力历史信号。"),
    ).toBeInTheDocument();
    expect(screen.getByText(/上游诊断：connector_requested_symbol_mismatch/i)).toBeInTheDocument();
  });

  it("renders summary cards and latest contract whale signals", async () => {
    render(<ContractWhaleMonitor />);

    expect(await screen.findByText("主力合约监控")).toBeInTheDocument();
    expect(screen.getByText("强异动")).toBeInTheDocument();
    expect(screen.getByText("健康")).toBeInTheDocument();
    expect(screen.getAllByText("Dry-run").length).toBeGreaterThan(0);
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
    expect(screen.queryByText("Institutional Analysis Terminal")).not.toBeInTheDocument();
    expect(screen.getByText("Market Structure")).toBeInTheDocument();
    expect(screen.getByText("Liquidity Map")).toBeInTheDocument();
    expect(screen.getByText("Structure Setups")).toBeInTheDocument();
    expect(screen.getByText("Risk Context")).toBeInTheDocument();
    expect(screen.getByText("Market Regime")).toBeInTheDocument();
    expect(screen.getAllByText("Liquidity Behavior").length).toBeGreaterThan(0);
    expect(screen.getByText("Signal Strength Ranking")).toBeInTheDocument();
    expect(screen.getByText("Opportunity Map")).toBeInTheDocument();
    expect(screen.getAllByText("RANGING").length).toBeGreaterThan(0);
    expect(screen.getByText("Regime 78%")).toBeInTheDocument();
    expect(screen.getByText("Absorption")).toBeInTheDocument();
    expect(screen.getByText("Fake Breakout")).toBeInTheDocument();
    expect(screen.getAllByText("Absorption Zone").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Fake Breakout Risk").length).toBeGreaterThan(0);
    expect(screen.getAllByText("69,760 - 69,890").length).toBeGreaterThan(0);
    expect(screen.getAllByText("69,980 - 70,040").length).toBeGreaterThan(0);
    expect(screen.queryByText("Entry Zone")).not.toBeInTheDocument();
    expect(screen.queryByText("Invalidation")).not.toBeInTheDocument();
    expect(screen.getByText("Top Structures")).toBeInTheDocument();
    expect(screen.getByText("当前 Regime")).toBeInTheDocument();
    expect(screen.getByText("Desk Mode")).toBeInTheDocument();
    expect(screen.getAllByText("87/100").length).toBeGreaterThan(0);
    expect(screen.getAllByText("主力拉盘").length).toBeGreaterThan(0);
    expect(screen.getByText(/多窗口主买一致/)).toBeInTheDocument();
    expect(screen.getByText("Absorption continuation")).toBeInTheDocument();
    expect(screen.getByText("HIGH CONF")).toBeInTheDocument();
    expect(screen.getAllByText("当前风险").length).toBeGreaterThan(0);
    expect(screen.getByText("Whale Entity List")).toBeInTheDocument();
    expect(screen.getByText("Trajectory Timeline")).toBeInTheDocument();
    expect(screen.getByText("Stealth Curve (gamma)")).toBeInTheDocument();
    expect(screen.getByText("Hazard Curve (lambda proxy)")).toBeInTheDocument();
    expect(screen.getByText("合约市场事件")).toBeInTheDocument();
    expect(screen.getByText(/当前列表为历史事件流，不是 latest 快照/)).toBeInTheDocument();
    expect(screen.getAllByText("窗口总流量 BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("峰值窗口流量 BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText(/总流量 = 主动买量 \+ 主动卖量/).length).toBeGreaterThan(0);
    expect(screen.getAllByText("市场冲击等级").length).toBeGreaterThan(0);
    expect(screen.getByText("ACTIVE EVENTS (updated)")).toBeInTheDocument();
    expect(screen.getByText("CLOSED EVENTS (finalized)")).toBeInTheDocument();
    expect(screen.getAllByText(/已加载 \d+ 条/).length).toBeGreaterThan(0);
    expect(screen.getByTestId("raw-contract-whale-signals")).toBeInTheDocument();
    expect(screen.getAllByText("Q 86").length).toBeGreaterThan(0);
    expect(screen.getAllByText(/2023/).length).toBeGreaterThan(0);
    expect(screen.getAllByText("主力拉盘").length).toBeGreaterThan(0);
    expect(
      screen.getAllByText((_, element) => {
        const text = element?.textContent || "";
        return text.includes("BTC") && text.includes("$69,917");
      }).length,
    ).toBeGreaterThan(0);
    expect(screen.getAllByText("4,820 BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("$337M").length).toBeGreaterThan(0);
    expect(screen.getAllByText((_, element) => hasPriceText(element?.textContent || "")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("0.12%").length).toBeGreaterThan(0);
    expect(screen.getAllByText("87/100").length).toBeGreaterThan(0);
    expect(screen.getAllByText("S 81 / C 94").length).toBeGreaterThan(0);
    expect(screen.getAllByText("净买入 3,260 BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("67.6%").length).toBeGreaterThan(0);
    expect(screen.getAllByText("9.4x").length).toBeGreaterThan(0);
    expect(screen.getAllByText("P99.9").length).toBeGreaterThan(0);
    expect(screen.getByTestId("raw-contract-whale-signals")).toHaveTextContent("S / S");
    expect(screen.getByTestId("raw-contract-whale-signals")).toHaveTextContent("SHOCK IMPACT EVENT");
    expect(screen.getByTestId("raw-contract-whale-signals")).toHaveTextContent("9.40x · P99.9");
    expect(screen.getAllByText("+0.31%").length).toBeGreaterThan(0);
    expect(screen.getAllByText("疑似强平 420 BTC / 8.7%").length).toBeGreaterThan(0);
    expect(screen.getAllByText("+900 BTC / +1.20% OI上升").length).toBeGreaterThan(0);
    expect(screen.getAllByText("+0.02% 偏多").length).toBeGreaterThan(0);
    expect(screen.getAllByText("待推").length).toBeGreaterThan(0);
    expect(screen.getByText("主力结构事件历史")).toBeInTheDocument();
    expect(screen.getByText("让你知道这里发生过什么主力行为")).toBeInTheDocument();
    expect(screen.getByText("结构判断")).toBeInTheDocument();
    expect(screen.getAllByText("主力评分").length).toBeGreaterThan(0);
    expect(screen.getByText("现货确认")).toBeInTheDocument();
    expect(screen.getByText("Dry-run 1h")).toBeInTheDocument();
    expect(screen.getByText("would-send 3")).toBeInTheDocument();
    expect(screen.getByTestId("main-force-event-7")).toBeInTheDocument();
    expect(screen.getAllByText("主力建多").length).toBeGreaterThan(0);
    expect(screen.getByText("峰值主力评分")).toBeInTheDocument();
    expect(screen.getByText("非清算驱动")).toBeInTheDocument();
    expect(screen.getByText("已结束")).toBeInTheDocument();
    expect(fetchContractEvents).toHaveBeenCalledWith(
      expect.objectContaining({
        symbol: "BTC",
        range: "7d",
        limit: 20,
      }),
    );
    expect(fetchFinalEventsV2).toHaveBeenCalledWith(
      expect.objectContaining({
        symbol: "BTC",
        range: "24h",
        limit: 30,
      }),
    );
    expect(fetchContractWhaleIntelligenceTerminal).toHaveBeenCalledWith(
      expect.objectContaining({
        symbol: "BTC",
        range: "24h",
      }),
    );
    expect(fetchContractWhaleTradingDecisions).not.toHaveBeenCalled();
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
    fetchContractEvents.mockResolvedValueOnce({
      items: [
        {
          eventId: "contract-event:eth:1",
          sourceSignalId: "eth-contract-whale-row",
          ts: 1_700_000_100_000,
          symbol: "ETH",
          price: 1675,
          status: "active",
          signalType: "aggressive_buy",
          severity: "medium",
          windowSec: 60,
          volumeBtc: 16869,
          notionalUsd: 28_000_000,
          netVolumeBtc: 610,
          direction: "buy",
          netDirection: "net_buy",
          mainForceScore: 34,
          exchangeSpotCount: 0,
          exchangeContractCount: 1,
          source: "contract_whale_signals",
          isRetentionProtected: false,
          retentionReason: null,
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      error: null,
    });
    fetchFinalEventsV2.mockResolvedValueOnce({
      active: [],
      closed: [
        {
          id: "cwm-event:ETH:aggressive_buy:1700000100000",
          finalEventId: "cwm-event:ETH:aggressive_buy:1700000100000",
          sourceSignalId: "eth-contract-whale-row",
          ts: 1_700_000_100_000,
          symbol: "ETH",
          baseAsset: "ETH",
          quantityUnit: "ETH",
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
          eventLifecycle: {
            eventId: "cwm-event:ETH:aggressive_buy:1700000100000",
            status: "active",
            startTime: 1_700_000_100_000,
            lastUpdateTime: 1_700_000_100_000,
            volumeAccumulated: 16869,
            updateCount: 1,
          },
          eventQuality: {
            qualityScore: 0.71,
            mergeSimilarityScore: 1,
            valid: true,
            falseEventFlags: [],
          },
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      error: null,
    });

    render(<ContractWhaleMonitor lockedSymbol="ETH" />);

    expect(await screen.findByText("16,869 ETH")).toBeInTheDocument();
    expect(screen.getByText("净买入 614 ETH")).toBeInTheDocument();
    expect(screen.getByText("总量 761 ETH · dominance 80.7%")).toBeInTheDocument();
    expect(screen.getAllByText("窗口总流量 ETH").length).toBeGreaterThan(0);
    expect(screen.getAllByText("峰值窗口流量 ETH").length).toBeGreaterThan(0);
    expect(screen.queryByText("净买入 614 BTC")).not.toBeInTheDocument();
  });

  it("uses the locked ETH symbol as the trend unit when ETH summary omits trend symbol", async () => {
    fetchContractWhaleLatest.mockResolvedValueOnce({
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
    fetchContractEvents.mockResolvedValueOnce({
      items: [
        {
          eventId: "contract-event:eth:selected:1",
          sourceSignalId: "eth-selected-contract-whale-row",
          ts: 1_700_000_100_000,
          symbol: "ETH",
          price: 1675,
          status: "active",
          windowSec: 60,
          signalType: "aggressive_buy",
          direction: "buy",
          severity: "medium",
          mainForceScore: 34,
          volumeBtc: 16869,
          netVolumeBtc: 610,
          notionalUsd: 28_000_000,
          netDirection: "net_buy",
          exchangeSpotCount: 0,
          exchangeContractCount: 1,
          source: "contract_whale_signals",
          isRetentionProtected: false,
          retentionReason: null,
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      error: null,
    });
    fetchFinalEventsV2.mockResolvedValueOnce({
      active: [],
      closed: [
        {
          id: "cwm-event:ETH:aggressive_buy:1700000100000",
          eventId: "cwm-event:ETH:aggressive_buy:1700000100000",
          finalEventId: "cwm-event:ETH:aggressive_buy:1700000100000",
          sourceSignalId: "eth-selected-contract-whale-row",
          ts: 1_700_000_100_000,
          symbol: "ETH",
          baseAsset: "ETH",
          quantityUnit: "ETH",
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
          eventLifecycle: {
            eventId: "cwm-event:ETH:aggressive_buy:1700000100000",
            status: "active",
            startTime: 1_700_000_100_000,
            lastUpdateTime: 1_700_000_100_000,
            volumeAccumulated: 16869,
            updateCount: 1,
          },
          eventQuality: {
            qualityScore: 0.71,
            mergeSimilarityScore: 1,
            valid: true,
            falseEventFlags: [],
          },
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      error: null,
    });

    render(<ContractWhaleMonitor lockedSymbol="ETH" />);

    await screen.findByText("主力合约监控");
    expect(screen.getByText("币种：ETH（当前页面固定）")).toBeInTheDocument();
    expect(screen.queryByLabelText("币种")).not.toBeInTheDocument();
    expect(screen.queryByText("SOL")).not.toBeInTheDocument();
    await waitFor(() => expect(fetchContractWhaleLatest).toHaveBeenLastCalledWith(50, "ETH"));
    expect(await screen.findByText("总量 761 ETH · dominance 80.7%")).toBeInTheDocument();
    expect(screen.getByText("净买入 614 ETH")).toBeInTheDocument();
    expect(screen.queryByText("总量 761 BTC · dominance 80.7%")).not.toBeInTheDocument();
    expect(screen.queryByText("净买入 614 BTC")).not.toBeInTheDocument();
  });

  it("syncs filters to the history API", async () => {
    const user = userEvent.setup();
    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.selectOptions(screen.getByLabelText("等级"), "critical");
    expect(screen.getByRole("option", { name: "大于 500（正负）" })).toBeInTheDocument();
    await user.selectOptions(screen.getByLabelText("净方向"), "abs500");
    const impactSelect = screen.getByLabelText("冲击等级");
    expect(impactSelect).toHaveDisplayValue("全部");
    await user.selectOptions(impactSelect, "A");
    expect(impactSelect).toHaveDisplayValue("A");

    await waitFor(() =>
      expect(fetchContractEvents).toHaveBeenLastCalledWith(
        expect.objectContaining({
          symbol: "BTC",
          severity: "critical",
          net_direction: "abs500",
          impact_level: "A",
          range: "7d",
          limit: 20,
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

  it("renders dedicated structure setups and risk context panels without execution wording", async () => {
    render(<ContractWhaleMonitor />);

    expect(await screen.findByText("Structure Setups")).toBeInTheDocument();
    expect(screen.getByText("结构机会")).toBeInTheDocument();
    expect(screen.getByText("Bullish bias")).toBeInTheDocument();
    expect(screen.getByText("HIGH CONF")).toBeInTheDocument();
    expect(screen.getByText(/跌破主力吸收参考位/)).toBeInTheDocument();
    expect(screen.queryByText("立即做多")).not.toBeInTheDocument();
    expect(screen.queryByText("立即做空")).not.toBeInTheDocument();

    expect(screen.getByText("Risk Context")).toBeInTheDocument();
    expect(screen.getByText("No-Trade Zones")).toBeInTheDocument();
    expect(screen.getAllByText("HIGH RISK").length).toBeGreaterThan(0);
    expect(screen.getByText("当前存在较强假突破风险，交易参考需要让位于风险抑制。")).toBeInTheDocument();
    expect(screen.getAllByText("69,900 - 70,040").length).toBeGreaterThan(0);
  });

  it("renders contract market event rows from the FinalEventStore projection", async () => {
    fetchContractWhaleLatest.mockResolvedValueOnce({
      summary: {
        status: "active",
        healthStatus: "healthy",
        latestDirection: "neutral",
        latestSeverity: "medium",
        signalCount: 0,
        readOnly: true,
        enabled: true,
        dryRun: true,
        contractDataQuality: 90,
        spotDataQuality: 80,
        overallDataQuality: 85,
        discordDryRunStats: {},
        marketStructureLite: {},
        trend60s: {},
        exchanges: {},
        platforms: {},
      },
      items: [],
      error: null,
    });
    fetchContractEvents.mockResolvedValueOnce({
      items: [],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      error: null,
    });
    fetchFinalEventsV2.mockResolvedValueOnce({
      active: [],
      closed: [
        {
          id: "cwm-event:BTC:downside_absorption:1700000015000",
          eventId: "cwm-event:BTC:downside_absorption:1700000015000",
          finalEventId: "cwm-event:BTC:downside_absorption:1700000015000",
          ts: 1_700_000_015_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
          windowSec: 15,
          rawVolume: 4876,
          impactScore: 2.14,
          zScore: 2.14,
          percentile: 93,
          normalizedScore: 0.88,
          normalizedStrength: "EXTREME",
          impactLevel: "A",
          signalLevel: "L3",
          signalLabel: "HIGH IMPACT EVENT",
          signalType: "downside_absorption",
          direction: "sell",
          severity: "medium",
          score: 51,
          mainForceScore: 51,
          spotScore: 59,
          contractScore: 47,
          totalVolumeBtc: 4876,
          netVolumeBtc: -4619,
          totalNotionalUsd: 313_000_000,
          dominance: 0.947,
          triggerPriceUsd: 64_166,
          orderPriceUsd: 64_166,
          priceMovePct: 0.19,
          mainExchange: "binance",
          mergedFrom: [
            "contract-whale:BTC:5:1700000015000:downside_absorption",
          ],
          eventLifecycle: {
            eventId: "cwm-event:BTC:downside_absorption:1700000015000",
            status: "closed",
            startTime: 1_700_000_000_000,
            lastUpdateTime: 1_700_000_015_000,
            volumeAccumulated: 4876,
            updateCount: 2,
          },
          eventQuality: {
            qualityScore: 0.81,
            mergeSimilarityScore: 0.84,
            valid: true,
            falseEventFlags: [],
          },
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      error: null,
    });

    render(<ContractWhaleMonitor />);

    expect(await screen.findByText("CLOSED EVENTS (finalized)")).toBeInTheDocument();
    await waitFor(() =>
      expect(fetchFinalEventsV2).toHaveBeenCalledWith(
        expect.objectContaining({
          symbol: "BTC",
          range: "24h",
          limit: 30,
        }),
      ),
    );
    expect(screen.getByTestId("raw-contract-whale-signals-closed")).toHaveTextContent("下方吸收");
    expect(screen.getByTestId("raw-contract-whale-signals-closed")).toHaveTextContent("4,876 BTC");
    expect(screen.getByTestId("raw-contract-whale-signals-closed")).toHaveTextContent("Q 81");
    expect(screen.getByTestId("raw-contract-whale-signals-closed")).toHaveTextContent("L3 / A");
    expect(screen.getByTestId("raw-contract-whale-signals-closed")).toHaveTextContent("HIGH IMPACT EVENT");
    expect(screen.getByTestId("raw-contract-whale-signals-closed")).toHaveTextContent("2.14x · z 2.14 · P93");
  });

  it("shows a sync lag warning when latest is newer than the historical event stream", async () => {
    fetchContractWhaleLatest.mockResolvedValueOnce({
      summary: {
        status: "active",
        healthStatus: "healthy",
        latestDirection: "buy",
        latestSeverity: "medium",
        signalCount: 1,
        readOnly: true,
        enabled: true,
        dryRun: true,
        contractDataQuality: 90,
        spotDataQuality: 80,
        overallDataQuality: 85,
        discordDryRunStats: {},
        marketStructureLite: {},
        trend60s: {},
        exchanges: {},
        platforms: {},
      },
      items: [{ id: "latest-lag-row", ts: 1_700_000_060_000, symbol: "BTC", windowSec: 15, signalType: "aggressive_buy" }],
      error: null,
    });
    fetchContractEvents.mockResolvedValueOnce({
      items: [],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      serverTime: 1_700_000_060_500,
      lastEventTs: 1_700_000_000_000,
      error: null,
    });

    render(<ContractWhaleMonitor />);

    expect(await screen.findByText(/数据延迟：latest 已更新到/)).toBeInTheDocument();
  });

  it("shows a lifecycle sync warning when final-events lags behind history", async () => {
    fetchContractEvents.mockResolvedValueOnce({
      items: [],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      serverTime: 1_700_000_060_500,
      lastEventTs: 1_700_000_040_000,
      maxEventTs: 1_700_000_040_000,
      historyLagSec: 20,
      latestLagSec: 0,
      cacheAgeSec: 1,
      cacheTtlSec: 5,
      error: null,
    });
    fetchFinalEventsV2.mockResolvedValueOnce({
      active: [],
      closed: [],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      serverTime: 1_700_000_060_500,
      lastEventTs: 1_700_000_010_000,
      maxEventTs: 1_700_000_010_000,
      generatedAt: 1_700_000_060_000,
      cacheAgeSec: 1,
      cacheTtlSec: 10,
      projectionLagSec: 30,
      error: null,
    });

    render(<ContractWhaleMonitor />);

    expect(
      await screen.findByText("生命周期视图同步中：落后历史事件流 30 秒，不代表数据丢失。"),
    ).toBeInTheDocument();
  });

  it("renders a latency guard summary panel from latency debug diagnostics", async () => {
    fetchContractWhaleLatencyDebug.mockResolvedValueOnce({
      symbol: "BTC",
      range: "24h",
      serverTime: 1_700_000_060_500,
      timeline: {
        symbol: "BTC",
        range: "24h",
        source: "contract_whale_signals",
        eventTs: 1_700_000_030_000,
        processedTs: 1_700_000_054_000,
        persistedTs: 1_700_000_052_000,
        servedTs: 1_700_000_060_500,
        timelineLagSec: 30,
        views: {
          latest: { count: 8, maxEventTs: 1_700_000_050_000, driftVsCanonicalSec: 20 },
          history: { count: 6, maxEventTs: 1_700_000_030_000, driftVsCanonicalSec: 0 },
          finalEventsV2: { count: 6, maxEventTs: 1_700_000_020_000, driftVsCanonicalSec: 10 },
          flow: { updatedAt: 1_700_000_058_000, driftVsCanonicalSec: 28 },
        },
      },
      latest: {
        count: 8,
        maxTs: 1_700_000_050_000,
        ageSec: 12,
        staleCount: 0,
      },
      contractEvents: {
        count: 6,
        maxEventTs: 1_700_000_030_000,
        lagSec: 30,
        lagVsLatestSec: 20,
        cacheAgeSec: 4,
        cacheTtlSec: 5,
      },
      finalEventsV2: {
        activeCount: 2,
        closedCount: 4,
        maxEventTs: 1_700_000_020_000,
        projectionLagSec: 30,
        cacheAgeSec: 6,
        cacheTtlSec: 10,
        generatedAt: 1_700_000_054_000,
      },
      flow: {
        updatedAt: 1_700_000_058_000,
        flowLagSec: 2,
      },
      diagnosis: {
        layer: "final_events_v2",
        reason: "projection_lagging_history",
      },
      error: null,
    });

    render(<ContractWhaleMonitor />);

    expect(await screen.findByText(/LATENCY GUARD/i)).toBeInTheDocument();
    expect(screen.getByText(/final_events_v2/i)).toBeInTheDocument();
    expect(screen.getByText(/projection_lagging_history/i)).toBeInTheDocument();
    expect(screen.getAllByText(/Market Time/i).length).toBeGreaterThan(0);
    expect(screen.getByText(/^System Lag$/i)).toBeInTheDocument();
    expect(screen.getByText(/lag 30 秒/i)).toBeInTheDocument();
    expect(screen.getByText(/latest 20 秒/i)).toBeInTheDocument();
    expect(screen.getByText(/history 0 秒/i)).toBeInTheDocument();
  });

  it("shows contract event debug counts and explains latest versus history drift", async () => {
    fetchContractEventDebugCounts.mockResolvedValueOnce({
      symbol: "BTC",
      range: "24h",
      generatedAt: "2026-06-27T00:00:00Z",
      db: {
        contractWhaleSignalsTotal24h: 12,
        contractWhaleSignalsBtc24h: 8,
        oldestTs: 1_700_000_000_000,
        newestTs: 1_700_000_100_000,
      },
      apiQuery: {
        matchedBeforeFilter: 8,
        matchedAfterSymbolFilter: 8,
        matchedAfterRangeFilter: 8,
        matchedAfterSeverityFilter: null,
        matchedAfterWindowFilter: null,
        matchedAfterDirectionFilter: null,
        returnedItems: 1,
        limit: 100,
      },
      visibility: {
        visibleCount: 1,
        hiddenCount: 7,
        hiddenReasons: {
          priceDeviationGt5pct: 6,
          missingPrice: 0,
          badQuality: 1,
          disabledMonitor: 0,
          unknown: 0,
        },
      },
      latest: {
        latestCount: 2,
        latestSymbols: ["BTC", "BTC"],
      },
      finalEventsV2: {
        activeCount: 1,
        closedCount: 0,
      },
      latestVsHistory: [
        {
          latestEventId: "latest-visible",
          symbol: "BTC",
          ts: 1_700_000_000_000,
          existsInHistory: true,
          historyEventId: "cwm-event:BTC:aggressive_buy:1700000000000",
          notInHistoryReason: null,
        },
        {
          latestEventId: "latest-pending",
          symbol: "BTC",
          ts: 1_700_000_050_000,
          existsInHistory: false,
          historyEventId: null,
          notInHistoryReason: "latest_snapshot_not_persisted_yet",
        },
      ],
      finalEventsProjection: {
        source: "contract_whale_signals",
        rawSignals: 8,
        afterFilter: 1,
        mergedEvents: 1,
        active: 1,
        closed: 0,
        range: "24h",
      },
      error: null,
    });

    render(<ContractWhaleMonitor />);

    expect(await screen.findByText(/24h BTC 历史事件：后端返回 1 条，可见 1 条，隐藏 7 条/)).toBeInTheDocument();
    expect(screen.getByText(/latest 是实时快照，history 是持久化历史事件流/)).toBeInTheDocument();
  });

  it("can expand hidden contract events with explicit hidden reasons", async () => {
    const user = userEvent.setup();
    fetchContractEventDebugCounts.mockResolvedValueOnce({
      symbol: "BTC",
      range: "24h",
      generatedAt: "2026-06-27T00:00:00Z",
      db: {
        contractWhaleSignalsTotal24h: 12,
        contractWhaleSignalsBtc24h: 8,
        oldestTs: 1_700_000_000_000,
        newestTs: 1_700_000_100_000,
      },
      apiQuery: {
        matchedBeforeFilter: 8,
        matchedAfterSymbolFilter: 8,
        matchedAfterRangeFilter: 8,
        matchedAfterSeverityFilter: null,
        matchedAfterWindowFilter: null,
        matchedAfterDirectionFilter: null,
        returnedItems: 1,
        limit: 100,
      },
      visibility: {
        visibleCount: 1,
        hiddenCount: 7,
        hiddenReasons: {
          priceDeviationGt5pct: 6,
          missingPrice: 0,
          badQuality: 1,
          disabledMonitor: 0,
          unknown: 0,
        },
      },
      latest: {
        latestCount: 2,
        latestSymbols: ["BTC", "BTC"],
      },
      finalEventsV2: {
        activeCount: 1,
        closedCount: 0,
      },
      latestVsHistory: [],
      finalEventsProjection: {
        source: "contract_whale_signals",
        rawSignals: 8,
        afterFilter: 1,
        mergedEvents: 1,
        active: 1,
        closed: 0,
        range: "24h",
      },
      error: null,
    });
    fetchContractEvents.mockResolvedValueOnce({
      items: [
        {
          id: "contract-event-row",
          eventId: "cwm-event:BTC:aggressive_buy:1700000000000",
          sourceSignalId: "contract-whale-row",
          ts: 1_700_000_000_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
          windowSec: 15,
          signalType: "aggressive_buy",
          direction: "buy",
          severity: "s",
          volumeBtc: 4820,
          netVolumeBtc: 3260,
          notionalUsd: 337_000_000,
          isVisible: true,
          hiddenReason: null,
          hiddenDetail: null,
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 50,
      range: "24h",
      serverTime: 1_700_000_060_000,
      lastEventTs: 1_700_000_000_000,
      error: null,
    });
    fetchContractEvents.mockResolvedValueOnce({
      items: [
        {
          id: "contract-event-row",
          eventId: "cwm-event:BTC:aggressive_buy:1700000000000",
          sourceSignalId: "contract-whale-row",
          ts: 1_700_000_000_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
          windowSec: 15,
          signalType: "aggressive_buy",
          direction: "buy",
          severity: "s",
          volumeBtc: 4820,
          netVolumeBtc: 3260,
          notionalUsd: 337_000_000,
          isVisible: true,
          hiddenReason: null,
          hiddenDetail: null,
        },
        {
          id: "contract-event-hidden-row",
          eventId: "cwm-event:BTC:aggressive_sell:hidden",
          sourceSignalId: "contract-whale-hidden-row",
          ts: 1_700_000_030_000,
          symbol: "BTC",
          baseAsset: "BTC",
          quantityUnit: "BTC",
          windowSec: 5,
          signalType: "aggressive_sell",
          direction: "sell",
          severity: "medium",
          volumeBtc: 920,
          netVolumeBtc: -640,
          notionalUsd: 61_000_000,
          price: 64_500,
          priceDeviationPct: 7.3,
          isVisible: false,
          hiddenReason: "price_deviation_gt_5pct",
          hiddenDetail: "price deviation 7.3% > max 5%",
        },
      ],
      nextCursor: null,
      hasMore: false,
      limit: 100,
      range: "24h",
      serverTime: 1_700_000_060_000,
      lastEventTs: 1_700_000_030_000,
      error: null,
    });

    render(<ContractWhaleMonitor />);

    const expandButton = await screen.findByRole("button", { name: "查看隐藏事件" });
    await user.click(expandButton);

    await waitFor(() =>
      expect(fetchContractEvents).toHaveBeenLastCalledWith(
        expect.objectContaining({
          symbol: "BTC",
          range: "7d",
          limit: 100,
          includeHidden: true,
        }),
      ),
    );
    expect(screen.getByText("隐藏事件")).toBeInTheDocument();
    expect(await screen.findByText("price_deviation_gt_5pct")).toBeInTheDocument();
    expect(await screen.findByText("price deviation 7.3% > max 5%")).toBeInTheDocument();
  });

  it("keeps the main historical event list visible when debug counts fail", async () => {
    fetchContractEventDebugCounts.mockResolvedValueOnce({ error: "debug_counts_unavailable" });

    render(<ContractWhaleMonitor />);

    expect(await screen.findByTestId("raw-contract-whale-signals")).toBeInTheDocument();
    expect(screen.queryByText(/24h BTC 历史事件：后端返回/)).not.toBeInTheDocument();
  });

  it("shows a spot-only explanation when coinbase is selected in contract history", async () => {
    const user = userEvent.setup();
    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.selectOptions(screen.getByLabelText("交易所"), "coinbase");

    await waitFor(() =>
      expect(fetchContractEvents).toHaveBeenLastCalledWith(
        expect.objectContaining({
          symbol: "BTC",
          exchange: "coinbase",
          range: "7d",
          limit: 20,
        }),
      ),
    );
    expect(screen.getByText("Coinbase 当前仅启用现货，未启用合约；本页只统计 perp 合约成交，因此不会返回 Coinbase 合约信号。")).toBeInTheDocument();
  });

  it("opens a read-only detail modal from the signal row", async () => {
    const user = userEvent.setup();
    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.click(screen.getAllByRole("button", { name: /查看主力合约信号详情 contract-whale-row raw-contract-whale-signals/ })[0]);

    const dialog = screen.getByRole("dialog", { name: "主力合约信号详情" });
    expect(dialog).toBeInTheDocument();
    expect(dialog).toHaveClass("workspace-dialog", "contract-detail-inspector");
    expect(screen.getByTestId("contract-detail-header")).toHaveTextContent("EVENT INSPECTOR");
    expect(screen.getByTestId("contract-detail-summary")).toHaveTextContent("事件状态");
    expect(screen.getByTestId("contract-detail-body")).toBeInTheDocument();
    expect(screen.getByTestId("contract-detail-rail")).toHaveTextContent("Discord Gate");
    expect(dialog).toHaveTextContent("READ ONLY");
    expect(screen.getByText("Contract Whale Detail")).toBeInTheDocument();
    expect(screen.getByText("Discord Gate")).toBeInTheDocument();
    expect(screen.getByText("可进入推送判断")).toBeInTheDocument();
    expect(screen.getByText("critical_or_s_gate")).toBeInTheDocument();
    expect(screen.getByText("dry-run 会推送")).toBeInTheDocument();
    expect(screen.getAllByText("现货确认").length).toBeGreaterThan(0);
    expect(screen.getByText("现货与合约同向")).toBeInTheDocument();
    expect(screen.getByText("现货主动买入跟随合约方向")).toBeInTheDocument();
    expect(screen.getByText("流量口径")).toBeInTheDocument();
    expect(screen.getAllByText("窗口总流量 BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("主动买 BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("主动卖 BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("净方向 BTC").length).toBeGreaterThan(0);
    expect(screen.getByText("来源交易所")).toBeInTheDocument();
    expect(screen.getByText("合并窗口")).toBeInTheDocument();
    expect(screen.getByText("跨交易所聚合")).toBeInTheDocument();
    expect(screen.getByText("生命周期累计")).toBeInTheDocument();
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
    expect(screen.getAllByText(/总流量 = 主动买量 \+ 主动卖量/).length).toBeGreaterThan(0);
    expect(screen.getByText("contract-whale:BTC:5:1700000000000:buy")).toBeInTheDocument();
    expect(screen.queryByText(/rawPayload/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/webhook/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/token/i)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "关闭主力合约信号详情" }));
    expect(screen.queryByRole("dialog", { name: "主力合约信号详情" })).not.toBeInTheDocument();
  });

  it("shows impact-level discord gate semantics for medium B events", async () => {
    const mediumBImpactSignal = {
      id: "medium-b-impact-row",
      sourceSignalId: "medium-b-impact-row",
      eventId: "cwm-event:BTC:aggressive_sell:1700000010000",
      ts: 1_700_000_010_000,
      symbol: "BTC",
      baseAsset: "BTC",
      quantityUnit: "BTC",
      windowSec: 60,
      signalType: "aggressive_sell",
      direction: "sell",
      severity: "medium",
      score: 61,
      mainForceScore: 61,
      spotScore: 42,
      contractScore: 61,
      totalVolumeBtc: 556,
      netVolumeBtc: -420,
      totalNotionalUsd: 33_000_000,
      dominance: 0.755,
      triggerPriceUsd: 59_386,
      orderPriceUsd: 59_386,
      currentMarketPriceUsd: 59_386,
      priceDeviationPct: 0,
      priceDeviationFiltered: false,
      priceMovePct: -0.08,
      mainExchange: "binance",
      dynamicMultiple: 1.87,
      percentileLevel: 86,
      impactLevel: "B",
      signalLevel: "L2",
      signalLabel: "MEDIUM IMPACT EVENT",
      normalizedStrength: "MEDIUM",
      impactScore: 1.87,
      impactZScore: 1.98,
      dataQuality: 88,
      discordEligible: true,
      discordSent: false,
      discordReason: "impact_level_gate",
      discordWouldSend: true,
      exchanges: [
        {
          exchange: "binance",
          buyVolumeBtc: 68,
          sellVolumeBtc: 488,
          totalVolumeBtc: 556,
          buyShare: 0.122,
          sellShare: 0.878,
          netVolumeBtc: -420,
          dominance: 0.755,
          netContributionShare: 1,
        },
      ],
      eventLifecycle: {
        eventId: "cwm-event:BTC:aggressive_sell:1700000010000",
        status: "active",
        startTime: 1_700_000_010_000,
        lastUpdateTime: 1_700_000_010_000,
        volumeAccumulated: 556,
        updateCount: 1,
      },
      eventQuality: {
        qualityScore: 0.88,
        mergeSimilarityScore: 0.76,
        valid: true,
        falseEventFlags: [],
      },
      finalResult: "B 级市场冲击触发合约主力提醒",
    };

    fetchContractWhaleLatest.mockResolvedValueOnce({
      summary: {
        status: "strong",
        healthStatus: "healthy",
        direction: "sell",
        latestDirection: "sell",
        latestSeverity: "medium",
        signalCount: 1,
        readOnly: true,
        enabled: true,
        dryRun: true,
        exchanges: {},
      },
      items: [mediumBImpactSignal],
      error: null,
    });
    fetchContractEvents.mockResolvedValueOnce({
      items: [mediumBImpactSignal],
      error: null,
    });

    const user = userEvent.setup();
    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.click(screen.getByRole("button", { name: /查看主力合约信号详情 medium-b-impact-row/ }));

    expect(screen.getByRole("dialog", { name: "主力合约信号详情" })).toBeInTheDocument();
    expect(screen.getByText("Discord Gate")).toBeInTheDocument();
    expect(screen.getByText("信号等级")).toBeInTheDocument();
    expect(screen.getAllByText("Medium").length).toBeGreaterThan(0);
    expect(screen.getByText("市场冲击")).toBeInTheDocument();
    expect(screen.getByText("B / L2")).toBeInTheDocument();
    expect(screen.getByText("推送原因")).toBeInTheDocument();
    expect(screen.getByText("市场冲击 B")).toBeInTheDocument();
    expect(screen.getByText("dry-run 会推送")).toBeInTheDocument();
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

  it("refreshes status every 5s and heavyweight event projections every 15s", async () => {
    vi.useFakeTimers();

    render(<ContractWhaleMonitor />);
    await vi.advanceTimersByTimeAsync(0);

    expect(screen.getByText("主力合约监控")).toBeInTheDocument();
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(1);
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(1);
    expect(fetchContractWhaleSummary).toHaveBeenLastCalledWith("BTC");
    expect(fetchContractEvents).toHaveBeenCalledTimes(1);
    expect(fetchFinalEventsV2).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(5_000);
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(2);
    expect(fetchContractWhaleSummary).toHaveBeenLastCalledWith("BTC");
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(2);
    expect(fetchContractEvents).toHaveBeenCalledTimes(1);
    expect(fetchFinalEventsV2).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(10_000);
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(4);
    expect(fetchContractWhaleSummary).toHaveBeenLastCalledWith("BTC");
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(4);
    expect(fetchContractEvents).toHaveBeenCalledTimes(2);
    expect(fetchFinalEventsV2).toHaveBeenCalledTimes(2);
  });

  it("rerenders a stable event row when its evidence changes", async () => {
    const defaultContractEvents = await fetchContractEvents.getMockImplementation()();
    const updatedContractEvents = structuredClone(defaultContractEvents);
    updatedContractEvents.items[0].discordSent = true;
    fetchContractEvents
      .mockResolvedValueOnce(defaultContractEvents)
      .mockResolvedValueOnce(updatedContractEvents);
    vi.useFakeTimers();

    render(<ContractWhaleMonitor />);
    await vi.advanceTimersByTimeAsync(0);

    expect(within(screen.getByTestId("raw-contract-whale-signals")).getByText("待推")).toBeInTheDocument();

    await vi.advanceTimersByTimeAsync(15_000);

    expect(within(screen.getByTestId("raw-contract-whale-signals")).getByText("已推")).toBeInTheDocument();
  });

  it("keeps latest requests scoped to locked ETH", async () => {
    render(<ContractWhaleMonitor lockedSymbol="ETH" />);

    await screen.findByText("主力合约监控");
    expect(screen.getByText("币种：ETH（当前页面固定）")).toBeInTheDocument();
    expect(screen.queryByLabelText("币种")).not.toBeInTheDocument();
    expect(screen.queryByText("SOL")).not.toBeInTheDocument();

    await waitFor(() => expect(fetchContractWhaleLatest).toHaveBeenLastCalledWith(50, "ETH"));
  });
});
