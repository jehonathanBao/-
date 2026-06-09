import axios from "axios";

const calmSummary = {
  status: "calm",
  healthStatus: "disabled",
  healthReason: "contract_whale_monitor_disabled",
  marketType: "perp",
  meta: null,
  thresholdProfile: "binance_bitfinex",
  thresholdProfileReason: "active_contract_sources=binance,bitfinex",
  configuredContractSources: ["binance", "bitfinex"],
  eligibleContractSources: ["binance", "bitfinex"],
  activeExchangeCount: 0,
  enabledExchanges: [],
  disabledExchanges: ["binance", "okx", "bitfinex"],
  activeContractExchanges: [],
  activeContractSources: [],
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
  contractDataQuality: 0,
  spotDataQuality: 0,
  overallDataQuality: 0,
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
    coinbase: { connected: false, status: "spot_only", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
  },
  platforms: {
    binance: { platformEnabled: true, status: "active", markets: {} },
    bitfinex: { platformEnabled: true, status: "active", markets: {} },
    coinbase: { platformEnabled: true, status: "spot_only", markets: {} },
    okx: { platformEnabled: false, status: "disabled", markets: {} },
  },
};

export function normalizePlatformStatus(platform) {
  const item = platform && typeof platform === "object" ? platform : {};
  const status = String(item.status || "disabled").toLowerCase();
  const enabled = Boolean(item.platformEnabled ?? item.enabled);

  if (!enabled || status === "disabled") {
    return {
      key: "disabled",
      label: "未启用",
      description: "当前平台未启用，不参与合约监控、现货确认或 Discord gate。",
      tone: "slate",
    };
  }
  if (status === "spot_only") {
    return {
      key: "spot_only",
      label: "现货专用",
      description: "当前仅启用现货确认，不参与 CWM 合约成交量、阈值和 Discord gate。",
      tone: "cyan",
    };
  }
  if (status === "degraded" || status === "auth_missing") {
    return {
      key: status,
      label: status === "auth_missing" ? "缺少凭证" : "降级",
      description: "平台已配置，但部分只读 market data 能力不可用。",
      tone: "yellow",
    };
  }
  if (status === "reconnecting") {
    return {
      key: "reconnecting",
      label: "重连中",
      description: "平台已启用，正在等待连接恢复或新成交数据。",
      tone: "yellow",
    };
  }
  return {
    key: "active",
    label: "运行中",
    description: "平台能力已配置，按启用 market role 参与对应统计。",
    tone: "emerald",
  };
}

export function normalizeMarketStatus(market, marketType = "") {
  const item = market && typeof market === "object" ? market : {};
  const status = String(item.status || (item.enabled ? "enabled" : "disabled")).toLowerCase();
  const role = String(item.role || "").toLowerCase();
  const enabled = Boolean(item.enabled);
  const type = String(marketType || "").toLowerCase();
  const hasRecentTrade = Number.isFinite(Number(item.lastTradeAt)) && Number(item.lastTradeAt) > 0;

  if (!enabled || status === "disabled") {
    return {
      key: "disabled",
      label: "未启用",
      detail: type === "perp" ? "不参与当前合约监控" : "不参与当前数据源",
      tone: "slate",
    };
  }
  if (status === "spot_only" || role === "spot_confirmation") {
    return {
      key: "spot_only",
      label: type === "spot" ? "现货确认源" : "现货专用",
      detail: "只用于现货确认，不进入合约成交量统计。",
      tone: "cyan",
    };
  }
  if (status === "auth_missing") {
    return {
      key: "auth_missing",
      label: "缺少凭证",
      detail: "缺少只读 market data 配置。",
      tone: "yellow",
    };
  }
  if (status === "reconnecting") {
    return {
      key: "reconnecting",
      label: "重连中",
      detail: "等待连接恢复。",
      tone: "yellow",
    };
  }
  if (status === "stale") {
    return {
      key: "stale",
      label: "数据延迟",
      detail: "连接存在，但近期没有新成交。",
      tone: "orange",
    };
  }
  if ((status === "active" || status === "connected" || status === "online") && (hasRecentTrade || type !== "perp")) {
    return {
      key: "active",
      label: "运行中",
      detail: "该 market 已参与对应统计。",
      tone: "emerald",
    };
  }
  return {
    key: "waiting_for_data",
    label: "已启用 / 等待数据",
    detail: "配置已启用，等待 collector 或下一笔成交更新。",
    tone: "cyan",
  };
}

export async function fetchContractWhaleSummary(symbol = "BTC") {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({ symbol });
    const response = await axios.get(`${baseURL}/api/contract-whale/summary?${query}`);
    return {
      summary: normalizeSummary(response.data),
      meta: normalizeResponseMeta(response.data?.meta),
      error: null,
    };
  } catch {
    return { summary: calmSummary, meta: null, error: "summary_unavailable" };
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
      meta: normalizeResponseMeta(response.data?.meta),
      error: null,
    };
  } catch {
    return { summary: calmSummary, items: [], meta: null, error: "latest_unavailable" };
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
      meta: normalizeResponseMeta(response.data?.meta),
      error: null,
    };
  } catch {
    return { summary: calmSummary, items: [], meta: null, error: "history_unavailable" };
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
  const totalVolumeBtc = numberOrNull(item.totalVolumeBtc) || 0;
  const totalNotionalUsd = numberOrNull(item.totalNotionalUsd) || 0;
  const activeSources = normalizeActiveSources(item.activeSources);
  return {
    id: item.id || `${item.symbol || "BTC"}-${item.windowSec || 0}-${item.ts || Date.now()}`,
    ts: numberOrNull(item.ts),
    symbol: item.symbol || "BTC",
    windowSec: numberOrNull(item.windowSec) || 0,
    signalType: item.signalType || "unknown",
    direction: item.direction || "neutral",
    severity: item.severity || "medium",
    score: numberOrNull(item.score) || 0,
    totalVolumeBtc,
    netVolumeBtc: numberOrNull(item.netVolumeBtc) || 0,
    totalNotionalUsd,
    dominance: numberOrNull(item.dominance) || 0,
    priceMovePct: numberOrNull(item.priceMovePct),
    triggerPriceUsd: normalizeTriggerPrice(item, totalVolumeBtc, totalNotionalUsd),
    mainExchange: item.mainExchange || "Multi",
    marketType: item.marketType ? String(item.marketType).toLowerCase() : "perp",
    sourceRole: item.sourceRole ? String(item.sourceRole).toLowerCase() : "optional",
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
    thresholdProfile: item.thresholdProfile ? String(item.thresholdProfile).toLowerCase() : activeSources.thresholdProfile,
    thresholdProfileReason: item.thresholdProfileReason ? String(item.thresholdProfileReason) : activeSources.thresholdProfileReason,
    configuredContractSources: normalizeStringArray(item.configuredContractSources).length
      ? normalizeStringArray(item.configuredContractSources)
      : activeSources.configuredContractSources,
    eligibleContractSources: normalizeStringArray(item.eligibleContractSources).length
      ? normalizeStringArray(item.eligibleContractSources)
      : activeSources.eligibleContractSources,
    activeContractSources: normalizeStringArray(item.activeContractSources).length
      ? normalizeStringArray(item.activeContractSources)
      : activeSources.activeContractSources,
    activeSources,
    discordEligible: Boolean(item.discordEligible),
    discordSent: Boolean(item.discordSent),
    discordSentAt: numberOrNull(item.discordSentAt),
    discordReason: item.discordReason || "not_sent",
    finalResult: item.finalResult || "contract whale flow candidate",
    mergedFrom: Array.isArray(item.mergedFrom) ? item.mergedFrom.filter(Boolean).map(String) : [],
  };
}

function normalizeTriggerPrice(item, totalVolumeBtc, totalNotionalUsd) {
  const explicit =
    numberOrNull(item.triggerPriceUsd) ??
    numberOrNull(item.triggerPrice) ??
    numberOrNull(item.price) ??
    numberOrNull(item.avgPriceUsd);
  if (explicit !== null && explicit > 0) {
    return explicit;
  }
  if (totalVolumeBtc > 0 && totalNotionalUsd > 0) {
    return totalNotionalUsd / totalVolumeBtc;
  }
  return null;
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

function normalizeActiveSources(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    contract: normalizeActiveSourceEntries(source.contract),
    spot: normalizeActiveSourceEntries(source.spot),
    thresholdProfile: source.thresholdProfile ? String(source.thresholdProfile).toLowerCase() : "three_exchange",
    thresholdProfileReason: source.thresholdProfileReason ? String(source.thresholdProfileReason) : "",
    configuredContractSources: normalizeStringArray(source.configuredContractSources),
    eligibleContractSources: normalizeStringArray(source.eligibleContractSources),
    activeContractSources: normalizeStringArray(source.activeContractSources),
  };
}

function normalizeActiveSourceEntries(entries) {
  if (!Array.isArray(entries)) return [];
  return entries.map((entry) => ({
    exchange: entry.exchange ? String(entry.exchange).toLowerCase() : "unknown",
    marketType: entry.marketType ? String(entry.marketType).toLowerCase() : "perp",
    sourceRole: entry.sourceRole ? String(entry.sourceRole).toLowerCase() : "optional",
    enabled: Boolean(entry.enabled),
    status: entry.status ? String(entry.status).toLowerCase() : "configured",
    productId: entry.productId ? String(entry.productId) : null,
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
    marketType: summary.marketType ? String(summary.marketType).toLowerCase() : calmSummary.marketType,
    meta: normalizeResponseMeta(summary.meta),
    thresholdProfile: summary.thresholdProfile || calmSummary.thresholdProfile,
    thresholdProfileReason: summary.thresholdProfileReason || calmSummary.thresholdProfileReason,
    configuredContractSources: normalizeStringArray(summary.configuredContractSources),
    eligibleContractSources: normalizeStringArray(summary.eligibleContractSources),
    activeExchangeCount: numberOrNull(summary.activeExchangeCount) || 0,
    enabledExchanges: normalizeStringArray(summary.enabledExchanges),
    disabledExchanges: normalizeStringArray(summary.disabledExchanges),
    activeContractExchanges: normalizeStringArray(summary.activeContractExchanges),
    activeContractSources: normalizeStringArray(summary.activeContractSources).length
      ? normalizeStringArray(summary.activeContractSources)
      : normalizeStringArray(summary.activeContractExchanges),
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
    contractDataQuality: numberOrNull(summary.contractDataQuality) || 0,
    spotDataQuality: numberOrNull(summary.spotDataQuality) || 0,
    overallDataQuality: numberOrNull(summary.overallDataQuality) || 0,
    trend60s: normalizeTrend60s(summary.trend60s),
    exchanges: normalizeExchanges(summary.exchanges),
    platforms: normalizePlatforms(summary.platforms),
  };
}

function normalizeResponseMeta(meta) {
  if (!meta || typeof meta !== "object") return null;
  return {
    exchange: meta.exchange ? String(meta.exchange).toLowerCase() : null,
    marketType: meta.marketType ? String(meta.marketType).toLowerCase() : null,
    exchangeStatus: meta.exchangeStatus ? String(meta.exchangeStatus).toLowerCase() : null,
    reason: meta.reason ? String(meta.reason).toLowerCase() : null,
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
  return ["binance", "okx", "bitfinex", "coinbase"].reduce((acc, key) => {
    const item = source[key] && typeof source[key] === "object" ? source[key] : {};
    acc[key] = {
      connected: Boolean(item.connected),
      status: item.status || (item.connected ? "connected" : "disconnected"),
      lastTradeAt: numberOrNull(item.lastTradeAt),
      latencyMs: numberOrNull(item.latencyMs),
      reconnectCount: numberOrNull(item.reconnectCount) || 0,
      platformEnabled: Boolean(item.platformEnabled ?? item.enabled),
      contractEnabled: Boolean(item.contractEnabled),
      enabledMarkets: normalizeStringArray(item.enabledMarkets),
      marketRoles: normalizeStringMap(item.marketRoles),
    };
    return acc;
  }, {});
}

function normalizePlatforms(platforms) {
  const source = platforms && typeof platforms === "object" ? platforms : {};
  return ["binance", "okx", "bitfinex", "coinbase"].reduce((acc, key) => {
    const item = source[key] && typeof source[key] === "object" ? source[key] : {};
    acc[key] = {
      platformEnabled: Boolean(item.platformEnabled ?? item.enabled),
      status: item.status || "disabled",
      markets: normalizePlatformMarkets(item.markets),
    };
    return acc;
  }, {});
}

function normalizePlatformMarkets(markets) {
  const source = markets && typeof markets === "object" ? markets : {};
  return ["spot", "perp", "level2", "funding", "oi", "liquidation"].reduce((acc, key) => {
    const item = source[key] && typeof source[key] === "object" ? source[key] : {};
    acc[key] = {
      enabled: Boolean(item.enabled),
      status: item.status || (item.enabled ? "enabled" : "disabled"),
      role: item.role || "disabled",
      product: item.product ? String(item.product) : null,
      source: item.source ? String(item.source) : null,
      lastTradeAt: numberOrNull(item.lastTradeAt),
      latencyMs: numberOrNull(item.latencyMs),
      reconnectCount: numberOrNull(item.reconnectCount) || 0,
      requiresAuth: Boolean(item.requiresAuth),
      marketDataOnly: item.marketDataOnly !== false,
      authConfigured: Boolean(item.authConfigured),
    };
    return acc;
  }, {});
}

function normalizeStringMap(value) {
  if (!value || typeof value !== "object") return {};
  return Object.fromEntries(
    Object.entries(value)
      .map(([key, item]) => [String(key || "").toLowerCase(), String(item || "").toLowerCase()])
      .filter(([key]) => Boolean(key)),
  );
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
