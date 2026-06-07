import axios from "axios";
import { mockSignals } from "../data/mockSignals.js";

export async function fetchSignals() {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const response = await axios.get(`${baseURL}/api/toxicity/signal-inbox/recent`);
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    if (items.length === 0) {
      return demoSignalsForEmptyInbox();
    }
    return items.map(mapInboxItemToSignal);
  } catch {
    return demoSignalsForEmptyInbox();
  }
}

function demoSignalsForEmptyInbox() {
  return mockSignals.map((signal) => ({
    ...signal,
    isLive: false,
  }));
}

export function mapInboxItemToSignal(item) {
  const signalId = item.signalId ?? item.id;
  const signalKind = item.signalKind ?? item.detector;
  const directionBias = item.directionBias ?? item.direction;
  const createdAtMs = item.createdAtMs ?? Date.parse(item.createdAt || "");
  const score = Number.isFinite(Number(item.finalRiskScore))
    ? Number(item.finalRiskScore)
    : Number.isFinite(Number(item.advancedTofMetrics?.finalRiskScore))
      ? Number(item.advancedTofMetrics.finalRiskScore)
    : Number.isFinite(Number(item.riskScore))
      ? Number(item.riskScore)
      : scoreFromSeverity(item.severity);
  const dataQuality = Number.isFinite(Number(item.dataQuality))
    ? Number(item.dataQuality)
    : Number.isFinite(Number(item.advancedTofMetrics?.dataQuality))
      ? Number(item.advancedTofMetrics.dataQuality)
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
    perpTofMetrics: normalizePerpTofMetrics(item.perpTofMetrics),
    perpScore: numberOrNull(item.perpScore),
    perpCandidateType: item.perpCandidateType || item.perpTofMetrics?.candidateType || null,
    advancedTofMetrics: normalizeAdvancedTofMetrics(item.advancedTofMetrics),
    advancedScore: numberOrNull(item.advancedScore),
    advancedCandidateType: item.advancedCandidateType || item.advancedTofMetrics?.candidateType || null,
    finalCandidateType: item.finalCandidateType || null,
    metricsDirection:
      item.metricsDirection || item.advancedTofMetrics?.metricsDirection || item.perpTofMetrics?.metricsDirection || null,
    mergedConfidence: numberOrNull(item.mergedConfidence),
    finalRiskScore: numberOrNull(item.finalRiskScore ?? item.advancedTofMetrics?.finalRiskScore),
    candidateType: item.candidateType || item.signalKind || item.detector || "toxic_flow_candidate",
    explainTags: Array.isArray(item.explainTags) ? item.explainTags.filter((tag) => typeof tag === "string") : [],
    directionLabel: item.directionLabel || directionLabel(directionBias),
    directionConfidence: numberOrNull(item.directionConfidence),
    directionSource: item.directionSource || "detector",
    alertStatus: item.alertStatus || item.discordAlert?.lastDecision || "not_evaluated",
    alertReason: item.alertReason || item.discordAlert?.reason || null,
    discordAlert: normalizeDiscordAlert(item.discordAlert),
    replaySnapshot: redactSnapshot(item.replaySnapshot || item.redactedReplaySnapshot || item.replay?.snapshot),
    status: "unhandled",
    pushedAt: null,
    isLive: true,
  };
}

function redactSnapshot(value) {
  if (Array.isArray(value)) {
    return value.map(redactSnapshot);
  }
  if (!value || typeof value !== "object") {
    return value ?? null;
  }
  const forbidden = new Set([
    "rawpayload",
    "evidence",
    "markout",
    "token",
    "webhook",
    "authorization",
    "apikey",
    "secret",
  ]);
  return Object.fromEntries(
    Object.entries(value)
      .filter(([key]) => !forbidden.has(key.toLowerCase()))
      .map(([key, item]) => [key, redactSnapshot(item)]),
  );
}

function normalizeAdvancedTofMetrics(metrics) {
  if (!metrics || typeof metrics !== "object") {
    return null;
  }
  return {
    vpinEnhanced: numberOrNull(metrics.vpinEnhanced),
    largeOrderFlowCluster: numberOrNull(metrics.largeOrderFlowCluster),
    historicalFundingOiTrend: numberOrNull(metrics.historicalFundingOiTrend),
    marketPressureHeatmap: numberOrNull(metrics.marketPressureHeatmap),
    spotRiskScore: numberOrNull(metrics.spotRiskScore),
    spotTofScore: numberOrNull(metrics.spotTofScore),
    perpScore: numberOrNull(metrics.perpScore),
    finalRiskScore: numberOrNull(metrics.finalRiskScore),
    dataQuality: numberOrNull(metrics.dataQuality),
    metricsCompleteness: numberOrNull(metrics.metricsCompleteness),
    freshDataCoverage: numberOrNull(metrics.freshDataCoverage),
    candidateType: typeof metrics.candidateType === "string" ? metrics.candidateType : "AdvancedTofCandidate",
    finalCandidateType: typeof metrics.finalCandidateType === "string" ? metrics.finalCandidateType : null,
    metricsDirection: typeof metrics.metricsDirection === "string" ? metrics.metricsDirection : "neutral",
    confidence: numberOrNull(metrics.confidence),
    explainTags: Array.isArray(metrics.explainTags) ? metrics.explainTags.filter((tag) => typeof tag === "string") : [],
  };
}

function normalizePerpTofMetrics(metrics) {
  if (!metrics || typeof metrics !== "object") {
    return null;
  }
  return {
    oiChange: numberOrNull(metrics.oiChange),
    oiDirection: typeof metrics.oiDirection === "string" ? metrics.oiDirection : "neutral",
    fundingRate: numberOrNull(metrics.fundingRate),
    fundingSide: typeof metrics.fundingSide === "string" ? metrics.fundingSide : "neutral",
    liquidationPressure: numberOrNull(metrics.liquidationPressure),
    squeezeSide: typeof metrics.squeezeSide === "string" ? metrics.squeezeSide : "neutral",
    aggBuyVolume: numberOrNull(metrics.aggBuyVolume),
    aggSellVolume: numberOrNull(metrics.aggSellVolume),
    directionBias: typeof metrics.directionBias === "string" ? metrics.directionBias : "neutral",
    metricsDirection: typeof metrics.metricsDirection === "string" ? metrics.metricsDirection : "neutral",
    riskScore: numberOrNull(metrics.riskScore),
    dataQuality: numberOrNull(metrics.dataQuality),
    candidateType: typeof metrics.candidateType === "string" ? metrics.candidateType : "PerpTofCandidate",
    explainTags: Array.isArray(metrics.explainTags) ? metrics.explainTags.filter((tag) => typeof tag === "string") : [],
    confidence: numberOrNull(metrics.confidence),
  };
}

function normalizeDiscordAlert(alert) {
  if (!alert || typeof alert !== "object") {
    return {
      autoEligible: false,
      autoSent: false,
      lastDecision: "not_evaluated",
      reason: null,
      sentAt: null,
      manualSentAt: null,
    };
  }
  return {
    autoEligible: Boolean(alert.autoEligible),
    autoSent: Boolean(alert.autoSent),
    lastDecision: typeof alert.lastDecision === "string" ? alert.lastDecision : "not_evaluated",
    reason: typeof alert.reason === "string" ? alert.reason : null,
    sentAt: typeof alert.sentAt === "string" ? alert.sentAt : null,
    manualSentAt: typeof alert.manualSentAt === "string" ? alert.manualSentAt : null,
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
