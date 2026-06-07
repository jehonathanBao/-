import "@testing-library/jest-dom/vitest";
import { render, screen } from "@testing-library/react";
import React from "react";
import { describe, expect, it } from "vitest";
import PerpTofPanel from "../components/PerpTofPanel.jsx";

describe("PerpTofPanel", () => {
  it("renders safe contract-side TOF aggregate metrics", () => {
    render(
      <PerpTofPanel
        compact
        metrics={{
          oiChange: 150000,
          oiDirection: "long_increase",
          fundingRate: -0.071,
          fundingSide: "short",
          liquidationPressure: 82,
          squeezeSide: "short",
          aggBuyVolume: 1500000,
          aggSellVolume: 420000,
          metricsDirection: "bullish",
          riskScore: 87,
        }}
      />,
    );

    expect(screen.getByText("OI")).toBeInTheDocument();
    expect(screen.getByText("150,000")).toBeInTheDocument();
    expect(screen.getByText("long_increase")).toBeInTheDocument();
    expect(screen.getByText("-0.0710% short")).toBeInTheDocument();
    expect(screen.getByText("Perp Risk")).toBeInTheDocument();
    expect(screen.getByText("bullish")).toBeInTheDocument();
  });
});
