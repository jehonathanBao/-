import axios from "axios";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { pushDiscordAlert, sendDiscordTestMessage } from "../api/discord.js";

vi.mock("axios", () => ({
  default: {
    post: vi.fn(),
  },
}));

describe("pushDiscordAlert", () => {
  beforeEach(() => {
    axios.post.mockReset();
    vi.stubEnv("VITE_API_BASE_URL", "");
  });

  afterEach(() => {
    vi.unstubAllEnvs();
  });

  it("returns a safe failure response when Discord is not configured", async () => {
    axios.post.mockResolvedValueOnce({
      data: { ok: false, reason: "DISCORD_NOT_CONFIGURED" },
    });

    const result = await pushDiscordAlert("sig_001");

    expect(axios.post).toHaveBeenCalledWith("/api/discord/push", {
      signalId: "sig_001",
      alertFamily: "short_toxic_order",
    });
    expect(result).toEqual({ ok: false, reason: "DISCORD_NOT_CONFIGURED" });
  });

  it("sends score, data quality and final result for backend alert gate checks", async () => {
    axios.post.mockResolvedValueOnce({
      data: { ok: true, reason: "DISCORD_WEBHOOK_SENT" },
    });

    await pushDiscordAlert({
      id: "sig_001",
      dedupeKey: "binance:BTCUSDT:spoofing",
      exchange: "Binance",
      symbol: "BTCUSDT",
      type: "SpoofingCandidate",
      level: "S",
      side: "Ask/Sell",
      score: 92,
      dataQuality: 88,
      reason: "suspected candidate",
      impact: "candidate only",
      time: "2025-01-01 00:00:00",
    });

    expect(axios.post).toHaveBeenCalledWith("/api/discord/push", {
      alertFamily: "short_toxic_order",
      signalId: "sig_001",
      dedupeKey: "binance:BTCUSDT:spoofing",
      exchange: "Binance",
      symbol: "BTCUSDT",
      signalType: "SpoofingCandidate",
      level: "S",
      side: "Ask/Sell",
      score: 92,
      dataQuality: 88,
      reason: "卖方挂单诱导，潜在下行压力",
      time: "2025-01-01 00:00:00",
    });
  });

  it("passes only short-term toxic summary fields without raw evidence", async () => {
    axios.post.mockResolvedValueOnce({
      data: { ok: true, reason: "DISCORD_WEBHOOK_SENT" },
    });

    await pushDiscordAlert({
      id: "sig_tof",
      symbol: "BTC-PERP",
      type: "spoofing_candidate",
      level: "S",
      side: "Ask/Sell",
      score: 94,
      dataQuality: 90,
      reason: "core reason",
      tofScore: 88.4,
      candidateType: "spoofing_candidate",
      explainTags: ["high_vpin_proxy", "bid_depth_withdrawal"],
      directionConfidence: 84.1,
      tofMetrics: {
        tradeImbalance: -0.43,
        vpinProxy: 89,
        bidDepthWithdrawal: 58,
      },
      advancedTofMetrics: {
        vpinEnhanced: 88,
        largeOrderFlowCluster: 76,
        historicalFundingOiTrend: 84,
        marketPressureHeatmap: 91,
        finalRiskScore: 89,
      },
      evidence: { raw: "must not be sent" },
      markout: { p50: -12 },
    });

    expect(axios.post).toHaveBeenCalledWith(
      "/api/discord/push",
      expect.objectContaining({
        alertFamily: "short_toxic_order",
        signalId: "sig_tof",
        tofScore: 88.4,
        candidateType: "spoofing_candidate",
        explainTags: ["high_vpin_proxy", "bid_depth_withdrawal"],
        directionConfidence: 84.1,
        tofMetrics: expect.objectContaining({ vpinProxy: 89 }),
      }),
    );
    expect(axios.post.mock.calls[0][1]).not.toHaveProperty("evidence");
    expect(axios.post.mock.calls[0][1]).not.toHaveProperty("markout");
    expect(axios.post.mock.calls[0][1]).not.toHaveProperty("perpTofMetrics");
    expect(axios.post.mock.calls[0][1]).not.toHaveProperty("advancedTofMetrics");
    expect(axios.post.mock.calls[0][1]).not.toHaveProperty("advancedScore");
    expect(axios.post.mock.calls[0][1]).not.toHaveProperty("advancedCandidateType");
  });

  it("switches to market-structure family for main-force alerts", async () => {
    axios.post.mockResolvedValueOnce({
      data: { ok: true, reason: "DISCORD_WEBHOOK_SENT" },
    });

    await pushDiscordAlert({
      id: "sig_ms",
      symbol: "BTC",
      type: "market_structure",
      level: "Major",
      side: "Bid/Buy",
      mainForceScore: 84,
      marketStructureConfidence: 76,
      marketStructureDataQuality: 74,
      extremeImpactScore: 58,
      structureBias: 62,
      regimeType: "main_force_long_build",
      marketStructureSeverity: "Major",
      spotScore: 71,
      contractScore: 86,
      crossConfirmScore: 74,
      reason: "高概率主力建多",
    });

    expect(axios.post).toHaveBeenCalledWith(
      "/api/discord/push",
      expect.objectContaining({
        alertFamily: "market_structure",
        mainForceScore: 84,
        marketStructureConfidence: 76,
        marketStructureDataQuality: 74,
        extremeImpactScore: 58,
        structureBias: 62,
        regimeType: "main_force_long_build",
        spotScore: 71,
        contractScore: 86,
        crossConfirmScore: 74,
      }),
    );
    expect(axios.post.mock.calls[0][1]).not.toHaveProperty("perpTofMetrics");
  });

  it("sends isolated test messages without signal fields", async () => {
    axios.post.mockResolvedValueOnce({
      data: { ok: true, reason: "DISCORD_WEBHOOK_SENT" },
    });

    const result = await sendDiscordTestMessage();

    expect(axios.post).toHaveBeenCalledWith("/api/discord/push", { test: true });
    expect(axios.post.mock.calls[0][1]).not.toHaveProperty("signalId");
    expect(axios.post.mock.calls[0][1]).not.toHaveProperty("dedupeKey");
    expect(result).toEqual({ ok: true, reason: "DISCORD_WEBHOOK_SENT" });
  });
});
