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
  const riskSystems = normalizeRiskSystems(item.riskSystems);
  const toxicShortScore = normalizeToxicShortScore(item.toxicShortScore ?? riskSystems.shortTermToxic);
  const marketStructureScore = normalizeMarketStructureScore(
    item.marketStructureScore ?? riskSystems.marketStructureScore ?? riskSystems.mainForceStructure,
  );
  const score =
    numberOrNull(item.toxicScore) ??
    numberOrNull(toxicShortScore?.toxicScore) ??
    numberOrNull(item.finalRiskScore) ??
    numberOrNull(item.advancedTofMetrics?.finalRiskScore) ??
    numberOrNull(item.riskScore) ??
    scoreFromSeverity(item.severity);
  const dataQuality =
    numberOrNull(item.dataQuality) ??
    numberOrNull(toxicShortScore?.dataQuality) ??
    numberOrNull(item.advancedTofMetrics?.dataQuality) ??
    dataQualityFromBucket(item.quality?.qualityBucket ?? item.qualityBucket);
  const confidence = confidencePercent(
    item.confidence ??
      toxicShortScore?.confidence ??
      riskSystems.shortTermToxic?.confidence ??
      item.directionConfidence,
  );
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
    confidence,
    dataQuality,
    tofMetrics: normalizeTofMetrics(item.tofMetrics),
    tofScore: numberOrNull(item.tofScore),
    perpTofMetrics: normalizePerpTofMetrics(item.perpTofMetrics),
    perpScore: numberOrNull(item.perpScore),
    perpCandidateType: item.perpCandidateType || item.perpTofMetrics?.candidateType || null,
    advancedTofMetrics: normalizeAdvancedTofMetrics(item.advancedTofMetrics),
    advancedScore: numberOrNull(item.advancedScore),
    advancedCandidateType: item.advancedCandidateType || item.advancedTofMetrics?.candidateType || null,
    cwmContribution: normalizeCwmContribution(item.cwmContribution),
    riskSystems,
    toxicShortScore,
    marketStructureScore,
    toxicScore: numberOrNull(item.toxicScore ?? toxicShortScore?.toxicScore ?? item.finalRiskScore),
    shortPressure: numberOrNull(item.shortPressure ?? toxicShortScore?.shortPressure),
    toxicSeverity: item.toxicSeverity || toxicShortScore?.severity || null,
    toxicType: item.toxicType || toxicShortScore?.toxicType || null,
    toxicTtlSec: numberOrNull(item.toxicTtlSec ?? toxicShortScore?.ttlSec),
    toxicExpiresAt: numberOrNull(item.toxicExpiresAt ?? toxicShortScore?.expiresAt),
    toxicHalfLifeSec: numberOrNull(item.toxicHalfLifeSec ?? toxicShortScore?.halfLifeSec),
    toxicMaxTtlSec: numberOrNull(item.toxicMaxTtlSec ?? toxicShortScore?.maxTtlSec),
    toxicDecayedScore: numberOrNull(item.toxicDecayedScore ?? toxicShortScore?.decayedScore),
    toxicDecayFormula: item.toxicDecayFormula || toxicShortScore?.decayFormula || null,
    toxicReasons: normalizeToxicReasons(item.toxicReasons ?? toxicShortScore?.reasons),
    mainForceScore: numberOrNull(item.mainForceScore ?? marketStructureScore?.mainForceScore),
    mainForceConfirmed:
      typeof (item.mainForceConfirmed ?? marketStructureScore?.mainForceConfirmed) === "boolean"
        ? (item.mainForceConfirmed ?? marketStructureScore?.mainForceConfirmed)
        : false,
    mainForceConfirmationCount: numberOrNull(
      item.mainForceConfirmationCount ?? marketStructureScore?.mainForceConfirmationCount,
    ),
    mainForceConfirmationTotal: numberOrNull(
      item.mainForceConfirmationTotal ?? marketStructureScore?.mainForceConfirmationTotal,
    ),
    mainForceConfirmationThreshold: numberOrNull(
      item.mainForceConfirmationThreshold ?? marketStructureScore?.mainForceConfirmationThreshold,
    ),
    structureBias: numberOrNull(item.structureBias ?? marketStructureScore?.structureBias),
    extremeImpactScore: numberOrNull(item.extremeImpactScore ?? marketStructureScore?.extremeImpactScore),
    extremeImpactConfirmed:
      typeof (item.extremeImpactConfirmed ?? marketStructureScore?.extremeImpactConfirmed) === "boolean"
        ? (item.extremeImpactConfirmed ?? marketStructureScore?.extremeImpactConfirmed)
        : false,
    regimeType: item.regimeType || marketStructureScore?.regimeType || null,
    marketStructureSeverity: item.marketStructureSeverity || marketStructureScore?.severity || null,
    marketStructureConfidence: numberOrNull(
      item.marketStructureConfidence ?? marketStructureScore?.confidence,
    ),
    marketStructureDataQuality: numberOrNull(item.marketStructureDataQuality ?? marketStructureScore?.dataQuality),
    structureRaw: numberOrNull(item.structureRaw ?? marketStructureScore?.structureRaw),
    spotContractFloor: numberOrNull(item.spotContractFloor ?? marketStructureScore?.spotContractFloor),
    durationScore: numberOrNull(item.durationScore ?? marketStructureScore?.durationScore),
    liquidationPenalty: numberOrNull(item.liquidationPenalty ?? marketStructureScore?.liquidationPenalty),
    crowdingPenalty: numberOrNull(item.crowdingPenalty ?? marketStructureScore?.crowdingPenalty),
    spotScore: numberOrNull(item.spotScore ?? marketStructureScore?.spotScore),
    spotCvdScore: numberOrNull(item.spotCvdScore ?? marketStructureScore?.spotCvdScore),
    spotVolumeAnomaly: numberOrNull(item.spotVolumeAnomaly ?? marketStructureScore?.spotVolumeAnomaly),
    spotAbsorption: numberOrNull(item.spotAbsorption ?? marketStructureScore?.spotAbsorption),
    spotLiquidityShift: numberOrNull(item.spotLiquidityShift ?? marketStructureScore?.spotLiquidityShift),
    spotPriceResponse: numberOrNull(item.spotPriceResponse ?? marketStructureScore?.spotPriceResponse),
    contractScore: numberOrNull(item.contractScore ?? marketStructureScore?.contractScore),
    cwmAggressiveFlow: numberOrNull(item.cwmAggressiveFlow ?? marketStructureScore?.cwmAggressiveFlow),
    oiImpulse: numberOrNull(item.oiImpulse ?? marketStructureScore?.oiImpulse),
    liquidationContext: numberOrNull(item.liquidationContext ?? marketStructureScore?.liquidationContext),
    fundingCrowding: numberOrNull(item.fundingCrowding ?? marketStructureScore?.fundingCrowding),
    basisPremium: numberOrNull(item.basisPremium ?? marketStructureScore?.basisPremium),
    activeExchangeConfirmation: numberOrNull(
      item.activeExchangeConfirmation ?? marketStructureScore?.activeExchangeConfirmation,
    ),
    crossConfirmScore: numberOrNull(item.crossConfirmScore ?? marketStructureScore?.crossConfirmScore),
    spotContractDirectionConsistency: numberOrNull(
      item.spotContractDirectionConsistency ?? marketStructureScore?.spotContractDirectionConsistency,
    ),
    multiWindowConsistency: numberOrNull(item.multiWindowConsistency ?? marketStructureScore?.multiWindowConsistency),
    priceResponseConsistency: numberOrNull(
      item.priceResponseConsistency ?? marketStructureScore?.priceResponseConsistency,
    ),
    sourceCoverage: numberOrNull(item.sourceCoverage ?? marketStructureScore?.sourceCoverage),
    signalAgreement: numberOrNull(item.signalAgreement ?? marketStructureScore?.signalAgreement),
    oiScore: numberOrNull(item.oiScore ?? marketStructureScore?.oiScore),
    liquidationScore: numberOrNull(item.liquidationScore ?? marketStructureScore?.liquidationScore),
    fundingCrowdingScore: numberOrNull(item.fundingCrowdingScore ?? marketStructureScore?.fundingCrowdingScore),
    cwmScore: numberOrNull(item.cwmScore ?? marketStructureScore?.cwmScore),
    marketStructureReasons: normalizeMarketStructureReasons(item.marketStructureReasons ?? marketStructureScore?.reasons),
    finalCandidateType: item.finalCandidateType || null,
    metricsDirection:
      item.metricsDirection || item.advancedTofMetrics?.metricsDirection || item.perpTofMetrics?.metricsDirection || null,
    mergedConfidence: numberOrNull(item.mergedConfidence),
    finalRiskScore: numberOrNull(item.finalRiskScore ?? item.toxicScore ?? riskSystems.shortTermToxic?.toxicScore),
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

function normalizeCwmContribution(contribution) {
  if (!contribution || typeof contribution !== "object") {
    return {
      available: false,
      source: "contract_whale_monitor",
      formula: "MarketStructureScore: spotScore and contractScore are separate composites; contractScore = 0.30*CwmAggressiveFlow + 0.20*OiImpulse + 0.15*LiquidationContext + 0.15*FundingCrowding + 0.10*BasisPremium + 0.10*ActiveExchangeConfirmation; crossConfirmScore = 0.40*SpotContractDirectionConsistency + 0.25*MultiWindowConsistency + 0.20*PriceResponseConsistency + 0.15*SourceCoverage; mainForceScore uses structureRaw, min(spotScore, contractScore), durationScore, liquidationPenalty, and crowdingPenalty; not fused into toxicScore",
      contributionWeight: 0.25,
      score: null,
      weightedContribution: 0,
      summary: "CWM signal unavailable",
      discordGateIndependent: true,
    };
  }
  return {
    available: Boolean(contribution.available),
    source: typeof contribution.source === "string" ? contribution.source : "contract_whale_monitor",
    formula: typeof contribution.formula === "string" ? contribution.formula : "",
    contributionWeight: numberOrNull(contribution.contributionWeight),
    score: numberOrNull(contribution.score),
    weightedContribution: numberOrNull(contribution.weightedContribution) || 0,
    signalId: typeof contribution.signalId === "string" ? contribution.signalId : null,
    severity: typeof contribution.severity === "string" ? contribution.severity : null,
    signalType: typeof contribution.signalType === "string" ? contribution.signalType : null,
    direction: typeof contribution.direction === "string" ? contribution.direction : null,
    windowSec: numberOrNull(contribution.windowSec),
    dataQuality: numberOrNull(contribution.dataQuality),
    dominance: numberOrNull(contribution.dominance),
    mainExchange: typeof contribution.mainExchange === "string" ? contribution.mainExchange : null,
    exchangeCount: numberOrNull(contribution.exchangeCount),
    summary: typeof contribution.summary === "string" ? contribution.summary : "CWM contribution",
    discordGateIndependent: contribution.discordGateIndependent !== false,
  };
}

function normalizeRiskSystems(systems) {
  const source = systems && typeof systems === "object" ? systems : {};
  const shortTerm = source.shortTermToxic && typeof source.shortTermToxic === "object"
    ? source.shortTermToxic
    : {};
  const mainForceSource = source.marketStructureScore && typeof source.marketStructureScore === "object"
    ? source.marketStructureScore
    : source.mainForceStructure;
  const mainForce = normalizeMarketStructureScore(mainForceSource);
  return {
    shortTermToxic: normalizeToxicShortScore(shortTerm),
    marketStructureScore: mainForce,
    mainForceStructure: mainForce,
  };
}

function normalizeMarketStructureScore(structure) {
  const source = structure && typeof structure === "object" ? structure : {};
  return {
    ts: numberOrNull(source.ts),
    symbol: typeof source.symbol === "string" ? source.symbol : null,
    mainForceScore: numberOrNull(source.mainForceScore),
    mainForceConfirmed: typeof source.mainForceConfirmed === "boolean" ? source.mainForceConfirmed : false,
    mainForceConfirmationCount: numberOrNull(source.mainForceConfirmationCount),
    mainForceConfirmationTotal: numberOrNull(source.mainForceConfirmationTotal),
    mainForceConfirmationThreshold: numberOrNull(source.mainForceConfirmationThreshold),
    extremeImpactScore: numberOrNull(source.extremeImpactScore),
    extremeImpactConfirmed: typeof source.extremeImpactConfirmed === "boolean" ? source.extremeImpactConfirmed : false,
    structureBias: numberOrNull(source.structureBias),
    confidence: numberOrNull(source.confidence),
    dataQuality: numberOrNull(source.dataQuality),
    severity: typeof source.severity === "string" ? source.severity : null,
    regimeType: typeof source.regimeType === "string" ? source.regimeType : null,
    structureRaw: numberOrNull(source.structureRaw),
    spotContractFloor: numberOrNull(source.spotContractFloor),
    durationScore: numberOrNull(source.durationScore),
    liquidationPenalty: numberOrNull(source.liquidationPenalty),
    crowdingPenalty: numberOrNull(source.crowdingPenalty),
    spotScore: numberOrNull(source.spotScore),
    spotCvdScore: numberOrNull(source.spotCvdScore),
    spotVolumeAnomaly: numberOrNull(source.spotVolumeAnomaly),
    spotAbsorption: numberOrNull(source.spotAbsorption),
    spotLiquidityShift: numberOrNull(source.spotLiquidityShift),
    spotPriceResponse: numberOrNull(source.spotPriceResponse),
    contractScore: numberOrNull(source.contractScore),
    cwmAggressiveFlow: numberOrNull(source.cwmAggressiveFlow),
    oiImpulse: numberOrNull(source.oiImpulse),
    liquidationContext: numberOrNull(source.liquidationContext),
    fundingCrowding: numberOrNull(source.fundingCrowding),
    basisPremium: numberOrNull(source.basisPremium),
    activeExchangeConfirmation: numberOrNull(source.activeExchangeConfirmation),
    crossConfirmScore: numberOrNull(source.crossConfirmScore),
    spotContractDirectionConsistency: numberOrNull(source.spotContractDirectionConsistency),
    multiWindowConsistency: numberOrNull(source.multiWindowConsistency),
    priceResponseConsistency: numberOrNull(source.priceResponseConsistency),
    sourceCoverage: numberOrNull(source.sourceCoverage),
    signalAgreement: numberOrNull(source.signalAgreement),
    oiScore: numberOrNull(source.oiScore),
    liquidationScore: numberOrNull(source.liquidationScore),
    fundingCrowdingScore: numberOrNull(source.fundingCrowdingScore),
    cwmScore: numberOrNull(source.cwmScore),
    reasons: normalizeMarketStructureReasons(source.reasons),
    timeframes: normalizeStringList(source.timeframes),
    formula: typeof source.formula === "string" ? source.formula : "",
    cwmContribution: normalizeCwmContribution(source.cwmContribution),
    discordGateIndependent: source.discordGateIndependent !== false,
  };
}

function normalizeToxicShortScore(shortTerm) {
  const source = shortTerm && typeof shortTerm === "object" ? shortTerm : {};
  return {
    ts: numberOrNull(source.ts),
    symbol: typeof source.symbol === "string" ? source.symbol : null,
    toxicScore: numberOrNull(source.toxicScore),
    shortPressure: numberOrNull(source.shortPressure),
    confidence: numberOrNull(source.confidence),
    dataQuality: numberOrNull(source.dataQuality),
    severity: typeof source.severity === "string" ? source.severity : null,
    toxicType: typeof source.toxicType === "string" ? source.toxicType : null,
    ttlSec: numberOrNull(source.ttlSec),
    expiresAt: numberOrNull(source.expiresAt),
    halfLifeSec: numberOrNull(source.halfLifeSec),
    maxTtlSec: numberOrNull(source.maxTtlSec),
    decayedScore: numberOrNull(source.decayedScore),
    decayFormula: typeof source.decayFormula === "string" ? source.decayFormula : "",
    reasons: normalizeToxicReasons(source.reasons),
    timeframes: normalizeStringList(source.timeframes),
    formula: typeof source.formula === "string" ? source.formula : "",
    discordGate: typeof source.discordGate === "string" ? source.discordGate : "",
  };
}

function normalizeToxicReasons(reasons) {
  if (!Array.isArray(reasons)) {
    return [];
  }
  return reasons
    .filter((reason) => reason && typeof reason === "object")
    .map((reason) => ({
      reasonType: typeof reason.reasonType === "string" ? reason.reasonType : "unknown",
      score: numberOrNull(reason.score),
      weight: numberOrNull(reason.weight),
      windowSec: numberOrNull(reason.windowSec),
      direction: typeof reason.direction === "string" ? reason.direction : "neutral",
      description: typeof reason.description === "string" ? reason.description : "",
    }));
}

function normalizeMarketStructureReasons(reasons) {
  if (!Array.isArray(reasons)) {
    return [];
  }
  return reasons
    .filter((reason) => reason && typeof reason === "object")
    .map((reason) => ({
      reasonType: typeof reason.reasonType === "string" ? reason.reasonType : "unknown",
      score: numberOrNull(reason.score),
      weight: numberOrNull(reason.weight),
      timeframe: typeof reason.timeframe === "string" ? reason.timeframe : "",
      direction: typeof reason.direction === "string" ? reason.direction : "neutral",
      description: typeof reason.description === "string" ? reason.description : "",
    }));
}

function normalizeStringList(value) {
  return Array.isArray(value) ? value.filter((item) => typeof item === "string") : [];
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
  if (value === null || value === undefined || value === "") {
    return null;
  }
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function confidencePercent(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return null;
  }
  return number <= 1 ? Math.round(number * 100) : Math.round(number);
}

function formatTime(ms) {
  const value = Number(ms);
  if (!Number.isFinite(value) || value <= 0) {
    return "";
  }
  return new Date(value).toLocaleString("zh-CN", { hour12: false }).replace(/\//g, "-");
}
