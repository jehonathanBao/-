import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import NewTokenWatch from "../components/NewTokenWatch.jsx";
import {
  addNewTokenWatch,
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
            evidence: ["buy_aggression_with_compressed_price"],
          },
        },
      ],
      maxActiveTokens: 10,
      activeCount: 1,
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
    render(<NewTokenWatch />);

    expect(await screen.findByText("新币合约行为探针")).toBeInTheDocument();
    expect(screen.getByText("ABCUSDT")).toBeInTheDocument();
    expect(screen.getByText("吸筹")).toBeInTheDocument();
    expect(screen.getByText("Smart Money 75%")).toBeInTheDocument();
    expect(screen.getByText("30s +0.61")).toBeInTheDocument();
    expect(screen.getByText("吸收 0.30%")).toBeInTheDocument();
    expect(screen.getByText("PVG 建议允许 22%")).toBeInTheDocument();
    expect(screen.getByText("SMP +0.62 · MFE -0.12 · LSM +0.18")).toBeInTheDocument();
    expect(screen.getByText("流动扩张 · 偏多")).toBeInTheDocument();
    expect(screen.getByText("Q 69% · Size 54%")).toBeInTheDocument();
    expect(screen.getByText("1/10")).toBeInTheDocument();
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
