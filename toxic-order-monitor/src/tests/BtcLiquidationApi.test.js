import { afterEach, describe, expect, it, vi } from "vitest";
import axios from "axios";
import {
  fetchBtcLiquidationDashboard,
  normalizeBtcLiquidationDashboard,
} from "../api/btcLiquidation.js";

vi.mock("axios");

describe("btcLiquidation API", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("requests the read-only BTC liquidation dashboard", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        symbol: "BTC",
        currentPriceUsd: 62000,
        live: true,
        readOnly: true,
        liquidationHeatmap: [{ priceUsd: 62100, normalizedPrice: 1.001, riskScore: 0.72 }],
      },
    });

    const result = await fetchBtcLiquidationDashboard();

    expect(axios.get).toHaveBeenCalledWith("/api/btc-liquidation/dashboard");
    expect(result.dashboard.symbol).toBe("BTC");
    expect(result.dashboard.currentPriceUsd).toBe(62000);
    expect(result.dashboard.liquidationHeatmap[0].riskScore).toBe(0.72);
    expect(result.dashboard.readOnly).toBe(true);
  });

  it("normalizes empty payloads safely", () => {
    const dashboard = normalizeBtcLiquidationDashboard(null);

    expect(dashboard.symbol).toBe("BTC");
    expect(dashboard.readOnly).toBe(true);
    expect(dashboard.liquidationHeatmap).toEqual([]);
  });
});
