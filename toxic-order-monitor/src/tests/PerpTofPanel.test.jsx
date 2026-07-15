import "@testing-library/jest-dom/vitest";
import { render, screen, within } from "@testing-library/react";
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

  it("labels inferred squeeze context as an unavailable alert-ineligible proxy", () => {
    render(
      <PerpTofPanel
        metrics={{
          oiChange: null,
          oiDirection: null,
          fundingRate: null,
          fundingSide: null,
          liquidationPressure: null,
          squeezeRiskProxy: 41,
          squeezeSide: null,
          aggBuyVolume: null,
          aggSellVolume: null,
          riskScore: null,
          metricsDirection: null,
          lineage: {
            provenance: "calculated_from_observed",
            available: true,
            fresh: true,
            alertEligible: true,
            source: "contract_whale_monitor",
          },
          liquidationLineage: {
            provenance: "inferred",
            available: true,
            fresh: false,
            alertEligible: false,
            source: "liquidity_cluster_proxy",
          },
        }}
      />,
    );

    const proxyCell = screen.getByText("Squeeze 风险代理").closest("div");
    expect(within(proxyCell).getByText("推断代理 · 不参与 Discord")).toBeInTheDocument();
    expect(within(proxyCell).getByText("数据过期")).toBeInTheDocument();
    expect(within(proxyCell).getByText("来源：liquidity_cluster_proxy")).toBeInTheDocument();
    expect(screen.getAllByText("不可用").length).toBeGreaterThan(0);
    expect(screen.queryByText("NEUTRAL")).not.toBeInTheDocument();
  });

  it("shows observed liquidation notional instead of the inferred squeeze proxy", () => {
    const { container } = render(
      <PerpTofPanel
        metrics={{
          oiChange: 320000,
          oiDirection: "long_increase",
          fundingRate: 0.012,
          fundingSide: "long",
          liquidationPressure: 82,
          squeezeRiskProxy: 82,
          observedLiquidationNotional: 25000000,
          squeezeSide: "long",
          aggBuyVolume: 1500000,
          aggSellVolume: 420000,
          riskScore: 87,
          metricsDirection: "bearish",
          lineage: {
            provenance: "calculated_from_observed",
            available: true,
            fresh: true,
            alertEligible: true,
            source: "contract_whale_monitor",
          },
          liquidationLineage: {
            provenance: "observed",
            available: true,
            fresh: true,
            alertEligible: true,
            source: "contract_whale_liquidation",
          },
        }}
      />,
    );

    const observedCell = screen.getByText("已观测清算名义额 USD").closest("div");
    expect(within(observedCell).getByText("25,000,000")).toBeInTheDocument();
    expect(within(observedCell).getByText("已观测")).toBeInTheDocument();
    expect(within(observedCell).getByText("来源：contract_whale_liquidation")).toBeInTheDocument();
    expect(within(container).queryByText("Squeeze 风险代理")).not.toBeInTheDocument();
  });
});
