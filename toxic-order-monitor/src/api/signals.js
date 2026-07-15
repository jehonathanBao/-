import axios from "axios";
import { mockSignals } from "../data/mockSignals.js";

export async function fetchSignalsSnapshot() {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  const fetchedAtMs = Date.now();
  try {
    const response = await axios.get(`${baseURL}/api/toxicity/signal-inbox/recent`);
    if (!response.data || typeof response.data !== "object" || !Array.isArray(response.data.items)) {
      return errorSnapshot("MALFORMED_RESPONSE", fetchedAtMs);
    }
    const runtime = runtimeFromPayload(response.data, fetchedAtMs);
    const demoSignals = response.data.items.length === 0 ? demoSignalsIfEnabled() : [];
    const source = demoSignals.length > 0 ? "cache" : "backend";
    const items = demoSignals.length > 0 ? demoSignals : response.data.items.map(mapInboxItemToSignal);
    return {
      signals: items.map((signal) => ({
        ...signal,
        runtimeBoundary: runtime,
      })),
      request: {
        phase: "ready",
        source,
        errorCode: null,
        fetchedAtMs,
      },
      runtime,
    };
  } catch (error) {
    return errorSnapshot(requestErrorCode(error), fetchedAtMs);
  }
}

export async function fetchSignals() {
  return (await fetchSignalsSnapshot()).signals;
}

function demoSignalsIfEnabled() {
  if (import.meta.env.VITE_USE_DEMO_SIGNALS !== "true") {
    return [];
  }
  return mockSignals.map((signal) => ({
    ...signal,
    isLive: false,
  }));
}

export function runtimeFromPayload(payload, checkedAtMs = Date.now()) {
  const source = payload && typeof payload === "object" ? payload : {};
  const readOnly = booleanOrNull(source.readOnly);
  const monitoringStarted = booleanOrNull(source.monitoringStarted);
  const executionEnabled = booleanOrNull(source.executionEnabled);
  const runtimeModified = booleanOrNull(source.runtimeModified);
  const analysisOnly = booleanOrNull(source.analysisOnly);
  const confirmed =
    readOnly !== null &&
    monitoringStarted !== null &&
    executionEnabled !== null &&
    runtimeModified !== null &&
    analysisOnly !== null;
  return {
    phase: confirmed ? "confirmed" : "unavailable",
    readOnly,
    monitoringStarted,
    executionEnabled,
    runtimeModified,
    analysisOnly,
    checkedAtMs,
  };
}

function errorSnapshot(errorCode, fetchedAtMs) {
  return {
    signals: [],
    request: {
      phase: "error",
      source: null,
      errorCode,
      fetchedAtMs,
    },
    runtime: runtimeFromPayload(null, fetchedAtMs),
  };
}

function requestErrorCode(error) {
  const status = Number(error?.response?.status);
  if (Number.isInteger(status) && status > 0) {
    return `HTTP_${status}`;
  }
  return "NETWORK_ERROR";
}

export function mapInboxItemToSignal(item) {
  const signalId = item.signalId ?? item.id;
  const signalKind = item.signalKind ?? item.detector;
  const directionBias = item.directionBias ?? item.direction;
  const createdAtMs = item.createdAtMs ?? Date.parse(item.createdAt || "");
  const normalizedRiskSystems = normalizeRiskSystems(item.riskSystems);
  const toxicShortScore = normalizeToxicShortScore(item.toxicShortScore ?? normalizedRiskSystems.shortTermToxic);
  const marketStructureScore = normalizeMarketStructureScore(
    ownValueOrFallback(
      item,
      "marketStructureScore",
      normalizedRiskSystems.marketStructureScore ?? normalizedRiskSystems.mainForceStructure,
    ),
  );
  const riskSystems = {
    ...normalizedRiskSystems,
    marketStructureScore,
    mainForceStructure: marketStructureScore,
  };
  const authoritativeRiskScore = numberOrNull(item.riskScore);
  const authoritativeDataQuality = numberOrNull(item.dataQualityScore);
  const score = authoritativeRiskScore;
  const dataQuality = authoritativeDataQuality ?? numberOrNull(item.dataQuality);
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
    riskScore: authoritativeRiskScore,
    authoritativeRiskScore,
    confidence,
    dataQuality,
    dataQualityScore: authoritativeDataQuality,
    authoritativeDataQuality,
    triggerPriceUsd: normalizeSignalPrice(item),
    tofMetrics: normalizeTofMetrics(item.tofMetrics),
    tofScore: numberOrNull(item.tofScore),
    perpTofMetrics: normalizePerpTofMetrics(item.perpTofMetrics),
    perpScore: numberOrNull(item.perpScore),
    perpCandidateType: item.perpCandidateType || item.perpTofMetrics?.candidateType || null,
    advancedTofMetrics: normalizeAdvancedTofMetrics(item.advancedTofMetrics),
    advancedScore: numberOrNull(item.advancedScore),
    advancedCandidateType: item.advancedCandidateType || item.advancedTofMetrics?.candidateType || null,
    alertEligible: item.alertEligible === true,
    lineage: normalizeLineage(item.lineage),
    runtimeBoundary: runtimeFromPayload(item),
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
    mainForceScore: numberOrNull(ownValueOrFallback(item, "mainForceScore", marketStructureScore?.mainForceScore)),
    mainForceConfirmed: booleanOrNull(
      ownValueOrFallback(item, "mainForceConfirmed", marketStructureScore?.mainForceConfirmed),
    ),
    mainForceConfirmationCount: numberOrNull(
      ownValueOrFallback(item, "mainForceConfirmationCount", marketStructureScore?.mainForceConfirmationCount),
    ),
    mainForceConfirmationTotal: numberOrNull(
      ownValueOrFallback(item, "mainForceConfirmationTotal", marketStructureScore?.mainForceConfirmationTotal),
    ),
    mainForceConfirmationThreshold: numberOrNull(
      ownValueOrFallback(item, "mainForceConfirmationThreshold", marketStructureScore?.mainForceConfirmationThreshold),
    ),
    structureBias: numberOrNull(ownValueOrFallback(item, "structureBias", marketStructureScore?.structureBias)),
    extremeImpactScore: numberOrNull(
      ownValueOrFallback(item, "extremeImpactScore", marketStructureScore?.extremeImpactScore),
    ),
    extremeImpactConfirmed: booleanOrNull(
      ownValueOrFallback(item, "extremeImpactConfirmed", marketStructureScore?.extremeImpactConfirmed),
    ),
    regimeType: stringOrNull(ownValueOrFallback(item, "regimeType", marketStructureScore?.regimeType)),
    marketStructureSeverity: stringOrNull(
      ownValueOrFallback(item, "marketStructureSeverity", marketStructureScore?.severity),
    ),
    marketStructureConfidence: numberOrNull(
      ownValueOrFallback(item, "marketStructureConfidence", marketStructureScore?.confidence),
    ),
    marketStructureDataQuality: numberOrNull(
      ownValueOrFallback(item, "marketStructureDataQuality", marketStructureScore?.dataQuality),
    ),
    structureRaw: numberOrNull(ownValueOrFallback(item, "structureRaw", marketStructureScore?.structureRaw)),
    spotContractFloor: numberOrNull(
      ownValueOrFallback(item, "spotContractFloor", marketStructureScore?.spotContractFloor),
    ),
    durationScore: numberOrNull(ownValueOrFallback(item, "durationScore", marketStructureScore?.durationScore)),
    liquidationPenalty: numberOrNull(
      ownValueOrFallback(item, "liquidationPenalty", marketStructureScore?.liquidationPenalty),
    ),
    crowdingPenalty: numberOrNull(
      ownValueOrFallback(item, "crowdingPenalty", marketStructureScore?.crowdingPenalty),
    ),
    spotScore: numberOrNull(ownValueOrFallback(item, "spotScore", marketStructureScore?.spotScore)),
    spotCvdScore: numberOrNull(ownValueOrFallback(item, "spotCvdScore", marketStructureScore?.spotCvdScore)),
    spotVolumeAnomaly: numberOrNull(
      ownValueOrFallback(item, "spotVolumeAnomaly", marketStructureScore?.spotVolumeAnomaly),
    ),
    spotAbsorption: numberOrNull(
      ownValueOrFallback(item, "spotAbsorption", marketStructureScore?.spotAbsorption),
    ),
    spotLiquidityShift: numberOrNull(
      ownValueOrFallback(item, "spotLiquidityShift", marketStructureScore?.spotLiquidityShift),
    ),
    spotPriceResponse: numberOrNull(
      ownValueOrFallback(item, "spotPriceResponse", marketStructureScore?.spotPriceResponse),
    ),
    contractScore: numberOrNull(ownValueOrFallback(item, "contractScore", marketStructureScore?.contractScore)),
    cwmAggressiveFlow: numberOrNull(
      ownValueOrFallback(item, "cwmAggressiveFlow", marketStructureScore?.cwmAggressiveFlow),
    ),
    oiImpulse: numberOrNull(ownValueOrFallback(item, "oiImpulse", marketStructureScore?.oiImpulse)),
    liquidationContext: numberOrNull(
      ownValueOrFallback(item, "liquidationContext", marketStructureScore?.liquidationContext),
    ),
    fundingCrowding: numberOrNull(
      ownValueOrFallback(item, "fundingCrowding", marketStructureScore?.fundingCrowding),
    ),
    basisPremium: numberOrNull(ownValueOrFallback(item, "basisPremium", marketStructureScore?.basisPremium)),
    activeExchangeConfirmation: numberOrNull(
      ownValueOrFallback(item, "activeExchangeConfirmation", marketStructureScore?.activeExchangeConfirmation),
    ),
    crossConfirmScore: numberOrNull(
      ownValueOrFallback(item, "crossConfirmScore", marketStructureScore?.crossConfirmScore),
    ),
    spotContractDirectionConsistency: numberOrNull(
      ownValueOrFallback(
        item,
        "spotContractDirectionConsistency",
        marketStructureScore?.spotContractDirectionConsistency,
      ),
    ),
    multiWindowConsistency: numberOrNull(
      ownValueOrFallback(item, "multiWindowConsistency", marketStructureScore?.multiWindowConsistency),
    ),
    priceResponseConsistency: numberOrNull(
      ownValueOrFallback(item, "priceResponseConsistency", marketStructureScore?.priceResponseConsistency),
    ),
    sourceCoverage: numberOrNull(ownValueOrFallback(item, "sourceCoverage", marketStructureScore?.sourceCoverage)),
    signalAgreement: numberOrNull(ownValueOrFallback(item, "signalAgreement", marketStructureScore?.signalAgreement)),
    oiScore: numberOrNull(ownValueOrFallback(item, "oiScore", marketStructureScore?.oiScore)),
    liquidationScore: numberOrNull(
      ownValueOrFallback(item, "liquidationScore", marketStructureScore?.liquidationScore),
    ),
    fundingCrowdingScore: numberOrNull(
      ownValueOrFallback(item, "fundingCrowdingScore", marketStructureScore?.fundingCrowdingScore),
    ),
    cwmScore: numberOrNull(ownValueOrFallback(item, "cwmScore", marketStructureScore?.cwmScore)),
    marketStructureReasons: normalizeMarketStructureReasons(
      ownValueOrFallback(item, "marketStructureReasons", marketStructureScore?.reasons),
    ),
    finalCandidateType: item.finalCandidateType || null,
    metricsDirection:
      item.metricsDirection || item.advancedTofMetrics?.metricsDirection || item.perpTofMetrics?.metricsDirection || null,
    mergedConfidence: numberOrNull(item.mergedConfidence),
    finalRiskScore: authoritativeRiskScore,
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

function normalizeSignalPrice(item) {
  const candidates = [
    item.triggerPriceUsd,
    item.triggerPrice,
    item.priceUsd,
    item.price,
    item.markPrice,
    item.midPrice,
    item.currentPrice,
    item.indexMid,
    item.markout?.priceAtSignal,
    item.markout?.price_at_signal,
    item.markout?.windows?.find?.((window) => numberOrNull(window?.priceAtSignal) !== null)?.priceAtSignal,
    item.markout?.windows?.find?.((window) => numberOrNull(window?.price_at_signal) !== null)?.price_at_signal,
    item.replaySnapshot?.price?.currentPriceReference,
    item.replaySnapshot?.price?.triggerPriceUsd,
    item.redactedReplaySnapshot?.price?.currentPriceReference,
    item.replay?.snapshot?.price?.currentPriceReference,
  ];
  for (const candidate of candidates) {
    const value = numberOrNull(candidate);
    if (value !== null && value > 0) {
      return value;
    }
  }
  return priceFromRange(item.priceRange ?? item.price_range);
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
  const mainForceSource = ownValueOrFallback(source, "marketStructureScore", source.mainForceStructure);
  const mainForce = normalizeMarketStructureScore(mainForceSource);
  return {
    shortTermToxic: normalizeToxicShortScore(shortTerm),
    marketStructureScore: mainForce,
    mainForceStructure: mainForce,
  };
}

function normalizeMarketStructureScore(structure) {
  if (!structure || typeof structure !== "object" || Array.isArray(structure)) {
    return null;
  }
  const source = structure;
  return {
    ts: numberOrNull(source.ts),
    symbol: typeof source.symbol === "string" ? source.symbol : null,
    mainForceScore: numberOrNull(source.mainForceScore),
    mainForceConfirmed: booleanOrNull(source.mainForceConfirmed),
    mainForceConfirmationCount: numberOrNull(source.mainForceConfirmationCount),
    mainForceConfirmationTotal: numberOrNull(source.mainForceConfirmationTotal),
    mainForceConfirmationThreshold: numberOrNull(source.mainForceConfirmationThreshold),
    extremeImpactScore: numberOrNull(source.extremeImpactScore),
    extremeImpactConfirmed: booleanOrNull(source.extremeImpactConfirmed),
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
    metricsDirection: typeof metrics.metricsDirection === "string" ? metrics.metricsDirection : null,
    confidence: numberOrNull(metrics.confidence),
    explainTags: Array.isArray(metrics.explainTags) ? metrics.explainTags.filter((tag) => typeof tag === "string") : [],
    lineage: normalizeLineage(metrics.lineage),
  };
}

function normalizePerpTofMetrics(metrics) {
  if (!metrics || typeof metrics !== "object") {
    return null;
  }
  return {
    oiChange: numberOrNull(metrics.oiChange),
    oiDirection: typeof metrics.oiDirection === "string" ? metrics.oiDirection : null,
    fundingRate: numberOrNull(metrics.fundingRate),
    fundingSide: typeof metrics.fundingSide === "string" ? metrics.fundingSide : null,
    liquidationPressure: numberOrNull(metrics.liquidationPressure),
    squeezeRiskProxy: numberOrNull(metrics.squeezeRiskProxy),
    observedLiquidationNotional: numberOrNull(metrics.observedLiquidationNotional),
    squeezeSide: typeof metrics.squeezeSide === "string" ? metrics.squeezeSide : null,
    aggBuyVolume: numberOrNull(metrics.aggBuyVolume),
    aggSellVolume: numberOrNull(metrics.aggSellVolume),
    directionBias: typeof metrics.directionBias === "string" ? metrics.directionBias : null,
    metricsDirection: typeof metrics.metricsDirection === "string" ? metrics.metricsDirection : null,
    riskScore: numberOrNull(metrics.riskScore),
    dataQuality: numberOrNull(metrics.dataQuality),
    candidateType: typeof metrics.candidateType === "string" ? metrics.candidateType : "PerpTofCandidate",
    explainTags: Array.isArray(metrics.explainTags) ? metrics.explainTags.filter((tag) => typeof tag === "string") : [],
    confidence: numberOrNull(metrics.confidence),
    lineage: normalizeLineage(metrics.lineage),
    liquidationLineage: normalizeLineage(metrics.liquidationLineage),
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
    vpinZscore: numberOrNull(metrics.vpinZscore),
    vpinPercentile: numberOrNull(metrics.vpinPercentile),
    perVenueVpin: normalizePerVenueVpin(metrics.perVenueVpin),
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
    lineage: normalizeLineage(metrics.lineage),
    metricLineage: normalizeLineageMap(metrics.metricLineage),
  };
}

function normalizePerVenueVpin(value) {
  if (Array.isArray(value)) {
    return value
      .filter((item) => item && typeof item === "object")
      .map((item) => ({
        venue: typeof item.venue === "string" ? item.venue : null,
        vpin: numberOrNull(item.vpin),
        zscore: numberOrNull(item.zscore),
        percentile: numberOrNull(item.percentile),
      }));
  }
  if (!value || typeof value !== "object") {
    return [];
  }
  return Object.entries(value).map(([venue, item]) => ({
    venue,
    vpin: numberOrNull(item?.vpin ?? item),
    zscore: numberOrNull(item?.zscore),
    percentile: numberOrNull(item?.percentile),
  }));
}

function normalizeLineage(lineage) {
  const source = lineage && typeof lineage === "object" ? lineage : {};
  const allowed = new Set(["observed", "calculated_from_observed", "inferred", "unavailable"]);
  const provenance = allowed.has(source.provenance) ? source.provenance : "unavailable";
  const available = source.available === true;
  const fresh = source.fresh === true;
  const eligibleProvenance = provenance === "observed" || provenance === "calculated_from_observed";
  return {
    provenance,
    available,
    fresh,
    source: typeof source.source === "string" ? source.source : "unknown",
    observedAtMs: numberOrNull(source.observedAtMs),
    unavailableReason: typeof source.unavailableReason === "string" ? source.unavailableReason : null,
    alertEligible: source.alertEligible === true && available && fresh && eligibleProvenance,
  };
}

function normalizeLineageMap(lineages) {
  if (!lineages || typeof lineages !== "object" || Array.isArray(lineages)) {
    return {};
  }
  return Object.fromEntries(
    Object.entries(lineages).map(([metric, lineage]) => [metric, normalizeLineage(lineage)]),
  );
}

function numberOrNull(value) {
  if (value === null || value === undefined || value === "") {
    return null;
  }
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function booleanOrNull(value) {
  return typeof value === "boolean" ? value : null;
}

function ownValueOrFallback(source, key, fallback) {
  return source && Object.prototype.hasOwnProperty.call(source, key) ? source[key] : fallback;
}

function stringOrNull(value) {
  return typeof value === "string" && value.length > 0 ? value : null;
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
