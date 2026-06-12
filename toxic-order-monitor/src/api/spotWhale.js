import axios from "axios";

const calmSummary = {
  status: "calm",
  healthStatus: "disabled",
  healthReason: "spot_whale_monitor_disabled",
  direction: "neutral",
  latestDirection: "neutral",
  latestSeverity: "calm",
  latestSignalAt: null,
  lastDiscordSentAt: null,
  updatedAtMs: null,
  signalCount: 0,
  readOnly: true,
  enabled: false,
  dryRun: true,
  symbol: "BTC",
  trend60s: {
    buyVolumeBase: 0,
    sellVolumeBase: 0,
    totalVolumeBase: 0,
    netVolumeBase: 0,
    dominance: 0,
    buyRatio: 0,
    sellRatio: 0,
    updatedAtMs: null,
  },
  exchanges: {
    binance: { connected: false, status: "disconnected", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
    coinbase: { connected: false, status: "disconnected", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
    bitfinex: { connected: false, status: "disconnected", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
  },
};

export async function fetchSpotWhaleSummary(symbol = "BTC") {
  const baseURL = apiBaseUrl();
  try {
    const query = buildSpotWhaleQuery({ symbol });
    const response = await axios.get(`${baseURL}/api/spot-whale/summary?${query}`);
    return { summary: normalizeSummary(response.data), error: null };
  } catch {
    return { summary: calmSummary, error: "summary_unavailable" };
  }
}

export async function fetchSpotWhaleLatest(limit = 50, symbol = "BTC") {
  const baseURL = apiBaseUrl();
  try {
    const query = buildSpotWhaleQuery({ limit, symbol });
    const response = await axios.get(`${baseURL}/api/spot-whale/latest?${query}`);
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    return {
      summary: normalizeSummary(response.data?.summary),
      items: items.map(normalizeSpotWhaleSignal),
      error: null,
    };
  } catch {
    return { summary: calmSummary, items: [], error: "latest_unavailable" };
  }
}

export async function fetchSpotWhaleHistory(filters = {}) {
  const baseURL = apiBaseUrl();
  try {
    const query = buildSpotWhaleQuery({ ...filters, limit: filters.limit ?? 50 });
    const response = await axios.get(`${baseURL}/api/spot-whale/history?${query}`);
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    return {
      summary: normalizeSummary(response.data?.summary),
      items: items.map(normalizeSpotWhaleSignal),
      error: null,
    };
  } catch {
    return { summary: calmSummary, items: [], error: "history_unavailable" };
  }
}

export function normalizeSpotWhaleSignal(item) {
  const totalVolumeBase = numberOrNull(item.totalVolumeBase) || 0;
  const totalNotionalUsd = numberOrNull(item.totalNotionalUsd) || 0;
  return {
    id: item.id || `${item.symbol || "BTC"}-${item.windowSec || 0}-${item.ts || Date.now()}`,
    ts: numberOrNull(item.ts),
    symbol: item.symbol || "BTC",
    windowSec: numberOrNull(item.windowSec) || 0,
    signalType: item.signalType || "unknown",
    direction: item.direction || "neutral",
    severity: item.severity || "medium",
    score: numberOrNull(item.score) || 0,
    totalVolumeBase,
    netVolumeBase: numberOrNull(item.netVolumeBase) || 0,
    totalNotionalUsd,
    triggerPriceUsd: normalizeTriggerPrice(item, totalVolumeBase, totalNotionalUsd),
    dominance: numberOrNull(item.dominance) || 0,
    priceMovePct: numberOrNull(item.priceMovePct),
    coinbasePremiumPct: numberOrNull(item.coinbasePremiumPct),
    mainExchange: item.mainExchange || "multi",
    dynamicMultiple: numberOrNull(item.dynamicMultiple),
    multiExchangeConfirmed: Boolean(item.multiExchangeConfirmed),
    exchanges: normalizeSignalExchanges(item.exchanges),
    dataQuality: numberOrNull(item.dataQuality) || 0,
    discordEligible: Boolean(item.discordEligible),
    discordSent: Boolean(item.discordSent),
    discordSentAt: numberOrNull(item.discordSentAt),
    discordReason: item.discordReason || "not_sent",
    finalResult: item.finalResult || "spot whale flow candidate",
  };
}

function normalizeTriggerPrice(item, totalVolumeBase, totalNotionalUsd) {
  const explicit =
    numberOrNull(item.triggerPriceUsd) ??
    numberOrNull(item.triggerPrice) ??
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

function normalizeSummary(summary) {
  if (!summary || typeof summary !== "object") {
    return calmSummary;
  }
  return {
    status: summary.status || calmSummary.status,
    healthStatus: summary.healthStatus || calmSummary.healthStatus,
    healthReason: summary.healthReason || calmSummary.healthReason,
    direction: summary.direction || calmSummary.direction,
    latestDirection: summary.latestDirection || summary.direction || calmSummary.latestDirection,
    latestSeverity: summary.latestSeverity || calmSummary.latestSeverity,
    latestSignalAt: numberOrNull(summary.latestSignalAt),
    lastDiscordSentAt: numberOrNull(summary.lastDiscordSentAt),
    updatedAtMs: numberOrNull(summary.updatedAtMs),
    signalCount: numberOrNull(summary.signalCount) || 0,
    readOnly: summary.readOnly !== false,
    enabled: Boolean(summary.enabled),
    dryRun: summary.dryRun !== false,
    symbol: summary.symbol || "BTC",
    trend60s: normalizeTrend60s(summary.trend60s),
    exchanges: normalizeExchanges(summary.exchanges),
  };
}

function normalizeSignalExchanges(exchanges) {
  if (!Array.isArray(exchanges)) return [];
  return exchanges.map((item) => ({
    exchange: item.exchange || "unknown",
    buyVolumeBase: numberOrNull(item.buyVolumeBase) || 0,
    sellVolumeBase: numberOrNull(item.sellVolumeBase) || 0,
    totalVolumeBase: numberOrNull(item.totalVolumeBase) || 0,
    netVolumeBase: numberOrNull(item.netVolumeBase) || 0,
    dominance: numberOrNull(item.dominance) || 0,
  }));
}

function normalizeExchanges(exchanges) {
  const source = exchanges && typeof exchanges === "object" ? exchanges : {};
  return ["binance", "coinbase", "bitfinex"].reduce((acc, key) => {
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
    buyVolumeBase: numberOrNull(source.buyVolumeBase) || 0,
    sellVolumeBase: numberOrNull(source.sellVolumeBase) || 0,
    totalVolumeBase: numberOrNull(source.totalVolumeBase) || 0,
    netVolumeBase: numberOrNull(source.netVolumeBase) || 0,
    dominance: numberOrNull(source.dominance) || 0,
    buyRatio: numberOrNull(source.buyRatio) || 0,
    sellRatio: numberOrNull(source.sellRatio) || 0,
    updatedAtMs: numberOrNull(source.updatedAtMs),
  };
}

function buildSpotWhaleQuery(filters) {
  const params = new URLSearchParams();
  Object.entries(filters || {}).forEach(([key, value]) => {
    if (value === null || value === undefined || value === "" || value === "all") return;
    params.set(key, String(value));
  });
  return params.toString();
}

function numberOrNull(value) {
  if (value === null || value === undefined || value === "") return null;
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function apiBaseUrl() {
  return (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
}
