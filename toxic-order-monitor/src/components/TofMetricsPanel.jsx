export default function TofMetricsPanel({ metrics, compact = false }) {
  if (!metrics) {
    return null;
  }

  const rows = [
    ["TOF Score", metrics.tofScore],
    ["VPIN Proxy", metrics.vpinProxy],
    ["成交失衡", metrics.tradeImbalance],
    ["Bid 撤出", metrics.bidDepthWithdrawal],
    ["Ask 撤出", metrics.askDepthWithdrawal],
    ["价差 bps", metrics.spreadBps],
    ["方向置信", metrics.metricsConfidence],
  ];

  return (
    <div className={compact ? "mt-3 grid grid-cols-2 gap-2 text-xs md:grid-cols-4" : "grid gap-3 md:grid-cols-3"}>
      {rows.map(([label, value]) => (
        <MetricCell compact={compact} key={label} label={label} value={formatMetric(label, value)} />
      ))}
    </div>
  );
}

function MetricCell({ label, value, compact }) {
  return (
    <div className={compact ? "rounded-lg border border-slate-800 bg-slate-900/60 px-2.5 py-2" : "rounded-xl border border-slate-700/60 bg-slate-950/40 p-3"}>
      <p className="text-[11px] text-slate-500">{label}</p>
      <p className="mt-1 font-semibold text-cyan-200">{value}</p>
    </div>
  );
}

function formatMetric(label, value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return "N/A";
  }
  if (label === "成交失衡") {
    return number.toFixed(2);
  }
  if (label === "价差 bps") {
    return `${number.toFixed(1)}bps`;
  }
  return number.toFixed(0);
}
