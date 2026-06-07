export default function CandidateExplanation({ signal, compact = false }) {
  if (!signal) {
    return null;
  }
  const tags = Array.isArray(signal.explainTags) ? signal.explainTags : [];
  const label = signal.directionLabel || signal.side || "中性 / 未知";

  return (
    <div className={compact ? "mt-3 space-y-2" : "rounded-xl border border-slate-700/60 bg-slate-950/40 p-4"}>
      <div className="flex flex-wrap items-center gap-2">
        <span className="rounded-full border border-cyan-400/40 px-2.5 py-1 text-xs font-semibold text-cyan-200">
          {label}
        </span>
        {signal.directionConfidence ? (
          <span className="text-xs text-slate-400">置信度 {Math.round(signal.directionConfidence)}</span>
        ) : null}
        <span className="text-xs text-slate-500">{signal.directionSource || "detector"}</span>
      </div>
      <div className="grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        <span>Risk {formatNumber(signal.finalRiskScore ?? signal.score)} / Quality {formatNumber(signal.dataQuality)}</span>
        <span>Severity {signal.level || signal.risk || "N/A"}</span>
        {signal.finalCandidateType ? <span>Final {signal.finalCandidateType}</span> : null}
        {signal.metricsDirection ? <span>Metrics {signal.metricsDirection}</span> : null}
      </div>
      <p className="text-xs font-semibold text-slate-300">Type: {signal.candidateType || signal.type}</p>
      {signal.perpCandidateType ? (
        <p className="text-xs font-semibold text-indigo-200">Perp: {signal.perpCandidateType}</p>
      ) : null}
      {signal.advancedCandidateType ? (
        <p className="text-xs font-semibold text-fuchsia-200">Advanced: {signal.advancedCandidateType}</p>
      ) : null}
      {tags.length > 0 ? (
        <div className="flex flex-wrap gap-2">
          {tags.slice(0, compact ? 3 : 8).map((tag) => (
            <span className="rounded-full border border-slate-700 px-2 py-0.5 text-[11px] text-slate-400" key={tag}>
              {tag}
            </span>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function formatNumber(value) {
  const number = Number(value);
  return Number.isFinite(number) ? Math.round(number) : "N/A";
}
