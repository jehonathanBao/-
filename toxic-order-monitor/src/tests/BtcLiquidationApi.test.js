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
        forceField: {
          symbol: "BTC",
          totalStress: 0.74,
          liquidityField: 0.66,
          nextMoveBias: "upward_squeeze",
          squeezeProbability: 0.69,
          predictedRegime: "compression",
        },
        liquidationHeatmap: [{ priceUsd: 62100, normalizedPrice: 1.001, riskScore: 0.72 }],
      },
    });

    const result = await fetchBtcLiquidationDashboard();

    expect(axios.get).toHaveBeenCalledWith("/api/btc-liquidation/dashboard");
    expect(result.dashboard.symbol).toBe("BTC");
    expect(result.dashboard.currentPriceUsd).toBe(62000);
    expect(result.dashboard.forceField.totalStress).toBe(0.74);
    expect(result.dashboard.forceField.nextMoveBias).toBe("upward_squeeze");
    expect(result.dashboard.liquidationHeatmap[0].riskScore).toBe(0.72);
    expect(result.dashboard.readOnly).toBe(true);
  });

  it("normalizes empty payloads safely", () => {
    const dashboard = normalizeBtcLiquidationDashboard(null);

    expect(dashboard.symbol).toBe("BTC");
    expect(dashboard.readOnly).toBe(true);
    expect(dashboard.forceField.symbol).toBe("BTC");
    expect(dashboard.forceField.totalStress).toBe(0);
    expect(dashboard.liquidationHeatmap).toEqual([]);
  });

  it("falls back to marketStress when forceField is absent", () => {
    const dashboard = normalizeBtcLiquidationDashboard({
      marketStress: {
        stressScore: 0.51,
        liquidityField: 0.44,
        instabilityIndex: 0.39,
        directionalBias: "sell",
        regime: "compression",
      },
    });

    expect(dashboard.forceField.totalStress).toBe(0.51);
    expect(dashboard.forceField.liquidityField).toBe(0.44);
    expect(dashboard.forceField.nextMoveBias).toBe("sell");
    expect(dashboard.forceField.predictedRegime).toBe("compression");
  });
});
