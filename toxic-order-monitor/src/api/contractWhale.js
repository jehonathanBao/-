import axios from "axios";

const calmSummary = {
  status: "calm",
  healthStatus: "disabled",
  healthReason: "contract_whale_monitor_disabled",
  thresholdProfile: "binance_bitfinex",
  activeExchangeCount: 0,
  enabledExchanges: [],
  disabledExchanges: ["binance", "okx", "bitfinex"],
  direction: "neutral",
  latestSeverity: "calm",
  latestPushedAtMs: null,
  lastDiscordSentAt: null,
  latestSignalAt: null,
  latestDirection: "neutral",
  updatedAtMs: null,
  signalCount: 0,
  readOnly: true,
  enabled: false,
  dryRun: true,
  trend60s: {
    buyVolumeBtc: 0,
    sellVolumeBtc: 0,
    totalVolumeBtc: 0,
    netVolumeBtc: 0,
    dominance: 0,
    buyRatio: 0,
    sellRatio: 0,
    updatedAtMs: null,
  },
  exchanges: {
    binance: { connected: false, status: "disconnected", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
    okx: { connected: false, status: "disabled", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
    bitfinex: { connected: false, status: "disconnected", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
  },
};

export async function fetchContractWhaleSummary(symbol = "BTC") {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({ symbol });
    const response = await axios.get(`${baseURL}/api/contract-whale/summary?${query}`);
    return {
      summary: normalizeSummary(response.data),
      error: null,
    };
  } catch {
    return { summary: calmSummary, error: "summary_unavailable" };
  }
}

export async function fetchContractWhaleLatest(limit = 50, symbol = "BTC") {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({ limit, symbol });
    const response = await axios.get(`${baseURL}/api/contract-whale/latest?${query}`);
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    return {
      summary: normalizeSummary(response.data?.summary),
      items: items.map(normalizeContractWhaleSignal),
      error: null,
    };
  } catch {
    return { summary: calmSummary, items: [], error: "latest_unavailable" };
  }
}

export async function fetchContractWhaleHistory(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({ ...filters, limit: filters.limit ?? 50 });
    const response = await axios.get(`${baseURL}/api/contract-whale/history?${query}`);
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    return {
      summary: normalizeSummary(response.data?.summary),
      items: items.map(normalizeContractWhaleSignal),
      error: null,
    };
  } catch {
    return { summary: calmSummary, items: [], error: "history_unavailable" };
  }
}

export async function fetchContractWhaleEvents(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({ ...filters, limit: filters.limit ?? 20 });
    const response = await axios.get(`${baseURL}/api/contract-whale/events?${query}`);
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    return {
      items: items.map(normalizeMainForceEvent),
      error: null,
    };
  } catch {
    return { items: [], error: "events_unavailable" };
  }
}

export function normalizeContractWhaleSignal(item) {
  return {
    id: item.id || `${item.symbol || "BTC"}-${item.windowSec || 0}-${item.ts || Date.now()}`,
    ts: numberOrNull(item.ts),
    symbol: item.symbol || "BTC",
    windowSec: numberOrNull(item.windowSec) || 0,
    signalType: item.signalType || "unknown",
    direction: item.direction || "neutral",
    severity: item.severity || "medium",
    score: numberOrNull(item.score) || 0,
    totalVolumeBtc: numberOrNull(item.totalVolumeBtc) || 0,
    netVolumeBtc: numberOrNull(item.netVolumeBtc) || 0,
    totalNotionalUsd: numberOrNull(item.totalNotionalUsd) || 0,
    dominance: numberOrNull(item.dominance) || 0,
    priceMovePct: numberOrNull(item.priceMovePct),
    mainExchange: item.mainExchange || "Multi",
    dominantVenueNetContributionShare: numberOrNull(item.dominantVenueNetContributionShare),
    dynamicMultiple: numberOrNull(item.dynamicMultiple),
    percentileLevel: numberOrNull(item.percentileLevel),
    multiExchangeConfirmed: Boolean(item.multiExchangeConfirmed),
    liquidationSuspected: Boolean(item.liquidationSuspected),
    liquidationLongBtc: numberOrNull(item.liquidationLongBtc) || 0,
    liquidationShortBtc: numberOrNull(item.liquidationShortBtc) || 0,
    liquidationNotionalUsd: numberOrNull(item.liquidationNotionalUsd) || 0,
    liquidationRatio: numberOrNull(item.liquidationRatio),
    priceReversalRatio: numberOrNull(item.priceReversalRatio),
    oiChange1mBtc: numberOrNull(item.oiChange1mBtc),
    oiChange5mBtc: numberOrNull(item.oiChange5mBtc),
    oiChangePct: numberOrNull(item.oiChangePct),
    oiBias: item.oiBias || "unknown",
    fundingRate: numberOrNull(item.fundingRate),
    fundingBias: item.fundingBias || "unknown",
    exchanges: normalizeSignalExchanges(item.exchanges),
    dataQuality: numberOrNull(item.dataQuality) || 0,
    discordEligible: Boolean(item.discordEligible),
    discordSent: Boolean(item.discordSent),
    discordSentAt: numberOrNull(item.discordSentAt),
    discordReason: item.discordReason || "not_sent",
    finalResult: item.finalResult || "contract whale flow candidate",
    mergedFrom: Array.isArray(item.mergedFrom) ? item.mergedFrom.filter(Boolean).map(String) : [],
  };
}

export function normalizeMainForceEvent(item) {
  return {
    id: numberOrNull(item.id) || 0,
    symbol: item.symbol || "BTC",
    startedAt: numberOrNull(item.startedAt),
    endedAt: numberOrNull(item.endedAt),
    peakAt: numberOrNull(item.peakAt),
    regimeType: item.regimeType || "unclear",
    severity: item.severity || "Watch",
    peakMainForceScore: numberOrNull(item.peakMainForceScore) || 0,
    peakExtremeImpactScore: numberOrNull(item.peakExtremeImpactScore) || 0,
    peakStructureBias: numberOrNull(item.peakStructureBias) || 0,
    confidence: numberOrNull(item.confidence) || 0,
    spotScore: numberOrNull(item.spotScore),
    contractScore: numberOrNull(item.contractScore),
    crossConfirmScore: numberOrNull(item.crossConfirmScore),
    cwmScore: numberOrNull(item.cwmScore),
    oiScore: numberOrNull(item.oiScore),
    liquidationScore: numberOrNull(item.liquidationScore),
    fundingCrowdingScore: numberOrNull(item.fundingCrowdingScore),
    mainForceConfirmed: Boolean(item.mainForceConfirmed),
    extremeImpactConfirmed: Boolean(item.extremeImpactConfirmed),
    liquidationDriven: Boolean(item.liquidationDriven),
    reasons: normalizeEventReasons(item.reasonsJson),
  };
}

function normalizeSignalExchanges(exchanges) {
  if (!Array.isArray(exchanges)) return [];
  return exchanges.map((item) => ({
    exchange: item.exchange || "unknown",
    buyVolumeBtc: numberOrNull(item.buyVolumeBtc) || 0,
    sellVolumeBtc: numberOrNull(item.sellVolumeBtc) || 0,
    totalVolumeBtc: numberOrNull(item.totalVolumeBtc) || 0,
    buyShare: numberOrNull(item.buyShare) || 0,
    sellShare: numberOrNull(item.sellShare) || 0,
    netVolumeBtc: numberOrNull(item.netVolumeBtc) || 0,
    dominance: numberOrNull(item.dominance) || 0,
    netContributionShare: numberOrNull(item.netContributionShare) || 0,
  }));
}

function normalizeSummary(summary) {
  if (!summary || typeof summary !== "object") {
    return calmSummary;
  }
  return {
    status: summary.status || calmSummary.status,
    healthStatus: summary.healthStatus || calmSummary.healthStatus,
    healthReason: summary.healthReason || calmSummary.healthReason,
    thresholdProfile: summary.thresholdProfile || calmSummary.thresholdProfile,
    activeExchangeCount: numberOrNull(summary.activeExchangeCount) || 0,
    enabledExchanges: normalizeStringArray(summary.enabledExchanges),
    disabledExchanges: normalizeStringArray(summary.disabledExchanges),
    direction: summary.direction || calmSummary.direction,
    latestDirection: summary.latestDirection || summary.direction || calmSummary.latestDirection,
    latestSeverity: summary.latestSeverity || calmSummary.latestSeverity,
    latestSignalAt: numberOrNull(summary.latestSignalAt),
    latestPushedAtMs: numberOrNull(summary.latestPushedAtMs),
    lastDiscordSentAt: numberOrNull(summary.lastDiscordSentAt ?? summary.latestPushedAtMs),
    updatedAtMs: numberOrNull(summary.updatedAtMs),
    signalCount: numberOrNull(summary.signalCount) || 0,
    readOnly: summary.readOnly !== false,
    enabled: Boolean(summary.enabled),
    dryRun: summary.dryRun !== false,
    trend60s: normalizeTrend60s(summary.trend60s),
    exchanges: normalizeExchanges(summary.exchanges),
  };
}

function normalizeStringArray(value) {
  if (!Array.isArray(value)) return [];
  return value.map((item) => String(item || "").toLowerCase()).filter(Boolean);
}

function buildContractWhaleQuery(filters) {
  const params = new URLSearchParams();
  Object.entries(filters || {}).forEach(([key, value]) => {
    if (value === null || value === undefined || value === "" || value === "all") return;
    params.set(key, String(value));
  });
  return params.toString();
}

function numberOrNull(value) {
  if (value === null || value === undefined || value === "") {
    return null;
  }
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function normalizeExchanges(exchanges) {
  const source = exchanges && typeof exchanges === "object" ? exchanges : {};
  return ["binance", "okx", "bitfinex"].reduce((acc, key) => {
    const item = source[key] && typeof source[key] === "object" ? source[key] : {};
    acc[key] = {
      connected: Boolean(item.connected),
      status: item.status || (item.connected ? "connected" : "disconnected"),
      lastTradeAt: numberOrNull(item.lastTradeAt),
      latencyMs: numberOrNull(item.latencyMs),
      reconnectCount: numberOrNull(item.reconnectCount) || 0,
    };
    return acc;
  }, {});
}

function normalizeTrend60s(trend) {
  const source = trend && typeof trend === "object" ? trend : {};
  return {
    buyVolumeBtc: numberOrNull(source.buyVolumeBtc) || 0,
    sellVolumeBtc: numberOrNull(source.sellVolumeBtc) || 0,
    totalVolumeBtc: numberOrNull(source.totalVolumeBtc) || 0,
    netVolumeBtc: numberOrNull(source.netVolumeBtc) || 0,
    dominance: numberOrNull(source.dominance) || 0,
    buyRatio: numberOrNull(source.buyRatio) || 0,
    sellRatio: numberOrNull(source.sellRatio) || 0,
    updatedAtMs: numberOrNull(source.updatedAtMs),
  };
}

function normalizeEventReasons(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    coreReason: typeof source.coreReason === "string" ? source.coreReason : "",
    finalResult: typeof source.finalResult === "string" ? source.finalResult : "",
    explainTags: Array.isArray(source.explainTags) ? source.explainTags.filter(Boolean).map(String) : [],
    regimeType: typeof source.regimeType === "string" ? source.regimeType : "",
  };
}
