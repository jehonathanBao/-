import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
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
    expect(screen.getByText("成交失衡")).toBeInTheDocument();
    expect(screen.getByText("-0.37")).toBeInTheDocument();
    expect(screen.getByText("8.4bps")).toBeInTheDocument();
    expect(screen.getByText("74")).toBeInTheDocument();
  });
});
