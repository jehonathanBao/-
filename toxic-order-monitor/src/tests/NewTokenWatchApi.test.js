import { afterEach, describe, expect, it, vi } from "vitest";
import axios from "axios";
import {
  addNewTokenWatch,
  fetchNewTokenChart,
  fetchNewTokenReconstruction,
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
                stableSignals: {
                  smpStable: 0.55,
                  mfeStable: -0.08,
                  lsmStable: 0.12,
                  stabilityScore: 0.72,
                  persistenceWindows: 3,
                  flipPenalty: 0.1,
                },
                regimeState: {
                  current: "liquidity_expansion",
                  confidence: 0.74,
                  stability: 0.72,
                  transitionRisk: "low",
                },
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
                costBasis: { lower: 1.92, upper: 1.97, vwapAnchor: 1.94, densityPeak: 1.93, confidence: 0.78 },
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
    expect(result.items[0].lastSignal.signalCompression.stableSignals.smpStable).toBe(0.55);
    expect(result.items[0].lastSignal.signalCompression.stableSignals.stabilityScore).toBe(0.72);
    expect(result.items[0].lastSignal.signalCompression.regimeState.current).toBe("liquidity_expansion");
    expect(result.items[0].lastSignal.signalCompression.regimeState.transitionRisk).toBe("low");
    expect(result.items[0].lastSignal.signalCompression.positionValidityGate.tradePermission).toBe(true);
    expect(result.items[0].lastSignal.signalCompression.positionValidityGate.advisoryOnly).toBe(true);
    expect(result.items[0].lastSignal.signalCompression.stabilityKernel.regime).toBe("liquidity_expansion");
    expect(result.items[0].lastSignal.signalCompression.stabilityKernel.tradeSignal.direction).toBe("long");
    expect(result.items[0].lastSignal.signalCompression.stabilityKernel.positionSmoothing.suggestedSizeMultiplier).toBe(0.54);
    expect(result.items[0].lastSignal.capitalStructure.phase).toBe("accumulation");
    expect(result.items[0].lastSignal.capitalStructure.phaseConfidence).toBe(0.78);
    expect(result.items[0].lastSignal.capitalStructure.behaviorWindows[0].windowSec).toBe(300);
    expect(result.items[0].lastSignal.capitalStructure.costBasis.vwapAnchor).toBe(1.94);
    expect(result.items[0].lastSignal.capitalStructure.costBasis.densityPeak).toBe(1.93);
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

  it("passes the selected 4h behavior window to reconstruction and chart routes", async () => {
    axios.get
      .mockResolvedValueOnce({
        data: {
          symbol: "ABCUSDT",
          timeframe: "4h",
          currentPhase: "accumulation",
          densityPeak: 1.93,
          capitalTimeline: {
            dominantPhase: "accumulation",
            totalDurationSec: 420,
            narrative: "dominant_accumulation_phase",
            phases: [
              {
                phase: "accumulation",
                label: "silent_accumulation",
                startMs: 1,
                endMs: 181000,
                durationSec: 180,
                netFlowUsd: 1200000,
                transitionReason: "low volatility absorption",
              },
            ],
          },
          positionFlowCurve: {
            latestPositionUsd: 76050,
            accumulationSlopeUsdPerMin: 320000,
            distributionSlopeUsdPerMin: 0,
            points: [{ ts: 1, positionUsd: 36000, speedUsdPerMin: 120000 }],
          },
          liquidityReactionMap: {
            impactEfficiency: 0.42,
            absorptionRatio: 0.78,
            liquidityResponse: "absorption_dominant",
            vacuumZones: [{ lower: 1.88, upper: 1.9, intensity: 0.36, reason: "thin liquidity around current price" }],
            evidence: ["impact_efficiency=0.42"],
          },
          marketDynamics: {
            stateVector: {
              smp: 0.55,
              mfe: -0.08,
              lsm: 0.12,
              regime: "liquidity_expansion",
              positionUsd: 76050,
              costBasis: 1.94,
              liquidity: 0.78,
            },
            stateVelocity: {
              flowAcceleration: 0.24,
              liquidityShiftRate: 0.36,
              regimeTransitionSpeed: 0.14,
              positionVelocityUsdPerMin: 320000,
            },
            transitionMatrix: [
              {
                from: "accumulation",
                to: "markup",
                probability: 0.62,
                reason: "flow acceleration plus stable liquidity",
              },
            ],
            marketEnergy: {
              score: 0.31,
              level: "medium",
              flowStrength: 0.55,
              liquidityAvailability: 0.78,
              regimeStability: 0.72,
            },
            trajectorySummary: "accumulation_energy_expanding",
            readOnly: true,
          },
          liquidityForce: {
            liquidationZones: [
              {
                side: "long_liquidation",
                lower: 1.83,
                upper: 1.86,
                intensity: 0.42,
                leverageDensity: 0.5,
                reason: "downside stop-loss and long liquidation proxy",
              },
              {
                side: "short_liquidation",
                lower: 2.02,
                upper: 2.06,
                intensity: 0.67,
                leverageDensity: 0.58,
                reason: "upside stop-loss and short liquidation proxy",
              },
            ],
            stopLossCascade: {
              stopHuntProbability: 0.52,
              cascadeIntensity: 0.61,
              sweepDirection: "long",
              liquiditySweep: "upside_short_sweep",
            },
            forcedFlowAttribution: {
              whalePct: 0.35,
              retailPct: 0.22,
              liquidationPct: 0.43,
              dominantDriver: "liquidation_cascade",
            },
            priceImpactDecomposition: {
              whaleImpact: 0.3,
              liquidationCascade: 0.26,
              stopLossSweep: 0.52,
              passiveAbsorption: 0.78,
            },
            primaryDriver: "liquidation_cascade",
            activeZone: "short_squeeze_zone",
            readOnly: true,
          },
          tradingDecision: {
            direction: "long",
            entry: {
              orderType: "limit",
              zoneLow: 1.91,
              zoneHigh: 1.95,
              timing: "wait",
              condition: "enter_near_cost_basis_when_smp_regime_liquidity_align",
            },
            exit: {
              zoneLow: 1.97,
              zoneHigh: 2.02,
              condition: "exit_on_distribution_transition_or_mfe_exhaustion",
              timing: "wait",
            },
            positionSize: {
              pct: 34,
              multiplier: 0.34,
              reason: "confidence_x_regime_stability_x_liquidity_x_market_energy_x_pvg",
            },
            invalidation: {
              active: false,
              priceLevel: 1.88,
              regimeCondition: "regime_flip_against_direction",
              flowCondition: "smp_reversal_against_direction",
              liquidityCondition: "liquidity_collapse_or_vacuum_expansion",
            },
            confidence: 0.68,
            advisoryOnly: true,
            readOnly: true,
          },
          readOnly: true,
        },
      })
      .mockResolvedValueOnce({
        data: {
          symbol: "ABCUSDT",
          timeframe: "4h",
          points: [],
          phaseSegments: [],
          markers: [],
          readOnly: true,
        },
      });

    const reconstruction = await fetchNewTokenReconstruction("ABCUSDT", "4h");
    const chart = await fetchNewTokenChart("ABCUSDT", "4h");

    expect(axios.get).toHaveBeenNthCalledWith(1, "/api/new-token-watch/reconstruction", {
      params: { symbol: "ABCUSDT", tf: "4h" },
    });
    expect(axios.get).toHaveBeenNthCalledWith(2, "/api/new-token-watch/chart", {
      params: { symbol: "ABCUSDT", tf: "4h" },
    });
    expect(reconstruction.timeframe).toBe("4h");
    expect(reconstruction.densityPeak).toBe(1.93);
    expect(reconstruction.capitalTimeline.phases[0].netFlowUsd).toBe(1200000);
    expect(reconstruction.capitalTimeline.phases[0].transitionReason).toBe("low volatility absorption");
    expect(reconstruction.positionFlowCurve.latestPositionUsd).toBe(76050);
    expect(reconstruction.positionFlowCurve.points[0].speedUsdPerMin).toBe(120000);
    expect(reconstruction.liquidityReactionMap.liquidityResponse).toBe("absorption_dominant");
    expect(reconstruction.liquidityReactionMap.vacuumZones[0].intensity).toBe(0.36);
    expect(reconstruction.marketDynamics.stateVector.regime).toBe("liquidity_expansion");
    expect(reconstruction.marketDynamics.stateVelocity.positionVelocityUsdPerMin).toBe(320000);
    expect(reconstruction.marketDynamics.transitionMatrix[0].to).toBe("markup");
    expect(reconstruction.marketDynamics.marketEnergy.level).toBe("medium");
    expect(reconstruction.liquidityForce.liquidationZones[0].side).toBe("long_liquidation");
    expect(reconstruction.liquidityForce.stopLossCascade.sweepDirection).toBe("long");
    expect(reconstruction.liquidityForce.forcedFlowAttribution.dominantDriver).toBe("liquidation_cascade");
    expect(reconstruction.liquidityForce.priceImpactDecomposition.stopLossSweep).toBe(0.52);
    expect(reconstruction.tradingDecision.direction).toBe("long");
    expect(reconstruction.tradingDecision.entry.orderType).toBe("limit");
    expect(reconstruction.tradingDecision.positionSize.pct).toBe(34);
    expect(reconstruction.tradingDecision.invalidation.active).toBe(false);
    expect(chart.timeframe).toBe("4h");
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
    expect(item.lastSignal.signalCompression.stableSignals.stabilityScore).toBe(0);
    expect(item.lastSignal.signalCompression.regimeState.current).toBe("neutral");
    expect(item.lastSignal.signalCompression.positionValidityGate.reason).toBe("no_signal");
    expect(item.lastSignal.signalCompression.stabilityKernel.regime).toBe("neutral");
    expect(item.lastSignal.signalCompression.stabilityKernel.tradeSignal.direction).toBe("no_trade");
    expect(item.lastSignal.capitalStructure.phase).toBe("neutral");
    expect(item.lastSignal.capitalStructure.behaviorWindows).toEqual([]);
    expect(item.lastSignal.capitalStructure.costBasis.vwapAnchor).toBe(0);
    expect(item.lastSignal.capitalStructure.costBasis.densityPeak).toBe(0);
    expect(item.lastSignal.capitalStructure.distributionRisk.level).toBe("low");
    expect(item.lastSignal.positionReconstruction.accumulationPath).toEqual([]);
    expect(item.lastSignal.positionReconstruction.lastAccumulationNode).toBeNull();
    expect(item.lastSignal.positionReconstruction.regimeLabel).toBe("neutral");
    expect(item.readOnly).toBe(true);
  });

  it("normalizes institutional reconstruction layers with empty defaults", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        symbol: "XYZUSDT",
        timeframe: "15m",
        readOnly: true,
      },
    });

    const reconstruction = await fetchNewTokenReconstruction("XYZUSDT", "15m");

    expect(reconstruction.capitalTimeline.phases).toEqual([]);
    expect(reconstruction.capitalTimeline.dominantPhase).toBe("neutral");
    expect(reconstruction.positionFlowCurve.points).toEqual([]);
    expect(reconstruction.positionFlowCurve.latestPositionUsd).toBe(0);
    expect(reconstruction.liquidityReactionMap.liquidityResponse).toBe("unknown");
    expect(reconstruction.liquidityReactionMap.vacuumZones).toEqual([]);
    expect(reconstruction.marketDynamics.stateVector.regime).toBe("neutral");
    expect(reconstruction.marketDynamics.transitionMatrix).toEqual([]);
    expect(reconstruction.marketDynamics.marketEnergy.level).toBe("low");
    expect(reconstruction.liquidityForce.liquidationZones).toEqual([]);
    expect(reconstruction.liquidityForce.activeZone).toBe("neutral_zone");
    expect(reconstruction.liquidityForce.stopLossCascade.sweepDirection).toBe("no_trade");
    expect(reconstruction.tradingDecision.direction).toBe("no_trade");
    expect(reconstruction.tradingDecision.entry.orderType).toBe("none");
    expect(reconstruction.tradingDecision.positionSize.pct).toBe(0);
    expect(reconstruction.tradingDecision.invalidation.active).toBe(true);
  });
});
