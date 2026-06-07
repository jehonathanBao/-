import "@testing-library/jest-dom/vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import React from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { fetchScanLogs } from "../api/scanLogs.js";
import ScanLogPanel from "../components/ScanLogPanel.jsx";
import { useReconnectingWebSocket } from "../hooks/useReconnectingWebSocket.js";

const wsMock = vi.hoisted(() => ({
  options: null,
  status: "open",
}));

vi.mock("../api/scanLogs.js", async () => {
  const actual = await vi.importActual("../api/scanLogs.js");
  return {
    ...actual,
    fetchScanLogs: vi.fn(),
  };
});

vi.mock("../hooks/useReconnectingWebSocket.js", () => ({
  useReconnectingWebSocket: vi.fn((_path, options) => {
    wsMock.options = options;
    return { status: wsMock.status, socket: null };
  }),
}));

describe("ScanLogPanel", () => {
  beforeEach(() => {
    wsMock.options = null;
    wsMock.status = "open";
    fetchScanLogs.mockResolvedValue([]);
  });

  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  it("loads recent scan logs from the read-only API", async () => {
    fetchScanLogs.mockResolvedValueOnce([
      scanLog({ id: "1", message: "Market-data scanner started" }),
    ]);

    render(<ScanLogPanel />);

    expect(await screen.findByText("Market-data scanner started")).toBeInTheDocument();
    expect(fetchScanLogs).toHaveBeenCalledWith(100);
  });

  it("appends websocket scan log events", async () => {
    render(<ScanLogPanel />);

    await waitFor(() => expect(wsMock.options?.onMessage).toBeTypeOf("function"));
    wsMock.options.onMessage({
      data: JSON.stringify({
        type: "scan_log_event",
        item: scanLog({ id: "2", message: "Discord push queued for alert-only delivery" }),
      }),
    });

    expect(await screen.findByText("Discord push queued for alert-only delivery")).toBeInTheDocument();
    expect(useReconnectingWebSocket).toHaveBeenCalledWith(
      "/ws/scan-logs",
      expect.objectContaining({ retryMs: 1000, maxRetryMs: 15000 }),
    );
  });

  it("shows contract-side TOF candidate scan log events", async () => {
    render(<ScanLogPanel />);

    await waitFor(() => expect(wsMock.options?.onMessage).toBeTypeOf("function"));
    wsMock.options.onMessage({
      data: JSON.stringify({
        type: "scan_log_event",
        item: scanLog({
          id: "perp-1",
          kind: "perp_candidate_generated",
          message:
            "BTC-PERP perp candidate generated: type=OpenInterestCandidate direction=Bullish score=87",
        }),
      }),
    });

    expect(
      await screen.findByText(
        "BTC-PERP perp candidate generated: type=OpenInterestCandidate direction=Bullish score=87",
      ),
    ).toBeInTheDocument();
  });

  it("shows advanced metrics scan log events", async () => {
    render(<ScanLogPanel />);

    await waitFor(() => expect(wsMock.options?.onMessage).toBeTypeOf("function"));
    wsMock.options.onMessage({
      data: JSON.stringify({
        type: "scan_log_event",
        item: scanLog({
          id: "advanced-1",
          kind: "advanced_metrics_computed",
          message:
            "BTC-PERP advanced metrics computed: vpinEnhanced=88 flowCluster=76 fundingOiTrend=84 heatmap=91 finalScore=89",
        }),
      }),
    });

    expect(
      await screen.findByText(
        "BTC-PERP advanced metrics computed: vpinEnhanced=88 flowCluster=76 fundingOiTrend=84 heatmap=91 finalScore=89",
      ),
    ).toBeInTheDocument();
  });

  it("clears only the local panel display", async () => {
    const user = userEvent.setup();
    fetchScanLogs.mockResolvedValueOnce([
      scanLog({ id: "3", message: "Signal scan snapshot contains 1 candidate(s)" }),
    ]);
    render(<ScanLogPanel />);

    expect(await screen.findByText("Signal scan snapshot contains 1 candidate(s)")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "清空显示" }));

    expect(screen.queryByText("Signal scan snapshot contains 1 candidate(s)")).not.toBeInTheDocument();
    expect(screen.getByText("暂无扫描日志")).toBeInTheDocument();
    expect(fetchScanLogs).toHaveBeenCalledTimes(1);
  });

  it("does not render forbidden fields or raw secret labels from pushed frames", async () => {
    render(<ScanLogPanel />);

    await waitFor(() => expect(wsMock.options?.onMessage).toBeTypeOf("function"));
    wsMock.options.onMessage({
      data: JSON.stringify({
        type: "scan_log_event",
        item: {
          ...scanLog({
            id: "4",
            message: "Authorization Bearer abc123 token rawPayload markout evidence",
          }),
          rawPayload: "forbidden-raw-payload-value",
          evidence: "forbidden-evidence-value",
          markout: "forbidden-markout-value",
          token: "forbidden-token-value",
          webhook: "forbidden-webhook-value",
          apiKey: "forbidden-api-key-value",
        },
      }),
    });

    expect(await screen.findByText(/\[redacted\]/)).toBeInTheDocument();
    for (const forbidden of [
      "abc123",
      "rawPayload",
      "markout",
      "evidence",
      "forbidden-raw-payload-value",
      "forbidden-evidence-value",
      "forbidden-markout-value",
      "forbidden-token-value",
      "forbidden-webhook-value",
      "forbidden-api-key-value",
    ]) {
      expect(screen.queryByText(forbidden)).not.toBeInTheDocument();
    }
  });

  it("shows reconnecting status", () => {
    wsMock.status = "reconnecting";

    render(<ScanLogPanel />);

    expect(screen.getByText("reconnecting")).toBeInTheDocument();
  });
});

function scanLog(overrides = {}) {
  return {
    id: "scan-1",
    ts: "2026-06-05T00:00:00.000Z",
    tsMs: 1_780_000_000_000,
    level: "info",
    kind: "scanner_started",
    message: "Market-data scanner started",
    symbol: "BTC-PERP",
    candidateId: "sig_001",
    ...overrides,
  };
}
