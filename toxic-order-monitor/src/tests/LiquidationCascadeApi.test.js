import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { fetchLiquidationCascade } from "../api/liquidationCascade.js";

vi.mock("axios", () => ({
  default: {
    get: vi.fn(),
  },
}));

describe("liquidation cascade api availability", () => {
  beforeEach(() => {
    axios.get.mockReset();
  });

  it("returns explicit unavailable data on transport failure instead of a calm synthetic state", async () => {
    axios.get.mockRejectedValueOnce(new Error("network down"));

    const result = await fetchLiquidationCascade("BTCUSDT");

    expect(result.data).toBeNull();
    expect(result.error).toBe("network down");
    expect(result.state).toMatchObject({ phase: "unavailable", source: null });
    expect(JSON.stringify(result)).not.toMatch(/CALM|NEUTRAL|ACCUMULATION/);
  });

  it("treats a malformed success response as unavailable", async () => {
    axios.get.mockResolvedValueOnce({ data: {} });

    const result = await fetchLiquidationCascade("BTCUSDT");

    expect(result.data).toBeNull();
    expect(result.error).toBe("MALFORMED_RESPONSE");
    expect(result.state.phase).toBe("unavailable");
  });

  it("retains the timestamp of the last successful observation after a later failure", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        symbol: "BTCUSDT",
        cascadeProbability: 0.42,
        status: "WARNING",
        direction: "DOWN",
        components: {},
      },
    });
    const success = await fetchLiquidationCascade("BTCUSDT");
    axios.get.mockRejectedValueOnce(new Error("offline"));

    const failed = await fetchLiquidationCascade("BTCUSDT");

    expect(success.state).toMatchObject({ phase: "ready", source: "backend" });
    expect(success.state.lastSuccessAtMs).toBeGreaterThan(0);
    expect(failed.data).toBeNull();
    expect(failed.state.lastSuccessAtMs).toBe(success.state.lastSuccessAtMs);
  });

  it("does not record a mismatched-symbol response as the last successful observation", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        symbol: "BTCUSDT",
        cascadeProbability: 0.95,
        status: "IMMINENT",
        direction: "DOWN",
        components: {},
      },
    });

    const mismatch = await fetchLiquidationCascade("SOLUSDT");
    axios.get.mockRejectedValueOnce(new Error("offline"));
    const failed = await fetchLiquidationCascade("SOLUSDT");

    expect(mismatch.data).toBeNull();
    expect(mismatch.error).toBe("SYMBOL_MISMATCH");
    expect(mismatch.state.lastSuccessAtMs).toBeNull();
    expect(failed.state.lastSuccessAtMs).toBeNull();
  });
});
