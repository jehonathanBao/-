import axios from "axios";

export const DEFAULT_ALT_CONTRACT_DISPLAY_MIN_NOTIONAL_USD = 500_000;
export const DEFAULT_ALT_IMPACT_DISPLAY_THRESHOLD = 70;
export const DEFAULT_ALT_IMPACT_DISCORD_THRESHOLD = 85;
export const DEFAULT_ALT_IMPACT_S_THRESHOLD = 90;
const DEFAULT_ALT_CONTRACT_DISPLAY_THRESHOLDS_USD = {
  ultraCore: 750_000,
  mainstream: 500_000,
  alt: 150_000,
};

const calmSummary = {
  status: "calm",
  healthStatus: "disabled",
  healthReason: "binance_alt_contract_monitor_disabled",
  collectorStatus: "disabled",
  lastTradeAt: null,
  lastOiPollAt: null,
  lastForceOrderAt: null,
  flowBuckets1m: 0,
  signals1h: 0,
  wouldSend1h: 0,
  topActiveSymbols: [],
  errors1h: 0,
  latestDirection: "neutral",
  latestSeverity: "calm",
  latestSignalAt: null,
  signalCount: 0,
  monitoredSymbols: [],
  displayMinNotionalUsd: DEFAULT_ALT_CONTRACT_DISPLAY_MIN_NOTIONAL_USD,
  activeAnomalyCount: 0,
  recentCriticalOrSCount: 0,
  dryRunWouldSendCount: 0,
  enabled: false,
  dryRun: true,
  readOnly: true,
  symbol: null,
  displayThresholdsUsd: DEFAULT_ALT_CONTRACT_DISPLAY_THRESHOLDS_USD,
  trend60s: {
    buyVolumeBase: 0,
    sellVolumeBase: 0,
    totalVolumeBase: 0,
    netVolumeBase: 0,
    totalNotionalUsd: 0,
    dominance: 0,
    buyRatio: 0,
    sellRatio: 0,
    updatedAtMs: null,
  },
  exchanges: {
    binance: { connected: false, status: "disabled", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
  },
  dryRunStats: {
    signals1h: 0,
    high1h: 0,
    critical1h: 0,
    s1h: 0,
    wouldSend1h: 0,
    skippedLowScore1h: 0,
    skippedCooldown1h: 0,
    skippedDataQuality1h: 0,
    liquidationDriven1h: 0,
    signals24h: 0,
    high24h: 0,
    critical24h: 0,
    s24h: 0,
    wouldSend24h: 0,
    skippedLowScore24h: 0,
    skippedCooldown24h: 0,
    skippedDataQuality24h: 0,
    liquidationDriven24h: 0,
  },
  symbolUniverse: {
    mode: "auto",
    limit: 0,
    whitelist: [],
    blacklist: [],
    excludedSymbols: [],
    min24hQuoteVolumeUsd: 0,
  },
  smafReport: defaultSmafReport(),
  smllReport: defaultSmllReport(),
  atcaReport: defaultAtcaReport(),
  amiosReport: defaultAmiosReport(),
};

export async function fetchBinanceAltContractSummary(symbol = "all") {
  const baseURL = apiBaseUrl();
  try {
    const query = buildQuery({ symbol });
    const response = await axios.get(`${baseURL}/api/binance-alt-contract/summary?${query}`);
    return { summary: normalizeSummary(response.data), error: null };
  } catch {
    return { summary: calmSummary, error: "summary_unavailable" };
  }
}

export async function fetchBinanceAltContractLatest(limit = 50, symbol = "all") {
  const baseURL = apiBaseUrl();
  try {
    const query = buildQuery({ limit, symbol });
    const response = await axios.get(`${baseURL}/api/binance-alt-contract/latest?${query}`);
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    const summary = normalizeSummary(response.data?.summary);
    return {
      summary,
      items: filterDisplaySignals(items.map(normalizeAltContractSignal), summary),
      error: null,
    };
  } catch {
    return { summary: calmSummary, items: [], error: "latest_unavailable" };
  }
}

export async function fetchBinanceAltContractHistory(filters = {}) {
  const baseURL = apiBaseUrl();
  try {
    const query = buildQuery({ ...filters, limit: filters.limit ?? 50 });
    const response = await axios.get(`${baseURL}/api/binance-alt-contract/history?${query}`);
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    const summary = normalizeSummary(response.data?.summary);
    return {
      summary,
      items: filterDisplaySignals(items.map(normalizeAltContractSignal), summary),
      error: null,
    };
  } catch {
    return { summary: calmSummary, items: [], error: "history_unavailable" };
  }
}

export function normalizeAltContractSignal(item) {
  const totalVolumeBase = numberOrNull(item.totalVolumeBase) || 0;
  const totalNotionalUsd = numberOrNull(item.totalNotionalUsd) || 0;
  return {
    id: item.id || `${item.productId || item.symbol || "ALT"}-${item.windowSec || 0}-${item.ts || Date.now()}`,
    ts: numberOrNull(item.ts),
    symbol: item.symbol || productToBase(item.productId) || "ALT",
    productId: item.productId || `${item.symbol || "ALT"}USDT`,
    tier: item.tier || "b",
    marketTier: item.marketTier || "alt",
    displayThresholdUsd: numberOrNull(item.displayThresholdUsd) || 0,
    windowSec: numberOrNull(item.windowSec) || 0,
    signalType: item.signalType || "unclear_contract_anomaly",
    direction: item.direction || "neutral",
    severity: item.severity || "high",
    abnormalScore: numberOrNull(item.abnormalScore) || 0,
    buildScore: numberOrNull(item.buildScore) || 0,
    masterCapitalStrength: normalizeMasterCapitalStrength(item.masterCapitalStrength),
    altImpactScore: normalizeAltImpactScore(item.altImpactScore),
    liquidityMicrostructure: normalizeLiquidityMicrostructure(item.liquidityMicrostructure),
    marketControlGraph: normalizeMarketControlGraph(item.marketControlGraph),
    marketRegime: normalizeMarketRegime(item.marketRegime),
    smartMoneyLifecycle: normalizeSmartMoneyLifecycle(item.smartMoneyLifecycle),
    smartMoneyPrediction: normalizeSmartMoneyPrediction(item.smartMoneyPrediction),
    signalConfidence: normalizeSignalConfidence(item.signalConfidence),
    sGradeEligible: Boolean(item.sGradeEligible),
    sGradeConditions: Array.isArray(item.sGradeConditions) ? item.sGradeConditions : [],
    sGradeNotionalThresholdUsd: numberOrNull(item.sGradeNotionalThresholdUsd) || 0,
    sGradeVolumeThresholdBase: numberOrNull(item.sGradeVolumeThresholdBase) || 0,
    directionBias: numberOrNull(item.directionBias) || 0,
    dataQuality: numberOrNull(item.dataQuality) || 0,
    totalVolumeBase,
    netVolumeBase: numberOrNull(item.netVolumeBase) || 0,
    totalNotionalUsd,
    triggerPriceUsd: normalizePrice(item, totalVolumeBase, totalNotionalUsd),
    dominance: numberOrNull(item.dominance) || 0,
    priceMovePct: numberOrNull(item.priceMovePct),
    dynamicMultiple: numberOrNull(item.dynamicMultiple),
    oiChange1mBase: numberOrNull(item.oiChange1mBase),
    oiChange5mBase: numberOrNull(item.oiChange5mBase),
    oiChangePct: numberOrNull(item.oiChangePct),
    fundingRate: numberOrNull(item.fundingRate),
    liquidationNotionalUsd: numberOrNull(item.liquidationNotionalUsd),
    liquidationSuspected: Boolean(item.liquidationSuspected),
    forceOrderSnapshot: Boolean(item.forceOrderSnapshot),
    mainExchange: item.mainExchange || "binance",
    exchanges: normalizeSignalExchanges(item.exchanges),
    scoreBreakdown: normalizeScoreBreakdown(item.scoreBreakdown),
    activeSources: Array.isArray(item.activeSources) ? item.activeSources : [],
    explainTags: Array.isArray(item.explainTags) ? item.explainTags : [],
    abnormalExplanation: item.abnormalExplanation || "",
    buildExplanation: item.buildExplanation || "",
    liquidationExplanation: item.liquidationExplanation || "",
    discordEligible: Boolean(item.discordEligible),
    discordWouldSend: Boolean(item.discordWouldSend),
    discordSent: Boolean(item.discordSent),
    discordSentAt: numberOrNull(item.discordSentAt),
    discordReason: item.discordReason || "not_sent",
    discordAlertKind: item.discordAlertKind || "none",
    discordMinNotionalUsd: numberOrNull(item.discordMinNotionalUsd) || 0,
    finalResult: item.finalResult || "Binance alt contract anomaly candidate",
  };
}

function normalizeSummary(summary) {
  if (!summary || typeof summary !== "object") {
    return calmSummary;
  }
  return {
    status: summary.status || calmSummary.status,
    healthStatus: summary.healthStatus || calmSummary.healthStatus,
    healthReason: summary.healthReason || calmSummary.healthReason,
    collectorStatus: summary.collectorStatus || calmSummary.collectorStatus,
    lastTradeAt: numberOrNull(summary.lastTradeAt),
    lastOiPollAt: numberOrNull(summary.lastOiPollAt),
    lastForceOrderAt: numberOrNull(summary.lastForceOrderAt),
    flowBuckets1m: numberOrNull(summary.flowBuckets1m) || 0,
    signals1h: numberOrNull(summary.signals1h) || 0,
    wouldSend1h: numberOrNull(summary.wouldSend1h) || 0,
    topActiveSymbols: Array.isArray(summary.topActiveSymbols) ? summary.topActiveSymbols : [],
    errors1h: numberOrNull(summary.errors1h) || 0,
    latestDirection: summary.latestDirection || calmSummary.latestDirection,
    latestSeverity: summary.latestSeverity || calmSummary.latestSeverity,
    latestSignalAt: numberOrNull(summary.latestSignalAt),
    signalCount: numberOrNull(summary.signalCount) || 0,
    monitoredSymbols: Array.isArray(summary.monitoredSymbols) ? summary.monitoredSymbols : [],
    displayMinNotionalUsd:
      numberOrNull(summary.displayMinNotionalUsd) || DEFAULT_ALT_CONTRACT_DISPLAY_MIN_NOTIONAL_USD,
    displayThresholdsUsd: normalizeDisplayThresholds(summary.displayThresholdsUsd),
    activeAnomalyCount: numberOrNull(summary.activeAnomalyCount) || 0,
    recentCriticalOrSCount: numberOrNull(summary.recentCriticalOrSCount) || 0,
    dryRunWouldSendCount: numberOrNull(summary.dryRunWouldSendCount) || 0,
    enabled: Boolean(summary.enabled),
    dryRun: summary.dryRun !== false,
    readOnly: summary.readOnly !== false,
    symbol: summary.symbol || null,
    trend60s: normalizeTrend(summary.trend60s),
    exchanges: normalizeExchanges(summary.exchanges),
    dryRunStats: normalizeDryRunStats(summary.dryRunStats),
    symbolUniverse: normalizeSymbolUniverse(summary.symbolUniverse),
    allMarketContext: summary.allMarketContext || {},
    smafReport: normalizeSmafReport(summary.smafReport),
    smllReport: normalizeSmllReport(summary.smllReport),
    atcaReport: normalizeAtcaReport(summary.atcaReport),
    amiosReport: normalizeAmiosReport(summary.amiosReport),
  };
}

function defaultSmafReport() {
  return {
    dataAudit: {
      freshnessScore: 0,
      completenessScore: 0,
      consistencyScore: 0,
      integrityScore: 0,
      dataRiskLevel: "disabled",
    },
    signalAudit: {
      noiseRatio: 0,
      duplicationRate: 0,
      singleSourceDependency: 0,
      falseSignalEstimate: 0,
      integrityScore: 100,
    },
    behaviorAudit: {
      stateStability: 100,
      transitionEntropy: 0,
      manipulationNoise: 0,
      structuralIntegrity: 100,
    },
    predictionAudit: {
      accuracy: 100,
      flipRate: 0,
      overfittingScore: 0,
      followThroughRate: 100,
      integrityScore: 100,
    },
    smafScore: 0,
    riskLevel: "disabled",
    criticalIssues: [],
  };
}

function normalizeSmafReport(report) {
  const fallback = defaultSmafReport();
  const source = report && typeof report === "object" ? report : {};
  return {
    dataAudit: {
      freshnessScore: numberOrNull(source.dataAudit?.freshnessScore) ?? fallback.dataAudit.freshnessScore,
      completenessScore: numberOrNull(source.dataAudit?.completenessScore) ?? fallback.dataAudit.completenessScore,
      consistencyScore: numberOrNull(source.dataAudit?.consistencyScore) ?? fallback.dataAudit.consistencyScore,
      integrityScore: numberOrNull(source.dataAudit?.integrityScore) ?? fallback.dataAudit.integrityScore,
      dataRiskLevel: source.dataAudit?.dataRiskLevel || fallback.dataAudit.dataRiskLevel,
    },
    signalAudit: {
      noiseRatio: numberOrNull(source.signalAudit?.noiseRatio) ?? fallback.signalAudit.noiseRatio,
      duplicationRate: numberOrNull(source.signalAudit?.duplicationRate) ?? fallback.signalAudit.duplicationRate,
      singleSourceDependency:
        numberOrNull(source.signalAudit?.singleSourceDependency) ?? fallback.signalAudit.singleSourceDependency,
      falseSignalEstimate:
        numberOrNull(source.signalAudit?.falseSignalEstimate) ?? fallback.signalAudit.falseSignalEstimate,
      integrityScore: numberOrNull(source.signalAudit?.integrityScore) ?? fallback.signalAudit.integrityScore,
    },
    behaviorAudit: {
      stateStability: numberOrNull(source.behaviorAudit?.stateStability) ?? fallback.behaviorAudit.stateStability,
      transitionEntropy:
        numberOrNull(source.behaviorAudit?.transitionEntropy) ?? fallback.behaviorAudit.transitionEntropy,
      manipulationNoise:
        numberOrNull(source.behaviorAudit?.manipulationNoise) ?? fallback.behaviorAudit.manipulationNoise,
      structuralIntegrity:
        numberOrNull(source.behaviorAudit?.structuralIntegrity) ?? fallback.behaviorAudit.structuralIntegrity,
    },
    predictionAudit: {
      accuracy: numberOrNull(source.predictionAudit?.accuracy) ?? fallback.predictionAudit.accuracy,
      flipRate: numberOrNull(source.predictionAudit?.flipRate) ?? fallback.predictionAudit.flipRate,
      overfittingScore:
        numberOrNull(source.predictionAudit?.overfittingScore) ?? fallback.predictionAudit.overfittingScore,
      followThroughRate:
        numberOrNull(source.predictionAudit?.followThroughRate) ?? fallback.predictionAudit.followThroughRate,
      integrityScore: numberOrNull(source.predictionAudit?.integrityScore) ?? fallback.predictionAudit.integrityScore,
    },
    smafScore: numberOrNull(source.smafScore) ?? fallback.smafScore,
    riskLevel: source.riskLevel || fallback.riskLevel,
    criticalIssues: Array.isArray(source.criticalIssues) ? source.criticalIssues : [],
  };
}

function defaultSmllReport() {
  return {
    enabled: true,
    protectedRealtime: true,
    status: "collecting_outcomes",
    learningScore: 0,
    sampleSize: 0,
    minSamplesForUpdate: 3,
    accuracyRate: 100,
    wrongCount: 0,
    neutralCount: 0,
    outcomeRecords: [],
    errorReports: [],
    suggestedWeights: {
      volumeWeight: 1,
      oiWeight: 1,
      priceWeight: 1,
      liquidationWeight: 1,
      fundingWeight: 1,
    },
    driftReport: {
      driftDetected: false,
      affectedComponents: [],
      suggestedRetrain: false,
      reason: "no_material_drift",
    },
    calibrationUpdates: [],
  };
}

function normalizeSmllReport(report) {
  const fallback = defaultSmllReport();
  const source = report && typeof report === "object" ? report : {};
  return {
    enabled: source.enabled !== false,
    protectedRealtime: source.protectedRealtime !== false,
    status: source.status || fallback.status,
    learningScore: numberOrNull(source.learningScore) ?? fallback.learningScore,
    sampleSize: numberOrNull(source.sampleSize) ?? fallback.sampleSize,
    minSamplesForUpdate: numberOrNull(source.minSamplesForUpdate) ?? fallback.minSamplesForUpdate,
    accuracyRate: numberOrNull(source.accuracyRate) ?? fallback.accuracyRate,
    wrongCount: numberOrNull(source.wrongCount) ?? fallback.wrongCount,
    neutralCount: numberOrNull(source.neutralCount) ?? fallback.neutralCount,
    outcomeRecords: Array.isArray(source.outcomeRecords) ? source.outcomeRecords : [],
    errorReports: Array.isArray(source.errorReports) ? source.errorReports : [],
    suggestedWeights: {
      volumeWeight:
        numberOrNull(source.suggestedWeights?.volumeWeight) ?? fallback.suggestedWeights.volumeWeight,
      oiWeight: numberOrNull(source.suggestedWeights?.oiWeight) ?? fallback.suggestedWeights.oiWeight,
      priceWeight:
        numberOrNull(source.suggestedWeights?.priceWeight) ?? fallback.suggestedWeights.priceWeight,
      liquidationWeight:
        numberOrNull(source.suggestedWeights?.liquidationWeight) ??
        fallback.suggestedWeights.liquidationWeight,
      fundingWeight:
        numberOrNull(source.suggestedWeights?.fundingWeight) ?? fallback.suggestedWeights.fundingWeight,
    },
    driftReport: {
      driftDetected: Boolean(source.driftReport?.driftDetected),
      affectedComponents: Array.isArray(source.driftReport?.affectedComponents)
        ? source.driftReport.affectedComponents
        : [],
      suggestedRetrain: Boolean(source.driftReport?.suggestedRetrain),
      reason: source.driftReport?.reason || fallback.driftReport.reason,
    },
    calibrationUpdates: Array.isArray(source.calibrationUpdates) ? source.calibrationUpdates : [],
  };
}

function defaultAtcaReport() {
  return {
    enabled: true,
    protectedRealtime: true,
    cognitionStatus: "waiting_for_signals",
    memorySummary: "short_memory=0 symbols",
    perceptionCount: 0,
    interpretationCount: 0,
    intentionCount: 0,
    predictionCount: 0,
    decisionCount: 0,
    agents: [],
  };
}

function normalizeAtcaReport(report) {
  const fallback = defaultAtcaReport();
  const source = report && typeof report === "object" ? report : {};
  return {
    enabled: source.enabled !== false,
    protectedRealtime: source.protectedRealtime !== false,
    cognitionStatus: source.cognitionStatus || fallback.cognitionStatus,
    memorySummary: source.memorySummary || fallback.memorySummary,
    perceptionCount: numberOrNull(source.perceptionCount) ?? fallback.perceptionCount,
    interpretationCount: numberOrNull(source.interpretationCount) ?? fallback.interpretationCount,
    intentionCount: numberOrNull(source.intentionCount) ?? fallback.intentionCount,
    predictionCount: numberOrNull(source.predictionCount) ?? fallback.predictionCount,
    decisionCount: numberOrNull(source.decisionCount) ?? fallback.decisionCount,
    agents: Array.isArray(source.agents)
      ? source.agents.map((agent) => ({
          symbol: agent.symbol || "UNKNOWN",
          state: agent.state || "Unknown",
          intent: agent.intent || "monitor",
          prediction: agent.prediction || "unknown",
          confidence: numberOrNull(agent.confidence) ?? 0,
          risk: agent.risk || "low",
          decision: {
            notify: Boolean(agent.decision?.notify),
            severity: agent.decision?.severity || "Ignore",
            reason: agent.decision?.reason || "agent_filtered",
          },
          marketState: {
            symbol: agent.marketState?.symbol || agent.symbol || "UNKNOWN",
            priceStructure: agent.marketState?.priceStructure || "unknown",
            volumeFlow: agent.marketState?.volumeFlow || "mixed",
            oiMovement: agent.marketState?.oiMovement || "unknown",
            liquidationPressure: agent.marketState?.liquidationPressure || "normal",
            marketImbalance: numberOrNull(agent.marketState?.marketImbalance) ?? 0,
          },
        }))
      : [],
  };
}

function defaultAmiosReport() {
  return {
    enabled: true,
    protectedRealtime: true,
    osStatus: "idle",
    marketState: "CALM",
    kernelLoad: 0,
    signalThroughput: "quiet",
    confidence: 0,
    risk: "normal",
    activeProcesses: [],
    currentStates: [],
    schedulerDecision: "standby",
    auditSummary: "smaf=0 smll_samples=0 atca=waiting_for_signals read_only=true direct_discord_gate=false",
    readOnly: true,
    directDiscordGate: false,
  };
}

function normalizeAmiosReport(report) {
  const fallback = defaultAmiosReport();
  const source = report && typeof report === "object" ? report : {};
  return {
    enabled: source.enabled !== false,
    protectedRealtime: source.protectedRealtime !== false,
    osStatus: source.osStatus || fallback.osStatus,
    marketState: source.marketState || fallback.marketState,
    kernelLoad: numberOrNull(source.kernelLoad) ?? fallback.kernelLoad,
    signalThroughput: source.signalThroughput || fallback.signalThroughput,
    confidence: numberOrNull(source.confidence) ?? fallback.confidence,
    risk: source.risk || fallback.risk,
    activeProcesses: Array.isArray(source.activeProcesses)
      ? source.activeProcesses.map((process) => ({
          name: process.name || "unknown",
          layer: process.layer || "kernel",
          status: process.status || "standby",
          load: numberOrNull(process.load) ?? 0,
          role: process.role || "read_only_process",
        }))
      : [],
    currentStates: Array.isArray(source.currentStates)
      ? source.currentStates.map((state) => ({
          symbol: state.symbol || "UNKNOWN",
          marketState: state.marketState || "OBSERVATION_MODE",
          kernelLoad: numberOrNull(state.kernelLoad) ?? 0,
          confidence: numberOrNull(state.confidence) ?? 0,
          regime: state.regime || "Unknown",
          lifecycleState: state.lifecycleState || "Unknown",
          prediction: state.prediction || "Unknown",
          control: state.control || "neutral:NoClearControl",
          risk: state.risk || "low",
          explanation: state.explanation || "AMIOS observation state",
        }))
      : [],
    schedulerDecision: source.schedulerDecision || fallback.schedulerDecision,
    auditSummary: source.auditSummary || fallback.auditSummary,
    readOnly: source.readOnly !== false,
    directDiscordGate: Boolean(source.directDiscordGate),
  };
}

function filterDisplaySignals(items, summary = calmSummary) {
  return items.filter((item) => {
    if (shouldDisplayByAltImpact(item)) return true;
    if (!hasAltImpactScore(item)) {
      const min = displayThresholdForSignal(item, summary);
      return Number(item.totalNotionalUsd || 0) >= min;
    }
    return false;
  });
}

function normalizeDisplayThresholds(thresholds) {
  const source = thresholds && typeof thresholds === "object" ? thresholds : {};
  return {
    ultraCore: numberOrNull(source.ultraCore) || DEFAULT_ALT_CONTRACT_DISPLAY_THRESHOLDS_USD.ultraCore,
    mainstream: numberOrNull(source.mainstream) || DEFAULT_ALT_CONTRACT_DISPLAY_THRESHOLDS_USD.mainstream,
    alt: numberOrNull(source.alt) || DEFAULT_ALT_CONTRACT_DISPLAY_THRESHOLDS_USD.alt,
  };
}

export function displayThresholdForSignal(signal, summary = calmSummary) {
  const explicit = numberOrNull(signal?.displayThresholdUsd);
  if (explicit && explicit > 0) return explicit;
  return displayThresholdForMarketTier(signal?.marketTier || "alt", summary?.displayThresholdsUsd);
}

export function shouldDisplayByAltImpact(signal) {
  const score = numberOrNull(signal?.altImpactScore?.finalScore);
  const threshold =
    numberOrNull(signal?.altImpactScore?.displayThreshold) || DEFAULT_ALT_IMPACT_DISPLAY_THRESHOLD;
  return score !== null && score >= threshold;
}

function hasAltImpactScore(signal) {
  const score = numberOrNull(signal?.altImpactScore?.finalScore);
  if (score !== null && score > 0) return true;
  const impact = numberOrNull(signal?.altImpactScore?.marketImpactRatio);
  const liquidity = numberOrNull(signal?.altImpactScore?.liquidityImpact);
  const direction = numberOrNull(signal?.altImpactScore?.directionalScore);
  return Boolean((impact && impact > 0) || (liquidity && liquidity > 0) || (direction && direction > 0));
}

function displayThresholdForMarketTier(marketTier, thresholds = DEFAULT_ALT_CONTRACT_DISPLAY_THRESHOLDS_USD) {
  const normalized = String(marketTier || "alt").toLowerCase();
  if (normalized === "ultra_core" || normalized === "ultracore") {
    return numberOrNull(thresholds?.ultraCore) || DEFAULT_ALT_CONTRACT_DISPLAY_THRESHOLDS_USD.ultraCore;
  }
  if (normalized === "mainstream") {
    return numberOrNull(thresholds?.mainstream) || DEFAULT_ALT_CONTRACT_DISPLAY_THRESHOLDS_USD.mainstream;
  }
  return numberOrNull(thresholds?.alt) || DEFAULT_ALT_CONTRACT_DISPLAY_THRESHOLDS_USD.alt;
}

function normalizeDryRunStats(stats) {
  const source = stats && typeof stats === "object" ? stats : {};
  return {
    signals1h: numberOrNull(source.signals1h) || 0,
    high1h: numberOrNull(source.high1h) || 0,
    critical1h: numberOrNull(source.critical1h) || 0,
    s1h: numberOrNull(source.s1h) || 0,
    wouldSend1h: numberOrNull(source.wouldSend1h) || 0,
    skippedLowScore1h: numberOrNull(source.skippedLowScore1h) || 0,
    skippedCooldown1h: numberOrNull(source.skippedCooldown1h) || 0,
    skippedDataQuality1h: numberOrNull(source.skippedDataQuality1h) || 0,
    liquidationDriven1h: numberOrNull(source.liquidationDriven1h) || 0,
    signals24h: numberOrNull(source.signals24h) || 0,
    high24h: numberOrNull(source.high24h) || 0,
    critical24h: numberOrNull(source.critical24h) || 0,
    s24h: numberOrNull(source.s24h) || 0,
    wouldSend24h: numberOrNull(source.wouldSend24h) || 0,
    skippedLowScore24h: numberOrNull(source.skippedLowScore24h) || 0,
    skippedCooldown24h: numberOrNull(source.skippedCooldown24h) || 0,
    skippedDataQuality24h: numberOrNull(source.skippedDataQuality24h) || 0,
    liquidationDriven24h: numberOrNull(source.liquidationDriven24h) || 0,
  };
}

function normalizeSymbolUniverse(universe) {
  const source = universe && typeof universe === "object" ? universe : {};
  return {
    mode: source.mode || "auto",
    limit: numberOrNull(source.limit) || 0,
    whitelist: Array.isArray(source.whitelist) ? source.whitelist : [],
    blacklist: Array.isArray(source.blacklist) ? source.blacklist : [],
    excludedSymbols: Array.isArray(source.excludedSymbols) ? source.excludedSymbols : [],
    min24hQuoteVolumeUsd: numberOrNull(source.min24hQuoteVolumeUsd) || 0,
    monitoredCount: numberOrNull(source.monitoredCount) || 0,
    tierCounts: source.tierCounts && typeof source.tierCounts === "object" ? source.tierCounts : {},
  };
}

function normalizeTrend(trend) {
  const source = trend && typeof trend === "object" ? trend : {};
  return {
    buyVolumeBase: numberOrNull(source.buyVolumeBase) || 0,
    sellVolumeBase: numberOrNull(source.sellVolumeBase) || 0,
    totalVolumeBase: numberOrNull(source.totalVolumeBase) || 0,
    netVolumeBase: numberOrNull(source.netVolumeBase) || 0,
    totalNotionalUsd: numberOrNull(source.totalNotionalUsd) || 0,
    dominance: numberOrNull(source.dominance) || 0,
    buyRatio: numberOrNull(source.buyRatio) || 0,
    sellRatio: numberOrNull(source.sellRatio) || 0,
    updatedAtMs: numberOrNull(source.updatedAtMs),
  };
}

function normalizeExchanges(exchanges) {
  const item = exchanges?.binance || {};
  return {
    binance: {
      connected: Boolean(item.connected),
      status: item.status || (item.connected ? "connected" : "disconnected"),
      lastTradeAt: numberOrNull(item.lastTradeAt),
      latencyMs: numberOrNull(item.latencyMs),
      reconnectCount: numberOrNull(item.reconnectCount) || 0,
    },
  };
}

function normalizeSignalExchanges(exchanges) {
  if (!Array.isArray(exchanges)) return [];
  return exchanges.map((item) => ({
    exchange: item.exchange || "binance",
    totalVolumeBase: numberOrNull(item.totalVolumeBase) || 0,
    netVolumeBase: numberOrNull(item.netVolumeBase) || 0,
    totalNotionalUsd: numberOrNull(item.totalNotionalUsd) || 0,
    dominance: numberOrNull(item.dominance) || 0,
  }));
}

function normalizeScoreBreakdown(scoreBreakdown) {
  const source = scoreBreakdown && typeof scoreBreakdown === "object" ? scoreBreakdown : {};
  return {
    volumeScore: numberOrNull(source.volumeScore) || 0,
    dynamicScore: numberOrNull(source.dynamicScore) || 0,
    directionalScore: numberOrNull(source.directionalScore) || 0,
    oiScore: numberOrNull(source.oiScore) || 0,
    priceScore: numberOrNull(source.priceScore) || 0,
    liquidationScore: numberOrNull(source.liquidationScore) || 0,
    persistenceScore: numberOrNull(source.persistenceScore) || 0,
    fundingScore: numberOrNull(source.fundingScore) || 0,
    dataQualityScore: numberOrNull(source.dataQualityScore) || 0,
    penaltyScore: numberOrNull(source.penaltyScore) || 0,
  };
}

function normalizeAltImpactScore(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    marketImpactRatio: numberOrNull(source.marketImpactRatio) || 0,
    marketImpactScore: numberOrNull(source.marketImpactScore) || 0,
    liquidityImpact: numberOrNull(source.liquidityImpact) || 0,
    capImpact: numberOrNull(source.capImpact) || 0,
    directionalStrength: numberOrNull(source.directionalStrength) || 0,
    directionalScore: numberOrNull(source.directionalScore) || 0,
    oiConfirmation: numberOrNull(source.oiConfirmation) || 0,
    finalScore: numberOrNull(source.finalScore) || 0,
    displayThreshold:
      numberOrNull(source.displayThreshold) || DEFAULT_ALT_IMPACT_DISPLAY_THRESHOLD,
    discordThreshold:
      numberOrNull(source.discordThreshold) || DEFAULT_ALT_IMPACT_DISCORD_THRESHOLD,
    sThreshold: numberOrNull(source.sThreshold) || DEFAULT_ALT_IMPACT_S_THRESHOLD,
    referenceVolume24hUsd: numberOrNull(source.referenceVolume24hUsd),
    referenceSource: source.referenceSource || "unavailable",
    interpretation: source.interpretation || "暂无相对成交冲击解释",
  };
}

function normalizeLiquidityMicrostructure(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    lmsScore: numberOrNull(source.lmsScore) || 0,
    behavior: source.behavior || "OrdinaryFlow",
    marketControl: source.marketControl || "no_clear_control",
    liquidityPressure: source.liquidityPressure || "LOW",
    imbalance: numberOrNull(source.imbalance) || 0,
    spreadState: source.spreadState || "unknown",
    spoofingState: source.spoofingState || "none",
    orderFlowPressure: numberOrNull(source.orderFlowPressure) || 0,
    absorptionStrength: numberOrNull(source.absorptionStrength) || 0,
    imbalanceScore: numberOrNull(source.imbalanceScore) || 0,
    spreadBehavior: numberOrNull(source.spreadBehavior) || 0,
    spoofingPenalty: numberOrNull(source.spoofingPenalty) || 0,
    explanationTags: Array.isArray(source.explanationTags) ? source.explanationTags : [],
    interpretation: source.interpretation || "盘口结构信号较弱或缺少 L2 上下文",
    readOnly: source.readOnly !== false,
    directDiscordGate: Boolean(source.directDiscordGate),
  };
}

function normalizeMarketControlGraph(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    symbol: source.symbol || "",
    controlNodes: Array.isArray(source.controlNodes) ? source.controlNodes : [],
    controlEdges: Array.isArray(source.controlEdges) ? source.controlEdges : [],
    dominantSide: source.dominantSide || "neutral",
    controlStrength: numberOrNull(source.controlStrength) || 0,
    controlType: source.controlType || "NoClearControl",
    controlPath: Array.isArray(source.controlPath) ? source.controlPath : [],
    interpretation: source.interpretation || "控制关系未确认",
    readOnly: source.readOnly !== false,
    directDiscordGate: Boolean(source.directDiscordGate),
  };
}

function normalizeMasterCapitalStrength(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    mcss: numberOrNull(source.mcss) || 0,
    tier: source.tier || "Unknown",
    liquidityWeight: numberOrNull(source.liquidityWeight) || 0,
    notionalScore: numberOrNull(source.notionalScore) || 0,
    directionScore: numberOrNull(source.directionScore) || 0,
    oiScore: numberOrNull(source.oiScore) || 0,
    priceScore: numberOrNull(source.priceScore) || 0,
    anomalyScore: numberOrNull(source.anomalyScore) || 0,
    liquidationPenalty: numberOrNull(source.liquidationPenalty) || 0,
    interpretation: source.interpretation || "暂无主力资金强度解释",
  };
}

function normalizeMarketRegime(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    regime: source.regime || "Unclear",
    subType: source.subType || null,
    confidence: numberOrNull(source.confidence) || 0,
    mcScore: numberOrNull(source.mcScore) || 0,
    oiTrend: source.oiTrend || "unknown",
    priceTrend: source.priceTrend || "unknown",
    trend5m: source.trend5m || "unknown",
    trend15m: source.trend15m || "unknown",
    trend1h: source.trend1h || "unknown",
    efficiencyRatio: numberOrNull(source.efficiencyRatio) || 0,
    oiLagIndex: numberOrNull(source.oiLagIndex) || 0,
    explanationTags: Array.isArray(source.explanationTags) ? source.explanationTags : [],
  };
}

function normalizeSmartMoneyLifecycle(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    lifecycleState: source.lifecycleState || "Accumulation",
    stateConfidence: numberOrNull(source.stateConfidence) || 0,
    stateDurationMin: numberOrNull(source.stateDurationMin) || 0,
    transitionSignal: source.transitionSignal || null,
    flowConsistencyScore: numberOrNull(source.flowConsistencyScore) || 0,
    lifecycleScore: numberOrNull(source.lifecycleScore) || 0,
    statePath: Array.isArray(source.statePath) ? source.statePath : [],
    explanationTags: Array.isArray(source.explanationTags) ? source.explanationTags : [],
    currentExplanation: source.currentExplanation || "生命周期结构仍未确认。",
  };
}

function normalizeSmartMoneyPrediction(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    currentState: source.currentState || "Accumulation",
    nextState: source.nextState || "Sideways",
    probability: numberOrNull(source.probability) || 0,
    timeHorizonMin: numberOrNull(source.timeHorizonMin) || 0,
    directionBias: source.directionBias || "Sideways",
    directionProbability: numberOrNull(source.directionProbability) || 0,
    confidence: numberOrNull(source.confidence) || 0,
    predictionScore: numberOrNull(source.predictionScore) || 0,
    triggerFactors: Array.isArray(source.triggerFactors) ? source.triggerFactors : [],
    explanation: source.explanation || "预测层等待生命周期确认。",
  };
}

function normalizeSignalConfidence(value) {
  const source = value && typeof value === "object" ? value : {};
  const breakdown = source.breakdown && typeof source.breakdown === "object" ? source.breakdown : {};
  return {
    symbol: source.symbol || "",
    signalType: source.signalType || "",
    confidenceScore: numberOrNull(source.confidenceScore) || 0,
    confidenceLevel: source.confidenceLevel || "noise",
    reliabilityFactors: Array.isArray(source.reliabilityFactors) ? source.reliabilityFactors : [],
    riskFactors: Array.isArray(source.riskFactors) ? source.riskFactors : [],
    breakdown: {
      bacmSignalStrength: numberOrNull(breakdown.bacmSignalStrength) || 0,
      mcssStrength: numberOrNull(breakdown.mcssStrength) || 0,
      smleStability: numberOrNull(breakdown.smleStability) || 0,
      smpPredictionAlignment: numberOrNull(breakdown.smpPredictionAlignment) || 0,
      lmeMicrostructureSupport: numberOrNull(breakdown.lmeMicrostructureSupport) || 0,
      mcgControlCoherence: numberOrNull(breakdown.mcgControlCoherence) || 0,
      smafRiskPenalty: numberOrNull(breakdown.smafRiskPenalty) || 0,
    },
    interpretation: source.interpretation || "信号可信度不足或缺少多层确认",
    readOnly: source.readOnly !== false,
    directDiscordGate: Boolean(source.directDiscordGate),
  };
}

function normalizePrice(item, totalVolumeBase, totalNotionalUsd) {
  const explicit =
    numberOrNull(item.triggerPriceUsd) ??
    numberOrNull(item.priceUsd) ??
    numberOrNull(item.price) ??
    numberOrNull(item.avgPriceUsd);
  if (explicit !== null && explicit > 0) {
    return explicit;
  }
  if (totalVolumeBase > 0 && totalNotionalUsd > 0) {
    return totalNotionalUsd / totalVolumeBase;
  }
  return null;
}

function buildQuery(filters) {
  const params = new URLSearchParams();
  Object.entries(filters || {}).forEach(([key, value]) => {
    if (value === null || value === undefined || value === "" || value === "all") return;
    params.set(key, String(value));
  });
  return params.toString();
}

function productToBase(productId) {
  if (!productId) return null;
  return String(productId).toUpperCase().replace(/USDT$/, "");
}

function numberOrNull(value) {
  if (value === null || value === undefined || value === "") return null;
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function apiBaseUrl() {
  return (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
}
