import axios from "axios";
import { mockSignals } from "../data/mockSignals.js";

export async function fetchSignals() {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const response = await axios.get(`${baseURL}/api/toxicity/signal-inbox/recent`);
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    return items.map(mapInboxItemToSignal);
  } catch {
    return mockSignals;
  }
}

export function mapInboxItemToSignal(item) {
  const signalId = item.signalId ?? item.id;
  const signalKind = item.signalKind ?? item.detector;
  const directionBias = item.directionBias ?? item.direction;
  const createdAtMs = item.createdAtMs ?? Date.parse(item.createdAt || "");
  const score = Number.isFinite(Number(item.finalRiskScore))
    ? Number(item.finalRiskScore)
    : Number.isFinite(Number(item.riskScore))
      ? Number(item.riskScore)
      : scoreFromSeverity(item.severity);
  const dataQuality = Number.isFinite(Number(item.dataQuality))
    ? Number(item.dataQuality)
    : dataQualityFromBucket(item.quality?.qualityBucket ?? item.qualityBucket);
  return {
    id: signalId,
    dedupeKey: signalId,
    time: formatTime(createdAtMs),
    exchange: "Runtime",
    symbol: item.symbol,
    type: signalKind,
    side: directionLabel(directionBias),
    reason: item.fusion?.summary || item.coreReason || item.finalResult || item.recommendation?.action || "candidate signal",
    finalResult: finalResultFromInboxItem(item),
    level: levelFromSeverity(item.severity),
    risk: riskFromSeverity(item.severity),
    score,
    confidence: Math.round(Number(item.confidence || 0) * 100),
    dataQuality,
    tofMetrics: normalizeTofMetrics(item.tofMetrics),
    tofScore: numberOrNull(item.tofScore),
    finalRiskScore: numberOrNull(item.finalRiskScore),
    candidateType: item.candidateType || item.signalKind || item.detector || "toxic_flow_candidate",
    explainTags: Array.isArray(item.explainTags) ? item.explainTags.filter((tag) => typeof tag === "string") : [],
    directionLabel: item.directionLabel || directionLabel(directionBias),
    directionConfidence: numberOrNull(item.directionConfidence),
    directionSource: item.directionSource || "detector",
    status: "unhandled",
    pushedAt: null,
    isLive: true,
  };
}

function finalResultFromInboxItem(item) {
  if (item.finalResult) {
    return item.finalResult;
  }
  const direction = directionLabel(item.directionBias ?? item.direction);
  const reason = item.fusion?.summary || "候选信号需要人工复核";
  return `${direction} · ${reason}`;
}

function riskFromSeverity(severity) {
  const value = String(severity || "").toLowerCase();
  if (value === "critical" || value === "high") return "high";
  if (value === "medium") return "medium";
  return "low";
}

function levelFromSeverity(severity) {
  const value = String(severity || "").toLowerCase();
  if (value === "critical") return "CRITICAL";
  if (value === "high") return "A";
  if (value === "medium") return "B";
  return "D";
}

function scoreFromSeverity(severity) {
  const value = String(severity || "").toLowerCase();
  if (value === "critical") return 92;
  if (value === "high") return 85;
  if (value === "medium") return 72;
  return 45;
}

function dataQualityFromBucket(bucket) {
  const value = String(bucket || "").toLowerCase();
  if (value === "excellent") return 92;
  if (value === "good") return 82;
  if (value === "mixed") return 74;
  if (value === "weak") return 62;
  if (value === "bad") return 45;
  return 70;
}

function directionLabel(directionBias) {
  const value = String(directionBias || "").toLowerCase();
  if (value.includes("bearish")) return "Ask/Sell";
  if (value.includes("bullish")) return "Bid/Buy";
  if (value.includes("mixed")) return "Mixed";
  if (value.includes("short")) return "Ask/Sell";
  if (value.includes("long")) return "Bid/Buy";
  if (value.includes("trap")) return "Trap Risk";
  return "Neutral";
}

function normalizeTofMetrics(metrics) {
  if (!metrics || typeof metrics !== "object") {
    return null;
  }
  return {
    tradeImbalance: numberOrNull(metrics.tradeImbalance),
    tradeImbalanceScore: numberOrNull(metrics.tradeImbalanceScore),
    vpinProxy: numberOrNull(metrics.vpinProxy),
    vpinBucketCount: numberOrNull(metrics.vpinBucketCount),
    vpinWindowVolume: numberOrNull(metrics.vpinWindowVolume),
    bidDepthWithdrawal: numberOrNull(metrics.bidDepthWithdrawal),
    askDepthWithdrawal: numberOrNull(metrics.askDepthWithdrawal),
    depthWithdrawalScore: numberOrNull(metrics.depthWithdrawalScore),
    spreadBps: numberOrNull(metrics.spreadBps),
    spreadWideningScore: numberOrNull(metrics.spreadWideningScore),
    orderChurnScore: numberOrNull(metrics.orderChurnScore),
    liquidityVacuumScore: numberOrNull(metrics.liquidityVacuumScore),
    thinSide: typeof metrics.thinSide === "string" ? metrics.thinSide : "none",
    metricsDirection: typeof metrics.metricsDirection === "string" ? metrics.metricsDirection : "neutral",
    metricsConfidence: numberOrNull(metrics.metricsConfidence),
    tofScore: numberOrNull(metrics.tofScore),
    finalRiskScore: numberOrNull(metrics.finalRiskScore),
    metricsCompleteness: numberOrNull(metrics.metricsCompleteness),
  };
}

function numberOrNull(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function formatTime(ms) {
  const value = Number(ms);
  if (!Number.isFinite(value) || value <= 0) {
    return "";
  }
  return new Date(value).toLocaleString("zh-CN", { hour12: false }).replace(/\//g, "-");
}
