import { useEffect, useMemo, useState } from "react";
import { finalResultDescription } from "../utils/signalResult.js";

const SHORT_BREAKDOWN_ORDER = [
  ["ToxicOrderCluster", "异常订单聚集"],
  ["AggressiveSweep", "主动扫盘"],
  ["OrderbookDeformation", "盘口变形"],
  ["SpoofCancel", "虚假挂单"],
  ["AdverseMove", "反向伤害"],
  ["LiquidityGap", "流动性缺口"],
];

const STRUCTURE_BREAKDOWN_ITEMS = [
  ["spotScore", "现货评分"],
  ["contractScore", "合约评分"],
  ["crossConfirmScore", "现货合约确认"],
  ["cwmAggressiveFlow", "CWM 主力成交流"],
  ["oiScore", "OI 变化"],
  ["liquidationBlend", "清算环境"],
  ["fundingCrowdingScore", "Funding 拥挤"],
];

export default function RiskSystemSummaryCards({ signal }) {
  const [nowMs, setNowMs] = useState(() => Date.now());

  useEffect(() => {
    const timer = window.setInterval(() => setNowMs(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  const toxicCard = useMemo(() => buildToxicCard(signal, nowMs), [nowMs, signal]);
  const structureCard = useMemo(() => buildStructureCard(signal), [signal]);

  return (
    <section className="mb-5 grid gap-4 2xl:grid-cols-2" aria-label="风险评分概览">
      <SummaryCard
        accentClass="border-red-400/35 bg-red-950/20"
        badgeClass={severityBadgeClass(toxicCard.severity)}
        card={toxicCard}
        dataTestId="short-toxic-summary-card"
        summaryToneClass={pressureToneClass(toxicCard.pressure)}
        title="短线有毒订单评分"
      />
      <SummaryCard
        accentClass="border-emerald-400/35 bg-emerald-950/20"
        badgeClass={structureBadgeClass(structureCard.severity)}
        card={structureCard}
        dataTestId="market-structure-summary-card"
        summaryToneClass={structureToneClass(structureCard.bias)}
        title="现货 + 合约主力结构评分"
      />
    </section>
  );
}

function SummaryCard({ title, card, accentClass, badgeClass, summaryToneClass, dataTestId }) {
  return (
    <section
      className={`rounded-2xl border ${accentClass} px-5 py-5 shadow-[0_10px_30px_rgba(2,6,23,0.22)]`}
      data-testid={dataTestId}
    >
      <div className="flex flex-col gap-3 border-b border-slate-800/80 pb-4 lg:flex-row lg:items-start lg:justify-between">
        <div className="min-w-0">
          <p className="text-xs uppercase tracking-[0.26em] text-slate-400">{card.eyebrow}</p>
          <h3 className="mt-1 text-lg font-bold text-white">{title}</h3>
          <p className="mt-2 text-sm text-slate-400">{card.context}</p>
        </div>
        <span className={`shrink-0 rounded-full px-3 py-1.5 text-xs font-bold ${badgeClass}`}>
          {card.badge}
        </span>
      </div>

      <div className={`mt-4 grid gap-3 ${card.metrics.length >= 5 ? "sm:grid-cols-2 xl:grid-cols-5" : "sm:grid-cols-2 xl:grid-cols-4"}`}>
        {card.metrics.map((metric) => (
          <MetricTile
            key={metric.label}
            label={metric.label}
            toneClass={metric.primary ? summaryToneClass : "text-white"}
            value={metric.value}
          />
        ))}
      </div>

      <div className="mt-4 space-y-3">
        {card.breakdowns.map((item) => (
          <ScoreBar key={item.label} label={item.label} value={item.value} />
        ))}
      </div>

      {card.tags.length > 0 ? (
        <div className="mt-4 flex flex-wrap gap-2">
          {card.tags.map((tag) => (
            <span
              className="rounded-full border border-slate-700/80 bg-slate-900/70 px-2.5 py-1 text-xs font-semibold text-slate-200"
              key={tag}
            >
              {tag}
            </span>
          ))}
        </div>
      ) : null}

      <div className="mt-4 rounded-xl border border-slate-800/80 bg-slate-950/50 p-3">
        <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">底部原因</p>
        <ul className="mt-2 space-y-1.5 text-sm leading-6 text-slate-200">
          {card.reasons.map((reason) => (
            <li key={reason}>{reason}</li>
          ))}
        </ul>
      </div>
    </section>
  );
}

function MetricTile({ label, value, toneClass = "text-white" }) {
  return (
    <div className="rounded-xl border border-slate-800/80 bg-slate-950/50 px-3 py-3">
      <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">{label}</p>
      <p className={`mt-2 text-base font-bold ${toneClass}`}>{value}</p>
    </div>
  );
}

function ScoreBar({ label, value }) {
  const safeValue = clampScore(value);
  return (
    <div className="space-y-1.5">
      <div className="flex items-center justify-between gap-3 text-sm">
        <span className="truncate text-slate-300">{label}</span>
        <span className="shrink-0 font-semibold text-white">{safeValue}</span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-slate-800">
        <div
          className="h-full rounded-full bg-gradient-to-r from-cyan-400 via-emerald-400 to-amber-300"
          style={{ width: `${safeValue}%` }}
        />
      </div>
    </div>
  );
}

function buildToxicCard(signal, nowMs) {
  if (!signal) {
    return {
      eyebrow: "No focused signal",
      context: "当前没有可用的短线候选信号。",
      badge: "待数据",
      severity: "Calm",
      metrics: [
        { label: "毒性评分", value: "N/A", primary: true },
        { label: "短线压力", value: "中性 0" },
        { label: "有效期", value: "等待数据" },
        { label: "置信度", value: "N/A" },
      ],
      breakdowns: SHORT_BREAKDOWN_ORDER.map(([, label]) => ({ label, value: 0 })),
      reasons: ["暂无可用于短线评分的候选信号。"],
      pressure: 0,
      tags: [],
    };
  }

  const score = clampScore(signal.toxicScore ?? signal.finalRiskScore ?? signal.score);
  const severity = signal.toxicSeverity || toxicSeverityLabel(score);
  const pressure = signedNumber(signal.shortPressure) ?? inferPressure(signal.side, score);
  const confidence = normalizePercent(signal.toxicShortScore?.confidence ?? signal.confidence);
  const remaining = remainingTtlText(signal, nowMs);
  const breakdowns = buildShortBreakdowns(signal, score);

  return {
    eyebrow: `${signal.symbol || "Unknown"} · ${signal.type || "candidate"} · ${shortTime(signal.time)}`,
    context: finalResultDescription(signal),
    badge: `${score} / ${severity}`,
    severity,
    metrics: [
      { label: "毒性评分", value: `${score} / ${severity}`, primary: true },
      { label: "短线压力", value: `${pressureLabel(pressure)} ${formatSignedInteger(pressure)}` },
      { label: "有效期", value: remaining },
      { label: "置信度", value: normalizeDisplay(confidence) },
    ],
    breakdowns,
    reasons: buildShortReasons(signal, breakdowns),
    pressure,
    tags: [],
  };
}

function buildStructureCard(signal) {
  if (!signal) {
    return {
      eyebrow: "No focused signal",
      context: "当前没有可用的主力结构候选信号。",
      badge: "待数据",
      severity: "Calm",
      metrics: [
        { label: "主力评分", value: "N/A", primary: true },
        { label: "结构方向", value: "中性 0" },
        { label: "极端冲击", value: "N/A" },
        { label: "状态", value: "结构未明" },
        { label: "置信度", value: "N/A" },
      ],
      breakdowns: STRUCTURE_BREAKDOWN_ITEMS.map(([, label]) => ({ label, value: 0 })),
      reasons: ["暂无可用于主力结构评分的候选信号。"],
      tags: [],
      bias: 0,
    };
  }

  const mainForceScore = clampScore(signal.mainForceScore);
  const severity = signal.marketStructureSeverity || structureSeverityLabel(mainForceScore);
  const bias = signedNumber(signal.structureBias) ?? 0;
  const extremeImpact = clampScore(signal.extremeImpactScore);
  const regime = regimeTypeLabel(signal.regimeType);
  const confidence = normalizePercent(signal.marketStructureConfidence ?? signal.marketStructureScore?.confidence);

  return {
    eyebrow: `${signal.symbol || "Unknown"} · ${signal.type || "candidate"} · ${shortTime(signal.time)}`,
    context: finalResultDescription(signal),
    badge: `${mainForceScore} / ${severity}`,
    severity,
    metrics: [
      { label: "主力评分", value: `${mainForceScore} / ${severity}`, primary: true },
      { label: "结构方向", value: `${biasLabel(bias)} ${formatSignedInteger(bias)}` },
      { label: "极端冲击", value: normalizeDisplay(extremeImpact) },
      { label: "状态", value: regime },
      { label: "置信度", value: normalizeDisplay(confidence) },
    ],
    breakdowns: STRUCTURE_BREAKDOWN_ITEMS.map(([key, label]) => ({
      label,
      value: structureBreakdownValue(signal, key),
    })),
    reasons: buildStructureReasons(signal),
    tags: buildStructureTags(signal),
    bias,
  };
}

function buildShortBreakdowns(signal, fallbackScore) {
  const byType = new Map(
    (Array.isArray(signal.toxicReasons) ? signal.toxicReasons : []).map((reason) => [
      reason.reasonType,
      clampScore(reason.score),
    ]),
  );
  const metrics = signal.tofMetrics || {};
  return SHORT_BREAKDOWN_ORDER.map(([key, label]) => ({
    label,
    value:
      byType.get(key) ??
      shortBreakdownFallback(key, metrics, fallbackScore, signal.toxicScore ?? signal.score ?? 0),
  }));
}

function buildShortReasons(signal, breakdowns) {
  const direction = String(signal.side || "").toLowerCase();
  const metrics = signal.tofMetrics || {};
  const reasons = [];

  if (direction.includes("sell") || direction.includes("ask")) {
    reasons.push("5s 内主动卖出扫穿近端买盘");
  } else if (direction.includes("buy") || direction.includes("bid")) {
    reasons.push("5s 内主动买入扫穿近端卖盘");
  } else {
    reasons.push(finalResultDescription(signal));
  }

  if ((metrics.depthWithdrawalScore ?? 0) >= 55) {
    reasons.push(`${metrics.metricsDirection === "bullish" ? "卖盘" : "买盘"}深度快速消失`);
  }

  if ((metrics.spreadWideningScore ?? 0) >= 55 && Number.isFinite(metrics.spreadBps)) {
    reasons.push(`盘口价差快速扩大至 ${round1(metrics.spreadBps)} bps`);
  }

  if ((metrics.tradeImbalanceScore ?? 0) >= 65 && Number.isFinite(metrics.tradeImbalance)) {
    reasons.push(`主动成交方向失衡 ${signedDecimal(metrics.tradeImbalance * 100, 2)}%`);
  }

  if (reasons.length < 3) {
    breakdowns
      .filter((item) => item.value >= 70)
      .slice(0, 3 - reasons.length)
      .forEach((item) => reasons.push(`${item.label}显著抬升`));
  }

  return reasons.slice(0, 3);
}

function buildStructureReasons(signal) {
  const direction = signedNumber(signal.structureBias) ?? 0;
  const reasons = [];

  if (clampScore(signal.contractScore) >= 70) {
    reasons.push(direction >= 0 ? "合约主动买入显著放大" : "合约主动卖出显著放大");
  }
  if (clampScore(signal.oiScore ?? signal.oiImpulse) >= 70) {
    reasons.push(direction >= 0 ? "OI 同步上升，偏新多开仓" : "OI 同步上升，偏新空开仓");
  }
  if (clampScore(signal.spotScore) >= 60) {
    reasons.push(direction >= 0 ? "现货主动买入跟随" : "现货主动卖出跟随");
  }
  if (signal.regimeType === "downside_absorption") {
    reasons.push("价格回调未破，出现下方承接");
  } else if (signal.regimeType === "upside_resistance") {
    reasons.push("价格冲高未成，出现上方压制");
  } else if (signal.regimeType === "long_liquidation_cascade") {
    reasons.push("多头清算显著增加");
  }
  if (reasons.length < 4 && clampScore(signal.crossConfirmScore) >= 70) {
    reasons.push("现货与合约方向确认增强");
  }

  return reasons.slice(0, 4);
}

function buildStructureTags(signal) {
  const tags = [];
  if (signal.mainForceConfirmed) {
    tags.push("主力确认");
  }
  if (structureBreakdownValue(signal, "liquidationBlend") < 60 && signal.regimeType !== "long_liquidation_cascade") {
    tags.push("非清算驱动");
  }
  if (clampScore(signal.multiWindowConsistency) >= 70) {
    tags.push("多窗口确认");
  }
  if ((signal.cwmContribution?.exchangeCount ?? 0) >= 2) {
    tags.push("Binance + Bitfinex 同向");
  } else if (clampScore(signal.crossConfirmScore) >= 70) {
    tags.push("现货合约确认");
  }
  return tags.slice(0, 4);
}

function structureBreakdownValue(signal, key) {
  if (key === "liquidationBlend") {
    return clampScore(signal.liquidationScore ?? signal.liquidationContext);
  }
  return clampScore(signal[key]);
}

function shortBreakdownFallback(key, metrics, fallbackScore, rawScore) {
  switch (key) {
    case "ToxicOrderCluster":
      return clampScore(fallbackScore);
    case "AggressiveSweep":
      return clampScore(Math.max(Math.abs(number(metrics.tradeImbalance)) * 100, number(metrics.tradeImbalanceScore)));
    case "OrderbookDeformation":
      return clampScore(Math.max(number(metrics.depthWithdrawalScore), number(metrics.spreadWideningScore) * 0.7));
    case "SpoofCancel":
      return clampScore(Math.max(number(metrics.orderChurnScore), rawScore * 0.45));
    case "AdverseMove":
      return clampScore(rawScore * 0.85);
    case "LiquidityGap":
      return clampScore(Math.max(number(metrics.liquidityVacuumScore), number(metrics.spreadWideningScore)));
    default:
      return 0;
  }
}

function severityBadgeClass(severity) {
  if (["S", "Critical"].includes(severity)) {
    return "border border-red-500/50 bg-red-500/15 text-red-200";
  }
  if (severity === "High") {
    return "border border-orange-500/50 bg-orange-500/15 text-orange-200";
  }
  return "border border-slate-700 bg-slate-900/70 text-slate-300";
}

function structureBadgeClass(severity) {
  if (severity === "Extreme") {
    return "border border-fuchsia-500/50 bg-fuchsia-500/15 text-fuchsia-200";
  }
  if (severity === "Major") {
    return "border border-emerald-500/50 bg-emerald-500/15 text-emerald-200";
  }
  if (severity === "Confirmed") {
    return "border border-cyan-500/50 bg-cyan-500/15 text-cyan-200";
  }
  return "border border-slate-700 bg-slate-900/70 text-slate-300";
}

function pressureToneClass(pressure) {
  if (pressure >= 15) {
    return "text-emerald-200";
  }
  if (pressure <= -15) {
    return "text-red-200";
  }
  return "text-slate-100";
}

function structureToneClass(bias) {
  if (bias >= 15) {
    return "text-emerald-200";
  }
  if (bias <= -15) {
    return "text-red-200";
  }
  return "text-slate-100";
}

function toxicSeverityLabel(score) {
  if (score >= 90) {
    return "S";
  }
  if (score >= 75) {
    return "Critical";
  }
  if (score >= 60) {
    return "High";
  }
  if (score >= 40) {
    return "Watch";
  }
  return "Calm";
}

function structureSeverityLabel(score) {
  if (score >= 90) {
    return "Extreme";
  }
  if (score >= 75) {
    return "Major";
  }
  if (score >= 60) {
    return "Confirmed";
  }
  if (score >= 40) {
    return "Watch";
  }
  return "Calm";
}

function regimeTypeLabel(value) {
  return {
    main_force_long_build: "主力建多",
    main_force_short_build: "主力建空",
    contract_flow_shock: "合约冲击",
    spot_accumulation: "现货吸筹",
    spot_distribution: "现货派发",
    contract_short_squeeze: "空头挤压",
    long_liquidation_cascade: "多头清算瀑布",
    downside_absorption: "下方吸收",
    upside_resistance: "上方压制",
    range_rotation: "高换手震荡",
    unclear: "结构未明",
  }[value] || "结构未明";
}

function pressureLabel(value) {
  if (value >= 15) {
    return "偏多";
  }
  if (value <= -15) {
    return "偏空";
  }
  return "中性";
}

function biasLabel(value) {
  if (value >= 15) {
    return "偏多";
  }
  if (value <= -15) {
    return "偏空";
  }
  return "中性";
}

function inferPressure(side, score) {
  const direction = String(side || "").toLowerCase();
  if (direction.includes("sell") || direction.includes("ask")) {
    return -clampScore(score);
  }
  if (direction.includes("buy") || direction.includes("bid")) {
    return clampScore(score);
  }
  return 0;
}

function remainingTtlText(signal, nowMs) {
  const expiresAt = number(signal.toxicExpiresAt);
  const ttlSec = number(signal.toxicTtlSec);
  if (expiresAt > 0) {
    return `剩余 ${Math.max(0, Math.ceil((expiresAt - nowMs) / 1000))} 秒`;
  }
  if (ttlSec > 0) {
    return `约 ${Math.round(ttlSec)} 秒`;
  }
  return "等待数据";
}

function shortTime(value) {
  if (!value) {
    return "--:--:--";
  }
  const text = String(value);
  const match = text.match(/(\d{2}:\d{2}:\d{2})/);
  return match ? match[1] : text;
}

function normalizeDisplay(value) {
  const safe = number(value);
  return safe > 0 || safe === 0 ? `${Math.round(safe)}` : "N/A";
}

function normalizePercent(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? Math.max(0, Math.min(100, parsed)) : null;
}

function formatSignedInteger(value) {
  const safe = signedNumber(value);
  if (safe === null) {
    return "0";
  }
  return safe > 0 ? `+${Math.round(safe)}` : `${Math.round(safe)}`;
}

function signedDecimal(value, digits = 1) {
  const safe = number(value);
  if (!Number.isFinite(safe)) {
    return "0";
  }
  const fixed = safe.toFixed(digits);
  return safe > 0 ? `+${fixed}` : fixed;
}

function round1(value) {
  return Math.round(number(value) * 10) / 10;
}

function clampScore(value) {
  const safe = number(value);
  if (!Number.isFinite(safe)) {
    return 0;
  }
  return Math.max(0, Math.min(100, Math.round(safe)));
}

function number(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function signedNumber(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}
