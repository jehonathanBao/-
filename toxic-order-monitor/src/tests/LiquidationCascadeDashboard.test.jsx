import "@testing-library/jest-dom/vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import React from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchBtcStructure,
  fetchLiquidationCascade,
  fetchLiquidationLeverageMap,
  fetchLiquidationLiquidityGap,
  fetchMarketRegime,
} from "../api/liquidationCascade.js";
import LiquidationCascadeDashboard from "../components/LiquidationCascadeDashboard.jsx";

vi.mock("../api/liquidationCascade.js", () => ({
  fetchLiquidationCascade: vi.fn(() => Promise.resolve(unavailableResult())),
  fetchLiquidationLeverageMap: vi.fn(() => Promise.resolve(unavailableResult())),
  fetchLiquidationLiquidityGap: vi.fn(() => Promise.resolve(unavailableResult())),
  fetchBtcStructure: vi.fn(() => Promise.resolve(unavailableResult())),
  fetchMarketRegime: vi.fn(() => Promise.resolve(unavailableResult())),
}));

describe("LiquidationCascadeDashboard unavailable state", () => {
  beforeEach(() => {
    for (const request of allRequests()) {
      request.mockReset();
      request.mockResolvedValue(unavailableResult());
    }
  });

  afterEach(() => cleanup());

  it("renders unavailable without fake calm, neutral, accumulation, or zero values", async () => {
    render(<LiquidationCascadeDashboard />);

    expect(screen.getByText("流动性簇风险代理 · 非真实清算源 · 不参与 Discord")).toBeInTheDocument();
    expect(await screen.findByText(/数据源不可用/)).toBeInTheDocument();
    expect(screen.getAllByText("不可用").length).toBeGreaterThan(0);
    expect(screen.queryByText("CALM")).not.toBeInTheDocument();
    expect(screen.queryByText("中性")).not.toBeInTheDocument();
    expect(screen.queryByText("ACCUMULATION")).not.toBeInTheDocument();
    expect(screen.queryByText("0%" )).not.toBeInTheDocument();
  });

  it("does not let a late BTC request overwrite a newer ETH result", async () => {
    const btc = deferred();
    const eth = deferred();
    mockRequestsBySymbol({ BTCUSDT: btc.promise, ETHUSDT: eth.promise });

    render(<LiquidationCascadeDashboard />);
    fireEvent.change(screen.getByRole("combobox", { name: /Symbol/i }), {
      target: { value: "ETHUSDT" },
    });

    await act(async () => {
      eth.resolve();
      await eth.promise;
    });
    expect(await screen.findByText("IMMINENT")).toBeInTheDocument();

    await act(async () => {
      btc.resolve();
      await btc.promise;
    });
    await waitFor(() => expect(screen.queryByText("ACTIVE")).not.toBeInTheDocument());
    expect(screen.getByText("IMMINENT")).toBeInTheDocument();
  });

  it("clears already-rendered BTC metrics while the new ETH request is pending", async () => {
    const eth = deferred();
    mockRequestsBySymbol({ BTCUSDT: Promise.resolve(), ETHUSDT: eth.promise });

    render(<LiquidationCascadeDashboard />);
    expect(await screen.findByText("ACTIVE")).toBeInTheDocument();

    fireEvent.change(screen.getByRole("combobox", { name: /Symbol/i }), {
      target: { value: "ETHUSDT" },
    });

    expect(screen.queryByText("ACTIVE")).not.toBeInTheDocument();
    expect(screen.queryByText("99%")).not.toBeInTheDocument();
    expect(screen.getByText("LOADING")).toBeInTheDocument();
  });

  it("marks a response unavailable when its symbol does not match the active request", async () => {
    const never = new Promise(() => {});
    mockRequestsBySymbol({ BTCUSDT: never, ETHUSDT: Promise.resolve() }, {
      cascadeSymbolByRequest: { ETHUSDT: "BTC" },
    });

    render(<LiquidationCascadeDashboard />);
    fireEvent.change(screen.getByRole("combobox", { name: /Symbol/i }), {
      target: { value: "ETHUSDT" },
    });

    expect(await screen.findByText(/SYMBOL_MISMATCH/)).toBeInTheDocument();
    expect(screen.queryByText("IMMINENT")).not.toBeInTheDocument();
    expect(screen.queryByText("95%")).not.toBeInTheDocument();
  });
});

function unavailableResult() {
  return {
    data: null,
    error: "NETWORK_ERROR",
    state: { phase: "unavailable", source: null, lastSuccessAtMs: null },
  };
}

function allRequests() {
  return [
    fetchLiquidationCascade,
    fetchLiquidationLeverageMap,
    fetchLiquidationLiquidityGap,
    fetchBtcStructure,
    fetchMarketRegime,
  ];
}

function deferred() {
  let resolve;
  const promise = new Promise((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function mockRequestsBySymbol(promises, options = {}) {
  const waitForRequest = (symbol) => promises[symbol] || Promise.resolve();
  fetchLiquidationCascade.mockImplementation((symbol) =>
    waitForRequest(symbol).then(() => readyResult({
      symbol: options.cascadeSymbolByRequest?.[symbol] || baseSymbol(symbol),
      cascadeProbability: symbol === "ETHUSDT" ? 0.95 : 0.99,
      status: symbol === "ETHUSDT" ? "IMMINENT" : "ACTIVE",
      direction: "UP",
      components: {},
    })),
  );
  fetchLiquidationLeverageMap.mockImplementation((symbol) =>
    waitForRequest(symbol).then(() => readyResult({
      symbol: baseSymbol(symbol),
      heatmap: [],
      highRiskZones: [],
    })),
  );
  fetchLiquidationLiquidityGap.mockImplementation((symbol) =>
    waitForRequest(symbol).then(() => readyResult({
      symbol: baseSymbol(symbol),
      belowPrice: 0.2,
      abovePrice: 0.3,
      dominantGap: "UP",
      signals: [],
    })),
  );
  fetchBtcStructure.mockImplementation((symbol) =>
    waitForRequest(symbol).then(() => readyResult({
      symbol: baseSymbol(symbol),
      regime: "EXPANSION",
      bias: "UP",
      confidence: 0.8,
      signals: [],
    })),
  );
  fetchMarketRegime.mockImplementation((symbol) =>
    waitForRequest(symbol).then(() => readyResult({
      symbol: baseSymbol(symbol),
      regime: "EXPANSION",
      directionBias: "UP",
      confidence: 0.8,
      signals: [],
    })),
  );
}

function readyResult(data) {
  return {
    data,
    error: null,
    state: { phase: "ready", source: "backend", lastSuccessAtMs: 1 },
  };
}

function baseSymbol(symbol) {
  return symbol.startsWith("ETH") ? "ETH" : "BTC";
}
