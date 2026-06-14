import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  displayThresholdForSignal,
  fetchBinanceAltContractHistory,
  fetchBinanceAltContractLatest,
  fetchBinanceAltContractSummary,
  normalizeAltContractSignal,
} from "../api/binanceAltContract.js";

vi.mock("axios", () => ({
  default: {
    get: vi.fn(),
  },
}));

describe("binance alt contract api", () => {
  beforeEach(() => {
    axios.get.mockReset();
    vi.stubEnv("VITE_API_BASE_URL", "");
  });

  it("maps latest alt contract response into frontend shape", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: {
          status: "active",
          healthStatus: "healthy",
          latestDirection: "buy",
          latestSeverity: "s",
          monitoredSymbols: ["SOLUSDT", "DOGEUSDT"],
          displayMinNotionalUsd: 500_000,
          displayThresholdsUsd: {
            ultraCore: 750_000,
            mainstream: 500_000,
            alt: 150_000,
          },
          activeAnomalyCount: 2,
          recentCriticalOrSCount: 1,
          dryRunWouldSendCount: 1,
          enabled: true,
          dryRun: true,
          readOnly: true,
          dryRunStats: {
            signals1h: 3,
            high1h: 1,
            critical1h: 1,
            s1h: 1,
            wouldSend1h: 2,
            skippedLowScore1h: 1,
            skippedCooldown1h: 0,
            skippedDataQuality1h: 0,
            liquidationDriven1h: 1,
            signals24h: 14,
            high24h: 8,
            critical24h: 4,
            s24h: 2,
            wouldSend24h: 5,
            skippedLowScore24h: 3,
            skippedCooldown24h: 2,
            skippedDataQuality24h: 1,
            liquidationDriven24h: 4,
          },
          trend60s: {
            buyVolumeBase: 120_000,
            sellVolumeBase: 30_000,
            totalVolumeBase: 150_000,
            netVolumeBase: 90_000,
            totalNotionalUsd: 12_000_000,
            buyRatio: 0.8,
            sellRatio: 0.2,
          },
          exchanges: {
            binance: {
              connected: true,
              status: "connected",
              lastTradeAt: 1_700_000_000_000,
              latencyMs: 90,
              reconnectCount: 0,
            },
          },
          smafReport: {
            dataAudit: {
              freshnessScore: 96,
              completenessScore: 88,
              consistencyScore: 94,
              integrityScore: 92.7,
              dataRiskLevel: "low",
            },
            signalAudit: {
              noiseRatio: 3,
              duplicationRate: 0,
              singleSourceDependency: 25,
              falseSignalEstimate: 5,
              integrityScore: 91.8,
            },
            behaviorAudit: {
              stateStability: 86,
              transitionEntropy: 12,
              manipulationNoise: 8,
              structuralIntegrity: 88,
            },
            predictionAudit: {
              accuracy: 84,
              flipRate: 10,
              overfittingScore: 5,
              followThroughRate: 84,
              integrityScore: 86,
            },
            smafScore: 89.6,
            riskLevel: "Stable but tuning needed",
            criticalIssues: ["single_source_dependency_high"],
          },
          smllReport: {
            enabled: true,
            protectedRealtime: true,
            status: "calibration_suggested",
            learningScore: 72,
            sampleSize: 8,
            minSamplesForUpdate: 3,
            accuracyRate: 58,
            wrongCount: 3,
            neutralCount: 1,
            outcomeRecords: [],
            errorReports: [
              {
                errorType: "prediction_error",
                severity: "high",
                rootCause: "smp_direction_or_stage_followthrough_failed",
                affectedModule: "SMP",
              },
            ],
            suggestedWeights: {
              volumeWeight: 0.95,
              oiWeight: 0.85,
              priceWeight: 1,
              liquidationWeight: 1,
              fundingWeight: 0.95,
            },
            driftReport: {
              driftDetected: true,
              affectedComponents: ["prediction_accuracy"],
              suggestedRetrain: true,
              reason: "accuracy_or_state_transition_changed",
            },
            calibrationUpdates: [
              {
                parameter: "smp.confidence_cap",
                oldValue: 100,
                newValue: 80,
                reason: "SMP accuracy 低于 60%，建议收紧预测置信度上限",
              },
            ],
          },
          atcaReport: {
            enabled: true,
            protectedRealtime: true,
            cognitionStatus: "active_cognition",
            memorySummary: "short_memory=1 symbols · smaf=90 · learning_samples=8",
            perceptionCount: 1,
            interpretationCount: 1,
            intentionCount: 1,
            predictionCount: 1,
            decisionCount: 1,
            agents: [
              {
                symbol: "SOLUSDT",
                state: "Markup",
                intent: "trend_drive",
                prediction: "Distribution",
                confidence: 82,
                risk: "high",
                decision: {
                  notify: true,
                  severity: "S",
                  reason: "trend_drive intent with 82% confidence",
                },
                marketState: {
                  symbol: "SOLUSDT",
                  priceStructure: "breakout_up",
                  volumeFlow: "aggressive_buy",
                  oiMovement: "expanding",
                  liquidationPressure: "normal",
                  marketImbalance: 74,
                },
              },
            ],
          },
          amiosReport: {
            enabled: true,
            protectedRealtime: true,
            osStatus: "running",
            marketState: "ACTIVE_CONTROL_MODE",
            kernelLoad: 78,
            signalThroughput: "normal",
            confidence: 84,
            risk: "market_risk",
            activeProcesses: [
              {
                name: "BACM",
                layer: "kernel",
                status: "interrupt",
                load: 89,
                role: "market_event_interrupts",
              },
              {
                name: "MCG",
                layer: "graph",
                status: "active",
                load: 82,
                role: "control_graph",
              },
            ],
            currentStates: [
              {
                symbol: "SOLUSDT",
                marketState: "ACTIVE_CONTROL_MODE",
                kernelLoad: 78,
                confidence: 84,
                regime: "Manipulation",
                lifecycleState: "Markup",
                prediction: "Distribution",
                control: "buy:ControlManipulation",
                risk: "market_risk",
                explanation: "控制图谱显示 ControlManipulation，控制强度 82/100。",
              },
            ],
            schedulerDecision: "monitor_high_confidence",
            auditSummary: "smaf=90 smll_samples=8 atca=active_cognition read_only=true direct_discord_gate=false",
            readOnly: true,
            directDiscordGate: false,
          },
        },
        items: [altSignal(), lowNotionalSignal()],
      },
    });

    const payload = await fetchBinanceAltContractLatest(25, "SOL");

    expect(axios.get).toHaveBeenCalledWith("/api/binance-alt-contract/latest?limit=25&symbol=SOL");
    expect(payload.summary).toMatchObject({
      status: "active",
      healthStatus: "healthy",
      latestDirection: "buy",
      latestSeverity: "s",
      monitoredSymbols: ["SOLUSDT", "DOGEUSDT"],
      displayMinNotionalUsd: 500_000,
      displayThresholdsUsd: {
        ultraCore: 750_000,
        mainstream: 500_000,
        alt: 150_000,
      },
      activeAnomalyCount: 2,
      recentCriticalOrSCount: 1,
      dryRunWouldSendCount: 1,
      enabled: true,
      dryRun: true,
      readOnly: true,
      dryRunStats: {
        signals1h: 3,
        wouldSend1h: 2,
        liquidationDriven1h: 1,
        signals24h: 14,
        critical24h: 4,
        s24h: 2,
        wouldSend24h: 5,
        liquidationDriven24h: 4,
      },
      trend60s: {
        totalNotionalUsd: 12_000_000,
        buyRatio: 0.8,
        sellRatio: 0.2,
      },
      exchanges: {
        binance: {
          connected: true,
          status: "connected",
          latencyMs: 90,
        },
      },
      smafReport: {
        smafScore: 89.6,
        riskLevel: "Stable but tuning needed",
        criticalIssues: ["single_source_dependency_high"],
        dataAudit: {
          integrityScore: 92.7,
          dataRiskLevel: "low",
        },
        predictionAudit: {
          flipRate: 10,
          integrityScore: 86,
        },
      },
      smllReport: {
        status: "calibration_suggested",
        learningScore: 72,
        sampleSize: 8,
        accuracyRate: 58,
        wrongCount: 3,
        protectedRealtime: true,
        suggestedWeights: {
          oiWeight: 0.85,
          volumeWeight: 0.95,
        },
        driftReport: {
          driftDetected: true,
          suggestedRetrain: true,
        },
        calibrationUpdates: [
          {
            parameter: "smp.confidence_cap",
          },
        ],
      },
      atcaReport: {
        cognitionStatus: "active_cognition",
        memorySummary: "short_memory=1 symbols · smaf=90 · learning_samples=8",
        perceptionCount: 1,
        decisionCount: 1,
        protectedRealtime: true,
        agents: [
          {
            symbol: "SOLUSDT",
            intent: "trend_drive",
            decision: {
              notify: true,
              severity: "S",
            },
            marketState: {
              priceStructure: "breakout_up",
            },
          },
        ],
      },
      amiosReport: {
        osStatus: "running",
        marketState: "ACTIVE_CONTROL_MODE",
        kernelLoad: 78,
        signalThroughput: "normal",
        confidence: 84,
        risk: "market_risk",
        protectedRealtime: true,
        readOnly: true,
        directDiscordGate: false,
        schedulerDecision: "monitor_high_confidence",
        activeProcesses: [
          {
            name: "BACM",
            layer: "kernel",
            status: "interrupt",
            load: 89,
            role: "market_event_interrupts",
          },
          {
            name: "MCG",
            layer: "graph",
            status: "active",
            load: 82,
            role: "control_graph",
          },
        ],
        currentStates: [
          {
            symbol: "SOLUSDT",
            marketState: "ACTIVE_CONTROL_MODE",
            confidence: 84,
            control: "buy:ControlManipulation",
          },
        ],
      },
    });
    expect(payload.items).toHaveLength(1);
    expect(payload.items[0]).toMatchObject({
      id: "bacm-sol-s",
      symbol: "SOL",
      productId: "SOLUSDT",
      marketTier: "ultra_core",
      displayThresholdUsd: 750_000,
      signalType: "main_force_long_build",
      severity: "s",
      abnormalScore: 91,
      buildScore: 87,
      masterCapitalStrength: {
        mcss: 88,
        tier: "Ultra Core",
        liquidityWeight: 0.6,
        interpretation: "疑似机构级别流入",
      },
      liquidityMicrostructure: {
        lmsScore: 82,
        behavior: "LiquiditySweepUp",
        marketControl: "buyer_side_control",
        directDiscordGate: false,
      },
      marketControlGraph: {
        dominantSide: "buy",
        controlStrength: 82,
        controlType: "ControlManipulation",
        directDiscordGate: false,
      },
      marketRegime: {
        regime: "Manipulation",
        subType: "Manipulation_UP",
        confidence: 82,
        oiTrend: "down",
        priceTrend: "spike_up",
        efficiencyRatio: 0.12,
        explanationTags: ["stop_hunt", "fake_breakout"],
      },
      smartMoneyLifecycle: {
        lifecycleState: "Markup",
        stateConfidence: 78,
        stateDurationMin: 42,
        transitionSignal: "Accumulation->Markup",
        flowConsistencyScore: 81,
        lifecycleScore: 84,
        statePath: ["Accumulation", "Markup"],
        explanationTags: ["oi_expansion", "flow_consistent", "mcss_confirmed"],
        currentExplanation: "当前接近拉升阶段。",
      },
      smartMoneyPrediction: {
        currentState: "Markup",
        nextState: "Distribution",
        probability: 76,
        timeHorizonMin: 45,
        directionBias: "BearishRisk",
        directionProbability: 0.62,
        confidence: 81,
        predictionScore: 79,
        triggerFactors: ["oi_momentum_divergence", "efficiency_decay"],
        explanation: "拉升后默认观察是否进入派发阶段。",
      },
      signalConfidence: {
        confidenceScore: 87,
        confidenceLevel: "high",
        directDiscordGate: false,
        reliabilityFactors: ["bacm_signal_strong", "mcss_strong_money", "lme_orderbook_support"],
        riskFactors: ["prediction_misaligned"],
        breakdown: {
          bacmSignalStrength: 93,
          mcssStrength: 88,
          smleStability: 81,
          smpPredictionAlignment: 68,
          lmeMicrostructureSupport: 82,
          mcgControlCoherence: 82,
          smafRiskPenalty: 10,
        },
      },
      triggerPriceUsd: 175.5,
      discordWouldSend: true,
      discordSent: false,
      activeSources: [
        {
          exchange: "binance",
          marketType: "perp",
          role: "primary",
          status: "active",
        },
      ],
    });
    expect(payload.items[0].smartMoneyLifecycle).toMatchObject({
      lifecycleState: "Markup",
      stateConfidence: 78,
      transitionSignal: "Accumulation->Markup",
      statePath: ["Accumulation", "Markup"],
    });
    expect(payload.items[0].smartMoneyPrediction).toMatchObject({
      currentState: "Markup",
      nextState: "Distribution",
      probability: 76,
      directionBias: "BearishRisk",
      directionProbability: 0.62,
    });
    expect(payload.items[0].signalConfidence).toMatchObject({
      confidenceScore: 87,
      confidenceLevel: "high",
      directDiscordGate: false,
      breakdown: {
        bacmSignalStrength: 93,
        mcssStrength: 88,
        smleStability: 81,
        smpPredictionAlignment: 68,
        lmeMicrostructureSupport: 82,
        mcgControlCoherence: 82,
        smafRiskPenalty: 10,
      },
    });
  });

  it("passes history filters including liquidationDriven and build threshold", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: {},
        items: [],
      },
    });

    const payload = await fetchBinanceAltContractHistory({
      symbol: "SOL",
      severity: "critical",
      signal_type: "main_force_long_build",
      direction: "buy",
      would_send: true,
      liquidationDriven: false,
      tier: "b",
      min_build_score: 85,
      limit: 25,
    });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/binance-alt-contract/history?symbol=SOL&severity=critical&signal_type=main_force_long_build&direction=buy&would_send=true&liquidationDriven=false&tier=b&min_build_score=85&limit=25",
    );
    expect(payload.items).toEqual([]);
    expect(payload.error).toBeNull();
  });

  it("computes trigger price from notional and volume and drops sensitive fields", () => {
    const signal = normalizeAltContractSignal({
      id: "fallback-price",
      symbol: "DOGE",
      totalVolumeBase: 10_000,
      totalNotionalUsd: 1_500,
      rawPayload: "must not render",
      webhook: "must not render",
      token: "must not render",
    });

    expect(signal.triggerPriceUsd).toBe(0.15);
    expect(signal.rawPayload).toBeUndefined();
    expect(signal.webhook).toBeUndefined();
    expect(signal.token).toBeUndefined();
  });

  it("filters new display signals by AIS and keeps legacy threshold fallback", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: {
          displayThresholdsUsd: {
            ultraCore: 750_000,
            mainstream: 500_000,
            alt: 150_000,
          },
        },
        items: [
          { ...altSignal(), id: "wif-relative", symbol: "WIF", productId: "WIFUSDT", marketTier: "alt", displayThresholdUsd: 0, totalNotionalUsd: 200_000, altImpactScore: impactScore(82) },
          { ...altSignal(), id: "btc-absolute-noise", symbol: "BTC", productId: "BTCUSDT", marketTier: "ultra_core", displayThresholdUsd: 0, totalNotionalUsd: 2_000_000, altImpactScore: impactScore(41) },
          { ...altSignal(), id: "legacy-eth-large", symbol: "ETH", productId: "ETHUSDT", marketTier: "ultra_core", displayThresholdUsd: 0, totalNotionalUsd: 800_000, altImpactScore: undefined },
          { ...altSignal(), id: "legacy-xrp-small", symbol: "XRP", productId: "XRPUSDT", marketTier: "mainstream", displayThresholdUsd: 0, totalNotionalUsd: 400_000, altImpactScore: undefined },
        ],
      },
    });

    const payload = await fetchBinanceAltContractLatest(50);
    const ids = payload.items.map((item) => item.id);

    expect(ids).toEqual(["wif-relative", "legacy-eth-large"]);
    expect(payload.items[0].altImpactScore.finalScore).toBe(82);
    expect(displayThresholdForSignal(payload.items[1], payload.summary)).toBe(750_000);
  });

  it("falls back to disabled read-only summary on request failure", async () => {
    axios.get.mockRejectedValueOnce(new Error("network"));

    const payload = await fetchBinanceAltContractSummary();

    expect(payload.error).toBe("summary_unavailable");
    expect(payload.summary).toMatchObject({
      enabled: false,
      dryRun: true,
      readOnly: true,
      healthStatus: "disabled",
    });
  });
});

function altSignal() {
  return {
    id: "bacm-sol-s",
    ts: 1_700_000_000_000,
    symbol: "SOL",
    productId: "SOLUSDT",
    tier: "b",
    windowSec: 60,
    signalType: "main_force_long_build",
    direction: "buy",
    severity: "s",
    abnormalScore: 91,
    buildScore: 87,
    directionBias: 76,
    dataQuality: 92,
    totalVolumeBase: 820_000,
    netVolumeBase: 610_000,
    totalNotionalUsd: 143_910_000,
    marketTier: "ultra_core",
    displayThresholdUsd: 750_000,
    masterCapitalStrength: {
      mcss: 88,
      tier: "Ultra Core",
      liquidityWeight: 0.6,
      notionalScore: 22,
      directionScore: 25,
      oiScore: 25,
      priceScore: 20,
      anomalyScore: 20,
      liquidationPenalty: 0,
      interpretation: "疑似机构级别流入",
    },
    altImpactScore: impactScore(94),
    liquidityMicrostructure: {
      lmsScore: 82,
      behavior: "LiquiditySweepUp",
      marketControl: "buyer_side_control",
      liquidityPressure: "HIGH",
      imbalance: 0.62,
      spreadState: "widening",
      spoofingState: "none",
      orderFlowPressure: 91,
      absorptionStrength: 22,
      imbalanceScore: 62,
      spreadBehavior: 64,
      spoofingPenalty: 0,
      explanationTags: ["read_only_microstructure", "liquidity_sweep", "aggressive_buy_pressure"],
      interpretation: "盘口微观结构显示强主力控盘迹象",
      readOnly: true,
      directDiscordGate: false,
    },
    marketControlGraph: {
      symbol: "SOL",
      controlNodes: [
        {
          id: "SOLUSDT:symbol",
          nodeType: "Symbol",
          label: "SOLUSDT",
          side: "buy",
          strength: 82,
          price: 175.5,
        },
        {
          id: "SOLUSDT:price-zone",
          nodeType: "PriceLevel",
          label: "control zone 175.500000",
          side: "buy",
          strength: 80,
          price: 175.5,
        },
      ],
      controlEdges: [
        {
          from: "SOLUSDT:symbol",
          to: "SOLUSDT:price-zone",
          relation: "manipulation_relation",
          strength: 82,
          evidence: ["LiquiditySweepUp"],
        },
      ],
      dominantSide: "buy",
      controlStrength: 82,
      controlType: "ControlManipulation",
      controlPath: ["Liquidity shaping", "Cognitive trap", "Sweep or revert risk"],
      interpretation: "操控/诱导 · 强控盘 · MCSS 88/100 · LMS 82/100",
      readOnly: true,
      directDiscordGate: false,
    },
    marketRegime: {
      regime: "Manipulation",
      subType: "Manipulation_UP",
      confidence: 82,
      mcScore: 88,
      oiTrend: "down",
      priceTrend: "spike_up",
      trend5m: "spike_up",
      trend15m: "spike_up",
      trend1h: "unknown",
      efficiencyRatio: 0.12,
      oiLagIndex: 1.4,
      explanationTags: ["stop_hunt", "fake_breakout"],
    },
    smartMoneyLifecycle: {
      lifecycleState: "Markup",
      stateConfidence: 78,
      stateDurationMin: 42,
      transitionSignal: "Accumulation->Markup",
      flowConsistencyScore: 81,
      lifecycleScore: 84,
      statePath: ["Accumulation", "Markup"],
      explanationTags: ["oi_expansion", "flow_consistent", "mcss_confirmed"],
      currentExplanation: "当前接近拉升阶段。",
    },
    smartMoneyPrediction: {
      currentState: "Markup",
      nextState: "Distribution",
      probability: 76,
      timeHorizonMin: 45,
      directionBias: "BearishRisk",
      directionProbability: 0.62,
      confidence: 81,
      predictionScore: 79,
      triggerFactors: ["oi_momentum_divergence", "efficiency_decay"],
      explanation: "拉升后默认观察是否进入派发阶段。",
    },
    signalConfidence: {
      symbol: "SOL",
      signalType: "main_force_long_build",
      confidenceScore: 87,
      confidenceLevel: "high",
      reliabilityFactors: ["bacm_signal_strong", "mcss_strong_money", "lme_orderbook_support"],
      riskFactors: ["prediction_misaligned"],
      breakdown: {
        bacmSignalStrength: 93,
        mcssStrength: 88,
        smleStability: 81,
        smpPredictionAlignment: 68,
        lmeMicrostructureSupport: 82,
        mcgControlCoherence: 82,
        smafRiskPenalty: 10,
      },
      interpretation: "多层确认较强，属于高可信主力行为候选；SCC 不直接触发 Discord。",
      readOnly: true,
      directDiscordGate: false,
    },
    triggerPriceUsd: 175.5,
    dominance: 0.74,
    priceMovePct: 2.4,
    dynamicMultiple: 10.2,
    oiChange1mBase: 210_000,
    oiChangePct: 1.8,
    fundingRate: 0.00021,
    liquidationSuspected: false,
    forceOrderSnapshot: true,
    activeSources: [
      {
        exchange: "binance",
        marketType: "perp",
        role: "primary",
        status: "active",
      },
    ],
    discordEligible: true,
    discordWouldSend: true,
    discordSent: false,
    discordReason: "dry_run",
    finalResult: "山寨合约主动买入爆发，OI 同步上升，疑似主力建多。",
  };
}

function impactScore(finalScore) {
  return {
    marketImpactRatio: finalScore >= 70 ? 0.03 : 0.0008,
    marketImpactScore: finalScore >= 70 ? 40 : 4,
    liquidityImpact: finalScore >= 70 ? 24 : 6,
    capImpact: 0,
    directionalStrength: 0.74,
    directionalScore: finalScore >= 70 ? 20 : 10,
    oiConfirmation: finalScore >= 70 ? 10 : 0,
    finalScore,
    displayThreshold: 70,
    discordThreshold: 85,
    sThreshold: 90,
    referenceVolume24hUsd: 4_797_000_000,
    referenceSource: "ticker_quote_volume_24h",
    interpretation: finalScore >= 70 ? "有效相对冲击，适合前端展示" : "相对市场冲击偏弱",
  };
}

function lowNotionalSignal() {
  return {
    ...altSignal(),
    id: "bacm-doge-small",
    symbol: "DOGE",
    productId: "DOGEUSDT",
    marketTier: "mainstream",
    displayThresholdUsd: 500_000,
    totalVolumeBase: 1_000,
    totalNotionalUsd: 499_999,
    triggerPriceUsd: 0.2,
    altImpactScore: impactScore(42),
  };
}
