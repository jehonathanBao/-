import { BarChart, PieChart } from "echarts/charts";
import { GridComponent, LegendComponent, TooltipComponent } from "echarts/components";
import { init, use } from "echarts/core";
import { CanvasRenderer } from "echarts/renderers";
import { useEffect, useMemo, useRef } from "react";

use([BarChart, CanvasRenderer, GridComponent, LegendComponent, PieChart, TooltipComponent]);

export default function RiskCharts({ signals }) {
  const pieRef = useRef(null);
  const barRef = useRef(null);
  const stats = useMemo(() => {
    const levels = { S: 0, A: 0, B: 0, C: 0, D: 0 };
    const buckets = { "0-39": 0, "40-59": 0, "60-79": 0, "80-100": 0 };
    signals.forEach((signal) => {
      if (levels[signal.level] !== undefined) {
        levels[signal.level] += 1;
      }
      if (signal.score >= 80) buckets["80-100"] += 1;
      else if (signal.score >= 60) buckets["60-79"] += 1;
      else if (signal.score >= 40) buckets["40-59"] += 1;
      else buckets["0-39"] += 1;
    });
    return { levels, buckets };
  }, [signals]);

  useEffect(() => {
    const chart = init(pieRef.current);
    chart.setOption({
      backgroundColor: "transparent",
      color: ["#ef4444", "#f97316", "#facc15", "#64748b", "#38bdf8"],
      tooltip: { trigger: "item" },
      series: [
        {
          type: "pie",
          radius: ["48%", "72%"],
          label: { color: "#cbd5e1" },
          data: Object.entries(stats.levels).map(([name, value]) => ({ name, value })),
        },
      ],
    });
    const resize = () => chart.resize();
    window.addEventListener("resize", resize);
    return () => {
      window.removeEventListener("resize", resize);
      chart.dispose();
    };
  }, [stats.levels]);

  useEffect(() => {
    const chart = init(barRef.current);
    chart.setOption({
      backgroundColor: "transparent",
      color: ["#38bdf8"],
      grid: { top: 20, right: 12, bottom: 28, left: 32 },
      tooltip: { trigger: "axis" },
      xAxis: {
        type: "category",
        data: Object.keys(stats.buckets),
        axisLabel: { color: "#94a3b8" },
        axisLine: { lineStyle: { color: "#334155" } },
      },
      yAxis: {
        type: "value",
        axisLabel: { color: "#94a3b8" },
        splitLine: { lineStyle: { color: "#1e293b" } },
      },
      series: [{ type: "bar", data: Object.values(stats.buckets), barWidth: 22 }],
    });
    const resize = () => chart.resize();
    window.addEventListener("resize", resize);
    return () => {
      window.removeEventListener("resize", resize);
      chart.dispose();
    };
  }, [stats.buckets]);

  return (
    <section aria-label="风险统计图表" className="rounded-2xl border border-slate-700/60 bg-slate-900/80 p-5 shadow-glow">
      <h3 className="font-bold text-white">风险图表</h3>
      <div className="mt-4 space-y-4">
        <div>
          <p className="mb-2 text-xs text-slate-400">风险等级分布</p>
          <div aria-label="风险等级分布环形图" className="h-56" ref={pieRef} role="img" />
        </div>
        <div>
          <p className="mb-2 text-xs text-slate-400">风险评分分布</p>
          <div aria-label="风险评分分布柱状图" className="h-52" ref={barRef} role="img" />
        </div>
      </div>
    </section>
  );
}
