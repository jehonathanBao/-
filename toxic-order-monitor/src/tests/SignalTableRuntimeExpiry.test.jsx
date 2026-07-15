import "@testing-library/jest-dom/vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RUNTIME_BOUNDARY_TTL_MS } from "../api/alertGate.js";
import SignalTable from "../components/SignalTable.jsx";

describe("SignalTable runtime expiry", () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("automatically disables a previously eligible push button when runtime truth expires", async () => {
    const now = 1_800_000_000_000;
    vi.useFakeTimers();
    vi.setSystemTime(now);
    render(
      <SignalTable
        inboxStats={{ all: 1, high: 1, low: 0, medium: 0, total: 1 }}
        onPush={vi.fn()}
        onSelect={vi.fn()}
        signals={[safeSignal(now)]}
      />,
    );

    const pushButton = screen.getByRole("button", { name: "推送 expiring-signal 到 Discord" });
    expect(pushButton).toBeEnabled();

    await act(async () => {
      vi.advanceTimersByTime(RUNTIME_BOUNDARY_TTL_MS + 1);
    });

    expect(pushButton).toBeDisabled();
  });

  it("continues scheduling later expiry times after the first signal becomes stale", async () => {
    const now = 1_800_000_000_000;
    vi.useFakeTimers();
    vi.setSystemTime(now);
    render(
      <SignalTable
        inboxStats={{ all: 2, high: 2, low: 0, medium: 0, total: 2 }}
        onPush={vi.fn()}
        onSelect={vi.fn()}
        signals={[
          safeSignal(now, "first-expiry"),
          safeSignal(now + 1_000, "later-expiry"),
        ]}
      />,
    );

    const firstButton = screen.getByRole("button", { name: "推送 first-expiry 到 Discord" });
    const laterButton = screen.getByRole("button", { name: "推送 later-expiry 到 Discord" });
    expect(firstButton).toBeEnabled();
    expect(laterButton).toBeEnabled();

    await act(async () => {
      vi.advanceTimersByTime(RUNTIME_BOUNDARY_TTL_MS + 1);
    });
    expect(firstButton).toBeDisabled();
    expect(laterButton).toBeEnabled();

    await act(async () => {
      vi.advanceTimersByTime(1_000);
    });
    expect(laterButton).toBeDisabled();
  });
});

function safeSignal(checkedAtMs, id = "expiring-signal") {
  return {
    id,
    dedupeKey: id,
    symbol: "BTCUSDT",
    type: "toxic_flow_candidate",
    time: "2026-07-16 00:00:00",
    side: "Ask/Sell",
    level: "A",
    risk: "high",
    status: "unhandled",
    authoritativeRiskScore: 91,
    riskScore: 91,
    confidence: 90,
    authoritativeDataQuality: 90,
    dataQualityScore: 90,
    alertEligible: true,
    isLive: true,
    runtimeBoundary: {
      phase: "confirmed",
      readOnly: true,
      monitoringStarted: true,
      executionEnabled: false,
      runtimeModified: false,
      analysisOnly: true,
      checkedAtMs,
    },
  };
}
