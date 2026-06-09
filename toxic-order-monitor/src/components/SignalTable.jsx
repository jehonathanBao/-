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
            <span className="rounded-full border border-cyan-800 px-2 py-1 text-cyan-200">
              价格 {formatPrice(signalTriggerPrice(signal))}
            </span>
            <span className="rounded-full border border-slate-700 px-2 py-1">
              短线毒性 {formatMetric(signal.toxicScore ?? signal.finalRiskScore ?? signal.score)}
            </span>
            <span className="rounded-full border border-slate-700 px-2 py-1">
              压力 {formatSignedMetric(signal.shortPressure)}
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
            <span className="rounded-full border border-emerald-800 px-2 py-1 text-emerald-200">
              主力结构 {formatMetric(signal.mainForceScore ?? signal.riskSystems?.mainForceStructure?.mainForceScore)}
            </span>
            <span className="rounded-full border border-slate-700 px-2 py-1">
              偏向 {formatSignedMetric(signal.structureBias ?? signal.marketStructureScore?.structureBias)}
            </span>
            <span
              className={[
                "rounded-full border px-2 py-1",
                signal.mainForceConfirmed
                  ? "border-emerald-500/60 text-emerald-200"
                  : "border-amber-500/60 text-amber-200",
              ].join(" ")}
            >
              主力确认 {signal.mainForceConfirmed ? "已确认" : "待确认"} ·{" "}
              {formatMetric(signal.mainForceConfirmationCount ?? signal.marketStructureScore?.mainForceConfirmationCount)}/
              {formatMetric(signal.mainForceConfirmationTotal ?? signal.marketStructureScore?.mainForceConfirmationTotal)}
            </span>
            <span
              className={[
                "rounded-full border px-2 py-1",
                signal.extremeImpactConfirmed
                  ? "border-rose-500/60 text-rose-200"
                  : "border-slate-700 text-slate-300",
              ].join(" ")}
            >
              极端行情 {signal.extremeImpactConfirmed ? "是" : "否"}
            </span>
            <span className="rounded-full border border-emerald-800 px-2 py-1 text-emerald-200">
              CWM {formatMetric(signal.cwmContribution?.score)} · 独立
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
    ["Trigger Price", formatPrice(signalTriggerPrice(signal))],
    ["Direction", signal.directionLabel || signal.side],
    ["Toxic Score", formatMetric(signal.toxicScore ?? signal.finalRiskScore ?? signal.score)],
    ["Short Pressure", formatSignedMetric(signal.shortPressure)],
    ["Toxic Severity", signal.toxicSeverity || signal.toxicShortScore?.severity],
    ["Toxic Type", signal.toxicType || signal.toxicShortScore?.toxicType],
    ["Toxic TTL", formatDuration(signal.toxicTtlSec ?? signal.toxicShortScore?.ttlSec)],
    ["Toxic Expires", formatEpochMs(signal.toxicExpiresAt ?? signal.toxicShortScore?.expiresAt)],
    ["Toxic Half-Life", formatDuration(signal.toxicHalfLifeSec ?? signal.toxicShortScore?.halfLifeSec)],
    ["Decayed Score", formatMetric(signal.toxicDecayedScore ?? signal.toxicShortScore?.decayedScore)],
    ["Decay Formula", signal.toxicDecayFormula || signal.toxicShortScore?.decayFormula],
    ["Toxic Reasons", toxicReasonsText(signal.toxicReasons ?? signal.toxicShortScore?.reasons)],
    ["Main Force Score", formatMetric(signal.mainForceScore ?? signal.riskSystems?.mainForceStructure?.mainForceScore)],
    ["Main Force Confirmed", booleanText(signal.mainForceConfirmed ?? signal.marketStructureScore?.mainForceConfirmed)],
    [
      "Main Force Confirmation Count",
      `${formatMetric(signal.mainForceConfirmationCount ?? signal.marketStructureScore?.mainForceConfirmationCount)}/${formatMetric(
        signal.mainForceConfirmationTotal ?? signal.marketStructureScore?.mainForceConfirmationTotal,
      )} (min ${formatMetric(
        signal.mainForceConfirmationThreshold ?? signal.marketStructureScore?.mainForceConfirmationThreshold,
      )})`,
    ],
    ["Structure Bias", formatSignedMetric(signal.structureBias)],
    ["Extreme Impact", formatMetric(signal.extremeImpactScore)],
    ["Extreme Market Impact", booleanText(signal.extremeImpactConfirmed ?? signal.marketStructureScore?.extremeImpactConfirmed)],
    ["Market Structure Severity", signal.marketStructureSeverity || signal.marketStructureScore?.severity],
    ["Market Structure Confidence", formatMetric(signal.marketStructureConfidence ?? signal.marketStructureScore?.confidence)],
    ["Regime Type", regimeTypeText(signal.regimeType)],
    ["Market Structure Quality", formatMetric(signal.marketStructureDataQuality ?? signal.marketStructureScore?.dataQuality)],
    ["Structure Raw", formatDecimalMetric(signal.structureRaw ?? signal.marketStructureScore?.structureRaw)],
    ["Spot/Contract Floor", formatMetric(signal.spotContractFloor ?? signal.marketStructureScore?.spotContractFloor)],
    ["Duration Score", formatMetric(signal.durationScore ?? signal.marketStructureScore?.durationScore)],
    ["Liquidation Penalty", formatDecimalMetric(signal.liquidationPenalty ?? signal.marketStructureScore?.liquidationPenalty)],
    ["Crowding Penalty", formatDecimalMetric(signal.crowdingPenalty ?? signal.marketStructureScore?.crowdingPenalty)],
    ["Spot Score", formatMetric(signal.spotScore ?? signal.marketStructureScore?.spotScore)],
    ["Spot CVD", formatMetric(signal.spotCvdScore ?? signal.marketStructureScore?.spotCvdScore)],
    ["Spot Volume Anomaly", formatMetric(signal.spotVolumeAnomaly ?? signal.marketStructureScore?.spotVolumeAnomaly)],
    ["Spot Absorption", formatMetric(signal.spotAbsorption ?? signal.marketStructureScore?.spotAbsorption)],
    ["Spot Liquidity Shift", formatMetric(signal.spotLiquidityShift ?? signal.marketStructureScore?.spotLiquidityShift)],
    ["Spot Price Response", formatMetric(signal.spotPriceResponse ?? signal.marketStructureScore?.spotPriceResponse)],
    ["Contract Score", formatMetric(signal.contractScore ?? signal.marketStructureScore?.contractScore)],
    ["CWM Aggressive Flow", formatMetric(signal.cwmAggressiveFlow ?? signal.marketStructureScore?.cwmAggressiveFlow)],
    ["OI Impulse", formatMetric(signal.oiImpulse ?? signal.marketStructureScore?.oiImpulse)],
    ["Liquidation Context", formatMetric(signal.liquidationContext ?? signal.marketStructureScore?.liquidationContext)],
    ["Funding Crowding", formatMetric(signal.fundingCrowding ?? signal.marketStructureScore?.fundingCrowding)],
    ["Basis Premium", formatMetric(signal.basisPremium ?? signal.marketStructureScore?.basisPremium)],
    [
      "Active Exchange Confirmation",
      formatMetric(signal.activeExchangeConfirmation ?? signal.marketStructureScore?.activeExchangeConfirmation),
    ],
    ["Cross Confirm", formatMetric(signal.crossConfirmScore ?? signal.marketStructureScore?.crossConfirmScore)],
    [
      "Spot/Contract Direction",
      formatMetric(
        signal.spotContractDirectionConsistency ?? signal.marketStructureScore?.spotContractDirectionConsistency,
      ),
    ],
    [
      "Multi-Window Consistency",
      formatMetric(signal.multiWindowConsistency ?? signal.marketStructureScore?.multiWindowConsistency),
    ],
    [
      "Price Response Consistency",
      formatMetric(signal.priceResponseConsistency ?? signal.marketStructureScore?.priceResponseConsistency),
    ],
    ["Source Coverage", formatMetric(signal.sourceCoverage ?? signal.marketStructureScore?.sourceCoverage)],
    ["Signal Agreement", formatMetric(signal.signalAgreement ?? signal.marketStructureScore?.signalAgreement)],
    ["OI Score", formatMetric(signal.oiScore ?? signal.marketStructureScore?.oiScore)],
    ["Liquidation Score", formatMetric(signal.liquidationScore ?? signal.marketStructureScore?.liquidationScore)],
    ["Funding Crowding Score", formatMetric(signal.fundingCrowdingScore ?? signal.marketStructureScore?.fundingCrowdingScore)],
    ["CWM Score", formatMetric(signal.cwmScore ?? signal.marketStructureScore?.cwmScore)],
    [
      "Market Structure Reasons",
      structureReasonsText(signal.marketStructureReasons ?? signal.marketStructureScore?.reasons),
    ],
    ["Data Quality", formatMetric(signal.dataQuality)],
    ["TOF Score", formatMetric(signal.tofScore ?? signal.tofMetrics?.tofScore)],
    ["Perp Score", formatMetric(signal.perpScore ?? signal.perpTofMetrics?.riskScore)],
    ["Advanced Score", formatMetric(signal.advancedScore ?? signal.advancedTofMetrics?.finalRiskScore)],
    ["CWM Contribution", cwmContributionText(signal.cwmContribution)],
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
    score_below_threshold: "toxicScore 低于 85",
    confidence_below_threshold: "confidence 低于 70",
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

function signalTriggerPrice(signal) {
  const explicit = Number(
    signal?.triggerPriceUsd ??
      signal?.triggerPrice ??
      signal?.priceUsd ??
      signal?.price ??
      signal?.markPrice ??
      signal?.midPrice ??
      signal?.currentPrice,
  );
  if (Number.isFinite(explicit) && explicit > 0) {
    return explicit;
  }
  return priceFromRange(signal?.priceRange ?? signal?.price_range);
}

function priceFromRange(value) {
  if (typeof value !== "string" || /depth|qty|quantity|volume|amount/i.test(value)) {
    return null;
  }
  const matches = value
    .replace(/,/g, "")
    .match(/-?\d+(?:\.\d+)?/g)
    ?.map(Number)
    .filter((number) => Number.isFinite(number) && number > 0);
  if (!matches || matches.length === 0) {
    return null;
  }
  if (matches.length === 1) {
    return matches[0];
  }
  return (matches[0] + matches[1]) / 2;
}

function formatPrice(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  if (number >= 1000) return `$${Math.round(number).toLocaleString("en-US")}`;
  if (number >= 1) return `$${number.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  return `$${number.toLocaleString("en-US", { minimumFractionDigits: 4, maximumFractionDigits: 4 })}`;
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

function formatDecimalMetric(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number.toFixed(2).replace(/\.00$/, "") : "N/A";
}

function formatContribution(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number.toFixed(1) : "0.0";
}

function formatSignedMetric(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) return "N/A";
  return `${number >= 0 ? "+" : ""}${Math.round(number)}`;
}

function formatDuration(value) {
  const number = Number(value);
  return Number.isFinite(number) ? `${Math.round(number)}s` : "N/A";
}

function formatEpochMs(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  return new Date(number).toLocaleTimeString("zh-CN", { hour12: false });
}

function toxicReasonsText(reasons) {
  if (!Array.isArray(reasons) || reasons.length === 0) {
    return "N/A";
  }
  return reasons
    .slice(0, 3)
    .map((reason) => `${reason.reasonType || "reason"} ${formatMetric(reason.score)}`)
    .join(" · ");
}

function structureReasonsText(reasons) {
  if (!Array.isArray(reasons) || reasons.length === 0) {
    return "N/A";
  }
  return reasons
    .slice(0, 4)
    .map((reason) => `${reason.reasonType || "reason"} ${formatMetric(reason.score)}`)
    .join(" · ");
}

function cwmContributionText(contribution) {
  if (!contribution?.available) {
    return "N/A · CWM gate independent";
  }
  const score = formatMetric(contribution.score);
  const weighted = formatContribution(contribution.weightedContribution);
  const window = contribution.windowSec ? `${contribution.windowSec}s` : "N/A";
  const mainExchange = contribution.mainExchange || "unknown venue";
  const exchangeCount = contribution.exchangeCount ?? "N/A";
  return `Score ${score} · main-force component +${weighted} · ${window} · ${mainExchange} · active venues ${exchangeCount} · CWM gate independent`;
}

function discordButtonText(gate) {
  if (gate.ok) {
    return "手动推送";
  }
  if (gate.reason === "DISCORD_SUPPRESSED_NON_HIGH_RISK") {
    return "仅页面展示";
  }
  if (gate.reason === "DISCORD_SUPPRESSED_LOW_CONFIDENCE") {
    return "置信度不足";
  }
  return "未达推送门槛";
}

function booleanText(value) {
  return value ? "Yes" : "No";
}

function regimeTypeText(value) {
  if (!value) {
    return "N/A";
  }
  const label = {
    main_force_long_build: "主力建多",
    main_force_short_build: "主力建空",
    contract_flow_shock: "合约冲击",
    spot_accumulation: "现货吸筹",
    spot_distribution: "现货派发",
    contract_short_squeeze: "空头挤压",
    long_liquidation_cascade: "多头踩踏",
    downside_absorption: "下方吸收",
    upside_resistance: "上方压制",
    range_rotation: "高换手震荡",
    unclear: "结构不清晰",
  }[value];
  return label ? `${label} · ${value}` : value;
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
