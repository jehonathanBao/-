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

  it("normalizes stale latest diagnostics from the summary", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        enabled: true,
        symbol: "BTC",
        latestAgeSec: 720,
        latestIsStale: true,
        latestStaleReason: "older_than_latest_ttl",
      },
    });

    const result = await fetchSpotWhaleSummary("BTC");

    expect(result.summary.latestAgeSec).toBe(720);
    expect(result.summary.latestIsStale).toBe(true);
    expect(result.summary.latestStaleReason).toBe("older_than_latest_ttl");
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

  it("drops latest rows whose explicit symbol does not match the requested symbol", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: { enabled: true, symbol: "ETH" },
        items: [
          { id: "btc-row", symbol: "BTC", totalVolumeBase: 10 },
          { id: "eth-row", symbol: "ETH", totalVolumeBase: 20 },
          { id: "sol-row", symbol: "SOL", totalVolumeBase: 30 },
        ],
      },
    });

    const result = await fetchSpotWhaleLatest(50, "ETH");

    expect(axios.get).toHaveBeenCalledWith("/api/spot-whale/latest?limit=50&symbol=ETH");
    expect(result.items.map((item) => item.id)).toEqual(["eth-row"]);
    expect(result.items[0].symbol).toBe("ETH");
  });

  it("uses the requested symbol for legacy rows when the backend omits symbol", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: { enabled: true },
        items: [
          { id: undefined, windowSec: 15, ts: 1_700_000_000_000, totalVolumeBase: 12 },
        ],
      },
    });

    const result = await fetchSpotWhaleLatest(50, "ETH");

    expect(result.summary.symbol).toBe("ETH");
    expect(result.items[0]).toMatchObject({
      id: "ETH-15-1700000000000",
      symbol: "ETH",
      totalVolumeBase: 12,
    });
  });

  it("normalizes missing fields safely", () => {
    const signal = normalizeSpotWhaleSignal({ symbol: "ETH" });

    expect(signal.symbol).toBe("ETH");
    expect(signal.totalVolumeBase).toBe(0);
    expect(signal.discordSent).toBe(false);
    expect(signal.isPermanent).toBe(false);
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

  it("passes abs50 and abs100 net direction filters to history endpoint", async () => {
    axios.get.mockResolvedValue({
      data: {
        summary: { enabled: true, symbol: "BTC" },
        items: [],
      },
    });

    await fetchSpotWhaleHistory({ symbol: "BTC", net_direction: "abs50", limit: 50 });
    await fetchSpotWhaleHistory({ symbol: "BTC", net_direction: "abs100", limit: 50 });

    expect(axios.get.mock.calls[0][0]).toContain("net_direction=abs50");
    expect(axios.get.mock.calls[1][0]).toContain("net_direction=abs100");
  });

  it("passes paging, time-range, and permanent filters to history endpoint", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: { enabled: true, symbol: "BTC" },
        items: [],
        limit: 50,
        offset: 50,
        total: 120,
        hasMore: true,
      },
    });

    const result = await fetchSpotWhaleHistory({
      symbol: "BTC",
      net_direction: "abs50",
      limit: 50,
      offset: 50,
      from_ts: 1_717_776_000_000,
      to_ts: 1_717_862_400_000,
      permanent_only: true,
    });

    const url = axios.get.mock.calls[0][0];
    expect(url).toContain("offset=50");
    expect(url).toContain("from_ts=1717776000000");
    expect(url).toContain("to_ts=1717862400000");
    expect(url).toContain("permanent_only=true");
    expect(result.offset).toBe(50);
    expect(result.total).toBe(120);
    expect(result.hasMore).toBe(true);
  });

  it("passes a stable composite cursor to the history endpoint", async () => {
    axios.get.mockResolvedValueOnce({
      data: { summary: { enabled: true, symbol: "BTC" }, items: [] },
    });

    await fetchSpotWhaleHistory({ symbol: "BTC", cursor: "stable-cursor", limit: 50 });

    expect(axios.get.mock.calls[0][0]).toContain("cursor=stable-cursor");
  });

  it("normalizes permanent flags from history results", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: { enabled: true, symbol: "BTC" },
        items: [
          {
            id: "spot-whale:BTC:15:2:sell",
            symbol: "BTC",
            netVolumeBase: -70,
            isPermanent: true,
          },
        ],
        limit: 50,
        offset: 0,
        total: 1,
        hasMore: false,
      },
    });

    const result = await fetchSpotWhaleHistory({ symbol: "BTC", permanent_only: true, limit: 50 });

    expect(result.items[0].isPermanent).toBe(true);
    expect(result.hasMore).toBe(false);
  });
});
