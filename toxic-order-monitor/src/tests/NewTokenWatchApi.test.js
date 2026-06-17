import { afterEach, describe, expect, it, vi } from "vitest";
import axios from "axios";
import {
  addNewTokenWatch,
  fetchNewTokenWatchList,
  normalizeNewTokenWatchItem,
  removeNewTokenWatch,
} from "../api/newTokenWatch.js";

vi.mock("axios");

describe("newTokenWatch API", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("loads active token watch list", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [
          {
            symbol: "ABCUSDT",
            streamStatus: "read_only_probe",
            lastSignal: {
              regime: "accumulation",
              strength: 0.72,
              confidence: 0.68,
              flowPersistence: 0.81,
              ofiWindows: [{ windowSec: 30, normalizedOfi: 0.62, persistence: 0.8 }],
              impactResponse: { classification: "absorption", absorptionScore: 0.74 },
              liquidityDepletion: { bidDepletionRate: 0.12, depletionPressure: 0.08 },
              actorDecomposition: {
                dominantActor: "smart_money",
                smartMoneyProbability: 0.64,
                liquidityProviderProbability: 0.24,
                momentumChaserProbability: 0.12,
                confidence: 0.75,
              },
              signalCompression: {
                smartMoneyPressure: 0.62,
                momentumFlowExhaustion: -0.12,
                liquidityStressManipulation: 0.18,
                positionValidityGate: {
                  riskScore: 0.22,
                  tradePermission: true,
                  positionSizeMultiplier: 0.78,
                  reason: "advisory_allowed",
                  advisoryOnly: true,
                },
                stabilityKernel: {
                  regime: "liquidity_expansion",
                  regimeQuality: 0.69,
                  tradeSignal: {
                    direction: "long",
                    confidence: 0.71,
                    expectedHoldTime: "mid",
                    invalidationCondition: "smp_turns_negative_or_lsm_above_0_65",
                    reason: "smart_money_pressure_supports_long_bias",
                    advisoryOnly: true,
                  },
                  positionSmoothing: {
                    suggestedSizeMultiplier: 0.54,
                    volatilityAdjustment: 0.92,
                    drawdownAdjustment: 1,
                    reason: "confidence_x_regime_quality_x_volatility_x_pvg",
                  },
                  readOnly: true,
                },
                explanationTags: ["pvg_advisory_allowed"],
                readOnly: true,
              },
              capitalStructure: {
                phase: "accumulation",
                phaseLabel: "accumulation",
                phaseConfidence: 0.78,
                behaviorWindows: [
                  {
                    windowSec: 300,
                    cumulativeDelta: 120,
                    normalizedOfi: 0.62,
                    vwap: 1.94,
                    volume: 250,
                    priceDriftPct: 0.002,
                    volatilityPct: 0.01,
                    absorptionScore: 0.74,
                    bidReplenishmentScore: 0.69,
                  },
                ],
                costBasis: { lower: 1.92, upper: 1.97, vwapAnchor: 1.94, confidence: 0.78 },
                estimatedPosition: { lowerUsd: 3200000, upperUsd: 5800000, confidence: 0.73 },
                horizon: { minMinutes: 18, maxMinutes: 42, detectedMinutes: 24 },
                distributionRisk: {
                  score: 0.27,
                  level: "low",
                  reasons: ["no_distribution_pressure_confirmed"],
                },
                evidence: ["phase=accumulation"],
                readOnly: true,
              },
              positionReconstruction: {
                accumulationPath: [
                  {
                    phase: "accumulation",
                    label: "silent_accumulation",
                    startPrice: 1.82,
                    endPrice: 1.86,
                    volume: 120,
                    cumulativeDelta: 86,
                    impact: 0.0001,
                    durationSec: 180,
                    confidence: 0.71,
                    characteristics: ["minimal_impact_flow"],
                  },
                ],
                lastAccumulationNode: {
                  lower: 1.88,
                  upper: 1.9,
                  durationSec: 402,
                  volatilityPct: 0.004,
                  absorptionEfficiency: 0.84,
                  confidence: 0.84,
                  characteristics: ["volume_without_breakout"],
                },
                distributionPath: [],
                latentPosition: [
                  {
                    timestamp: 1,
                    price: 1.86,
                    estimatedPosition: 20,
                    impactAdjustedPosition: 18,
                  },
                ],
                confidence: 0.82,
                regimeLabel: "accumulation_trajectory",
                evidence: ["last_accumulation_node_detected"],
                readOnly: true,
              },
            },
          },
        ],
        maxActiveTokens: 10,
        activeCount: 1,
        readOnly: true,
      },
    });

    const result = await fetchNewTokenWatchList();

    expect(axios.get).toHaveBeenCalledWith("/api/new-token-watch/list");
    expect(result.items[0].symbol).toBe("ABCUSDT");
    expect(result.items[0].lastSignal.regime).toBe("accumulation");
    expect(result.items[0].lastSignal.flowPersistence).toBe(0.81);
    expect(result.items[0].lastSignal.ofiWindows[0].windowSec).toBe(30);
    expect(result.items[0].lastSignal.impactResponse.classification).toBe("absorption");
    expect(result.items[0].lastSignal.actorDecomposition.dominantActor).toBe("smart_money");
    expect(result.items[0].lastSignal.actorDecomposition.smartMoneyProbability).toBe(0.64);
    expect(result.items[0].lastSignal.signalCompression.smartMoneyPressure).toBe(0.62);
    expect(result.items[0].lastSignal.signalCompression.positionValidityGate.tradePermission).toBe(true);
    expect(result.items[0].lastSignal.signalCompression.positionValidityGate.advisoryOnly).toBe(true);
    expect(result.items[0].lastSignal.signalCompression.stabilityKernel.regime).toBe("liquidity_expansion");
    expect(result.items[0].lastSignal.signalCompression.stabilityKernel.tradeSignal.direction).toBe("long");
    expect(result.items[0].lastSignal.signalCompression.stabilityKernel.positionSmoothing.suggestedSizeMultiplier).toBe(0.54);
    expect(result.items[0].lastSignal.capitalStructure.phase).toBe("accumulation");
    expect(result.items[0].lastSignal.capitalStructure.phaseConfidence).toBe(0.78);
    expect(result.items[0].lastSignal.capitalStructure.behaviorWindows[0].windowSec).toBe(300);
    expect(result.items[0].lastSignal.capitalStructure.costBasis.vwapAnchor).toBe(1.94);
    expect(result.items[0].lastSignal.capitalStructure.estimatedPosition.upperUsd).toBe(5800000);
    expect(result.items[0].lastSignal.capitalStructure.distributionRisk.level).toBe("low");
    expect(result.items[0].lastSignal.positionReconstruction.regimeLabel).toBe("accumulation_trajectory");
    expect(result.items[0].lastSignal.positionReconstruction.accumulationPath[0].label).toBe("silent_accumulation");
    expect(result.items[0].lastSignal.positionReconstruction.lastAccumulationNode.confidence).toBe(0.84);
    expect(result.items[0].lastSignal.positionReconstruction.latentPosition[0].impactAdjustedPosition).toBe(18);
    expect(result.maxActiveTokens).toBe(10);
  });

  it("adds and removes a symbol through backend routes", async () => {
    axios.post.mockResolvedValueOnce({ data: { ok: true, items: [], item: { symbol: "ABCUSDT" } } });
    await addNewTokenWatch("abc");
    expect(axios.post).toHaveBeenCalledWith("/api/new-token-watch/add", { symbol: "abc" });

    axios.post.mockResolvedValueOnce({ data: { ok: true, items: [], item: { symbol: "ABCUSDT" } } });
    await removeNewTokenWatch("ABCUSDT");
    expect(axios.post).toHaveBeenCalledWith("/api/new-token-watch/remove", { symbol: "ABCUSDT" });
  });

  it("normalizes missing signal fields safely", () => {
    const item = normalizeNewTokenWatchItem({ symbol: "xyzusdt" });

    expect(item.symbol).toBe("XYZUSDT");
    expect(item.lastSignal.regime).toBe("neutral");
    expect(item.lastSignal.strength).toBe(0);
    expect(item.lastSignal.ofiWindows).toEqual([]);
    expect(item.lastSignal.impactResponse.classification).toBe("unknown");
    expect(item.lastSignal.actorDecomposition.dominantActor).toBe("unknown");
    expect(item.lastSignal.signalCompression.smartMoneyPressure).toBe(0);
    expect(item.lastSignal.signalCompression.positionValidityGate.reason).toBe("no_signal");
    expect(item.lastSignal.signalCompression.stabilityKernel.regime).toBe("neutral");
    expect(item.lastSignal.signalCompression.stabilityKernel.tradeSignal.direction).toBe("no_trade");
    expect(item.lastSignal.capitalStructure.phase).toBe("neutral");
    expect(item.lastSignal.capitalStructure.behaviorWindows).toEqual([]);
    expect(item.lastSignal.capitalStructure.costBasis.vwapAnchor).toBe(0);
    expect(item.lastSignal.capitalStructure.distributionRisk.level).toBe("low");
    expect(item.lastSignal.positionReconstruction.accumulationPath).toEqual([]);
    expect(item.lastSignal.positionReconstruction.lastAccumulationNode).toBeNull();
    expect(item.lastSignal.positionReconstruction.regimeLabel).toBe("neutral");
    expect(item.readOnly).toBe(true);
  });
});
