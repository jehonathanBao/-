import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import BinanceAltContractMonitor from "../components/BinanceAltContractMonitor.jsx";
import {
  fetchBinanceAltContractHistory,
  fetchBinanceAltContractLatest,
  fetchBinanceAltContractSummary,
} from "../api/binanceAltContract.js";

vi.mock("../api/binanceAltContract.js", () => ({
  fetchBinanceAltContractSummary: vi.fn(() =>
    Promise.resolve({
      summary: altSummary(),
      error: null,
    }),
  ),
  fetchBinanceAltContractLatest: vi.fn(() =>
    Promise.resolve({
      summary: altSummary(),
      items: [altSignal(), lowNotionalSignal()],
      error: null,
    }),
  ),
  fetchBinanceAltContractHistory: vi.fn(() =>
    Promise.resolve({
      summary: altSummary(),
      items: [],
      error: null,
    }),
  ),
  displayThresholdForSignal: (signal, summary) => {
    const explicit = Number(signal?.displayThresholdUsd || 0);
    if (Number.isFinite(explicit) && explicit > 0) return explicit;
    const thresholds = summary?.displayThresholdsUsd || {};
    if (signal?.marketTier === "ultra_core") return Number(thresholds.ultraCore || 750_000);
    if (signal?.marketTier === "mainstream") return Number(thresholds.mainstream || 500_000);
    return Number(thresholds.alt || 150_000);
  },
  shouldDisplayByAltImpact: (signal) =>
    Number(signal?.altImpactScore?.finalScore || 0) >=
    Number(signal?.altImpactScore?.displayThreshold || 70),
}));

describe("BinanceAltContractMonitor", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it("renders summary, prices, dry-run state and latest alt contract signals", async () => {
    render(<BinanceAltContractMonitor />);

    expect(await screen.findByText("山寨合约异常监控")).toBeInTheDocument();
    expect(screen.getByText(/全量监控 Binance USDT 永续山寨合约/)).toBeInTheDocument();
    expect(screen.getAllByText(/全 Binance USDT 永续/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Tier A1 \/ B1 \/ C0 \/ D0 \/ E0/).length).toBeGreaterThan(0);
    expect(screen.getByText(/监控币种 SOLUSDT, DOGEUSDT/)).toBeInTheDocument();
    expect(screen.getByText(/相对冲击展示：AIS ≥ 70/)).toBeInTheDocument();
    expect(screen.getByText("在线")).toBeInTheDocument();
    expect(screen.getByText("Ultra Core")).toBeInTheDocument();
    expect(screen.getAllByText("累积压力观察").length).toBeGreaterThan(0);
    expect(screen.getByText("$175.50")).toBeInTheDocument();
    expect(screen.getAllByText("88/100").length).toBeGreaterThan(0);
    expect(screen.getAllByText("疑似机构级别流入").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Manipulation").length).toBeGreaterThan(0);
    expect(screen.getAllByText("(82%)").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Markup").length).toBeGreaterThan(0);
    expect(screen.getAllByText("(78%)").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Distribution").length).toBeGreaterThan(0);
    expect(screen.getAllByText("(76%)").length).toBeGreaterThan(0);
    expect(screen.getByText("91/100")).toBeInTheDocument();
    expect(screen.getByText("87/100")).toBeInTheDocument();
    expect(screen.getAllByText("94/100").length).toBeGreaterThan(0);
    expect(screen.getAllByText("3.00%").length).toBeGreaterThan(0);
    expect(screen.getByText("$143.9M")).toBeInTheDocument();
    expect(screen.getByText("10.2x")).toBeInTheDocument();
    expect(screen.getByText("+210,000 SOL")).toBeInTheDocument();
    expect(screen.getByText("+2.400%")).toBeInTheDocument();
    expect(screen.getByText("dry-run would_send")).toBeInTheDocument();
    expect(screen.getByText("Dry-run 24h")).toBeInTheDocument();
    expect(screen.getByText(/signals 14/)).toBeInTheDocument();
    expect(screen.getAllByText(/Candidate/).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Hot OI/).length).toBeGreaterThan(0);
    expect(screen.getByText(/markPrice/)).toBeInTheDocument();
    expect(screen.getByText(/ticker/)).toBeInTheDocument();
    expect(screen.getByText("SMAF System Audit")).toBeInTheDocument();
    expect(screen.getByText(/系统健康 90\/100/)).toBeInTheDocument();
    expect(screen.getByText("Stable but tuning")).toBeInTheDocument();
    expect(screen.getByText("单源依赖过高")).toBeInTheDocument();
    expect(screen.getByText("SMLL Self-Learning Loop")).toBeInTheDocument();
    expect(screen.getByText(/自学习 72\/100/)).toBeInTheDocument();
    expect(screen.getByText(/建议校准/)).toBeInTheDocument();
    expect(screen.getByText("只读建议")).toBeInTheDocument();
    expect(screen.getByText(/预测未跟随 · SMP/)).toBeInTheDocument();
    expect(screen.getByText(/smp.confidence_cap: 100.00 → 80.00/)).toBeInTheDocument();
    expect(screen.getByText("ATCA Cognition Agent")).toBeInTheDocument();
    expect(screen.getByText(/认知状态 主动认知/)).toBeInTheDocument();
    expect(screen.getByText("只读认知")).toBeInTheDocument();
    expect(screen.getAllByText("SOLUSDT").length).toBeGreaterThan(0);
    expect(screen.getByText(/Markup · 趋势推动 → Distribution/)).toBeInTheDocument();
    expect(screen.getByText(/trend_drive intent with 82% confidence/)).toBeInTheDocument();
    expect(screen.getByText("AMIOS Market OS")).toBeInTheDocument();
    expect(screen.getByText(/主动控盘模式 · Kernel Load 78\/100/)).toBeInTheDocument();
    expect(screen.getByText("只读 OS")).toBeInTheDocument();
    expect(screen.getByText("不接管")).toBeInTheDocument();
    expect(screen.getByText("高置信观察")).toBeInTheDocument();
    expect(screen.getByText(/smaf=90 smll_samples=8 atca=active_cognition/)).toBeInTheDocument();
    expect(screen.queryByTestId("alt-contract-row-bacm-doge-small")).not.toBeInTheDocument();
  });

  it("surfaces the alt contract event stream before downstream analysis panels", async () => {
    render(<BinanceAltContractMonitor />);

    const eventHeading = await screen.findByText("Alt Contract Event Stream");
    const flowHeading = screen.getByText("60s Alt Contract Flow");
    const auditHeading = screen.getByText("SMAF System Audit");

    expect(
      eventHeading.compareDocumentPosition(flowHeading) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      eventHeading.compareDocumentPosition(auditHeading) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("uses history when severity filter is selected", async () => {
    const user = userEvent.setup();
    render(<BinanceAltContractMonitor />);

    await screen.findByText("山寨合约异常监控");
    await user.selectOptions(screen.getByLabelText("等级"), "critical");

    await waitFor(() =>
      expect(fetchBinanceAltContractHistory).toHaveBeenCalledWith(
        expect.objectContaining({
          symbol: "all",
          severity: "critical",
          signal_type: "all",
          limit: 50,
        }),
      ),
    );
  });

  it("uses history when liquidationDriven and tier filters are selected", async () => {
    const user = userEvent.setup();
    render(<BinanceAltContractMonitor />);

    await screen.findByText("山寨合约异常监控");
    await user.selectOptions(screen.getByLabelText("清算"), "true");
    await user.selectOptions(screen.getByLabelText("流动性 Tier"), "b");

    await waitFor(() =>
      expect(fetchBinanceAltContractHistory).toHaveBeenCalledWith(
        expect.objectContaining({
          liquidationDriven: "true",
          tier: "b",
          limit: 50,
        }),
      ),
    );
  });

  it("opens a read-only detail modal with score and source snapshot", async () => {
    const user = userEvent.setup();
    render(<BinanceAltContractMonitor />);

    await screen.findByText("山寨合约异常监控");
    await user.click(screen.getByTestId("alt-contract-row-bacm-sol-s"));

    expect(screen.getByText("Alt Contract Review")).toBeInTheDocument();
    expect(screen.getByText("SOL · 累积压力观察")).toBeInTheDocument();
    expect(screen.getAllByText("$175.50").length).toBeGreaterThan(0);
    expect(screen.getByText("Discord dry-run")).toBeInTheDocument();
    expect(screen.getAllByText("dry-run would_send").length).toBeGreaterThan(0);
    expect(screen.getByText("语义层")).toBeInTheDocument();
    expect(screen.getByText("语义标签")).toBeInTheDocument();
    expect(screen.getByText("观测强度")).toBeInTheDocument();
    expect(screen.getByText("结构置信度")).toBeInTheDocument();
    expect(screen.getByText("证据数量")).toBeInTheDocument();
    expect(screen.getByText("Window Confirmations")).toBeInTheDocument();
    expect(screen.getByText("Market Tier")).toBeInTheDocument();
    expect(screen.getByText("AIS 展示门槛")).toBeInTheDocument();
    expect(screen.getAllByText("$750,000").length).toBeGreaterThan(0);
    expect(screen.getByText("Alt Impact Score")).toBeInTheDocument();
    expect(screen.getByText(/AIS 用相对市场冲击代替固定 USD 门槛/)).toBeInTheDocument();
    expect(screen.getAllByText("Liquidity Microstructure").length).toBeGreaterThan(0);
    expect(screen.getByText(/LME 只解释盘口微观结构/)).toBeInTheDocument();
    expect(screen.getByText("上扫流动性")).toBeInTheDocument();
    expect(screen.getAllByText("Market Control Graph").length).toBeGreaterThan(0);
    expect(screen.getByText(/MCG 是控制关系图谱/)).toBeInTheDocument();
    expect(screen.getAllByText("操控市场").length).toBeGreaterThan(0);
    expect(screen.getByText("S Grade Conditions")).toBeInTheDocument();
    expect(screen.getByText(/成交额达到 S 门槛/)).toBeInTheDocument();
    expect(screen.getByText("主动买入占优")).toBeInTheDocument();
    expect(screen.getByText("多窗口确认")).toBeInTheDocument();
    expect(screen.getByText("Score Breakdown")).toBeInTheDocument();
    expect(screen.getByText("Master Capital Strength")).toBeInTheDocument();
    expect(screen.getByText(/MCSS 只用于跨市场资金强度解释/)).toBeInTheDocument();
    expect(screen.getAllByText("Market Regime").length).toBeGreaterThan(0);
    expect(screen.getByText(/Regime 是滞后行为结构判断/)).toBeInTheDocument();
    expect(screen.getByText(/拉升诱多/)).toBeInTheDocument();
    expect(screen.getAllByText("Smart Money Lifecycle").length).toBeGreaterThan(0);
    expect(screen.getByText(/SMLE 是时间序列状态机视角/)).toBeInTheDocument();
    expect(screen.getByText(/Accumulation → Markup/)).toBeInTheDocument();
    expect(screen.getByText(/当前接近拉升阶段/)).toBeInTheDocument();
    expect(screen.getAllByText("Smart Money Prediction").length).toBeGreaterThan(0);
    expect(screen.getByText(/SMP 只预测主力行为阶段转移/)).toBeInTheDocument();
    expect(screen.getByText(/拉升后默认观察是否进入派发阶段/)).toBeInTheDocument();
    expect(screen.getByText("Bearish Risk (0.62)")).toBeInTheDocument();
    expect(screen.getAllByText("Signal Confidence Calibration").length).toBeGreaterThan(0);
    expect(screen.getByText(/SCC 是最终可信度校准层/)).toBeInTheDocument();
    expect(screen.getByText(/多层确认较强/)).toBeInTheDocument();
    expect(screen.getAllByText("高可信").length).toBeGreaterThan(0);
    expect(screen.getByText("BACM 信号强")).toBeInTheDocument();
    expect(screen.getByText("预测不一致")).toBeInTheDocument();
    expect(screen.getByText("Active Source Snapshot")).toBeInTheDocument();
    expect(screen.getByText("binance · perp · primary · active")).toBeInTheDocument();
    expect(screen.getByText(/主动买入、OI 与价格响应呈现同向强化/)).toBeInTheDocument();
    expect(screen.getByText("Abnormal Score")).toBeInTheDocument();
    expect(screen.getByText("Build Score")).toBeInTheDocument();
    expect(screen.getAllByText("OI").length).toBeGreaterThan(0);
    expect(screen.getByText("Price Move")).toBeInTheDocument();
    expect(screen.getByText(/有强平快照/)).toBeInTheDocument();
    expect(screen.queryByText(/rawPayload/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/webhook/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/token/i)).not.toBeInTheDocument();
  });
});

function altSummary() {
  return {
    status: "active",
    healthStatus: "healthy",
    latestDirection: "buy",
    latestSeverity: "s",
    latestSignalAt: 1_700_000_000_000,
    signalCount: 1,
    monitoredSymbols: ["SOLUSDT", "DOGEUSDT"],
    displayMinNotionalUsd: 500_000,
    displayThresholdsUsd: {
      ultraCore: 750_000,
      mainstream: 500_000,
      alt: 150_000,
    },
    activeAnomalyCount: 1,
    recentCriticalOrSCount: 1,
    dryRunWouldSendCount: 1,
    enabled: true,
    dryRun: true,
    readOnly: true,
    trend60s: {
      buyVolumeBase: 820_000,
      sellVolumeBase: 210_000,
      totalVolumeBase: 1_030_000,
      netVolumeBase: 610_000,
      totalNotionalUsd: 143_910_000,
      dominance: 0.59,
      buyRatio: 0.8,
      sellRatio: 0.2,
      updatedAtMs: 1_700_000_000_000,
    },
    exchanges: {
      binance: {
        connected: true,
        status: "connected",
        lastTradeAt: Date.now(),
        latencyMs: 92,
        reconnectCount: 0,
      },
    },
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
    symbolUniverse: {
      mode: "all_binance_usdt_perp",
      limit: 0,
      monitoredCount: 2,
      tierCounts: { A: 1, B: 1, C: 0, D: 0, E: 0 },
      whitelist: [],
      blacklist: [],
      excludedSymbols: ["BTCUSDT", "ETHUSDT"],
      min24hQuoteVolumeUsd: 0,
    },
    allMarketContext: {
      markPriceConnected: true,
      tickerConnected: true,
      forceOrderConnected: true,
      lastMarkPriceAt: Date.now(),
      lastTickerAt: Date.now(),
      lastForceOrderAt: Date.now(),
      candidateSymbols: ["SOLUSDT"],
      hotOiSymbols: ["SOLUSDT"],
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
    altImpactScore: {
      finalScore: 42,
      displayThreshold: 70,
      marketImpactRatio: 0.0005,
      liquidityImpact: 6,
      directionalScore: 10,
      oiConfirmation: 0,
      referenceVolume24hUsd: 1_000_000_000,
      referenceSource: "ticker_quote_volume_24h",
      interpretation: "相对市场冲击偏弱",
    },
  };
}

function altSignal() {
  return {
    id: "bacm-sol-s",
    ts: 1_700_000_000_000,
    symbol: "SOL",
    productId: "SOLUSDT",
    marketTier: "ultra_core",
    displayThresholdUsd: 750_000,
    tier: "b",
    windowSec: 60,
    signalType: "main_force_long_build",
    direction: "buy",
    severity: "s",
    abnormalScore: 91,
    buildScore: 87,
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
    altImpactScore: {
      marketImpactRatio: 0.03,
      marketImpactScore: 40,
      liquidityImpact: 24,
      capImpact: 0,
      directionalStrength: 0.74,
      directionalScore: 20,
      oiConfirmation: 10,
      finalScore: 94,
      displayThreshold: 70,
      discordThreshold: 85,
      sThreshold: 90,
      referenceVolume24hUsd: 4_797_000_000,
      referenceSource: "ticker_quote_volume_24h",
      interpretation: "极强相对成交冲击，可能影响该币市场结构",
    },
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
    sGradeEligible: true,
    sGradeNotionalThresholdUsd: 60_000_000,
    sGradeVolumeThresholdBase: 341_880.34,
    sGradeConditions: [
      {
        key: "notional_threshold",
        label: "成交额达到 S 门槛",
        passed: true,
        actual: "$143.9M",
        threshold: "$60.0M",
      },
      {
        key: "oi_expansion",
        label: "OI 增幅 > 1%",
        passed: true,
        actual: "1.80%",
        threshold: "> 1.00%",
      },
    ],
    mainForceConfidence: 84,
    evidenceCount: 5,
    evidenceTags: [
      "aggressive_buy_dominant",
      "oi_expanding",
      "dynamic_multiple_critical",
      "price_follow_through",
      "multi_window_confirmed",
    ],
    windowConfirmations: [
      {
        windowSec: 15,
        notionalUsd: 42_000_000,
        dynamicMultiple: 8.2,
        directionalStrength: 0.72,
        confirmed: true,
      },
      {
        windowSec: 60,
        notionalUsd: 143_910_000,
        dynamicMultiple: 10.2,
        directionalStrength: 0.74,
        confirmed: true,
      },
    ],
    marketWideMove: false,
    marketImpulseRatio: 0.04,
    relativeStrengthRank: 2,
    postSignalStatus: "pending",
    signalVwap: 175.5,
    retestStatus: "unknown",
    oiFreshnessSec: 14,
    oiQuality: "fresh",
    fundingCrowding: "neutral",
    fundingPenalty: 0,
    directionBias: 76,
    dataQuality: 92,
    totalVolumeBase: 820_000,
    netVolumeBase: 610_000,
    totalNotionalUsd: 143_910_000,
    triggerPriceUsd: 175.5,
    dominance: 0.74,
    priceMovePct: 2.4,
    dynamicMultiple: 10.2,
    oiChange1mBase: 210_000,
    oiChangePct: 1.8,
    fundingRate: 0.00021,
    liquidationSuspected: false,
    forceOrderSnapshot: true,
    mainExchange: "binance",
    exchanges: [
      {
        exchange: "binance",
        totalVolumeBase: 820_000,
        netVolumeBase: 610_000,
        totalNotionalUsd: 143_910_000,
        dominance: 0.74,
      },
    ],
    scoreBreakdown: {
      volumeScore: 25,
      dynamicScore: 18,
      directionalScore: 13,
      oiScore: 12,
      priceScore: 10,
      liquidationScore: 0,
      persistenceScore: 6,
      fundingScore: 3,
      dataQualityScore: 5,
      penaltyScore: 0,
    },
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
    semantic: {
      layer: "exposure",
      label: "accumulation_pressure",
      intensityLabel: "high_intensity_observation",
      exposureAllowed: true,
      exposureReason: "semantic_exposure_ready",
      title: "累积压力观察",
      summary: "主动买入、OI 与价格响应呈现同向强化，当前更适合作为累积压力解释；下一阶段暂观察 Distribution（76% 置信）且保持只读解释，不构成执行指令。",
      severityDescriptiveOnly: true,
    },
    finalResult: "山寨合约主动买入爆发，OI 同步上升，疑似主力建多。",
  };
}
