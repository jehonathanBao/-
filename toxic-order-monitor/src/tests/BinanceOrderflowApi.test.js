import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { fetchBinanceOrderflow, normalizeBinanceOrderflow } from "../api/binanceOrderflow.js";

vi.mock("axios", () => ({
  default: {
    get: vi.fn(),
  },
}));

describe("Binance orderflow API", () => {
  beforeEach(() => axios.get.mockReset());

  it("normalizes buy, sell and delta fields for a candle", () => {
    const payload = normalizeBinanceOrderflow({
      symbol: "btcusdt",
      interval: "1h",
      candles: [{ buyQuote: 600, sellQuote: 400, deltaQuote: 200, deltaPct: 20, closed: true }],
    });

    expect(payload.symbol).toBe("BTCUSDT");
    expect(payload.candles[0]).toMatchObject({ buyQuote: 600, sellQuote: 400, deltaQuote: 200, deltaPct: 20, closed: true });
  });

  it("requests the selected Binance symbol and interval", async () => {
    axios.get.mockResolvedValueOnce({ data: { symbol: "BTCUSDT", interval: "5m", candles: [] } });

    await fetchBinanceOrderflow({ symbol: "BTCUSDT", interval: "5m", limit: 100 });

    expect(axios.get).toHaveBeenCalledWith("/api/binance/orderflow", {
      params: { symbol: "BTCUSDT", interval: "5m", limit: 100 },
    });
  });
});
