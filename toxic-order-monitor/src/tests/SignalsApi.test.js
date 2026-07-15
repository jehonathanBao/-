import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { evaluateDiscordAlertGate } from "../api/alertGate.js";
import { fetchSignals, fetchSignalsSnapshot, mapInboxItemToSignal, runtimeFromPayload } from "../api/signals.js";

vi.mock("axios", () => ({
  default: {
    get: vi.fn(),
  },
}));

describe("signals api mapping", () => {
  beforeEach(() => {
    axios.get.mockReset();
    vi.unstubAllEnvs();
    vi.stubEnv("VITE_API_BASE_URL", "");
  });

  it("treats missing runtimeModified or analysisOnly as an unconfirmed runtime", () => {
    expect(runtimeFromPayload({
      readOnly: true,
      monitoringStarted: true,
      executionEnabled: false,
    }).phase).toBe("unavailable");
  });

  it("maps backend inbox items into persistent inbox signal shape", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        readOnly: true,
        monitoringStarted: true,
        executionEnabled: false,
        runtimeModified: false,
        analysisOnly: true,
        items: [
          inboxItem({ signalId: "runtime-high", severity: "high", riskScore: 83, dataQualityScore: 82 }),
          inboxItem({ signalId: "runtime-medium", severity: "medium", riskScore: 67, dataQualityScore: 74 }),
        ],
      },
    });

    const signals = await fetchSignals();

    expect(axios.get).toHaveBeenCalledWith("/api/toxicity/signal-inbox/recent");
    expect(signals.map((signal) => signal.id)).toEqual(["runtime-high", "runtime-medium"]);
    expect(signals[0]).toMatchObject({
      risk: "high",
      level: "A",
      score: 83,
      dataQuality: 82,
      status: "unhandled",
    });
    expect(signals[1]).toMatchObject({
      risk: "medium",
      level: "B",
      score: 67,
      dataQuality: 74,
    });
    expect(evaluateDiscordAlertGate(signals[0])).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_INELIGIBLE_PROVENANCE",
    });
    expect(evaluateDiscordAlertGate(signals[1])).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_NON_HIGH_RISK",
    });
  });

  it("preserves the backend detector score exactly and never derives one from severity", () => {
    const authoritative = mapInboxItemToSignal(
      inboxItem({ signalId: "authoritative", severity: "critical", riskScore: 83, dataQualityScore: 79 }),
    );
    const missing = mapInboxItemToSignal(
      inboxItem({ signalId: "missing", severity: "critical", riskScore: undefined, dataQualityScore: undefined }),
    );

    expect(authoritative.score).toBe(83);
    expect(authoritative.authoritativeRiskScore).toBe(83);
    expect(authoritative.dataQuality).toBe(79);
    expect(missing.score).toBeNull();
    expect(missing.dataQuality).toBeNull();
  });

  it("preserves per-metric TOF lineage and liquidation-specific perp lineage", () => {
    const vpinLineage = {
      provenance: "observed",
      available: true,
      fresh: true,
      source: "vpin_service",
      observedAtMs: 1_700_000_000_000,
      unavailableReason: null,
      alertEligible: true,
    };
    const inferredLiquidationLineage = {
      provenance: "inferred",
      available: true,
      fresh: true,
      source: "contract_whale_squeeze_proxy",
      observedAtMs: 1_700_000_000_000,
      unavailableReason: "inferred_not_alert_eligible",
      alertEligible: false,
    };
    const signal = mapInboxItemToSignal({
      ...inboxItem({ signalId: "lineaged-metrics" }),
      tofMetrics: {
        vpinProxy: 78,
        metricLineage: { vpin: vpinLineage },
      },
      perpTofMetrics: {
        observedLiquidationNotional: null,
        squeezeRiskProxy: 63,
        liquidationLineage: inferredLiquidationLineage,
      },
    });

    expect(signal.tofMetrics.metricLineage).toEqual({ vpin: vpinLineage });
    expect(signal.perpTofMetrics.observedLiquidationNotional).toBeNull();
    expect(signal.perpTofMetrics.liquidationLineage).toEqual(inferredLiquidationLineage);
  });

  it("keeps explicit unavailable market-structure fields null instead of falling back to legacy zeroes", () => {
    const signal = mapInboxItemToSignal({
      ...inboxItem({ signalId: "market-structure-unavailable", riskScore: 83, dataQualityScore: 79 }),
      marketStructureScore: null,
      mainForceScore: null,
      mainForceConfirmed: null,
      mainForceConfirmationCount: null,
      extremeImpactScore: null,
      extremeImpactConfirmed: null,
      structureBias: null,
      marketStructureConfidence: null,
      marketStructureDataQuality: null,
      spotScore: null,
      contractScore: null,
      crossConfirmScore: null,
      riskSystems: {
        shortTermToxic: {
          toxicScore: 83,
          shortPressure: -83,
          confidence: 82,
          dataQuality: 79,
        },
        marketStructureScore: {
          mainForceScore: 0,
          mainForceConfirmed: false,
          mainForceConfirmationCount: 0,
          extremeImpactScore: 0,
          extremeImpactConfirmed: false,
          structureBias: 0,
          confidence: 0,
          dataQuality: 0,
          spotScore: 0,
          contractScore: 0,
          crossConfirmScore: 0,
        },
        mainForceStructure: {
          mainForceScore: 0,
          mainForceConfirmed: false,
          extremeImpactScore: 0,
          extremeImpactConfirmed: false,
        },
      },
    });

    expect(signal.riskScore).toBe(83);
    expect(signal.dataQualityScore).toBe(79);
    expect(signal.shortPressure).toBe(-83);
    expect(signal.marketStructureScore).toBeNull();
    expect(signal.riskSystems.marketStructureScore).toBeNull();
    expect(signal.riskSystems.mainForceStructure).toBeNull();
    expect(signal.mainForceScore).toBeNull();
    expect(signal.mainForceConfirmed).toBeNull();
    expect(signal.mainForceConfirmationCount).toBeNull();
    expect(signal.extremeImpactScore).toBeNull();
    expect(signal.extremeImpactConfirmed).toBeNull();
    expect(signal.structureBias).toBeNull();
    expect(signal.marketStructureConfidence).toBeNull();
    expect(signal.marketStructureDataQuality).toBeNull();
    expect(signal.spotScore).toBeNull();
    expect(signal.contractScore).toBeNull();
    expect(signal.crossConfirmScore).toBeNull();
  });

  it("returns a ready snapshot for a successful empty inbox", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [],
        readOnly: true,
        monitoringStarted: true,
        executionEnabled: false,
        runtimeModified: false,
        analysisOnly: true,
      },
    });

    const snapshot = await fetchSignalsSnapshot();

    expect(snapshot.signals).toEqual([]);
    expect(snapshot.request).toMatchObject({ phase: "ready", source: "backend", errorCode: null });
    expect(snapshot.runtime).toMatchObject({
      phase: "confirmed",
      readOnly: true,
      monitoringStarted: true,
      executionEnabled: false,
    });
  });

  it.each([
    [401, "HTTP_401"],
    [403, "HTTP_403"],
    [404, "HTTP_404"],
    [500, "HTTP_500"],
  ])("keeps HTTP %s distinct from a successful empty inbox", async (status, errorCode) => {
    axios.get.mockRejectedValueOnce({ response: { status } });

    const snapshot = await fetchSignalsSnapshot();

    expect(snapshot.signals).toEqual([]);
    expect(snapshot.request).toMatchObject({ phase: "error", source: null, errorCode });
    expect(snapshot.runtime.phase).toBe("unavailable");
  });

  it("reports network and malformed payload failures instead of converting them to empty success", async () => {
    axios.get.mockRejectedValueOnce(new Error("network down"));
    const network = await fetchSignalsSnapshot();

    axios.get.mockResolvedValueOnce({ data: { items: "not-an-array" } });
    const malformed = await fetchSignalsSnapshot();

    expect(network.request).toMatchObject({ phase: "error", errorCode: "NETWORK_ERROR" });
    expect(malformed.request).toMatchObject({ phase: "error", errorCode: "MALFORMED_RESPONSE" });
  });

  it("keeps final result to direction plus core reason", () => {
    const signal = mapInboxItemToSignal(
      inboxItem({
        signalId: "runtime-short",
        directionBias: "short_bias",
        fusionSummary: "large ask wall removed",
        triggerPriceUsd: 103_250,
      }),
    );

    expect(signal.finalResult).toBe("Ask/Sell · large ask wall removed");
    expect(signal.triggerPriceUsd).toBe(103_250);
  });

  it("can derive a display price from a safe price range", () => {
    const signal = mapInboxItemToSignal({
      ...inboxItem({ signalId: "runtime-range" }),
      priceRange: "103,150 - 103,250",
    });

    expect(signal.triggerPriceUsd).toBe(103_200);
  });

  it("maps redacted websocket signal summaries without technical fields", () => {
    const signal = mapInboxItemToSignal({
      signalId: "ws-high",
      symbol: "BTC-PERP",
      signalKind: "spoofing_candidate",
      directionBias: "short_bias",
      severity: "high",
      confidence: 0.82,
      createdAtMs: 1_700_000_000_000,
      finalResult: "Ask/Sell · large ask wall removed",
      riskScore: 88,
      dataQuality: 81,
      qualityBucket: "good",
      readOnly: true,
      runtimeModified: false,
      analysisOnly: true,
      executionEnabled: false,
    });

    expect(signal).toMatchObject({
      id: "ws-high",
      risk: "high",
      score: 88,
      dataQuality: 81,
      finalResult: "Ask/Sell · large ask wall removed",
    });
    expect(signal.markout).toBeUndefined();
    expect(signal.evidence).toBeUndefined();
  });

  it("maps TOF-lite fields from backend inbox items", () => {
    const signal = mapInboxItemToSignal({
      signalId: "tof-high",
      symbol: "BTC-PERP",
      signalKind: "spoofing_candidate",
      directionBias: "bearish",
      severity: "high",
      confidence: 0.91,
      createdAtMs: 1_700_000_000_000,
      finalResult: "Ask/Sell · large ask wall removed",
      toxicScore: 91,
      finalRiskScore: 91,
      riskScore: 84,
      dataQuality: 86,
      shortPressure: -91,
      toxicSeverity: "Critical",
      toxicType: "spoofing",
      toxicTtlSec: 120,
      toxicExpiresAt: 1_700_000_120_000,
      toxicHalfLifeSec: 45,
      toxicMaxTtlSec: 300,
      toxicDecayedScore: 91,
      toxicDecayFormula: "decayedScore = previousScore * exp(-elapsedSec / halfLifeSec)",
      toxicReasons: [
        {
          reasonType: "ToxicOrderCluster",
          score: 88,
          weight: 0.25,
          windowSec: 5,
          direction: "bearish",
          description: "clustered toxic flow",
        },
      ],
      toxicShortScore: {
        ts: 1_700_000_000_000,
        symbol: "BTC-PERP",
        toxicScore: 91,
        shortPressure: -91,
        confidence: 87,
        dataQuality: 86,
        severity: "Critical",
        toxicType: "spoofing",
        ttlSec: 120,
        expiresAt: 1_700_000_120_000,
        halfLifeSec: 45,
        maxTtlSec: 300,
        decayedScore: 91,
        decayFormula: "decayedScore = previousScore * exp(-elapsedSec / halfLifeSec)",
        reasons: [
          {
            reasonType: "ToxicOrderCluster",
            score: 88,
            weight: 0.25,
            windowSec: 5,
            direction: "bearish",
            description: "clustered toxic flow",
          },
        ],
        timeframes: ["1s", "5s", "15s", "60s"],
        formula: "toxicScore = short-term order toxicity from L2/trade TOF-lite; CWM is not fused",
        discordGate: "Short toxic Discord only, toxicScore>=85, confidence>=70, dataQuality>=70, cooldown>=60s",
      },
      mainForceScore: 83,
      mainForceConfirmed: true,
      mainForceConfirmationCount: 6,
      mainForceConfirmationTotal: 7,
      mainForceConfirmationThreshold: 3,
      structureBias: 72,
      extremeImpactConfirmed: true,
      extremeImpactScore: 92,
      regimeType: "main_force_long_build",
      marketStructureSeverity: "Major",
      marketStructureConfidence: 93,
      marketStructureDataQuality: 86,
      structureRaw: 83,
      spotContractFloor: 75,
      durationScore: 100,
      liquidationPenalty: 0,
      crowdingPenalty: 0,
      spotScore: 75,
      spotCvdScore: 84,
      spotVolumeAnomaly: 72,
      spotAbsorption: 64,
      spotLiquidityShift: 73,
      spotPriceResponse: 85,
      contractScore: 85,
      cwmAggressiveFlow: 92,
      oiImpulse: 88,
      liquidationContext: 91,
      fundingCrowding: 88,
      basisPremium: 63,
      activeExchangeConfirmation: 70,
      crossConfirmScore: 92,
      spotContractDirectionConsistency: 90,
      multiWindowConsistency: 92,
      priceResponseConsistency: 90,
      sourceCoverage: 100,
      signalAgreement: 95,
      oiScore: 88,
      liquidationScore: 93,
      fundingCrowdingScore: 88,
      cwmScore: 92,
      marketStructureReasons: [
        {
          reasonType: "SpotScore",
          score: 75,
          weight: 0.4,
          timeframe: "5m/15m",
          direction: "bullish",
          description: "spot behavior composite",
        },
      ],
      tofScore: 88.4,
      perpScore: 87,
      perpCandidateType: "OpenInterestCandidate",
      advancedScore: 89,
      advancedCandidateType: "MarketPressureHeatmapCandidate",
      cwmContribution: {
        available: true,
        source: "contract_whale_monitor",
        formula: "MarketStructureScore: spotScore and contractScore are separate composites; contractScore = 0.30*CwmAggressiveFlow + 0.20*OiImpulse + 0.15*LiquidationContext + 0.15*FundingCrowding + 0.10*BasisPremium + 0.10*ActiveExchangeConfirmation; mainForceScore uses structureRaw, min(spotScore, contractScore), durationScore, liquidationPenalty, and crowdingPenalty; not fused into toxicScore",
        contributionWeight: 0.25,
        score: 92,
        weightedContribution: 23,
        signalId: "contract-whale:BTC:15:1700000000000:buy",
        severity: "s",
        signalType: "aggressive_buy",
        direction: "buy",
        windowSec: 15,
        dataQuality: 88,
        dominance: 0.676,
        mainExchange: "binance",
        exchangeCount: 2,
        summary: "多平台主动买入爆发",
        discordGateIndependent: true,
      },
      riskSystems: {
        shortTermToxic: {
          toxicScore: 91,
          shortPressure: -91,
          toxicType: "spoofing_candidate",
          ttlSec: 120,
          confidence: 87.2,
          timeframes: ["1s", "5s", "15s", "60s"],
          formula: "toxicScore = short-term order toxicity from L2/trade TOF-lite; CWM is not fused",
          discordGate: "Short toxic Discord only, toxicScore>=85, confidence>=70, dataQuality>=70, cooldown>=60s",
        },
        mainForceStructure: {
        mainForceScore: 83,
        mainForceConfirmed: true,
        mainForceConfirmationCount: 6,
        mainForceConfirmationTotal: 7,
        mainForceConfirmationThreshold: 3,
        structureBias: 72,
          extremeImpactConfirmed: true,
          extremeImpactScore: 92,
          regimeType: "main_force_long_build",
          confidence: 93,
          dataQuality: 86,
          severity: "Major",
          structureRaw: 83,
          spotContractFloor: 75,
          durationScore: 100,
          liquidationPenalty: 0,
          crowdingPenalty: 0,
          spotScore: 75,
          spotCvdScore: 84,
          spotVolumeAnomaly: 72,
          spotAbsorption: 64,
          spotLiquidityShift: 73,
          spotPriceResponse: 85,
          contractScore: 85,
          cwmAggressiveFlow: 92,
          oiImpulse: 88,
          liquidationContext: 91,
          fundingCrowding: 88,
          basisPremium: 63,
          activeExchangeConfirmation: 70,
          crossConfirmScore: 92,
          spotContractDirectionConsistency: 90,
          multiWindowConsistency: 92,
          priceResponseConsistency: 90,
          sourceCoverage: 100,
          signalAgreement: 95,
          oiScore: 88,
          liquidationScore: 93,
          fundingCrowdingScore: 88,
          cwmScore: 92,
          reasons: [
            {
              reasonType: "SpotScore",
              score: 88,
              weight: 0.2,
              timeframe: "5m/15m",
              direction: "bullish",
              description: "spot context",
            },
          ],
          confidence: 88,
          timeframes: ["5m", "15m", "1h", "4h"],
          formula: "MarketStructureScore: spotScore = 0.30*SpotCvdScore + 0.25*SpotVolumeAnomaly + 0.20*SpotAbsorption + 0.15*SpotLiquidityShift + 0.10*SpotPriceResponse; contractScore = 0.30*CwmAggressiveFlow + 0.20*OiImpulse + 0.15*LiquidationContext + 0.15*FundingCrowding + 0.10*BasisPremium + 0.10*ActiveExchangeConfirmation; structureRaw = 0.40*spotScore + 0.40*contractScore + 0.20*crossConfirmScore; mainForceScore = 0.65*structureRaw + 0.25*min(spotScore, contractScore) + 0.10*durationScore - liquidationPenalty - crowdingPenalty; independent from toxicScore",
          cwmContribution: {
            available: true,
            source: "contract_whale_monitor",
            formula: "MarketStructureScore: spotScore and contractScore are separate composites; contractScore = 0.30*CwmAggressiveFlow + 0.20*OiImpulse + 0.15*LiquidationContext + 0.15*FundingCrowding + 0.10*BasisPremium + 0.10*ActiveExchangeConfirmation; mainForceScore uses structureRaw, min(spotScore, contractScore), durationScore, liquidationPenalty, and crowdingPenalty; not fused into toxicScore",
            contributionWeight: 0.25,
            score: 92,
            weightedContribution: 23,
            mainExchange: "binance",
            exchangeCount: 2,
            discordGateIndependent: true,
          },
          discordGateIndependent: true,
        },
      },
      finalCandidateType: "High Risk Bullish Candidate",
      metricsDirection: "bullish",
      mergedConfidence: 87,
      candidateType: "spoofing_candidate",
      explainTags: ["high_vpin_proxy", "bid_depth_withdrawal"],
      directionLabel: "看跌 / Ask-Sell",
      directionConfidence: 82.5,
      directionSource: "detector+tof_metrics",
      alertStatus: "sent",
      alertReason: "sent",
      discordAlert: {
        autoEligible: true,
        autoSent: true,
        lastDecision: "sent",
        reason: "sent",
        sentAt: "2026-06-07T12:00:00Z",
        manualSentAt: null,
      },
      replaySnapshot: {
        safeSummary: "redacted snapshot",
        rawPayload: "must not map",
        evidence: "must not map",
        markout: "must not map",
      },
      tofMetrics: {
        tradeImbalance: -0.43,
        vpinProxy: 89.0,
        bidDepthWithdrawal: 58.0,
        spreadBps: 8.4,
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
    });

    expect(signal).toMatchObject({
      id: "tof-high",
      score: 84,
      toxicScore: 91,
      finalRiskScore: 84,
      shortPressure: -91,
      toxicSeverity: "Critical",
      toxicType: "spoofing",
      toxicTtlSec: 120,
      toxicExpiresAt: 1_700_000_120_000,
      toxicHalfLifeSec: 45,
      toxicMaxTtlSec: 300,
      toxicDecayedScore: 91,
      toxicDecayFormula: "decayedScore = previousScore * exp(-elapsedSec / halfLifeSec)",
      toxicReasons: [
        {
          reasonType: "ToxicOrderCluster",
          score: 88,
          weight: 0.25,
          windowSec: 5,
          direction: "bearish",
          description: "clustered toxic flow",
        },
      ],
      toxicShortScore: {
        ts: 1_700_000_000_000,
        symbol: "BTC-PERP",
        toxicScore: 91,
        shortPressure: -91,
        confidence: 87,
        dataQuality: 86,
        severity: "Critical",
        toxicType: "spoofing",
        ttlSec: 120,
        expiresAt: 1_700_000_120_000,
        halfLifeSec: 45,
        maxTtlSec: 300,
        decayedScore: 91,
        decayFormula: "decayedScore = previousScore * exp(-elapsedSec / halfLifeSec)",
        reasons: [
          {
            reasonType: "ToxicOrderCluster",
            score: 88,
            weight: 0.25,
            windowSec: 5,
            direction: "bearish",
            description: "clustered toxic flow",
          },
        ],
        timeframes: ["1s", "5s", "15s", "60s"],
        formula: "toxicScore = short-term order toxicity from L2/trade TOF-lite; CWM is not fused",
        discordGate: "Short toxic Discord only, toxicScore>=85, confidence>=70, dataQuality>=70, cooldown>=60s",
      },
      mainForceScore: 83,
      mainForceConfirmed: true,
      mainForceConfirmationCount: 6,
      mainForceConfirmationTotal: 7,
      mainForceConfirmationThreshold: 3,
      structureBias: 72,
      extremeImpactConfirmed: true,
      extremeImpactScore: 92,
      regimeType: "main_force_long_build",
      marketStructureSeverity: "Major",
      marketStructureConfidence: 93,
      marketStructureDataQuality: 86,
      structureRaw: 83,
      spotContractFloor: 75,
      durationScore: 100,
      liquidationPenalty: 0,
      crowdingPenalty: 0,
      spotScore: 75,
      spotCvdScore: 84,
      spotVolumeAnomaly: 72,
      spotAbsorption: 64,
      spotLiquidityShift: 73,
      spotPriceResponse: 85,
      contractScore: 85,
      cwmAggressiveFlow: 92,
      oiImpulse: 88,
      liquidationContext: 91,
      fundingCrowding: 88,
      basisPremium: 63,
      activeExchangeConfirmation: 70,
      crossConfirmScore: 92,
      spotContractDirectionConsistency: 90,
      multiWindowConsistency: 92,
      priceResponseConsistency: 90,
      sourceCoverage: 100,
      signalAgreement: 95,
      oiScore: 88,
      liquidationScore: 93,
      fundingCrowdingScore: 88,
      cwmScore: 92,
      marketStructureReasons: [
        {
          reasonType: "SpotScore",
          score: 75,
          weight: 0.4,
          timeframe: "5m/15m",
          direction: "bullish",
          description: "spot behavior composite",
        },
      ],
      perpScore: 87,
      perpCandidateType: "OpenInterestCandidate",
      advancedScore: 89,
      advancedCandidateType: "MarketPressureHeatmapCandidate",
      cwmContribution: {
        available: true,
        source: "contract_whale_monitor",
        formula: "MarketStructureScore: spotScore and contractScore are separate composites; contractScore = 0.30*CwmAggressiveFlow + 0.20*OiImpulse + 0.15*LiquidationContext + 0.15*FundingCrowding + 0.10*BasisPremium + 0.10*ActiveExchangeConfirmation; mainForceScore uses structureRaw, min(spotScore, contractScore), durationScore, liquidationPenalty, and crowdingPenalty; not fused into toxicScore",
        contributionWeight: 0.25,
        score: 92,
        weightedContribution: 23,
        signalId: "contract-whale:BTC:15:1700000000000:buy",
        severity: "s",
        signalType: "aggressive_buy",
        direction: "buy",
        windowSec: 15,
        dataQuality: 88,
        dominance: 0.676,
        mainExchange: "binance",
        exchangeCount: 2,
        summary: "多平台主动买入爆发",
        discordGateIndependent: true,
      },
      riskSystems: {
        shortTermToxic: {
          toxicScore: 91,
          shortPressure: -91,
          toxicType: "spoofing_candidate",
          ttlSec: 120,
          confidence: 87.2,
          timeframes: ["1s", "5s", "15s", "60s"],
          formula: "toxicScore = short-term order toxicity from L2/trade TOF-lite; CWM is not fused",
          discordGate: "Short toxic Discord only, toxicScore>=85, confidence>=70, dataQuality>=70, cooldown>=60s",
        },
        mainForceStructure: {
        mainForceScore: 83,
        mainForceConfirmed: true,
        mainForceConfirmationCount: 6,
        mainForceConfirmationTotal: 7,
        mainForceConfirmationThreshold: 3,
        structureBias: 72,
          extremeImpactConfirmed: true,
          extremeImpactScore: 92,
          regimeType: "main_force_long_build",
          confidence: 93,
          dataQuality: 86,
          severity: "Major",
          structureRaw: 83,
          spotContractFloor: 75,
          durationScore: 100,
          liquidationPenalty: 0,
          crowdingPenalty: 0,
          spotScore: 75,
          spotCvdScore: 84,
          spotVolumeAnomaly: 72,
          spotAbsorption: 64,
          spotLiquidityShift: 73,
          spotPriceResponse: 85,
          contractScore: 85,
          cwmAggressiveFlow: 92,
          oiImpulse: 88,
          liquidationContext: 91,
          fundingCrowding: 88,
          basisPremium: 63,
          activeExchangeConfirmation: 70,
          crossConfirmScore: 92,
          spotContractDirectionConsistency: 90,
          multiWindowConsistency: 92,
          priceResponseConsistency: 90,
          sourceCoverage: 100,
          signalAgreement: 95,
          oiScore: 88,
          liquidationScore: 93,
          fundingCrowdingScore: 88,
          cwmScore: 92,
          reasons: [
            {
              reasonType: "SpotScore",
              score: 88,
              weight: 0.2,
              timeframe: "5m/15m",
              direction: "bullish",
              description: "spot context",
            },
          ],
          confidence: 88,
          timeframes: ["5m", "15m", "1h", "4h"],
          formula: "MarketStructureScore: spotScore = 0.30*SpotCvdScore + 0.25*SpotVolumeAnomaly + 0.20*SpotAbsorption + 0.15*SpotLiquidityShift + 0.10*SpotPriceResponse; contractScore = 0.30*CwmAggressiveFlow + 0.20*OiImpulse + 0.15*LiquidationContext + 0.15*FundingCrowding + 0.10*BasisPremium + 0.10*ActiveExchangeConfirmation; structureRaw = 0.40*spotScore + 0.40*contractScore + 0.20*crossConfirmScore; mainForceScore = 0.65*structureRaw + 0.25*min(spotScore, contractScore) + 0.10*durationScore - liquidationPenalty - crowdingPenalty; independent from toxicScore",
          cwmContribution: {
            available: true,
            source: "contract_whale_monitor",
            formula: "MarketStructureScore: spotScore and contractScore are separate composites; contractScore = 0.30*CwmAggressiveFlow + 0.20*OiImpulse + 0.15*LiquidationContext + 0.15*FundingCrowding + 0.10*BasisPremium + 0.10*ActiveExchangeConfirmation; mainForceScore uses structureRaw, min(spotScore, contractScore), durationScore, liquidationPenalty, and crowdingPenalty; not fused into toxicScore",
            contributionWeight: 0.25,
            score: 92,
            weightedContribution: 23,
            mainExchange: "binance",
            exchangeCount: 2,
            discordGateIndependent: true,
          },
          discordGateIndependent: true,
        },
      },
      finalCandidateType: "High Risk Bullish Candidate",
      metricsDirection: "bullish",
      mergedConfidence: 87,
      tofScore: 88.4,
      candidateType: "spoofing_candidate",
      explainTags: ["high_vpin_proxy", "bid_depth_withdrawal"],
      directionLabel: "看跌 / Ask-Sell",
      directionConfidence: 82.5,
      directionSource: "detector+tof_metrics",
      alertStatus: "sent",
      alertReason: "sent",
      discordAlert: {
        autoEligible: true,
        autoSent: true,
        lastDecision: "sent",
        reason: "sent",
        sentAt: "2026-06-07T12:00:00Z",
        manualSentAt: null,
      },
      replaySnapshot: {
        safeSummary: "redacted snapshot",
      },
      tofMetrics: {
        tradeImbalance: -0.43,
        vpinProxy: 89.0,
        bidDepthWithdrawal: 58.0,
        spreadBps: 8.4,
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
    });
    expect(signal.extremeImpactConfirmed).toBe(true);
    expect(signal.riskSystems.mainForceStructure.extremeImpactConfirmed).toBe(true);
    expect(signal.replaySnapshot).toEqual({ safeSummary: "redacted snapshot" });
  });

  it("returns an empty inbox when backend inbox is reachable but empty", async () => {
    axios.get.mockResolvedValueOnce({ data: { items: [] } });

    const signals = await fetchSignals();

    expect(signals).toEqual([]);
  });

  it("uses demo signals only when explicitly enabled", async () => {
    vi.stubEnv("VITE_USE_DEMO_SIGNALS", "true");
    axios.get.mockResolvedValueOnce({ data: { items: [] } });

    const signals = await fetchSignals();

    expect(signals.length).toBeGreaterThan(0);
    expect(signals.every((signal) => signal.isLive === false)).toBe(true);
  });
});

function inboxItem({
  signalId = "runtime-signal",
  severity = "high",
  qualityBucket = "good",
  directionBias = "short_bias",
  fusionSummary = "runtime candidate",
  triggerPriceUsd,
  riskScore,
  dataQualityScore,
} = {}) {
  return {
    signalId,
    symbol: "BTC-PERP",
    signalKind: "spoofing_candidate",
    directionBias,
    severity,
    confidence: 0.82,
    createdAtMs: 1_700_000_000_000,
    triggerPriceUsd,
    riskScore,
    dataQualityScore,
    fusion: { available: true, summary: fusionSummary },
    quality: { available: true, qualityBucket },
    recommendation: { action: "review_evidence" },
    readOnly: true,
    runtimeModified: false,
    analysisOnly: true,
    monitoringStarted: true,
    executionEnabled: false,
  };
}
