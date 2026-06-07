export default function PerpTofPanel({ metrics, compact = false }) {
  if (!metrics) {
    return null;
  }

  const rows = [
    ["OI", formatNumber(metrics.oiChange)],
    ["OI Direction", metrics.oiDirection],
    ["Funding", formatFunding(metrics.fundingRate, metrics.fundingSide)],
    ["Liquidation", formatNumber(metrics.liquidationPressure)],
    ["Squeeze", metrics.squeezeSide],
    ["Agg Flow", `${formatNumber(metrics.aggBuyVolume)} / ${formatNumber(metrics.aggSellVolume)}`],
    ["Perp Risk", formatNumber(metrics.riskScore)],
    ["Direction", metrics.metricsDirection],
  ];

  return (
    <div className={compact ? "mt-3 grid grid-cols-2 gap-2 text-xs md:grid-cols-4" : "grid gap-3 md:grid-cols-3"}>
      {rows.map(([label, value]) => (
        <div className={compact ? "rounded-lg border border-indigo-900 bg-slate-900/60 px-2.5 py-2" : "rounded-xl border border-indigo-800/60 bg-slate-950/40 p-3"} key={label}>
          <p className="text-[11px] text-slate-500">{label}</p>
          <p className="mt-1 break-words font-semibold text-indigo-200">{value || "N/A"}</p>
        </div>
      ))}
    </div>
  );
}

function formatNumber(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return "N/A";
  }
  return Math.round(number).toLocaleString("en-US");
}

function formatFunding(rate, side) {
  const number = Number(rate);
  const value = Number.isFinite(number) ? `${number.toFixed(4)}%` : "N/A";
  return `${value} ${side || ""}`.trim();
}
