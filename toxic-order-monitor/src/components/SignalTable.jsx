import { evaluateDiscordAlertGate } from "../api/alertGate.js";
import CandidateExplanation from "./CandidateExplanation.jsx";
import TofMetricsPanel from "./TofMetricsPanel.jsx";
import { finalResultDescription } from "../utils/signalResult.js";

const levelColors = {
  S: "bg-red-500/15 text-red-300 ring-red-400/30",
  A: "bg-red-500/15 text-red-300 ring-red-400/30",
  B: "bg-orange-500/15 text-orange-300 ring-orange-400/30",
  C: "bg-yellow-500/15 text-yellow-300 ring-yellow-300/30",
  D: "bg-slate-500/15 text-slate-300 ring-slate-400/30",
};

export default function SignalTable({
  signals,
  selectedSignal,
  onSelect,
  onPush,
  pushStatus = {},
  inboxStats,
  title = "Signal Inbox",
  description = "候选信号会保留在前端缓存，直到手动清除。",
  emptyMessage = "暂无缓存的有毒订单候选信号",
  emptyHint = "新的候选信号出现后会继续追加",
}) {
  return (
    <section className="rounded-2xl border border-slate-700/60 bg-slate-900/80 shadow-glow">
      <div className="flex flex-col gap-4 border-b border-slate-700/60 px-5 py-4 xl:flex-row xl:items-center xl:justify-between">
        <div>
          <h3 className="font-bold text-white">{title}</h3>
          <p className="text-xs text-slate-400">{description}</p>
          <p className="mt-1 text-xs font-semibold text-cyan-200">
            {signals.length} 显示 / {inboxStats.total} 已缓存
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-5">
          <Counter label="已缓存候选" value={inboxStats.total} />
          <Counter label="高风险" value={inboxStats.high} accent="text-red-300" />
          <Counter label="中风险" value={inboxStats.medium} accent="text-orange-300" />
          <Counter label="低风险" value={inboxStats.low} accent="text-yellow-300" />
          <Counter label="当前显示" value={signals.length} accent="text-cyan-300" />
        </div>
      </div>

      {inboxStats.total === 0 ? (
        <div className="px-5 py-16 text-center">
          <p className="text-lg font-semibold text-slate-200">{emptyMessage}</p>
          <p className="mt-2 text-sm text-slate-500">{emptyHint}</p>
        </div>
      ) : signals.length === 0 ? (
        <div className="px-5 py-12 text-center">
          <p className="text-sm text-slate-400">当前筛选条件下暂无候选信号。</p>
        </div>
      ) : (
        <div className="space-y-3 p-4">
          {signals.map((signal) => (
            <SignalCard
              key={signal.id}
              onPush={onPush}
              onSelect={onSelect}
              pushStatus={pushStatus}
              selected={selectedSignal?.id === signal.id}
              signal={signal}
            />
          ))}
        </div>
      )}
    </section>
  );
}

function SignalCard({ signal, selected, onSelect, onPush, pushStatus }) {
  const finalResult = finalResultDescription(signal);
  const gate = evaluateDiscordAlertGate(signal);
  const status = pushStatus?.[signal.id];
  const pending = status?.status === "pending";
  const canPush = gate.ok && !pending;

  return (
    <article
      className={[
        "rounded-2xl border bg-slate-950/40 p-4 transition",
        selected ? "border-cyan-300/60 shadow-[0_0_0_1px_rgba(103,232,249,0.22)]" : "border-slate-700/60",
      ].join(" ")}
      data-testid={`signal-card-${signal.id}`}
    >
      <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
        <button
          className="min-w-0 flex-1 text-left"
          onClick={() => onSelect(signal)}
          type="button"
        >
          <div className="flex flex-wrap items-center gap-2">
            <h4 className="truncate text-base font-bold text-white">
              {signal.symbol} · {signal.type} · {shortTime(signal.time)}
            </h4>
            <span className={`rounded-full px-2.5 py-1 text-xs font-bold ring-1 ${levelColors[signal.level]}`}>
              {signal.level}
            </span>
            <span className="rounded-full border border-slate-600/70 px-2.5 py-1 text-xs text-slate-300">
              {signal.side || "N/A"}
            </span>
          </div>
          <p className="mt-3 line-clamp-2 text-sm font-semibold leading-6 text-slate-100">{finalResult}</p>
          <CandidateExplanation compact signal={signal} />
          <TofMetricsPanel compact metrics={signal.tofMetrics} />
        </button>

        <div className="flex shrink-0 flex-wrap items-center gap-2 xl:justify-end">
          <button
            aria-label={`查看回放 ${signal.id}`}
            className="rounded-lg border border-cyan-400/40 px-3 py-2 text-xs font-semibold text-cyan-200 hover:bg-cyan-400/10"
            onClick={() => onSelect(signal)}
            type="button"
          >
            查看回放
          </button>
          <button
            aria-label={`Review ${signal.id}`}
            className="rounded-lg border border-slate-600 px-3 py-2 text-xs font-semibold text-slate-200 hover:border-cyan-400 hover:text-cyan-200"
            onClick={() => onSelect(signal)}
            type="button"
          >
            Review
          </button>
          <button
            aria-label={`推送 ${signal.id} 到 Discord`}
            className="rounded-lg border border-emerald-500/40 px-3 py-2 text-xs font-semibold text-emerald-200 hover:bg-emerald-500/10 disabled:cursor-not-allowed disabled:opacity-50"
            disabled={!canPush}
            onClick={() => onPush(signal)}
            type="button"
          >
            {pending ? "推送中" : discordButtonText(gate)}
          </button>
        </div>
      </div>
    </article>
  );
}

function discordButtonText(gate) {
  if (gate.ok) {
    return "手动推送";
  }
  if (gate.reason === "DISCORD_SUPPRESSED_NON_HIGH_RISK") {
    return "仅页面展示";
  }
  return "未达推送门槛";
}

function Counter({ label, value, accent = "text-slate-100" }) {
  return (
    <div className="rounded-xl border border-slate-700/60 bg-slate-950/40 px-3 py-2">
      <p className="text-[11px] text-slate-500">{label}</p>
      <p className={`mt-1 text-lg font-bold ${accent}`}>{value}</p>
    </div>
  );
}

function shortTime(value) {
  if (!value) {
    return "N/A";
  }
  return String(value).split(" ").pop();
}
