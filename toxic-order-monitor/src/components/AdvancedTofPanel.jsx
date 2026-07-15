import MetricLineageBadge from "./MetricLineageBadge.jsx";

export default function AdvancedTofPanel({ metrics, compact = false }) {
  if (!metrics) {
    return null;
  }

  const rows = [
    ["VPIN+", formatNumber(metrics.vpinEnhanced)],
    ["Flow Cluster", formatNumber(metrics.largeOrderFlowCluster)],
    ["Funding/OI", formatNumber(metrics.historicalFundingOiTrend)],
    ["Heatmap", formatNumber(metrics.marketPressureHeatmap)],
    ["Final", formatNumber(metrics.finalRiskScore)],
    ["Data Quality", formatNumber(metrics.dataQuality)],
    ["Completeness", formatNumber(metrics.metricsCompleteness)],
    ["Freshness", formatNumber(metrics.freshDataCoverage)],
  ];

  return (
    <div className={compact ? "mt-3" : ""}>
      <MetricLineageBadge lineage={metrics.lineage} />
      <div className={compact ? "grid grid-cols-2 gap-2 text-xs md:grid-cols-4" : "grid gap-3 md:grid-cols-4"}>
        {rows.map(([label, value]) => (
          <div className={compact ? "rounded-lg border border-fuchsia-900 bg-slate-900/60 px-2.5 py-2" : "rounded-xl border border-fuchsia-800/60 bg-slate-950/40 p-3"} key={label}>
            <p className="text-[11px] text-slate-500">{label}</p>
            <p className="mt-1 break-words font-semibold text-fuchsia-200">{value}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function formatNumber(value) {
  if (value === null || value === undefined || value === "") {
    return "不可用";
  }
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return "不可用";
  }
  return Math.round(number).toLocaleString("en-US");
}
