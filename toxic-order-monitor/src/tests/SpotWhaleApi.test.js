import { afterEach, describe, expect, it, vi } from "vitest";
import axios from "axios";
import {
  fetchSpotWhaleLatest,
  fetchSpotWhaleSummary,
  normalizeSpotWhaleSignal,
} from "../api/spotWhale.js";

vi.mock("axios");

describe("spotWhale API", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("requests summary with selected symbol", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        status: "strong",
        healthStatus: "healthy",
        latestSeverity: "critical",
        enabled: true,
        dryRun: false,
        symbol: "ETH",
        exchanges: {
          binance: { connected: true, status: "connected" },
          coinbase: { connected: true, status: "connected" },
        },
      },
    });

    const result = await fetchSpotWhaleSummary("ETH");

    expect(axios.get).toHaveBeenCalledWith("/api/spot-whale/summary?symbol=ETH");
    expect(result.summary.symbol).toBe("ETH");
    expect(result.summary.enabled).toBe(true);
  });

  it("maps latest spot whale signal fields", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: { enabled: true, symbol: "BTC" },
        items: [
          {
            id: "spot-whale:BTC:15:1:buy",
            symbol: "BTC",
            signalType: "spot_aggressive_buy",
            severity: "critical",
            totalVolumeBase: 820,
            coinbasePremiumPct: 0.04,
            discordEligible: true,
            exchanges: [{ exchange: "coinbase", buyVolumeBase: 320 }],
          },
        ],
      },
    });

    const result = await fetchSpotWhaleLatest(50, "BTC");

    expect(axios.get).toHaveBeenCalledWith("/api/spot-whale/latest?limit=50&symbol=BTC");
    expect(result.items[0].totalVolumeBase).toBe(820);
    expect(result.items[0].coinbasePremiumPct).toBe(0.04);
    expect(result.items[0].exchanges[0].exchange).toBe("coinbase");
  });

  it("normalizes missing fields safely", () => {
    const signal = normalizeSpotWhaleSignal({ symbol: "ETH" });

    expect(signal.symbol).toBe("ETH");
    expect(signal.totalVolumeBase).toBe(0);
    expect(signal.discordSent).toBe(false);
  });
});
