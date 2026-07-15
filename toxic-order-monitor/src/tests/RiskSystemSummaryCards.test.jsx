import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { describe, expect, it } from "vitest";
import RiskSystemSummaryCards from "../components/RiskSystemSummaryCards.jsx";

describe("RiskSystemSummaryCards", () => {
  it("renders the short-toxic and market-structure score cards with requested fields", () => {
    render(
      <RiskSystemSummaryCards
        signal={{
          symbol: "BTCUSDT",
          type: "SpoofingCandidate",
          time: "2026-06-09 12:31:22",
          score: 87,
          finalRiskScore: 87,
          toxicScore: 87,
          toxicSeverity: "Critical",
          shortPressure: -72,
          toxicTtlSec: 48,
          confidence: 81,
          side: "Ask/Sell",
          tofMetrics: {
            depthWithdrawalScore: 76,
            spreadWideningScore: 61,
            tradeImbalanceScore: 83,
            tradeImbalance: -0.18,
            metricsDirection: "bearish",
          },
          toxicReasons: [
            { reasonType: "ToxicOrderCluster", score: 91 },
            { reasonType: "AggressiveSweep", score: 84 },
            { reasonType: "OrderbookDeformation", score: 76 },
            { reasonType: "SpoofCancel", score: 42 },
            { reasonType: "AdverseMove", score: 88 },
            { reasonType: "LiquidityGap", score: 69 },
          ],
          mainForceScore: 84,
          marketStructureSeverity: "Major",
          structureBias: 62,
          extremeImpactScore: 58,
          regimeType: "main_force_long_build",
          marketStructureConfidence: 76,
          spotScore: 71,
          contractScore: 86,
          crossConfirmScore: 74,
          cwmAggressiveFlow: 89,
          oiScore: 82,
          liquidationScore: 31,
          fundingCrowdingScore: 24,
          mainForceConfirmed: true,
          multiWindowConsistency: 82,
          cwmContribution: {
            exchangeCount: 2,
          },
        }}
      />,
    );

    expect(screen.getByText("短线有毒订单评分")).toBeInTheDocument();
    expect(screen.getByText("现货 + 合约主力结构评分")).toBeInTheDocument();
    expect(screen.getAllByText("87 / Critical").length).toBeGreaterThan(0);
    expect(screen.getByText("偏空 -72")).toBeInTheDocument();
    expect(screen.getByText("约 48 秒")).toBeInTheDocument();
    expect(screen.getByText("81")).toBeInTheDocument();
    expect(screen.getByText("异常订单聚集")).toBeInTheDocument();
    expect(screen.getByText("主动扫盘")).toBeInTheDocument();
    expect(screen.getByText("盘口变形")).toBeInTheDocument();
    expect(screen.getByText("虚假挂单")).toBeInTheDocument();
    expect(screen.getByText("反向伤害")).toBeInTheDocument();
    expect(screen.getByText("流动性缺口")).toBeInTheDocument();
    expect(screen.getByText("5s 内主动卖出扫穿近端买盘")).toBeInTheDocument();
    expect(screen.getByText("买盘深度快速消失")).toBeInTheDocument();

    expect(screen.getAllByText("84 / Major").length).toBeGreaterThan(0);
    expect(screen.getByText("偏多 +62")).toBeInTheDocument();
    expect(screen.getByText("主力建多")).toBeInTheDocument();
    expect(screen.getByText("现货评分")).toBeInTheDocument();
    expect(screen.getByText("合约评分")).toBeInTheDocument();
    expect(screen.getByText("现货合约确认")).toBeInTheDocument();
    expect(screen.getByText("CWM 主力成交流")).toBeInTheDocument();
    expect(screen.getByText("OI 变化")).toBeInTheDocument();
    expect(screen.getByText("清算环境")).toBeInTheDocument();
    expect(screen.getByText("Funding 拥挤")).toBeInTheDocument();
    expect(screen.getByText("主力确认")).toBeInTheDocument();
    expect(screen.getByText("非清算驱动")).toBeInTheDocument();
    expect(screen.getByText("多窗口确认")).toBeInTheDocument();
    expect(screen.getByText("Binance + Bitfinex 同向")).toBeInTheDocument();
  });

  it("renders nullable risk inputs as unavailable instead of calm zeroes", () => {
    render(
      <RiskSystemSummaryCards
        signal={{
          symbol: "BTCUSDT",
          type: "candidate",
          time: "2026-07-15 12:00:00",
          toxicScore: null,
          finalRiskScore: null,
          score: null,
          shortPressure: null,
          confidence: null,
          toxicTtlSec: null,
          toxicExpiresAt: null,
          mainForceScore: null,
          structureBias: null,
          extremeImpactScore: null,
          marketStructureConfidence: null,
          regimeType: null,
        }}
      />,
    );

    expect(screen.getAllByText("N/A / 不可用")).toHaveLength(4);
    expect(screen.getAllByText("不可用").length).toBeGreaterThan(2);
    expect(screen.queryByText("0 / Calm")).not.toBeInTheDocument();
    expect(screen.queryByText("中性 0")).not.toBeInTheDocument();
  });
});
