import "@testing-library/jest-dom/vitest";
import { render, screen, within } from "@testing-library/react";
import React from "react";
import { describe, expect, it } from "vitest";
import TofMetricsPanel from "../components/TofMetricsPanel.jsx";

describe("TofMetricsPanel", () => {
  it("renders compact TOF-lite aggregate metrics", () => {
    render(
      <TofMetricsPanel
        compact
        metrics={{
          tofScore: 74.3,
          vpinProxy: 89,
          vpinZscore: 2.1,
          vpinPercentile: 94,
          tradeImbalance: -0.37,
          bidDepthWithdrawal: 58,
          askDepthWithdrawal: 12,
          spreadBps: 8.4,
          metricsConfidence: 82,
        }}
      />,
    );

    expect(screen.getByText("TOF Score")).toBeInTheDocument();
    expect(screen.getByText("VPIN Proxy")).toBeInTheDocument();
    expect(screen.getByText("VPIN Z-Score")).toBeInTheDocument();
    expect(screen.getByText("VPIN Percentile")).toBeInTheDocument();
    expect(screen.getByText("94%" )).toBeInTheDocument();
    expect(screen.getByText("成交失衡")).toBeInTheDocument();
    expect(screen.getByText("-0.37")).toBeInTheDocument();
    expect(screen.getByText("8.4bps")).toBeInTheDocument();
    expect(screen.getByText("74")).toBeInTheDocument();
  });

  it("renders null metrics as unavailable and exposes stale provenance", () => {
    render(
      <TofMetricsPanel
        metrics={{
          tofScore: null,
          vpinProxy: null,
          vpinZscore: null,
          vpinPercentile: null,
          tradeImbalance: null,
          bidDepthWithdrawal: null,
          askDepthWithdrawal: null,
          spreadBps: null,
          metricsConfidence: null,
          lineage: {
            provenance: "observed",
            available: true,
            fresh: false,
            alertEligible: false,
            source: "flow_window",
          },
        }}
      />,
    );

    expect(screen.getAllByText("不可用")).toHaveLength(9);
    expect(screen.getByText("数据过期")).toBeInTheDocument();
    expect(screen.queryByText("0")).not.toBeInTheDocument();
  });

  it("shows the source lineage for each TOF metric family", () => {
    render(
      <TofMetricsPanel
        metrics={{
          tofScore: 76,
          vpinProxy: 81,
          vpinZscore: 1.9,
          vpinPercentile: 92,
          tradeImbalance: -0.31,
          bidDepthWithdrawal: null,
          askDepthWithdrawal: null,
          spreadBps: 7.2,
          metricsConfidence: 78,
          lineage: lineage("calculated_from_observed", "observed_tof_formula_v1"),
          metricLineage: {
            hazard: lineage("calculated_from_observed", "observed_tof_formula_v1"),
            vpin: lineage("observed", "vpin_service"),
            tradeImbalance: lineage("calculated_from_observed", "flow_window_service"),
            depth: unavailableLineage("l2_depth_unavailable"),
            spread: lineage("observed", "flow_window_service_l2"),
          },
        }}
      />,
    );

    const vpinCell = screen.getAllByText("VPIN Proxy").at(-1).closest("div");
    const depthCell = screen.getAllByText("Bid 撤出").at(-1).closest("div");
    const spreadCell = screen.getAllByText("价差 bps").at(-1).closest("div");

    expect(within(vpinCell).getByText("已观测")).toBeInTheDocument();
    expect(within(vpinCell).getByText("来源：vpin_service")).toBeInTheDocument();
    expect(within(depthCell).getByText("来源不可用")).toBeInTheDocument();
    expect(within(spreadCell).getByText("来源：flow_window_service_l2")).toBeInTheDocument();
  });
});

function lineage(provenance, source) {
  return {
    provenance,
    available: true,
    fresh: true,
    alertEligible: true,
    source,
  };
}

function unavailableLineage(unavailableReason) {
  return {
    provenance: "unavailable",
    available: false,
    fresh: false,
    alertEligible: false,
    source: "unavailable",
    unavailableReason,
  };
}
