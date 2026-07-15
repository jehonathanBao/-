const riskConfig = {
  high: { label: "高风险异常", color: "text-red-300", bar: "bg-red-500", border: "hover:border-red-400/70" },
  medium: { label: "中风险异常", color: "text-orange-300", bar: "bg-orange-500", border: "hover:border-orange-400/70" },
  low: { label: "低风险异常", color: "text-yellow-300", bar: "bg-yellow-400", border: "hover:border-yellow-300/70" },
  all: { label: "今日信号总数", color: "text-cyan-300", bar: "bg-cyan-400", border: "hover:border-cyan-300/70" },
};

export default function RiskCard({ risk, count, percentage, active, onClick }) {
  const config = riskConfig[risk] ?? riskConfig.all;

  return (
    <button
      aria-label={`筛选 ${risk} 风险`}
      aria-pressed={active}
      className={[
        "workspace-risk-card border p-5 text-left transition",
        config.border,
        active ? "ring-2 ring-cyan-300/60" : "",
      ].join(" ")}
      onClick={onClick}
      type="button"
    >
      <div className="flex items-center justify-between">
        <p className="text-sm text-slate-400">{config.label}</p>
        <span className={`h-3 w-3 rounded-full ${config.bar}`} />
      </div>
      <div className="mt-4 flex items-end justify-between">
        <p className={`text-4xl font-bold ${config.color}`}>{count}</p>
        <p className="text-sm text-slate-400">{percentage}%</p>
      </div>
      <div className="mt-4 h-2 overflow-hidden rounded-full bg-slate-800">
        <div className={`h-full ${config.bar}`} style={{ width: `${Math.min(100, Number(percentage))}%` }} />
      </div>
    </button>
  );
}
