import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen } from "@testing-library/react";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import LiquidationCascadeDashboard from "../components/LiquidationCascadeDashboard.jsx";
import RiskCard from "../components/RiskCard.jsx";

vi.mock("../api/liquidationCascade.js", () => ({
  fetchLiquidationCascade: vi.fn(() => Promise.resolve({ data: null, error: null })),
  fetchLiquidationLeverageMap: vi.fn(() => Promise.resolve({ data: null, error: null })),
  fetchLiquidationLiquidityGap: vi.fn(() => Promise.resolve({ data: null, error: null })),
  fetchBtcStructure: vi.fn(() => Promise.resolve({ data: null, error: null })),
  fetchMarketRegime: vi.fn(() => Promise.resolve({ data: null, error: null })),
}));

describe("Institutional workspace panel primitives", () => {
  afterEach(cleanup);

  it("uses route-neutral panels throughout the liquidation workspace", () => {
    const { container } = render(<LiquidationCascadeDashboard />);

    expect(screen.getByRole("heading", { name: "强平瀑布预测" })).toBeInTheDocument();
    expect(container.querySelectorAll(".workspace-panel").length).toBeGreaterThanOrEqual(4);
  });

  it("uses the compact institutional risk-card primitive", () => {
    render(<RiskCard active count={3} onClick={vi.fn()} percentage={75} risk="high" />);

    expect(screen.getByRole("button", { name: "筛选 high 风险" })).toHaveClass("workspace-risk-card");
  });
});
