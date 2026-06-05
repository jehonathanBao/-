import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { evaluateDiscordAlertGate } from "../api/alertGate.js";
import { fetchSignals, mapInboxItemToSignal } from "../api/signals.js";

vi.mock("axios", () => ({
  default: {
    get: vi.fn(),
  },
}));

describe("signals api mapping", () => {
  beforeEach(() => {
    axios.get.mockReset();
    vi.stubEnv("VITE_API_BASE_URL", "");
  });

  it("maps backend inbox items into persistent inbox signal shape", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [
          inboxItem({ signalId: "runtime-high", severity: "high", qualityBucket: "good" }),
          inboxItem({ signalId: "runtime-medium", severity: "medium", qualityBucket: "mixed" }),
        ],
      },
    });

    const signals = await fetchSignals();

    expect(axios.get).toHaveBeenCalledWith("/api/toxicity/signal-inbox/recent");
    expect(signals.map((signal) => signal.id)).toEqual(["runtime-high", "runtime-medium"]);
    expect(signals[0]).toMatchObject({
      risk: "high",
      level: "A",
      score: 85,
      dataQuality: 82,
      status: "unhandled",
    });
    expect(signals[1]).toMatchObject({
      risk: "medium",
      level: "B",
      score: 72,
      dataQuality: 74,
    });
    expect(evaluateDiscordAlertGate(signals[0])).toEqual({ ok: true, reason: null });
    expect(evaluateDiscordAlertGate(signals[1])).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_NON_HIGH_RISK",
    });
  });

  it("keeps final result to direction plus core reason", () => {
    const signal = mapInboxItemToSignal(
      inboxItem({
        signalId: "runtime-short",
        directionBias: "short_bias",
        fusionSummary: "large ask wall removed",
      }),
    );

    expect(signal.finalResult).toBe("Ask/Sell · large ask wall removed");
  });

  it("maps redacted websocket signal summaries without technical fields", () => {
    const signal = mapInboxItemToSignal({
      signalId: "ws-high",
      symbol: "BTC-PERP",
      signalKind: "spoofing_candidate",
      directionBias: "short_bias",
      severity: "high",
      confidence: 0.82,
      createdAtMs: 1_700_000_000_000,
      finalResult: "Ask/Sell · large ask wall removed",
      riskScore: 88,
      dataQuality: 81,
      qualityBucket: "good",
      readOnly: true,
      runtimeModified: false,
      analysisOnly: true,
      executionEnabled: false,
    });

    expect(signal).toMatchObject({
      id: "ws-high",
      risk: "high",
      score: 88,
      dataQuality: 81,
      finalResult: "Ask/Sell · large ask wall removed",
    });
    expect(signal.markout).toBeUndefined();
    expect(signal.evidence).toBeUndefined();
  });

  it("does not inject mock data when backend inbox is reachable but empty", async () => {
    axios.get.mockResolvedValueOnce({ data: { items: [] } });

    await expect(fetchSignals()).resolves.toEqual([]);
  });
});

function inboxItem({
  signalId = "runtime-signal",
  severity = "high",
  qualityBucket = "good",
  directionBias = "short_bias",
  fusionSummary = "runtime candidate",
} = {}) {
  return {
    signalId,
    symbol: "BTC-PERP",
    signalKind: "spoofing_candidate",
    directionBias,
    severity,
    confidence: 0.82,
    createdAtMs: 1_700_000_000_000,
    fusion: { available: true, summary: fusionSummary },
    quality: { available: true, qualityBucket },
    recommendation: { action: "review_evidence" },
    readOnly: true,
    runtimeModified: false,
    analysisOnly: true,
    executionEnabled: false,
  };
}
