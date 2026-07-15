import "@testing-library/jest-dom/vitest";
import { act, cleanup, render, screen } from "@testing-library/react";
import React from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RUNTIME_BOUNDARY_TTL_MS } from "../api/alertGate.js";
import RuntimeBoundaryBadge, { runtimeDisplay } from "../components/RuntimeBoundaryBadge.jsx";

describe("RuntimeBoundaryBadge", () => {
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("shows UNKNOWN when an otherwise safe runtime confirmation has expired", () => {
    const now = 1_800_000_000_000;

    const display = runtimeDisplay(safeRuntime(now - RUNTIME_BOUNDARY_TTL_MS - 1), now);

    expect(display.label).toBe("RUNTIME UNKNOWN");
    expect(display.detail).toContain("stale");
  });

  it("shows READ ONLY while the safe runtime confirmation remains fresh", () => {
    const now = 1_800_000_000_000;

    const display = runtimeDisplay(safeRuntime(now - RUNTIME_BOUNDARY_TTL_MS), now);

    expect(display.label).toBe("READ ONLY");
  });

  it("automatically rerenders as UNKNOWN when the runtime confirmation expires", async () => {
    const now = 1_800_000_000_000;
    vi.useFakeTimers();
    vi.setSystemTime(now);
    render(<RuntimeBoundaryBadge runtimeBoundary={safeRuntime(now)} showDetail />);

    expect(screen.getByText("READ ONLY")).toBeInTheDocument();

    await act(async () => {
      vi.advanceTimersByTime(RUNTIME_BOUNDARY_TTL_MS + 1);
    });

    expect(screen.getByText("RUNTIME UNKNOWN")).toBeInTheDocument();
    expect(screen.getByText("Runtime status stale · Push disabled")).toBeInTheDocument();
  });
});

function safeRuntime(checkedAtMs) {
  return {
    phase: "confirmed",
    readOnly: true,
    monitoringStarted: true,
    executionEnabled: false,
    runtimeModified: false,
    analysisOnly: true,
    checkedAtMs,
  };
}
