import MetricLineageBadge from "./MetricLineageBadge.jsx";

export default function TofMetricsPanel({ metrics, compact = false }) {
  if (!metrics) {
    return null;
  }

  const rows = [
    ["TOF Score", metrics.tofScore, "hazard"],
    ["VPIN Z-Score", metrics.vpinZscore, "vpin"],
    ["VPIN Percentile", metrics.vpinPercentile, "vpin"],
    ["VPIN Proxy", metrics.vpinProxy, "vpin"],
    ["成交失衡", metrics.tradeImbalance, "tradeImbalance"],
    ["Bid 撤出", metrics.bidDepthWithdrawal, "depth"],
    ["Ask 撤出", metrics.askDepthWithdrawal, "depth"],
    ["价差 bps", metrics.spreadBps, "spread"],
    ["方向置信", metrics.metricsConfidence, "hazard"],
  ];

  return (
    <div className={compact ? "mt-3" : ""}>
      <MetricLineageBadge lineage={metrics.lineage} />
      <div className={compact ? "grid grid-cols-2 gap-2 text-xs md:grid-cols-4" : "grid gap-3 md:grid-cols-3"}>
        {rows.map(([label, value, lineageKey]) => (
          <MetricCell
            compact={compact}
            key={label}
            label={label}
            lineage={metrics.metricLineage?.[lineageKey]}
            value={formatMetric(label, value)}
          />
        ))}
      </div>
    </div>
  );
}

function MetricCell({ label, value, lineage, compact }) {
  return (
    <div className={compact ? "rounded-lg border border-slate-800 bg-slate-900/60 px-2.5 py-2" : "rounded-xl border border-slate-700/60 bg-slate-950/40 p-3"}>
      <p className="text-[11px] text-slate-500">{label}</p>
      <p className="mt-1 font-semibold text-cyan-200">{value}</p>
      <MetricLineageBadge lineage={lineage} />
    </div>
  );
}

function formatMetric(label, value) {
  if (value === null || value === undefined || value === "") {
    return "不可用";
  }
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return "不可用";
  }
  if (label === "成交失衡") {
    return number.toFixed(2);
  }
  if (label === "VPIN Z-Score") {
    return number.toFixed(2);
  }
  if (label === "VPIN Percentile") {
    return `${number.toFixed(0)}%`;
  }
  if (label === "价差 bps") {
    return `${number.toFixed(1)}bps`;
  }
  return number.toFixed(0);
}
