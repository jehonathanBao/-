import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import ContractWhaleMonitor from "../components/ContractWhaleMonitor.jsx";
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
          signalId: "contract-whale-row",
          rank: 1,
          direction: "LONG",
          setupType: "主力拉盘",
          score: 87,
          confidence: 79,
          confidenceLabel: "HIGH",
          regimeContext: "main_force_long_build",
          windowSec: 15,
          entryZone: {
            lowPrice: 69810,
            highPrice: 69950,
            label: "69,810 - 69,950",
          },
          invalidation: {
            priceLevel: 69640,
            reason: "跌破主力吸收参考位，说明顺势跟随失效。",
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
          signalId: "contract-whale-row",
          rank: 1,
          setupType: "Absorption continuation",
          directionBias: "BULLISH_BIAS",
          score: 87,
          confidence: 84,
          confidenceLabel: "HIGH",
          entryZone: {
            lowPrice: 69810,
            highPrice: 69950,
            label: "69,810 - 69,950",
          },
          invalidation: {
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
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("does not show enabled contract platforms as offline while initial requests are pending", () => {
    fetchContractWhaleLatest.mockReturnValueOnce(new Promise(() => {}));
    fetchFinalEventsV2.mockReturnValueOnce(new Promise(() => {}));
    fetchContractEvents.mockReturnValueOnce(new Promise(() => {}));
    fetchContractEventDebugCounts.mockReturnValueOnce(new Promise(() => {}));
    fetchContractRetentionStatus.mockReturnValueOnce(new Promise(() => {}));

    render(<ContractWhaleMonitor />);

    expect(screen.getByTestId("platform-status-chip-binance")).toHaveTextContent("等待数据");
    expect(screen.getByTestId("platform-status-chip-bitfinex")).toHaveTextContent("等待数据");
    expect(screen.getByTestId("platform-status-chip-binance")).not.toHaveTextContent("离线");
    expect(screen.getByTestId("platform-status-chip-bitfinex")).not.toHaveTextContent("离线");
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(1);
    expect(fetchContractEventDebugCounts).toHaveBeenCalledTimes(1);
  });

  it("keeps the core contract-whale content visible while retention stays deferred", async () => {
    fetchContractRetentionStatus.mockReturnValueOnce(new Promise(() => {}));

    render(<ContractWhaleMonitor />);

    expect(screen.getByText("主力合约监控")).toBeInTheDocument();
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(1);
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(1);
    expect(fetchContractEvents).toHaveBeenCalledTimes(1);
    expect(fetchFinalEventsV2).toHaveBeenCalledTimes(1);
    expect(fetchContractEventDebugCounts).toHaveBeenCalledTimes(1);
    expect(fetchContractWhaleEvents).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.getByText("强异动")).toBeInTheDocument());
    expect(fetchContractRetentionStatus).not.toHaveBeenCalled();
    expect(screen.getByText("Buy 62.0% / Sell 38.0%")).toBeInTheDocument();
    expect(screen.getByText("ACTIVE EVENTS (updated)")).toBeInTheDocument();
    expect(screen.getByText("CLOSED EVENTS (finalized)")).toBeInTheDocument();
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
    expect(screen.getByText("Institutional Analysis Terminal")).toBeInTheDocument();
    expect(screen.getByText("半机构级分析终端")).toBeInTheDocument();
    expect(screen.getByText("Market Regime")).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Market Intelligence" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Trade Ideas" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Risk / No-Trade" })).toBeInTheDocument();
    expect(screen.getByText("Liquidity Behavior")).toBeInTheDocument();
    expect(screen.getByText("Signal Strength Ranking")).toBeInTheDocument();
    expect(screen.getByText("Opportunity Map")).toBeInTheDocument();
    expect(screen.getByText("RANGING")).toBeInTheDocument();
    expect(screen.getByText("Regime 78%")).toBeInTheDocument();
    expect(screen.getByText("Absorption")).toBeInTheDocument();
    expect(screen.getByText("Fake Breakout")).toBeInTheDocument();
    expect(screen.getByText("Absorption Zone")).toBeInTheDocument();
    expect(screen.getByText("Fake Breakout Risk")).toBeInTheDocument();
    expect(screen.getAllByText("69,760 - 69,890").length).toBeGreaterThan(0);
    expect(screen.getAllByText("69,980 - 70,040").length).toBeGreaterThan(0);
    expect(screen.queryByText("Entry Zone")).not.toBeInTheDocument();
    expect(screen.queryByText("Invalidation")).not.toBeInTheDocument();
    expect(screen.getByText("原始候选")).toBeInTheDocument();
    expect(screen.getByText("降噪后事件")).toBeInTheDocument();
    expect(screen.getByText("结构机会")).toBeInTheDocument();
    expect(screen.getByText("压缩质量")).toBeInTheDocument();
    expect(screen.getByText("67%")).toBeInTheDocument();
    expect(screen.getAllByText("87/100").length).toBeGreaterThan(0);
    expect(screen.getAllByText("主力拉盘").length).toBeGreaterThan(0);
    expect(screen.getByText(/多窗口主买一致/)).toBeInTheDocument();
    expect(screen.getByText("Whale Entity List")).toBeInTheDocument();
    expect(screen.getByText("Trajectory Timeline")).toBeInTheDocument();
    expect(screen.getByText("Stealth Curve (gamma)")).toBeInTheDocument();
    expect(screen.getByText("Hazard Curve (lambda proxy)")).toBeInTheDocument();
    expect(screen.getByText("合约市场事件")).toBeInTheDocument();
    expect(screen.getByText(/当前列表为历史事件流，不是 latest 快照/)).toBeInTheDocument();
    expect(screen.getAllByText("窗口总流量 BTC").length).toBeGreaterThan(0);
    expect(screen.getAllByText("生命周期累计流量 BTC").length).toBeGreaterThan(0);
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
        range: "24h",
        limit: 50,
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

    render(<ContractWhaleMonitor />);

    expect(await screen.findByText("16,869 ETH")).toBeInTheDocument();
    expect(screen.getByText("净买入 614 ETH")).toBeInTheDocument();
    expect(screen.getByText("总量 761 ETH · dominance 80.7%")).toBeInTheDocument();
    expect(screen.queryByText("净买入 614 BTC")).not.toBeInTheDocument();
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
    fetchContractEvents
      .mockResolvedValueOnce({ items: [], nextCursor: null, hasMore: false, limit: 100, range: "24h", error: null })
      .mockResolvedValueOnce({
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
    fetchFinalEventsV2
      .mockResolvedValueOnce({ active: [], closed: [], nextCursor: null, hasMore: false, limit: 100, range: "24h", error: null })
      .mockResolvedValueOnce({
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

    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.selectOptions(screen.getByLabelText("币种"), "ETH");

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

    await waitFor(() =>
      expect(fetchContractEvents).toHaveBeenLastCalledWith(
        expect.objectContaining({
          symbol: "BTC",
          severity: "critical",
          net_direction: "abs500",
          range: "24h",
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

  it("switches institutional terminal tabs into trade ideas and risk context without execution wording", async () => {
    const user = userEvent.setup();
    render(<ContractWhaleMonitor />);

    expect(await screen.findByText("Institutional Analysis Terminal")).toBeInTheDocument();

    await user.click(screen.getByRole("tab", { name: "Trade Ideas" }));
    expect(screen.getByText("Trade Ideas")).toBeInTheDocument();
    expect(screen.getByText("方向偏置")).toBeInTheDocument();
    expect(screen.getByText("参考区")).toBeInTheDocument();
    expect(screen.getByText("失效参考位")).toBeInTheDocument();
    expect(screen.getByText("BULLISH_BIAS")).toBeInTheDocument();
    expect(screen.getByText("69,810 - 69,950")).toBeInTheDocument();
    expect(screen.getByText("跌破主力吸收参考位，说明当前结构支持减弱。")).toBeInTheDocument();
    expect(screen.queryByText("立即做多")).not.toBeInTheDocument();
    expect(screen.queryByText("立即做空")).not.toBeInTheDocument();

    await user.click(screen.getByRole("tab", { name: "Risk / No-Trade" }));
    expect(screen.getByText("No-trade Zones")).toBeInTheDocument();
    expect(screen.getByText("HIGH")).toBeInTheDocument();
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
          range: "24h",
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
          range: "24h",
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
    await user.click(screen.getAllByRole("button", { name: /查看主力合约信号详情 contract-whale-row raw-contract-whale-signals/ })[0]);

    expect(screen.getByRole("dialog", { name: "主力合约信号详情" })).toBeInTheDocument();
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

  it("polls summary every 5s and latest signals every 3s while visible", async () => {
    vi.useFakeTimers();

    render(<ContractWhaleMonitor />);

    expect(screen.getByText("主力合约监控")).toBeInTheDocument();
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(1);
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(1);
    expect(fetchContractWhaleSummary).toHaveBeenLastCalledWith("BTC");

    await vi.advanceTimersByTimeAsync(5_000);
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(2);
    expect(fetchContractWhaleSummary).toHaveBeenLastCalledWith("BTC");
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(2);

    await vi.advanceTimersByTimeAsync(5_000);
    expect(fetchContractWhaleSummary).toHaveBeenCalledTimes(3);
    expect(fetchContractWhaleSummary).toHaveBeenLastCalledWith("BTC");
    expect(fetchContractWhaleLatest).toHaveBeenCalledTimes(4);
  });

  it("keeps latest requests scoped to ETH after symbol switch", async () => {
    const user = userEvent.setup();

    render(<ContractWhaleMonitor />);

    await screen.findByText("主力合约监控");
    await user.selectOptions(screen.getByLabelText("币种"), "ETH");

    await waitFor(() => expect(fetchContractWhaleLatest).toHaveBeenLastCalledWith(50, "ETH"));
  });
});
