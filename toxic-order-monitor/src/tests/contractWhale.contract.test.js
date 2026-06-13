import { describe, expect, it } from "vitest";
import * as API from "../api/contractWhale.js";

describe("contractWhale API contract", () => {
  it("exports the price deviation threshold used by the monitor UI", () => {
    expect(API.CWM_MAX_PRICE_DEVIATION_PCT).toBeDefined();
    expect(typeof API.CWM_MAX_PRICE_DEVIATION_PCT).toBe("number");
  });
});
