import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { describe, expect, it } from "vitest";
import AdvancedTofPanel from "../components/AdvancedTofPanel.jsx";

describe("AdvancedTofPanel", () => {
  it("renders safe advanced aggregate metrics", () => {
    render(
      <AdvancedTofPanel
        compact
        metrics={{
          vpinEnhanced: 88,
          largeOrderFlowCluster: 76,
          historicalFundingOiTrend: 84,
          marketPressureHeatmap: 91,
          finalRiskScore: 89,
          dataQuality: 86,
          metricsCompleteness: 95,
          freshDataCoverage: 92,
        }}
      />,
    );

    expect(screen.getByText("VPIN+")).toBeInTheDocument();
    expect(screen.getByText("Flow Cluster")).toBeInTheDocument();
    expect(screen.getByText("Funding/OI")).toBeInTheDocument();
    expect(screen.getByText("Heatmap")).toBeInTheDocument();
    expect(screen.getByText("89")).toBeInTheDocument();
  });

  it("does not coerce unavailable advanced metrics to zero", () => {
    render(
      <AdvancedTofPanel
        metrics={{
          vpinEnhanced: null,
          largeOrderFlowCluster: null,
          historicalFundingOiTrend: null,
          marketPressureHeatmap: null,
          finalRiskScore: null,
          dataQuality: null,
          metricsCompleteness: null,
          freshDataCoverage: null,
          lineage: {
            provenance: "unavailable",
            available: false,
            fresh: false,
            alertEligible: false,
            source: "advanced_tof",
          },
        }}
      />,
    );

    expect(screen.getAllByText("不可用").length).toBeGreaterThanOrEqual(8);
    expect(screen.queryByText("0")).not.toBeInTheDocument();
  });
});
