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
          finalRiskScore: 91,
          score: 84,
          dataQuality: 86,
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
    expect(screen.getByText("Risk 91 / Quality 86")).toBeInTheDocument();
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
