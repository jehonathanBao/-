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
    expect(item.readOnly).toBe(true);
  });
});
