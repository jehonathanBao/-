import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { evaluateDiscordAlertGate } from "../api/alertGate.js";
import { fetchSignals, mapInboxItemToSignal } from "../api/signals.js";

vi.mock("axios", () => ({
  default: {
    get: vi.fn(),
  },
}));

describe("signals api mapping", () => {
  beforeEach(() => {
    axios.get.mockReset();
    vi.stubEnv("VITE_API_BASE_URL", "");
  });

  it("maps backend inbox items into persistent inbox signal shape", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [
          inboxItem({ signalId: "runtime-high", severity: "high", qualityBucket: "good" }),
          inboxItem({ signalId: "runtime-medium", severity: "medium", qualityBucket: "mixed" }),
        ],
      },
    });

    const signals = await fetchSignals();

    expect(axios.get).toHaveBeenCalledWith("/api/toxicity/signal-inbox/recent");
    expect(signals.map((signal) => signal.id)).toEqual(["runtime-high", "runtime-medium"]);
    expect(signals[0]).toMatchObject({
      risk: "high",
      level: "A",
      score: 85,
      dataQuality: 82,
      status: "unhandled",
    });
    expect(signals[1]).toMatchObject({
      risk: "medium",
      level: "B",
      score: 72,
      dataQuality: 74,
    });
    expect(evaluateDiscordAlertGate(signals[0])).toEqual({ ok: true, reason: null });
    expect(evaluateDiscordAlertGate(signals[1])).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_NON_HIGH_RISK",
    });
  });

  it("keeps final result to direction plus core reason", () => {
    const signal = mapInboxItemToSignal(
      inboxItem({
        signalId: "runtime-short",
        directionBias: "short_bias",
        fusionSummary: "large ask wall removed",
      }),
    );

    expect(signal.finalResult).toBe("Ask/Sell · large ask wall removed");
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
      finalRiskScore: 91,
      riskScore: 84,
      dataQuality: 86,
      tofScore: 88.4,
      perpScore: 87,
      perpCandidateType: "OpenInterestCandidate",
      advancedScore: 89,
      advancedCandidateType: "MarketPressureHeatmapCandidate",
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
      score: 91,
      finalRiskScore: 91,
      perpScore: 87,
      perpCandidateType: "OpenInterestCandidate",
      advancedScore: 89,
      advancedCandidateType: "MarketPressureHeatmapCandidate",
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
    expect(signal.replaySnapshot).toEqual({ safeSummary: "redacted snapshot" });
  });

  it("uses demo signals when backend inbox is reachable but empty", async () => {
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
} = {}) {
  return {
    signalId,
    symbol: "BTC-PERP",
    signalKind: "spoofing_candidate",
    directionBias,
    severity,
    confidence: 0.82,
    createdAtMs: 1_700_000_000_000,
    fusion: { available: true, summary: fusionSummary },
    quality: { available: true, qualityBucket },
    recommendation: { action: "review_evidence" },
    readOnly: true,
    runtimeModified: false,
    analysisOnly: true,
    executionEnabled: false,
  };
}
