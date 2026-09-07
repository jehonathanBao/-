import "@testing-library/jest-dom/vitest";
import React from "react";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import MarketObservatory from "../components/observatory/MarketObservatory.jsx";
import { buildRidgePath, normalizeFlowSamples, normalizeObservations } from "../components/observatory/model.js";

const time = Date.UTC(2026, 8, 6);
const samples = [{ time, buyBase: 240, sellBase: 150 }, { time: time + 3600000, buyBase: 125, sellBase: 306 }];
const observations = [{ id: "a", symbol: "BTC", ts: time, direction: "buy", mainExchange: "binance", impactLevel: "A", score: 87, dataQuality: 91 }, { id: "b", symbol: "BTC", ts: time + 1000, direction: "sell", mainExchange: "bitfinex", impactLevel: "C", score: 41, dataQuality: 83 }];
afterEach(() => { cleanup(); vi.restoreAllMocks(); delete document.documentElement.dataset.motion; });

describe("observatory presentation", () => {
  it("rejects missing, invalid, zero fallback and other-asset flow; derives only valid total/net pairs", () => {
    const result = normalizeFlowSamples([...samples,
      { time: time + 1, buyBase: null, sellBase: 22 },
      { time: time + 2, buyBase: 0, sellBase: 0 },
      { time: Infinity, buyBase: 30, sellBase: 20 },
      { time: time + 3, buyBase: -1, sellBase: 30 },
      { time: time + 4, buyBase: 10, sellBase: 20, symbol: "ETH" },
      { id: "valid", ts: time + 5, totalVolumeBtc: 180, netVolumeBtc: -40 },
      { id: "invalid", ts: time + 6, totalVolumeBtc: 10, netVolumeBtc: -40 },
    ]);
    expect(result).toHaveLength(3);
    expect(result.find(row => row.id === "valid")).toMatchObject({ buy: 70, sell: 110, delta: -40 });
    expect(buildRidgePath(result[0], 0, result.length, 306)).not.toMatch(/NaN|Infinity/);
  });
  it("bounds samples, deduplicates events, and preserves the canonical grade", () => {
    const data = normalizeObservations([...observations, { ...observations[1], cohortImpactLevel: "S" }, { ...observations[0], id: "eth", symbol: "ETH" }], "BTC");
    expect(data).toHaveLength(2);
    expect(data[1].grade).toBe("C");
    expect(normalizeFlowSamples(Array.from({ length: 100 }, (_, i) => ({ time: time + i, buyBase: i + 1, sellBase: 2 })))).toHaveLength(36);
  });
  it("shows missing data honestly without synthetic readings", () => {
    const { container } = render(<MarketObservatory />);
    expect(screen.getByText("等待成交数据")).toBeInTheDocument();
    expect(screen.getByLabelText("选择成交地形样本")).toBeDisabled();
    expect(screen.queryByText("0%")).not.toBeInTheDocument();
    expect(screen.getByRole("img", { name: "方向关系图，0 个已加载事件" })).toBeInTheDocument();
    expect(container.querySelector(".ridge-ribbon.is-selected")).toBeNull();
  });
  it("keeps every bounded venue group including unknown sources", () => {
    render(<MarketObservatory observations={["binance", "bitfinex", "coinbase", "okx", "bybit", "unknown"].map((mainExchange, index) => ({ ...observations[0], id: `venue-${index}`, mainExchange }))} />);
    expect(screen.getByText("UNKNOWN")).toBeInTheDocument();
    expect(screen.getByText("来源数量").nextElementSibling).toHaveTextContent("6");
  });
  it("supports keyboard sample inspection, filtering and freezing without changing event values", async () => {
    const user = userEvent.setup();
    render(<MarketObservatory samples={samples} observations={observations} />);
    expect(screen.getByText("-181 BTC")).toBeInTheDocument();
    // Range input change uses the same standard event as keyboard/browser interaction.
    const { fireEvent } = await import("@testing-library/react");
    fireEvent.change(screen.getByLabelText("选择成交地形样本"), { target: { value: "0" } });
    expect(screen.getByText("+90 BTC")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "买方", exact: true }));
    expect(screen.getByRole("img", { name: "方向关系图，1 个已加载事件" })).toBeInTheDocument();
    expect(screen.getByText("41")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "冻结视角" }));
    expect(screen.getByRole("button", { name: "继续旋转" })).toHaveAttribute("aria-pressed", "true");
  });
  it("cancels camera frames for the global motion switch and on unmount", async () => {
    const request = vi.spyOn(window, "requestAnimationFrame").mockReturnValue(71);
    const cancel = vi.spyOn(window, "cancelAnimationFrame");
    const { unmount } = render(<MarketObservatory observations={observations} />);
    expect(request).toHaveBeenCalled();
    request.mockClear();
    document.documentElement.dataset.motion = "off";
    await waitFor(() => expect(cancel).toHaveBeenCalledWith(71));
    expect(request).not.toHaveBeenCalled();
    unmount();
    expect(cancel).toHaveBeenCalled();
  });
});
