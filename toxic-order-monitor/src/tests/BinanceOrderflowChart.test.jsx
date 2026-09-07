import { describe, expect, it } from "vitest";
import { buildChartOption } from "../components/BinanceOrderflowChart.jsx";

const samples = (count) => Array.from({ length: count }, (_, index) => ({
  time: 1_788_750_000_000 + index * 60_000,
  open: 70_000 + index, close: 70_010 + index, low: 69_990 + index, high: 70_020 + index,
  deltaBase: 20, deltaPct: 10, vpin: 0.5,
}));

describe("orderflow chart observation viewport", () => {
  it.each([1, 24])("keeps actual candles inside the default view with only %i samples", (count) => {
    const option = buildChartOption(samples(count), true);
    const categories = option.xAxis[0].data.length;
    const firstVisibleIndex = Math.floor((categories - 1) * option.dataZoom[0].start / 100);
    expect(firstVisibleIndex).toBeLessThan(count);
    expect(categories).toBe(count + Math.floor(count * 0.4));
    expect(option.series[0].data).toHaveLength(count);
  });

  it("preserves the established long-history gutter and never invents series values", () => {
    const option = buildChartOption(samples(150), true);
    expect(option.xAxis[0].data).toHaveLength(210);
    expect(option.series[0].data).toHaveLength(150);
    expect(option.dataZoom[0]).toMatchObject({ start: 30, end: 90 });
    const empty = buildChartOption([], false);
    expect(empty.xAxis[0].data).toHaveLength(0);
    expect(empty.series[0].data).toHaveLength(0);
  });
});
