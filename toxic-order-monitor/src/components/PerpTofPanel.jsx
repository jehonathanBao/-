import MetricLineageBadge from "./MetricLineageBadge.jsx";

export default function PerpTofPanel({ metrics, compact = false }) {
  if (!metrics) {
    return null;
  }

  const liquidationMetric = resolveLiquidationMetric(metrics);
  const rows = [
    ["OI", formatNumber(metrics.oiChange)],
    ["OI Direction", formatText(metrics.oiDirection)],
    ["Funding", formatFunding(metrics.fundingRate, metrics.fundingSide)],
    [liquidationMetric.label, liquidationMetric.value, liquidationMetric.lineage],
    ["Squeeze 方向", formatText(metrics.squeezeSide)],
    ["Agg Flow", `${formatNumber(metrics.aggBuyVolume)} / ${formatNumber(metrics.aggSellVolume)}`],
    ["Perp Risk", formatNumber(metrics.riskScore)],
    ["Direction", formatText(metrics.metricsDirection)],
  ];

  return (
    <div className={compact ? "mt-3" : ""}>
      <MetricLineageBadge lineage={metrics.lineage} />
      <div className={compact ? "grid grid-cols-2 gap-2 text-xs md:grid-cols-4" : "grid gap-3 md:grid-cols-3"}>
        {rows.map(([label, value, lineage]) => (
          <div className={compact ? "rounded-lg border border-indigo-900 bg-slate-900/60 px-2.5 py-2" : "rounded-xl border border-indigo-800/60 bg-slate-950/40 p-3"} key={label}>
            <p className="text-[11px] text-slate-500">{label}</p>
            <p className="mt-1 break-words font-semibold text-indigo-200">{value}</p>
            <MetricLineageBadge lineage={lineage} />
          </div>
        ))}
      </div>
    </div>
  );
}

function resolveLiquidationMetric(metrics) {
  const lineage = metrics.liquidationLineage;
  const provenance = lineage?.provenance;
  const observedAvailable = metrics.observedLiquidationNotional !== null
    && metrics.observedLiquidationNotional !== undefined;
  const squeezeProxyAvailable = metrics.squeezeRiskProxy !== null
    && metrics.squeezeRiskProxy !== undefined;

  if (provenance === "observed" || (observedAvailable && provenance !== "inferred")) {
    return {
      label: "已观测清算名义额 USD",
      value: formatNumber(metrics.observedLiquidationNotional),
      lineage,
    };
  }
  if (provenance === "inferred" || (!provenance && squeezeProxyAvailable)) {
    return {
      label: "Squeeze 风险代理",
      value: formatNumber(squeezeProxyAvailable ? metrics.squeezeRiskProxy : metrics.liquidationPressure),
      lineage,
    };
  }
  return {
    label: "Liquidation Risk Score",
    value: formatNumber(metrics.liquidationPressure),
    lineage,
  };
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

function formatFunding(rate, side) {
  if (rate === null || rate === undefined || rate === "") {
    return "不可用";
  }
  const number = Number(rate);
  const value = Number.isFinite(number) ? `${number.toFixed(4)}%` : "不可用";
  return `${value} ${side || ""}`.trim();
}

function formatText(value) {
  return typeof value === "string" && value.trim() ? value : "不可用";
}
