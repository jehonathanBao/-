import "@testing-library/jest-dom/vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import PriceChart, { normalizePricePoints } from "../components/PriceChart.jsx";

afterEach(cleanup);
const start = 1_788_000_000_000;
const points = Array.from({ length: 16 }, (_, index) => ({ time: start + index * 60_000, price: 60_000 + index * 10 }));

describe("presentation-only price chart", () => {
  it("filters invalid data, deduplicates timestamps, sorts and does not mutate its input", () => {
    const input = [points[2], { time: start, price: null }, { time: start + 1, price: -1 }, points[0], points[1], points[2]];
    const before = JSON.stringify(input);
    expect(normalizePricePoints(input)).toEqual(points.slice(0, 3));
    expect(JSON.stringify(input)).toBe(before);
    expect(normalizePricePoints(null)).toEqual([]);
  });

  it("shows a truthful empty state without inventing a chart or price", () => {
    render(<PriceChart points={[]} title="BTC 价格走势" symbol="BTC" description="Binance · 1H 收盘价" />);
    expect(screen.getByText("暂无价格样本")).toBeInTheDocument();
    expect(screen.queryByRole("img", { name: /价格曲线/ })).not.toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("lets operators select a bounded sample range and inspect with the keyboard", async () => {
    const user = userEvent.setup();
    render(<PriceChart points={points} title="BTC 价格走势" symbol="BTC" description="Binance · 1H 收盘价" />);
    expect(screen.getByText("16 个价格样本")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "最近 12 点" }));
    expect(screen.getByText("12 个价格样本")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "最近 12 点" })).toHaveAttribute("aria-pressed", "true");
    const chart = screen.getByRole("group", { name: /键盘左右键/ });
    fireEvent.keyDown(chart, { key: "Home" });
    expect(screen.getByTestId("chart-inspection")).toHaveTextContent("60,040.00");
    fireEvent.keyDown(chart, { key: "ArrowRight" });
    expect(screen.getByTestId("chart-inspection")).toHaveTextContent("60,050.00");
  });

  it("does not join sparse event observations with a fictitious continuous price line", () => {
    render(<PriceChart points={points.slice(0, 3)} title="事件价格轨迹" symbol="ETH" description="事件触发价 · 非连续行情" discrete />);
    expect(screen.getByText("事件触发价 · 非连续行情")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "ETH 事件价格散点图" })).toBeInTheDocument();
    expect(screen.queryByTestId("price-line")).not.toBeInTheDocument();
  });
});
