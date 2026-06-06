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
          evidence: "raw order book evidence",
          markout: "hidden markout",
          stale: true,
        }}
      />,
    );

    expect(screen.getByText("看跌 / Ask-Sell")).toBeInTheDocument();
    expect(screen.getByText("置信度 83")).toBeInTheDocument();
    expect(screen.getByText("detector+tof_metrics")).toBeInTheDocument();
    expect(screen.getByText("Type: spoofing_candidate")).toBeInTheDocument();
    expect(screen.getByText("high_vpin_proxy")).toBeInTheDocument();
    expect(screen.getByText("bid_depth_withdrawal")).toBeInTheDocument();
    expect(screen.queryByText("raw order book evidence")).not.toBeInTheDocument();
    expect(screen.queryByText("hidden markout")).not.toBeInTheDocument();
  });
});
