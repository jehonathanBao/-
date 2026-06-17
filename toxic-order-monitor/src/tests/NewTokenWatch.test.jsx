import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import NewTokenWatch from "../components/NewTokenWatch.jsx";
import {
  addNewTokenWatch,
  fetchNewTokenChart,
  fetchNewTokenReconstruction,
  fetchNewTokenWatchList,
  removeNewTokenWatch,
} from "../api/newTokenWatch.js";

vi.mock("../hooks/useReconnectingWebSocket.js", () => ({
  useReconnectingWebSocket: vi.fn(() => ({ status: "open", socket: null })),
}));

vi.mock("../api/newTokenWatch.js", () => ({
  fetchNewTokenWatchList: vi.fn(() =>
    Promise.resolve({
      items: [
        {
          symbol: "ABCUSDT",
          streamStatus: "read_only_probe",
          readOnly: true,
          lastSignal: {
            regime: "accumulation",
            strength: 0.72,
            confidence: 0.76,
            flowPersistence: 0.8,
            ofiWindows: [{ windowSec: 30, normalizedOfi: 0.61, persistence: 0.8 }],
            impactResponse: { classification: "absorption", priceMovePct: 0.003, absorptionScore: 0.74, thinLiquidityScore: 0.2 },
            liquidityDepletion: { bidDepletionRate: 0.1, askDepletionRate: 0.2, depletionPressure: 0.08 },
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
                { windowSec: 300, normalizedOfi: 0.62, vwap: 1.94 },
                { windowSec: 900, normalizedOfi: 0.41, vwap: 1.95 },
                { windowSec: 3600, normalizedOfi: 0.1, vwap: 1.96 },
              ],
              costBasis: { lower: 1.92, upper: 1.97, vwapAnchor: 1.94, confidence: 0.78 },
              estimatedPosition: { lowerUsd: 3200000, upperUsd: 5800000, confidence: 0.73 },
              horizon: { minMinutes: 18, maxMinutes: 42, detectedMinutes: 24 },
              distributionRisk: {
                score: 0.27,
                level: "low",
                reasons: ["no_distribution_pressure_confirmed"],
              },
              evidence: ["phase=accumulation", "cost_vwap=1.940000"],
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
                {
                  phase: "accumulation",
                  label: "final_accumulation",
                  startPrice: 1.88,
                  endPrice: 1.9,
                  volume: 160,
                  cumulativeDelta: 112,
                  impact: 0.00008,
                  durationSec: 240,
                  confidence: 0.84,
                  characteristics: ["volatility_compression"],
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
                { timestamp: 1, price: 1.86, estimatedPosition: 20, impactAdjustedPosition: 18 },
                { timestamp: 2, price: 1.9, estimatedPosition: 42, impactAdjustedPosition: 39 },
              ],
              confidence: 0.82,
              regimeLabel: "accumulation_trajectory",
              evidence: ["last_accumulation_node_detected"],
              readOnly: true,
            },
            evidence: ["buy_aggression_with_compressed_price"],
          },
        },
      ],
      maxActiveTokens: 10,
      activeCount: 1,
      readOnly: true,
    }),
  ),
  fetchNewTokenReconstruction: vi.fn(() =>
    Promise.resolve({
      symbol: "ABCUSDT",
      timeframe: "15m",
      currentPhase: "accumulation",
      currentPrice: 1.95,
      change24hPct: null,
      volume24hUsd: null,
      high24h: null,
      low24h: null,
      marketCapUsd: null,
      costBasisLow: 1.92,
      costBasisHigh: 1.97,
      vwapAnchor: 1.94,
      estimatedTotalPositionUsdtLow: 3200000,
      estimatedTotalPositionUsdtHigh: 5800000,
      estimatedNetPositionUsdt: 76050,
      floatingPnlLowPct: 1.56,
      floatingPnlHighPct: -1.02,
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
        {
          phase: "accumulation",
          label: "final_accumulation",
          startPrice: 1.88,
          endPrice: 1.9,
          volume: 160,
          cumulativeDelta: 112,
          impact: 0.00008,
          durationSec: 240,
          confidence: 0.84,
          characteristics: ["volatility_compression"],
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
      distributionCompletionPct: 0,
      distributionIntensityScore: 27,
      shortTermBehaviorProbabilities: {
        continueDistribution: 0.1,
        rangeConsolidation: 0.2,
        reboundMarkup: 0.25,
        secondaryAccumulation: 0.45,
      },
      phaseTimeline: [
        {
          phase: "accumulation",
          label: "silent_accumulation",
          startMs: 1,
          endMs: 181000,
          durationSec: 180,
          lower: 1.82,
          upper: 1.86,
        },
      ],
      costDistribution: [
        { label: "核心成本区", lower: 1.92, upper: 1.97, pct: 0.62 },
        { label: "早期吸筹区", lower: 1.88, upper: 1.92, pct: 0.23 },
      ],
      smartLevels: [
        { label: "VWAP锚点", price: 1.94, role: "anchor" },
        { label: "最后吸筹点", price: 1.89, role: "last_accumulation" },
      ],
      confidence: 0.82,
      readOnly: true,
    }),
  ),
  fetchNewTokenChart: vi.fn(() =>
    Promise.resolve({
      symbol: "ABCUSDT",
      timeframe: "15m",
      points: [
        { ts: 1, price: 1.86, volume: 8, netPosition: 18 },
        { ts: 2, price: 1.9, volume: 12, netPosition: 39 },
        { ts: 3, price: 1.95, volume: 10, netPosition: 42 },
      ],
      phaseSegments: [],
      markers: [{ ts: 2, price: 1.89, label: "最后吸筹点", kind: "last_accumulation" }],
      readOnly: true,
    }),
  ),
  addNewTokenWatch: vi.fn(() =>
    Promise.resolve({
      ok: true,
      item: { symbol: "DEFUSDT" },
      items: [
        {
          symbol: "DEFUSDT",
          streamStatus: "read_only_probe",
          lastSignal: {
            regime: "building",
            strength: 0.65,
            confidence: 0.7,
            flowPersistence: 0.7,
            ofiWindows: [{ windowSec: 30, normalizedOfi: 0.52, persistence: 0.7 }],
            impactResponse: { classification: "thin_liquidity", priceMovePct: 0.02, absorptionScore: 0.2, thinLiquidityScore: 0.78 },
            liquidityDepletion: { bidDepletionRate: 0.05, askDepletionRate: 0.3, depletionPressure: 0.15 },
            actorDecomposition: {
              dominantActor: "momentum_chaser",
              smartMoneyProbability: 0.18,
              liquidityProviderProbability: 0.16,
              momentumChaserProbability: 0.66,
              confidence: 0.8,
            },
            signalCompression: {
              smartMoneyPressure: 0.18,
              momentumFlowExhaustion: 0.61,
              liquidityStressManipulation: 0.32,
              positionValidityGate: {
                riskScore: 0.16,
                tradePermission: true,
                positionSizeMultiplier: 0.84,
                reason: "advisory_allowed",
                advisoryOnly: true,
              },
              stabilityKernel: {
                regime: "trend",
                regimeQuality: 0.7,
                tradeSignal: {
                  direction: "long",
                  confidence: 0.68,
                  expectedHoldTime: "short",
                  invalidationCondition: "mfe_falls_below_0_15_or_lsm_above_0_55",
                  reason: "momentum_continuation_window",
                  advisoryOnly: true,
                },
                positionSmoothing: {
                  suggestedSizeMultiplier: 0.4,
                  volatilityAdjustment: 0.85,
                  drawdownAdjustment: 1,
                  reason: "confidence_x_regime_quality_x_volatility_x_pvg",
                },
              },
            },
            capitalStructure: {
              phase: "markup",
              phaseLabel: "markup",
              phaseConfidence: 0.66,
              behaviorWindows: [
                { windowSec: 300, normalizedOfi: 0.52, vwap: 2.12 },
                { windowSec: 900, normalizedOfi: 0.31, vwap: 2.11 },
                { windowSec: 3600, normalizedOfi: 0.08, vwap: 2.1 },
              ],
              costBasis: { lower: 2.08, upper: 2.16, vwapAnchor: 2.12, confidence: 0.62 },
              estimatedPosition: { lowerUsd: 900000, upperUsd: 2100000, confidence: 0.58 },
              horizon: { minMinutes: 4, maxMinutes: 12, detectedMinutes: 5 },
              distributionRisk: { score: 0.18, level: "low", reasons: ["no_distribution_pressure_confirmed"] },
              evidence: ["phase=markup"],
              readOnly: true,
            },
            positionReconstruction: {
              accumulationPath: [
                {
                  phase: "markup",
                  label: "markup_expansion",
                  startPrice: 2.08,
                  endPrice: 2.16,
                  volume: 180,
                  cumulativeDelta: 96,
                  impact: 0.0003,
                  durationSec: 120,
                  confidence: 0.66,
                  characteristics: ["positive_delta"],
                },
              ],
              lastAccumulationNode: null,
              distributionPath: [],
              latentPosition: [{ timestamp: 1, price: 2.16, estimatedPosition: 22, impactAdjustedPosition: 17 }],
              confidence: 0.61,
              regimeLabel: "markup_after_accumulation",
              evidence: [],
              readOnly: true,
            },
            evidence: [],
          },
        },
      ],
      maxActiveTokens: 10,
      readOnly: true,
    }),
  ),
  removeNewTokenWatch: vi.fn(() =>
    Promise.resolve({
      ok: true,
      item: { symbol: "ABCUSDT" },
      items: [],
      maxActiveTokens: 10,
      readOnly: true,
    }),
  ),
}));

describe("NewTokenWatch", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("renders active token behavior signals", async () => {
    const user = userEvent.setup();
    render(<NewTokenWatch />);

    expect(await screen.findByText("智能资金仓位重建引擎")).toBeInTheDocument();
    expect(screen.getByText("ABCUSDT")).toBeInTheDocument();
    expect(await screen.findByText("主力仓位摘要")).toBeInTheDocument();
    expect(screen.getAllByText("静默吸筹").length).toBeGreaterThan(0);
    expect(screen.getAllByText("$1.92 - $1.97").length).toBeGreaterThan(0);
    expect(screen.getByText("VWAP $1.94")).toBeInTheDocument();
    expect(screen.getByText("$3.2M - $5.8M")).toBeInTheDocument();
    expect(screen.getByText("分批建仓路径")).toBeInTheDocument();
    expect(screen.getByText("拉升前最后吸筹点")).toBeInTheDocument();
    expect(screen.getByText("出货分布轨迹")).toBeInTheDocument();
    expect(screen.getAllByText("$1.88 - $1.9").length).toBeGreaterThan(0);
    expect(screen.getAllByText("6m 42s · Abs 84% · Conf 84%").length).toBeGreaterThan(0);
    expect(screen.getByText("短期行为概率")).toBeInTheDocument();
    expect(fetchNewTokenReconstruction).toHaveBeenCalledWith("ABCUSDT", "15m");
    expect(fetchNewTokenChart).toHaveBeenCalledWith("ABCUSDT", "15m");
    expect(screen.getByText("1/10")).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("行为窗口"), "4h");

    await waitFor(() => {
      expect(fetchNewTokenReconstruction).toHaveBeenCalledWith("ABCUSDT", "4h");
      expect(fetchNewTokenChart).toHaveBeenCalledWith("ABCUSDT", "4h");
    });
  });

  it("adds and stops token watches", async () => {
    const user = userEvent.setup();
    render(<NewTokenWatch />);

    await screen.findByText("ABCUSDT");
    await user.type(screen.getByLabelText("新币合约 symbol"), "defusdt");
    await user.click(screen.getByRole("button", { name: "加入监控" }));

    await waitFor(() => {
      expect(addNewTokenWatch).toHaveBeenCalledWith("defusdt");
    });
    expect(await screen.findByText("DEFUSDT")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Stop" }));
    await waitFor(() => {
      expect(removeNewTokenWatch).toHaveBeenCalledWith("DEFUSDT");
    });
  });
});
