import axios from "axios";

export const DEFAULT_ALT_CONTRACT_DISPLAY_MIN_NOTIONAL_USD = 500_000;

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
      items: filterDisplaySignals(items.map(normalizeAltContractSignal), summary.displayMinNotionalUsd),
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
      items: filterDisplaySignals(items.map(normalizeAltContractSignal), summary.displayMinNotionalUsd),
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
    windowSec: numberOrNull(item.windowSec) || 0,
    signalType: item.signalType || "unclear_contract_anomaly",
    direction: item.direction || "neutral",
    severity: item.severity || "high",
    abnormalScore: numberOrNull(item.abnormalScore) || 0,
    buildScore: numberOrNull(item.buildScore) || 0,
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
  };
}

function filterDisplaySignals(items, minNotionalUsd) {
  const min = Number(minNotionalUsd || DEFAULT_ALT_CONTRACT_DISPLAY_MIN_NOTIONAL_USD);
  return items.filter((item) => Number(item.totalNotionalUsd || 0) >= min);
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
