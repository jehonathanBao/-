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
      <p className="text-xs font-semibold text-slate-300">Type: {signal.candidateType || signal.type}</p>
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
