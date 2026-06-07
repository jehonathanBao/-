import { useState } from "react";
import { evaluateDiscordAlertGate } from "../api/alertGate.js";
import AdvancedTofPanel from "./AdvancedTofPanel.jsx";
import CandidateExplanation from "./CandidateExplanation.jsx";
import PerpTofPanel from "./PerpTofPanel.jsx";
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
  onMarkStatus = () => {},
}) {
  const [reviewSignal, setReviewSignal] = useState(null);
  const [replaySignal, setReplaySignal] = useState(null);
  const latestUpdatedAt = latestSignalTimeLabel(signals);

  return (
    <>
      <section className="rounded-2xl border border-slate-700/60 bg-slate-900/80 shadow-glow">
        <div className="flex flex-col gap-4 border-b border-slate-700/60 px-5 py-4 xl:flex-row xl:items-center xl:justify-between">
          <div>
            <h3 className="font-bold text-white">{title}</h3>
            <p className="text-xs text-slate-400">{description}</p>
            <p className="mt-1 text-xs font-semibold text-cyan-200">
              {signals.length} 显示 / {inboxStats.total} 已缓存
            </p>
          </div>
          <div className="grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-3 2xl:grid-cols-6">
            <Counter label="已缓存候选" value={inboxStats.total} />
            <Counter label="高风险" value={inboxStats.high} accent="text-red-300" />
            <Counter label="中风险" value={inboxStats.medium} accent="text-orange-300" />
            <Counter label="低风险" value={inboxStats.low} accent="text-yellow-300" />
            <Counter label="当前显示" value={signals.length} accent="text-cyan-300" />
            <Counter label="更新时间" value={latestUpdatedAt} accent="text-emerald-200" testId="signal-inbox-updated-at" />
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
                onReplay={setReplaySignal}
                onReview={setReviewSignal}
                onSelect={onSelect}
                pushStatus={pushStatus}
                selected={selectedSignal?.id === signal.id}
                signal={signal}
              />
            ))}
          </div>
        )}
      </section>
      {reviewSignal ? (
        <CandidateReviewModal
          onMarkStatus={onMarkStatus}
          onClose={() => setReviewSignal(null)}
          signal={reviewSignal}
        />
      ) : null}
      {replaySignal ? (
        <ReplayModal onClose={() => setReplaySignal(null)} signal={replaySignal} />
      ) : null}
    </>
  );
}

function SignalCard({ signal, selected, onSelect, onPush, onReview, onReplay, pushStatus }) {
  const finalResult = finalResultDescription(signal);
  const gate = evaluateDiscordAlertGate(signal);
  const status = pushStatus?.[signal.id];
  const pending = status?.status === "pending";
  const pushed = status?.status === "success" || signal.status === "pushed" || signal.alertStatus === "sent";
  const canPush = gate.ok && !pending && !pushed;
  const replaySnapshot = replaySnapshotFor(signal);
  const riskHighlight = isHighOrCritical(signal)
    ? "border-red-400/60 bg-red-950/20 shadow-[0_0_0_1px_rgba(248,113,113,0.20)]"
    : "border-slate-700/60 bg-slate-950/40";

  return (
    <article
      className={[
        "rounded-2xl border bg-slate-950/40 p-4 transition",
        selected ? "border-cyan-300/60 shadow-[0_0_0_1px_rgba(103,232,249,0.22)]" : riskHighlight,
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
            <span className={`rounded-full border px-2.5 py-1 text-xs ${directionClass(signal)}`}>
              {signal.side || "N/A"}
            </span>
            {signal.reviewStatus ? (
              <span className={`rounded-full border px-2.5 py-1 text-xs font-semibold ${reviewStatusClass(signal.reviewStatus)}`}>
                {reviewStatusLabel(signal.reviewStatus)}
              </span>
            ) : null}
          </div>
          <p className="mt-3 line-clamp-2 text-sm font-semibold leading-6 text-slate-100">{finalResult}</p>
          <div className="mt-3 flex flex-wrap gap-2 text-xs font-semibold text-slate-300">
            <span className="rounded-full border border-slate-700 px-2 py-1">
              Risk {formatMetric(signal.finalRiskScore ?? signal.score)}
            </span>
            <span className="rounded-full border border-slate-700 px-2 py-1">
              Quality {formatMetric(signal.dataQuality)}
            </span>
            <span className="rounded-full border border-slate-700 px-2 py-1">
              TOF {formatMetric(signal.tofScore ?? signal.tofMetrics?.tofScore)}
            </span>
            <span className="rounded-full border border-indigo-800 px-2 py-1 text-indigo-200">
              Perp {formatMetric(signal.perpScore ?? signal.perpTofMetrics?.riskScore)}
            </span>
            <span className="rounded-full border border-fuchsia-800 px-2 py-1 text-fuchsia-200">
              Advanced {formatMetric(signal.advancedScore ?? signal.advancedTofMetrics?.finalRiskScore)}
            </span>
          </div>
          <CandidateExplanation compact signal={signal} />
          <TofMetricsPanel compact metrics={signal.tofMetrics} />
          <PerpTofPanel compact metrics={signal.perpTofMetrics} />
          <AdvancedTofPanel compact metrics={signal.advancedTofMetrics} />
          <DiscordAlertStatus signal={signal} />
        </button>

        <div className="flex shrink-0 flex-wrap items-center gap-2 xl:justify-end">
          <button
            aria-label={`查看回放 ${signal.id}`}
            className="rounded-lg border border-slate-700 px-3 py-2 text-xs font-semibold text-slate-300 hover:border-cyan-400 hover:text-cyan-200 disabled:cursor-not-allowed disabled:text-slate-500"
            disabled={!replaySnapshot}
            onClick={() => onReplay(signal)}
            title={replaySnapshot ? "查看 redacted replay snapshot" : "暂无 replay 数据，按钮保持禁用"}
            type="button"
          >
            查看回放
          </button>
          <button
            aria-label={`Review ${signal.id}`}
            className="rounded-lg border border-cyan-400/40 px-3 py-2 text-xs font-semibold text-cyan-200 hover:bg-cyan-400/10"
            onClick={() => onReview(signal)}
            type="button"
          >
            详情 / Review
          </button>
          <button
            aria-label={`推送 ${signal.id} 到 Discord`}
            className="rounded-lg border border-emerald-500/40 px-3 py-2 text-xs font-semibold text-emerald-200 hover:bg-emerald-500/10 disabled:cursor-not-allowed disabled:opacity-50"
            disabled={!canPush}
            onClick={() => onPush(signal)}
            type="button"
          >
            {pending ? "推送中" : pushed ? "已推送" : discordButtonText(gate)}
          </button>
        </div>
      </div>
    </article>
  );
}

function CandidateReviewModal({ signal, onClose, onMarkStatus }) {
  const finalResult = finalResultDescription(signal);
  const rows = [
    ["Symbol", signal.symbol],
    ["Direction", signal.directionLabel || signal.side],
    ["Risk Score", formatMetric(signal.finalRiskScore ?? signal.score)],
    ["Data Quality", formatMetric(signal.dataQuality)],
    ["TOF Score", formatMetric(signal.tofScore ?? signal.tofMetrics?.tofScore)],
    ["Perp Score", formatMetric(signal.perpScore ?? signal.perpTofMetrics?.riskScore)],
    ["Advanced Score", formatMetric(signal.advancedScore ?? signal.advancedTofMetrics?.finalRiskScore)],
    ["Candidate Type", signal.candidateType || signal.type],
    ["Perp Candidate Type", signal.perpCandidateType || signal.perpTofMetrics?.candidateType],
    ["Advanced Candidate Type", signal.advancedCandidateType || signal.advancedTofMetrics?.candidateType],
    ["Final Candidate Type", signal.finalCandidateType],
    ["Metrics Direction", signal.metricsDirection],
    ["Discord Alert Status", discordAlertText(signal)],
    ["Core Reason", signal.reason || signal.coreReason || "N/A"],
    ["Final Result", finalResult],
  ];
  const tags = Array.isArray(signal.explainTags) ? signal.explainTags : [];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 px-4 py-6">
      <div
        aria-modal="true"
        className="max-h-full w-full max-w-2xl overflow-y-auto rounded-2xl border border-slate-700 bg-slate-950 p-5 shadow-2xl"
        role="dialog"
      >
        <div className="flex items-start justify-between gap-4 border-b border-slate-800 pb-4">
          <div>
            <p className="text-xs uppercase tracking-[0.26em] text-cyan-300">Candidate Review</p>
            <h3 className="mt-2 text-lg font-bold text-white">{signal.symbol} · {signal.type}</h3>
          </div>
          <button
            aria-label="关闭 Review"
            className="rounded-lg border border-slate-700 px-3 py-2 text-xs font-semibold text-slate-200 hover:border-cyan-400 hover:text-cyan-200"
            onClick={onClose}
            type="button"
          >
            关闭
          </button>
        </div>

        <div className="mt-4 grid gap-3 sm:grid-cols-2">
          {rows.map(([label, value]) => (
            <ReviewField key={label} label={label} value={value} />
          ))}
        </div>

        <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/60 p-3">
          <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Review Status</p>
          <div className="mt-2 flex flex-wrap gap-2">
            {["watched", "acknowledged", "false_positive", "important"].map((status) => (
              <button
                className={`rounded-full border px-3 py-1.5 text-xs font-semibold ${reviewStatusClass(status)}`}
                key={status}
                onClick={() => onMarkStatus(signal.id, status)}
                type="button"
              >
                {reviewStatusLabel(status)}
              </button>
            ))}
          </div>
        </div>

        <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/60 p-3">
          <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Explain Tags</p>
          {tags.length > 0 ? (
            <div className="mt-2 flex flex-wrap gap-2">
              {tags.map((tag) => (
                <span className="rounded-full border border-slate-700 px-2 py-1 text-xs text-slate-300" key={tag}>
                  {tag}
                </span>
              ))}
            </div>
          ) : (
            <p className="mt-2 text-sm text-slate-300">N/A</p>
          )}
        </div>
      </div>
    </div>
  );
}

function ReplayModal({ signal, onClose }) {
  const snapshot = replaySnapshotFor(signal);
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 px-4 py-6">
      <div
        aria-modal="true"
        className="max-h-full w-full max-w-3xl overflow-y-auto rounded-2xl border border-slate-700 bg-slate-950 p-5 shadow-2xl"
        role="dialog"
      >
        <div className="flex items-start justify-between gap-4 border-b border-slate-800 pb-4">
          <div>
            <p className="text-xs uppercase tracking-[0.26em] text-cyan-300">Replay Snapshot</p>
            <h3 className="mt-2 text-lg font-bold text-white">{signal.symbol} · {signal.type}</h3>
          </div>
          <button
            aria-label="关闭 Replay"
            className="rounded-lg border border-slate-700 px-3 py-2 text-xs font-semibold text-slate-200 hover:border-cyan-400 hover:text-cyan-200"
            onClick={onClose}
            type="button"
          >
            关闭
          </button>
        </div>
        <pre className="mt-4 max-h-[60vh] overflow-auto rounded-xl border border-slate-800 bg-slate-900/70 p-4 text-xs leading-5 text-slate-200">
          {JSON.stringify(redactReplaySnapshot(snapshot), null, 2)}
        </pre>
      </div>
    </div>
  );
}

function ReviewField({ label, value }) {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-3">
      <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">{label}</p>
      <p className="mt-1 break-words text-sm font-semibold text-slate-100">{value || "N/A"}</p>
    </div>
  );
}

function DiscordAlertStatus({ signal }) {
  const alert = signal.discordAlert || {};
  const status = signal.alertStatus || alert.lastDecision || "not_evaluated";
  const reason = signal.alertReason || alert.reason;
  const text = alert.manualSentAt || reason === "manual_sent"
    ? "Discord：已手动推送"
    : alert.autoSent || status === "sent"
    ? "Discord：已自动推送"
    : status === "eligible"
      ? "Discord：符合自动推送门槛，等待后端发送"
      : status === "rejected" || status === "skipped"
        ? `Discord：未推送，原因：${reasonLabel(reason)}`
        : "Discord：未评估";
  const sentAt = alert.manualSentAt ? `Manual sent at ${shortTime(alert.manualSentAt)}` : alert.sentAt ? `Auto sent at ${shortTime(alert.sentAt)}` : null;
  return (
    <div className="mt-3 rounded-lg border border-slate-800 bg-slate-950/50 px-3 py-2 text-xs text-slate-300">
      <p>{text}</p>
      {sentAt ? <p className="mt-1 text-emerald-300">{sentAt}</p> : null}
    </div>
  );
}

function discordAlertText(signal) {
  const alert = signal.discordAlert || {};
  if (alert.manualSentAt || signal.alertReason === "manual_sent") {
    return "manual_sent";
  }
  if (alert.autoSent || signal.alertStatus === "sent") {
    return "auto_sent";
  }
  return signal.alertReason || alert.reason || signal.alertStatus || alert.lastDecision || "not_evaluated";
}

function reasonLabel(reason) {
  const labels = {
    data_quality_below_threshold: "dataQuality 低于 70",
    score_below_threshold: "score 低于 80",
    non_high_risk: "Medium/Low 只页面展示",
    auto_disabled: "auto push disabled",
    dry_run: "dry run",
    webhook_missing: "Discord 未配置",
    cached_on_boot: "历史缓存不自动推送",
    duplicate: "duplicate",
    duplicate_candidate: "duplicate",
    cooldown: "cooldown",
    rate_limited: "rate limit",
    manual_sent: "manual sent",
  };
  return labels[reason] || reason || "unknown";
}

function isHighOrCritical(signal) {
  const level = String(signal?.level || "").toUpperCase();
  return signal?.risk === "high" || level === "S" || level === "A" || level === "CRITICAL";
}

function directionClass(signal) {
  const value = String(signal?.metricsDirection || signal?.direction || signal?.side || "").toLowerCase();
  if (value.includes("bull") || value.includes("bid") || value.includes("buy") || value.includes("long")) {
    return "border-emerald-400/50 bg-emerald-400/10 text-emerald-200";
  }
  if (value.includes("bear") || value.includes("ask") || value.includes("sell") || value.includes("short")) {
    return "border-red-400/50 bg-red-400/10 text-red-200";
  }
  if (value.includes("mixed") || value.includes("conflict")) {
    return "border-slate-500/60 bg-slate-600/10 text-slate-200";
  }
  return "border-yellow-400/50 bg-yellow-400/10 text-yellow-200";
}

function reviewStatusLabel(status) {
  const labels = {
    watched: "watched",
    acknowledged: "acknowledged",
    false_positive: "false positive",
    important: "important",
  };
  return labels[status] || status || "unmarked";
}

function reviewStatusClass(status) {
  const classes = {
    watched: "border-slate-500/60 bg-slate-500/10 text-slate-200",
    acknowledged: "border-cyan-400/50 bg-cyan-400/10 text-cyan-200",
    false_positive: "border-yellow-400/50 bg-yellow-400/10 text-yellow-200",
    important: "border-fuchsia-400/50 bg-fuchsia-400/10 text-fuchsia-200",
  };
  return classes[status] || "border-slate-700 bg-slate-900 text-slate-300";
}

function replaySnapshotFor(signal) {
  return signal?.replaySnapshot || signal?.redactedReplaySnapshot || signal?.replay?.snapshot || null;
}

function redactReplaySnapshot(value) {
  if (Array.isArray(value)) {
    return value.map(redactReplaySnapshot);
  }
  if (!value || typeof value !== "object") {
    return value;
  }
  const forbidden = new Set([
    "rawPayload",
    "rawpayload",
    "evidence",
    "markout",
    "token",
    "webhook",
    "authorization",
    "apiKey",
    "secret",
  ]);
  return Object.fromEntries(
    Object.entries(value)
      .filter(([key]) => !forbidden.has(key) && !forbidden.has(key.toLowerCase()))
      .map(([key, item]) => [key, redactReplaySnapshot(item)]),
  );
}

function formatMetric(value) {
  const number = Number(value);
  return Number.isFinite(number) ? Math.round(number) : "N/A";
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

function Counter({ label, value, accent = "text-slate-100", testId }) {
  return (
    <div className="rounded-xl border border-slate-700/60 bg-slate-950/40 px-3 py-2" data-testid={testId}>
      <p className="text-[11px] text-slate-500">{label}</p>
      <p className={`mt-1 text-lg font-bold ${accent}`}>{value}</p>
    </div>
  );
}

function latestSignalTimeLabel(signals) {
  const latest = (Array.isArray(signals) ? signals : [])
    .map(signalTimeValue)
    .filter((value) => value > 0)
    .sort((left, right) => right - left)[0];
  return latest ? formatTimeLabel(latest) : "暂无";
}

function signalTimeValue(signal) {
  const lastSeenAt = Number(signal?.lastSeenAt);
  if (Number.isFinite(lastSeenAt) && lastSeenAt > 0) {
    return lastSeenAt;
  }
  const firstSeenAt = Number(signal?.firstSeenAt);
  if (Number.isFinite(firstSeenAt) && firstSeenAt > 0) {
    return firstSeenAt;
  }
  const parsedTime = Date.parse(signal?.time || "");
  return Number.isFinite(parsedTime) ? parsedTime : 0;
}

function formatTimeLabel(value) {
  return new Date(value).toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    hour12: false,
    minute: "2-digit",
    second: "2-digit",
  });
}

function shortTime(value) {
  if (!value) {
    return "N/A";
  }
  return String(value).split(" ").pop();
}
