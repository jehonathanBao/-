export default function MetricLineageBadge({ lineage }) {
  if (!lineage || typeof lineage !== "object") {
    return null;
  }

  const state = lineageState(lineage);
  return (
    <div className="mb-2 flex flex-wrap items-center gap-2 text-[11px]" data-testid="metric-lineage">
      <span className={state.className}>{state.label}</span>
      {lineage.available === true && lineage.fresh === false ? (
        <span className="rounded-full border border-amber-400/40 bg-amber-400/10 px-2 py-1 text-amber-200">
          数据过期
        </span>
      ) : null}
      {lineage.source && lineage.source !== "unknown" ? (
        <span className="text-slate-500">来源：{lineage.source}</span>
      ) : null}
    </div>
  );
}

function lineageState(lineage) {
  if (lineage.provenance === "inferred") {
    return {
      label: "推断代理 · 不参与 Discord",
      className: "rounded-full border border-violet-400/40 bg-violet-400/10 px-2 py-1 text-violet-200",
    };
  }
  if (lineage.provenance === "observed") {
    return {
      label: "已观测",
      className: "rounded-full border border-emerald-400/35 bg-emerald-400/10 px-2 py-1 text-emerald-200",
    };
  }
  if (lineage.provenance === "calculated_from_observed") {
    return {
      label: "由观测计算",
      className: "rounded-full border border-cyan-400/35 bg-cyan-400/10 px-2 py-1 text-cyan-200",
    };
  }
  return {
    label: "来源不可用",
    className: "rounded-full border border-slate-600 bg-slate-800/60 px-2 py-1 text-slate-400",
  };
}
