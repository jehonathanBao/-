import { afterEach, describe, expect, it, vi } from "vitest";
import axios from "axios";
import {
  fetchSpotWhaleHistory,
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
          bitfinex: { connected: true, status: "connected" },
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
            totalNotionalUsd: 51_250_000,
            coinbasePremiumPct: 0.04,
            discordEligible: true,
            exchanges: [
              { exchange: "coinbase", buyVolumeBase: 320 },
              { exchange: "bitfinex", buyVolumeBase: 40 },
            ],
          },
        ],
      },
    });

    const result = await fetchSpotWhaleLatest(50, "BTC");

    expect(axios.get).toHaveBeenCalledWith("/api/spot-whale/latest?limit=50&symbol=BTC");
    expect(result.items[0].totalVolumeBase).toBe(820);
    expect(result.items[0].triggerPriceUsd).toBe(62_500);
    expect(result.items[0].coinbasePremiumPct).toBe(0.04);
    expect(result.items[0].exchanges[0].exchange).toBe("coinbase");
    expect(result.items[0].exchanges[1].exchange).toBe("bitfinex");
  });

  it("normalizes missing fields safely", () => {
    const signal = normalizeSpotWhaleSignal({ symbol: "ETH" });

    expect(signal.symbol).toBe("ETH");
    expect(signal.totalVolumeBase).toBe(0);
    expect(signal.discordSent).toBe(false);
  });

  it("passes absolute net direction filter to history endpoint", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: { enabled: true, symbol: "BTC" },
        items: [],
      },
    });

    await fetchSpotWhaleHistory({ symbol: "BTC", net_direction: "abs500", limit: 50 });

    const url = axios.get.mock.calls[0][0];
    expect(url).toContain("/api/spot-whale/history?");
    expect(url).toContain("symbol=BTC");
    expect(url).toContain("net_direction=abs500");
    expect(url).toContain("limit=50");
  });
});
