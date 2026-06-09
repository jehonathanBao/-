import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { describe, expect, it } from "vitest";
import CandidateExplanation from "../components/CandidateExplanation.jsx";

describe("CandidateExplanation", () => {
  it("shows final direction and explanation tags without technical evidence", () => {
    render(
      <CandidateExplanation
        signal={{
          directionLabel: "看跌 / Ask-Sell",
          directionConfidence: 82.5,
          directionSource: "detector+tof_metrics",
          candidateType: "spoofing_candidate",
          explainTags: ["high_vpin_proxy", "bid_depth_withdrawal"],
          toxicScore: 91,
          finalRiskScore: 91,
          score: 84,
          dataQuality: 86,
          mainForceScore: 83,
          mainForceConfirmed: true,
          mainForceConfirmationCount: 6,
          mainForceConfirmationTotal: 7,
          extremeImpactScore: 92,
          extremeImpactConfirmed: true,
          marketStructureSeverity: "Major",
          marketStructureConfidence: 93,
          marketStructureDataQuality: 91,
          regimeType: "main_force_long_build",
          structureBias: 72,
          spotScore: 75,
          spotCvdScore: 84,
          spotVolumeAnomaly: 72,
          spotAbsorption: 64,
          contractScore: 85,
          cwmAggressiveFlow: 92,
          oiImpulse: 88,
          liquidationContext: 91,
          fundingCrowding: 88,
          basisPremium: 63,
          activeExchangeConfirmation: 70,
          spotContractFloor: 75,
          durationScore: 100,
          signalAgreement: 95,
          level: "A",
          finalCandidateType: "High Risk Bullish Candidate",
          metricsDirection: "bullish",
          perpCandidateType: "OpenInterestCandidate",
          advancedCandidateType: "MarketPressureHeatmapCandidate",
          evidence: "raw order book evidence",
          markout: "hidden markout",
          rawPayload: "hidden raw payload",
          stale: true,
        }}
      />,
    );

    expect(screen.getByText("看跌 / Ask-Sell")).toBeInTheDocument();
    expect(screen.getByText("置信度 83")).toBeInTheDocument();
    expect(screen.getByText("detector+tof_metrics")).toBeInTheDocument();
    expect(screen.getByText("Toxic 91 / Quality 86")).toBeInTheDocument();
    expect(screen.getByText("Main Force 83")).toBeInTheDocument();
    expect(screen.getByText("Bias +72")).toBeInTheDocument();
    expect(screen.getByText("Confirmed Yes · 6/7")).toBeInTheDocument();
    expect(screen.getByText("Structure Major · Extreme 92")).toBeInTheDocument();
    expect(screen.getByText("Conf 93 / Quality 91")).toBeInTheDocument();
    expect(screen.getByText("极端行情 是 · 主力建多")).toBeInTheDocument();
    expect(screen.getByText("Spot 75 / Contract 85")).toBeInTheDocument();
    expect(screen.getByText("Spot CVD 84 / Vol 72 / Abs 64")).toBeInTheDocument();
    expect(screen.getByText("Floor 75 / Duration 100")).toBeInTheDocument();
    expect(screen.getByText("CWM Flow 92 / OI 88 / Liq 91")).toBeInTheDocument();
    expect(screen.getByText("Severity A")).toBeInTheDocument();
    expect(screen.getByText("Final High Risk Bullish Candidate")).toBeInTheDocument();
    expect(screen.getByText("Metrics bullish")).toBeInTheDocument();
    expect(screen.getByText("Perp: OpenInterestCandidate")).toBeInTheDocument();
    expect(screen.getByText("Advanced: MarketPressureHeatmapCandidate")).toBeInTheDocument();
    expect(screen.getByText("Type: spoofing_candidate")).toBeInTheDocument();
    expect(screen.getByText("high_vpin_proxy")).toBeInTheDocument();
    expect(screen.getByText("bid_depth_withdrawal")).toBeInTheDocument();
    expect(screen.queryByText("raw order book evidence")).not.toBeInTheDocument();
    expect(screen.queryByText("hidden markout")).not.toBeInTheDocument();
    expect(screen.queryByText("hidden raw payload")).not.toBeInTheDocument();
  });
});
