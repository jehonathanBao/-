import axios from "axios";

export const CWM_MAX_PRICE_DEVIATION_PCT = 5;
const DEFAULT_REQUEST_TIMEOUT_MS = 8_000;
const VOLUME_DISPLAY_CONTEXT = {
  SINGLE_WINDOW: "single_window",
  CONTRACT_EVENT_STREAM: "contract_event_stream",
  FINAL_LIFECYCLE_EVENT: "final_lifecycle_event",
};

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
  discordDryRunStats: {
    signals1h: 0,
    high1h: 0,
    critical1h: 0,
    s1h: 0,
    wouldSend1h: 0,
    skippedLowScore1h: 0,
    skippedCooldown1h: 0,
    skippedDataQuality1h: 0,
    skippedWarmup1h: 0,
    skippedDisplayOnly1h: 0,
  },
  marketStructureLite: {
    status: "calm",
    regimeType: "unclear",
    mainForceScore: 0,
    extremeImpactScore: 0,
    structureBias: 0,
    confidence: 0,
    dataQuality: 0,
    spotScore: 0,
    contractScore: 0,
    crossConfirmScore: 0,
    mainForceConfirmed: false,
    extremeImpactConfirmed: false,
    reason: "",
  },
  trend60s: {
    symbol: "BTC",
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
    binance: { connected: false, status: "initializing", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
    okx: { connected: false, status: "disabled", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
    bitfinex: { connected: false, status: "initializing", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
    coinbase: { connected: false, status: "spot_only", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
  },
  platforms: {
    binance: { platformEnabled: true, status: "active", markets: {} },
    bitfinex: { platformEnabled: true, status: "active", markets: {} },
    coinbase: { platformEnabled: true, status: "spot_only", markets: {} },
    okx: { platformEnabled: false, status: "disabled", markets: {} },
  },
};

export async function fetchJsonWithTimeout(
  url,
  {
    timeoutMs = DEFAULT_REQUEST_TIMEOUT_MS,
    axiosConfig = {},
    retryCount = 0,
    retryDelayMs = 0,
  } = {},
) {
  const attempts = Math.max(0, Number(retryCount) || 0) + 1;
  let lastError = null;
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    const controller = new AbortController();
    let timeoutId = null;
    const timeoutError = new Error(`Request timed out after ${timeoutMs}ms`);
    timeoutError.code = "ERR_CWM_TIMEOUT";
    const requestPromise = axios.get(url, {
      ...axiosConfig,
      signal: controller.signal,
    });
    const guardedRequestPromise = requestPromise.catch((error) => {
      if (controller.signal.aborted) {
        throw timeoutError;
      }
      throw error;
    });
    const timeoutPromise = new Promise((_, reject) => {
      timeoutId = setTimeout(() => {
        controller.abort();
        reject(timeoutError);
      }, timeoutMs);
    });

    try {
      return await Promise.race([guardedRequestPromise, timeoutPromise]);
    } catch (error) {
      lastError = error;
      if (error?.code === "ERR_CWM_TIMEOUT" && attempt < attempts - 1) {
        if (retryDelayMs > 0) {
          await new Promise((resolve) => setTimeout(resolve, retryDelayMs));
        }
        continue;
      }
      throw error;
    } finally {
      if (timeoutId !== null) {
        clearTimeout(timeoutId);
      }
      controller.abort();
    }
  }
  throw lastError;
}

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
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-whale/summary?${query}`, {
      timeoutMs: 5_000,
    });
    return {
      summary: normalizeSummary(response.data, symbol),
      meta: normalizeResponseMeta(response.data?.meta),
      error: null,
    };
  } catch {
    return { summary: fallbackSummary(symbol), meta: null, error: "summary_unavailable" };
  }
}

export async function fetchContractWhaleLatest(limit = 50, symbol = "BTC", options = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({
      limit,
      symbol,
      hide_stale: (options.hide_stale ?? options.hideStale) ? "true" : undefined,
    });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-whale/latest?${query}`, {
      timeoutMs: 5_000,
    });
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    return {
      summary: normalizeSummary(response.data?.summary, symbol),
      items: items
        .filter((item) => signalMatchesRequestedSymbol(item, symbol))
        .map((item) => normalizeContractWhaleSignal(item, symbol))
        .filter(isVisibleContractWhaleSignal),
      serverTime: numberOrNull(response.data?.serverTime ?? response.data?.server_time),
      maxTs: numberOrNull(response.data?.maxTs ?? response.data?.max_ts),
      maxAgeSec: numberOrNull(response.data?.maxAgeSec ?? response.data?.max_age_sec),
      staleCount: numberOrNull(response.data?.staleCount ?? response.data?.stale_count),
      timeline: normalizeCanonicalTimeline(response.data?.timeline),
      meta: normalizeResponseMeta(response.data?.meta),
      error: null,
    };
  } catch {
    return {
      summary: fallbackSummary(symbol),
      items: [],
      serverTime: null,
      maxTs: null,
      maxAgeSec: null,
      staleCount: null,
      timeline: null,
      meta: null,
      error: "latest_unavailable",
    };
  }
}

export async function fetchContractWhaleTimeline(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({
      symbol: filters.symbol || "BTC",
      range: filters.range ?? "24h",
      limit: filters.limit ?? 100,
    });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-whale/timeline?${query}`, {
      timeoutMs: 4_000,
    });
    return normalizeCanonicalTimelineResponse(response.data, filters);
  } catch {
    return normalizeCanonicalTimelineResponse(null, filters, "timeline_unavailable");
  }
}

export async function fetchContractWhaleTradingDecisions(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  const requestedSymbol = filters.symbol || "BTC";
  try {
    const query = buildContractWhaleQuery({
      symbol: requestedSymbol,
      range: filters.range ?? "24h",
      limit: filters.limit ?? 50,
      exchange: filters.exchange,
    });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-whale/trading-decisions?${query}`, {
      timeoutMs: 5_000,
    });
    return normalizeTradingDecisionResponse(response.data, requestedSymbol);
  } catch {
    return normalizeTradingDecisionResponse(null, requestedSymbol, "trading_decisions_unavailable");
  }
}

export async function fetchContractWhaleIntelligenceTerminal(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  const requestedSymbol = filters.symbol || "BTC";
  try {
    const query = buildContractWhaleQuery({
      symbol: requestedSymbol,
      range: filters.range ?? "24h",
      limit: filters.limit ?? 50,
      exchange: filters.exchange,
    });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-whale/intelligence-terminal?${query}`, {
      timeoutMs: 5_000,
    });
    return normalizeContractWhaleIntelligenceResponse(response.data, requestedSymbol);
  } catch {
    return normalizeContractWhaleIntelligenceResponse(null, requestedSymbol, "intelligence_terminal_unavailable");
  }
}

export async function fetchContractWhaleHistory(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({ ...filters, limit: filters.limit ?? 50 });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-whale/history?${query}`, {
      timeoutMs: 6_000,
    });
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    const requestedSymbol = filters.symbol || "BTC";
    return {
      summary: normalizeSummary(response.data?.summary, requestedSymbol),
      items: items
        .filter((item) => signalMatchesRequestedSymbol(item, requestedSymbol))
        .map((item) => normalizeContractWhaleSignal(item, requestedSymbol))
        .filter(isVisibleContractWhaleSignal),
      meta: normalizeResponseMeta(response.data?.meta),
      error: null,
    };
  } catch {
    return { summary: fallbackSummary(filters.symbol || "BTC"), items: [], meta: null, error: "history_unavailable" };
  }
}

export async function fetchContractWhaleEvents(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({ ...filters, limit: filters.limit ?? 20 });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-whale/events?${query}`, {
      timeoutMs: 6_000,
    });
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    const requestedSymbol = filters.symbol || "BTC";
    return {
      items: items
        .filter((item) => signalMatchesRequestedSymbol(item, requestedSymbol))
        .map((item) => normalizeMainForceEvent(item, requestedSymbol)),
      error: null,
    };
  } catch {
    return { items: [], error: "events_unavailable" };
  }
}

export async function fetchContractEvents(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const includeHidden = Boolean(filters.includeHidden ?? filters.include_hidden);
    const query = buildContractWhaleQuery({
      ...filters,
      include_hidden: includeHidden ? "true" : undefined,
      includeHidden: undefined,
      range: filters.range ?? "24h",
      limit: filters.limit ?? 100,
      min_notional_usd: filters.min_notional_usd ?? 10_000_000,
    });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-events?${query}`, {
      timeoutMs: 6_000,
    });
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    const requestedSymbol = filters.symbol || "BTC";
    const normalizedItems = items
      .filter((item) => signalMatchesRequestedSymbol(item, requestedSymbol))
      .map((item) => normalizeContractEvent(item, requestedSymbol));
    return {
      items: includeHidden ? normalizedItems : normalizedItems.filter(isVisibleContractWhaleSignal),
      nextCursor: response.data?.nextCursor ?? response.data?.next_cursor ?? null,
      hasMore: Boolean(response.data?.hasMore ?? response.data?.has_more),
      limit: numberOrNull(response.data?.limit) ?? filters.limit ?? 100,
      range: String(response.data?.range || filters.range || "24h"),
      serverTime: numberOrNull(response.data?.serverTime ?? response.data?.server_time),
      lastEventTs: numberOrNull(response.data?.lastEventTs ?? response.data?.last_event_ts),
      maxEventTs: numberOrNull(response.data?.maxEventTs ?? response.data?.max_event_ts),
      maxPersistedAt: numberOrNull(response.data?.maxPersistedAt ?? response.data?.max_persisted_at),
      historyLagSec: numberOrNull(response.data?.historyLagSec ?? response.data?.history_lag_sec),
      latestLagSec: numberOrNull(response.data?.latestLagSec ?? response.data?.latest_lag_sec),
      cacheAgeSec: numberOrNull(response.data?.cacheAgeSec ?? response.data?.cache_age_sec),
      cacheTtlSec: numberOrNull(response.data?.cacheTtlSec ?? response.data?.cache_ttl_sec),
      timeline: normalizeCanonicalTimeline(response.data?.timeline),
      error: null,
    };
  } catch {
    return {
      items: [],
      nextCursor: null,
      hasMore: false,
      limit: filters.limit ?? 100,
      range: filters.range || "24h",
      serverTime: null,
      lastEventTs: null,
      maxEventTs: null,
      maxPersistedAt: null,
      historyLagSec: null,
      latestLagSec: null,
      cacheAgeSec: null,
      cacheTtlSec: null,
      timeline: null,
      error: "contract_events_unavailable",
    };
  }
}

export async function fetchContractEventDebugCounts(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({
      symbol: filters.symbol || "BTC",
      range: filters.range ?? "24h",
      include_hidden: filters.includeHidden || filters.include_hidden ? "true" : undefined,
    });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-events/debug-counts?${query}`, {
      timeoutMs: 4_000,
    });
    return {
      symbol: String(response.data?.symbol || filters.symbol || "BTC"),
      range: String(response.data?.range || filters.range || "24h"),
      generatedAt: response.data?.generatedAt ?? response.data?.generated_at ?? null,
      db: response.data?.db || null,
      apiQuery: response.data?.apiQuery ?? response.data?.api_query ?? null,
      visibility: response.data?.visibility || null,
      latest: response.data?.latest || null,
      latestStaleCount: numberOrNull(response.data?.latest?.staleCount ?? response.data?.latest?.stale_count),
      finalEventsV2: response.data?.finalEventsV2 ?? response.data?.final_events_v2 ?? null,
      latestVsHistory: Array.isArray(response.data?.latestVsHistory ?? response.data?.latest_vs_history)
        ? (response.data?.latestVsHistory ?? response.data?.latest_vs_history)
        : [],
      latestItems: Array.isArray(response.data?.latest?.items) ? response.data.latest.items : [],
      finalEventsProjection:
        response.data?.finalEventsProjection ?? response.data?.final_events_projection ?? null,
      error: response.data?.error || null,
    };
  } catch {
    return {
      symbol: String(filters.symbol || "BTC"),
      range: String(filters.range || "24h"),
      generatedAt: null,
      db: null,
      apiQuery: null,
      visibility: null,
      latest: null,
      finalEventsV2: null,
      latestVsHistory: [],
      finalEventsProjection: null,
      error: "debug_counts_unavailable",
    };
  }
}

export async function fetchContractWhaleRawFlowDebug(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({
      symbol: filters.symbol || "BTC",
      range: filters.range ?? "24h",
    });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-whale/raw-flow-debug?${query}`, {
      timeoutMs: 4_000,
    });
    return {
      symbol: String(response.data?.symbol || filters.symbol || "BTC"),
      range: String(response.data?.range || filters.range || "24h"),
      config: response.data?.config || null,
      rawTradeIngest: response.data?.rawTradeIngest ?? response.data?.raw_trade_ingest ?? null,
      normalizer: response.data?.normalizer || null,
      aggregator: response.data?.aggregator || null,
      contractFlow1s: response.data?.contractFlow1s ?? response.data?.contract_flow_1s ?? null,
      diagnosis: response.data?.diagnosis || null,
      error: response.data?.error || null,
    };
  } catch {
    return {
      symbol: String(filters.symbol || "BTC"),
      range: String(filters.range || "24h"),
      config: null,
      rawTradeIngest: null,
      normalizer: null,
      aggregator: null,
      contractFlow1s: null,
      diagnosis: null,
      error: "raw_flow_debug_unavailable",
    };
  }
}

export async function fetchFinalEvents(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({ ...filters, limit: filters.limit ?? 20 });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/final-events?${query}`, {
      timeoutMs: 6_000,
    });
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    const requestedSymbol = filters.symbol || "BTC";
    const normalizedItems = items
      .filter((item) => signalMatchesRequestedSymbol(item, requestedSymbol))
      .map((item) => normalizeFinalEvent(item, requestedSymbol))
      .filter(isVisibleContractWhaleSignal);
    return {
      count: numberOrNull(response.data?.count) ?? normalizedItems.length,
      items: normalizedItems,
      error: null,
    };
  } catch {
    return { count: 0, items: [], error: "final_events_unavailable" };
  }
}

export async function fetchFinalEventsV2(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({ ...filters, range: filters.range ?? "24h", limit: filters.limit ?? 100 });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/final-events-v2?${query}`, {
      timeoutMs: 6_000,
    });
    const requestedSymbol = filters.symbol || "BTC";
    const normalizeArray = (items) =>
      (Array.isArray(items) ? items : [])
        .filter((item) => signalMatchesRequestedSymbol(item, requestedSymbol))
        .map((item) => normalizeFinalEvent(item, requestedSymbol))
        .filter(isVisibleContractWhaleSignal);

    return {
      active: normalizeArray(response.data?.active),
      closed: normalizeArray(response.data?.closed),
      nextCursor: response.data?.nextCursor ?? response.data?.next_cursor ?? null,
      hasMore: Boolean(response.data?.hasMore ?? response.data?.has_more),
      limit: numberOrNull(response.data?.limit) ?? filters.limit ?? 100,
      range: String(response.data?.range || filters.range || "24h"),
      serverTime: numberOrNull(response.data?.serverTime ?? response.data?.server_time),
      lastEventTs: numberOrNull(response.data?.lastEventTs ?? response.data?.last_event_ts),
      maxEventTs: numberOrNull(response.data?.maxEventTs ?? response.data?.max_event_ts),
      generatedAt: numberOrNull(response.data?.generatedAt ?? response.data?.generated_at),
      cacheAgeSec: numberOrNull(response.data?.cacheAgeSec ?? response.data?.cache_age_sec),
      cacheTtlSec: numberOrNull(response.data?.cacheTtlSec ?? response.data?.cache_ttl_sec),
      projectionLagSec: numberOrNull(response.data?.projectionLagSec ?? response.data?.projection_lag_sec),
      timeline: normalizeCanonicalTimeline(response.data?.timeline),
      error: null,
    };
  } catch {
    return {
      active: [],
      closed: [],
      nextCursor: null,
      hasMore: false,
      limit: filters.limit ?? 100,
      range: filters.range || "24h",
      serverTime: null,
      lastEventTs: null,
      maxEventTs: null,
      generatedAt: null,
      cacheAgeSec: null,
      cacheTtlSec: null,
      projectionLagSec: null,
      timeline: null,
      error: "final_events_v2_unavailable",
    };
  }
}

export async function fetchContractWhaleLatencyDebug(filters = {}) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const query = buildContractWhaleQuery({
      symbol: filters.symbol || "BTC",
      range: filters.range ?? "24h",
    });
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-whale/latency-debug?${query}`, {
      timeoutMs: 4_000,
    });
    return {
      symbol: String(response.data?.symbol || filters.symbol || "BTC"),
      range: String(response.data?.range || filters.range || "24h"),
      serverTime: numberOrNull(response.data?.serverTime ?? response.data?.server_time),
      timeline: normalizeCanonicalTimelineResponse(response.data?.timeline, filters),
      latest: response.data?.latest || null,
      contractEvents: response.data?.contractEvents ?? response.data?.contract_events ?? null,
      finalEventsV2: response.data?.finalEventsV2 ?? response.data?.final_events_v2 ?? null,
      flow: response.data?.flow || null,
      diagnosis: response.data?.diagnosis || null,
      error: response.data?.error || null,
    };
  } catch {
    return {
      symbol: String(filters.symbol || "BTC"),
      range: String(filters.range || "24h"),
      serverTime: null,
      timeline: normalizeCanonicalTimelineResponse(null, filters),
      latest: null,
      contractEvents: null,
      finalEventsV2: null,
      flow: null,
      diagnosis: null,
      error: "latency_debug_unavailable",
    };
  }
}

export async function fetchContractRetentionStatus() {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const response = await fetchJsonWithTimeout(`${baseURL}/api/contract-retention-status`, {
      timeoutMs: 4_000,
    });
    return response.data || null;
  } catch {
    return {
      flowRetentionDays: 14,
      signalRetentionDays: 365,
      signalProtectSeverityS: true,
      signalProtectNetVolumeBtc: 500,
      cleanupIntervalHours: 1,
      tables: {
        contractFlow1s: { rowCount: null, reason: "query_failed" },
        contractWhaleSignals: { rowCount: null, reason: "query_failed" },
        mainForceEvents: { rowCount: null, reason: "query_failed" },
      },
      error: "retention_status_unavailable",
    };
  }
}

function normalizeTradingDecisionResponse(payload, symbol = "BTC", fallbackError = null) {
  const data = payload && typeof payload === "object" ? payload : {};
  return {
    symbol: String(data.symbol || symbol),
    semanticType: data.semanticType || data.semantic_type || "decision_support",
    riskState: data.riskState || data.risk_state || "low",
    timestamp: numberOrNull(data.timestamp),
    marketBias: String(data.marketBias || data.market_bias || "NEUTRAL"),
    biasConfidence: numberOrNull(data.biasConfidence ?? data.bias_confidence) ?? 0,
    biasReason: String(data.biasReason || data.bias_reason || "当前没有通过交易门槛的 setup，保持中性观察。"),
    noiseSuppression: data.noiseSuppression ?? data.noise_suppression ?? {
      rawCandidates: 0,
      mergedEvents: 0,
      lifecycleEvents: 0,
      filteredEvents: 0,
      tradeableSetups: 0,
      suppressedDuplicates: 0,
      noiseReductionPct: 0,
    },
    topSetups: Array.isArray(data.topSetups ?? data.top_setups)
      ? (data.topSetups ?? data.top_setups).map(normalizeTradingSetup)
      : [],
    noTradeZones: Array.isArray(data.noTradeZones ?? data.no_trade_zones)
      ? (data.noTradeZones ?? data.no_trade_zones).map(normalizeNoTradeZone)
      : [],
    error: fallbackError || data.error || null,
  };
}

function normalizeTradingSetup(setup = {}) {
  const pressureZone = setup.pressureZone ?? setup.pressure_zone ?? setup.entryZone ?? setup.entry_zone ?? {};
  const riskBoundary = setup.riskBoundary ?? setup.risk_boundary ?? setup.invalidation ?? {};
  return {
    semanticType: setup.semanticType || setup.semantic_type || "decision_support",
    riskState: setup.riskState || setup.risk_state || "low",
    signalId: setup.signalId || setup.signal_id || "",
    rank: Number(setup.rank || 0),
    directionBias: setup.directionBias || setup.direction_bias || setup.direction || "NEUTRAL_BIAS",
    setupType: setup.setupType || setup.setup_type || "结构观察",
    score: Number(setup.score || 0),
    confidence: Number(setup.confidence || 0),
    confidenceLabel: setup.confidenceLabel || setup.confidence_label || "LOW",
    regimeContext: setup.regimeContext || setup.regime_context || "unclear",
    windowSec: Number(setup.windowSec ?? setup.window_sec ?? 0),
    pressureZone: {
      lowPrice: Number(pressureZone.lowPrice ?? pressureZone.low_price ?? 0),
      highPrice: Number(pressureZone.highPrice ?? pressureZone.high_price ?? 0),
      label: pressureZone.label || "",
    },
    riskBoundary: {
      priceLevel: Number(riskBoundary.priceLevel ?? riskBoundary.price_level ?? 0),
      reason: riskBoundary.reason || "暂无风险边界说明",
    },
    reasons: Array.isArray(setup.reasons) ? setup.reasons : [],
  };
}

function normalizeNoTradeZone(zone = {}) {
  return {
    reason: zone.reason || "当前不满足交易门槛，保留为观察区。",
    rangeLabel: zone.rangeLabel || zone.range_label || "",
    lowPrice: Number(zone.lowPrice ?? zone.low_price ?? 0),
    highPrice: Number(zone.highPrice ?? zone.high_price ?? 0),
  };
}

function normalizeContractWhaleIntelligenceResponse(payload, symbol = "BTC", fallbackError = null) {
  const data = payload && typeof payload === "object" ? payload : {};
  return {
    symbol: String(data.symbol || symbol),
    semanticType: data.semanticType || data.semantic_type || "analysis",
    timestamp: numberOrNull(data.timestamp),
    marketRegime: {
      regime: String(data.marketRegime?.regime ?? data.market_regime?.regime ?? "RANGING"),
      confidence: numberOrNull(data.marketRegime?.confidence ?? data.market_regime?.confidence) ?? 0,
      reason: String(data.marketRegime?.reason ?? data.market_regime?.reason ?? "当前缺少足够的主力结构信号。"),
    },
    liquidityBehaviors: Array.isArray(data.liquidityBehaviors ?? data.liquidity_behaviors)
      ? (data.liquidityBehaviors ?? data.liquidity_behaviors).map(normalizeLiquidityBehavior)
      : [],
    rankedEvents: Array.isArray(data.rankedEvents ?? data.ranked_events)
      ? (data.rankedEvents ?? data.ranked_events).map(normalizeRankedEvent)
      : [],
    opportunityMap: Array.isArray(data.opportunityMap ?? data.opportunity_map)
      ? (data.opportunityMap ?? data.opportunity_map).map(normalizeOpportunityZone)
      : [],
    noiseSuppression: data.noiseSuppression ?? data.noise_suppression ?? {
      rawCandidates: 0,
      mergedEvents: 0,
      lifecycleEvents: 0,
      filteredEvents: 0,
      tradeableSetups: 0,
      suppressedDuplicates: 0,
      noiseReductionPct: 0,
    },
    signalCompression: normalizeSignalCompression(
      data.signalCompression ?? data.signal_compression,
    ),
    tradeIdeas: Array.isArray(data.tradeIdeas ?? data.trade_ideas)
      ? (data.tradeIdeas ?? data.trade_ideas).map(normalizeTradeIdea)
      : [],
    riskContext: normalizeRiskContext(data.riskContext ?? data.risk_context),
    error: fallbackError || data.error || null,
  };
}

function normalizeSignalCompression(summary = {}) {
  return {
    qualityScore: Number(summary.qualityScore ?? summary.quality_score ?? 0),
    topSignalCount: Number(summary.topSignalCount ?? summary.top_signal_count ?? 0),
    discardedCount: Number(summary.discardedCount ?? summary.discarded_count ?? 0),
    compressionReason: String(summary.compressionReason ?? summary.compression_reason ?? "compression pending"),
  };
}

function normalizeTradeIdea(idea = {}) {
  const pressureZone = idea.pressureZone ?? idea.pressure_zone ?? idea.entryZone ?? idea.entry_zone ?? {};
  const riskBoundary = idea.riskBoundary ?? idea.risk_boundary ?? idea.invalidation ?? {};
  return {
    semanticType: idea.semanticType || idea.semantic_type || "decision_support",
    riskState: idea.riskState || idea.risk_state || "low",
    signalId: idea.signalId || idea.signal_id || "",
    rank: Number(idea.rank || 0),
    setupType: idea.setupType || idea.setup_type || "结构观察",
    directionBias: idea.directionBias || idea.direction_bias || "NEUTRAL_BIAS",
    score: Number(idea.score || 0),
    confidence: Number(idea.confidence || 0),
    confidenceLabel: idea.confidenceLabel || idea.confidence_label || "LOW",
    pressureZone: {
      lowPrice: Number(pressureZone.lowPrice ?? pressureZone.low_price ?? 0),
      highPrice: Number(pressureZone.highPrice ?? pressureZone.high_price ?? 0),
      label: pressureZone.label || "",
    },
    riskBoundary: {
      priceLevel: Number(riskBoundary.priceLevel ?? riskBoundary.price_level ?? 0),
      reason: riskBoundary.reason || "暂无风险边界说明",
    },
    structureContext: idea.structureContext || idea.structure_context || "当前结构暂无补充说明。",
    regimeContext: idea.regimeContext || idea.regime_context || "unclear",
    windowSec: Number(idea.windowSec ?? idea.window_sec ?? 0),
  };
}

function normalizeRiskContext(context = {}) {
  return {
    semanticType: context.semanticType || context.semantic_type || "risk_override",
    riskState: context.riskState || context.risk_state || "low",
    fakeBreakoutRisk: String(context.fakeBreakoutRisk ?? context.fake_breakout_risk ?? "LOW"),
    summary: String(context.summary || "当前未发现显著 no-trade 结构风险。"),
    noTradeZones: Array.isArray(context.noTradeZones ?? context.no_trade_zones)
      ? (context.noTradeZones ?? context.no_trade_zones).map(normalizeNoTradeZone)
      : [],
  };
}

function normalizeLiquidityBehavior(item = {}) {
  return {
    behavior: item.behavior || "order_block_behavior",
    label: item.label || "Order Block",
    strengthScore: Number(item.strengthScore ?? item.strength_score ?? 0),
    confidence: Number(item.confidence ?? 0),
    reason: item.reason || "当前结构暂无额外解释。",
    rangeLabel: item.rangeLabel || item.range_label || "N/A",
    lowPrice: Number(item.lowPrice ?? item.low_price ?? 0),
    highPrice: Number(item.highPrice ?? item.high_price ?? 0),
  };
}

function normalizeRankedEvent(item = {}) {
  return {
    signalId: item.signalId || item.signal_id || "",
    rank: Number(item.rank || 0),
    eventType: item.eventType || item.event_type || "结构事件",
    directionBias: item.directionBias || item.direction_bias || "NEUTRAL",
    strengthScore: Number(item.strengthScore ?? item.strength_score ?? 0),
    strengthLabel: item.strengthLabel || item.strength_label || "LOW",
    regimeAlignment: item.regimeAlignment || item.regime_alignment || "mixed",
    liquidityBehavior: item.liquidityBehavior || item.liquidity_behavior || "order_block_behavior",
    windowSec: Number(item.windowSec ?? item.window_sec ?? 0),
    rationale: item.rationale || "当前结构机会仍需结合上下文理解。",
  };
}

function normalizeOpportunityZone(item = {}) {
  return {
    zoneType: item.zoneType || item.zone_type || "order_block_zone",
    label: item.label || "Order Block",
    lowPrice: Number(item.lowPrice ?? item.low_price ?? 0),
    highPrice: Number(item.highPrice ?? item.high_price ?? 0),
    rangeLabel: item.rangeLabel || item.range_label || "N/A",
    strengthScore: Number(item.strengthScore ?? item.strength_score ?? 0),
    description: item.description || "当前结构机会暂无补充说明。",
  };
}

function volumeDisplayLabelForContext(context) {
  if (context === VOLUME_DISPLAY_CONTEXT.CONTRACT_EVENT_STREAM) return "累计总流量 BTC";
  if (context === VOLUME_DISPLAY_CONTEXT.FINAL_LIFECYCLE_EVENT) return "生命周期累计流量 BTC";
  return "窗口总流量 BTC";
}

function volumeSemanticsForContext(context) {
  if (context === VOLUME_DISPLAY_CONTEXT.SINGLE_WINDOW) {
    return "single_window_bidirectional_cross_exchange";
  }
  return "multi_exchange_bidirectional_lifecycle_accumulated";
}

function deriveBuySellFromTotalNet(total, net) {
  const totalNumber = numberOrNull(total);
  const netNumber = numberOrNull(net);
  if (totalNumber === null || netNumber === null) {
    return { buy: null, sell: null };
  }
  const buy = (totalNumber + netNumber) / 2;
  const sell = (totalNumber - netNumber) / 2;
  if (!Number.isFinite(buy) || !Number.isFinite(sell) || buy < -1e-9 || sell < -1e-9) {
    return { buy: null, sell: null };
  }
  return {
    buy: Math.max(0, buy),
    sell: Math.max(0, sell),
  };
}

function uniqueExchangeNames(exchanges) {
  const values = new Set();
  for (const item of Array.isArray(exchanges) ? exchanges : []) {
    const exchange = String(item?.exchange || item?.name || item || "").trim().toLowerCase();
    if (exchange) {
      values.add(exchange);
    }
  }
  return Array.from(values);
}

function mergeWindowsSec(item, fallbackWindowSec) {
  const values = new Set();
  const candidateLists = [
    item?.mergedWindowsSec,
    item?.merged_windows_sec,
    item?.sourceSignal?.mergedWindowsSec,
    item?.sourceSignal?.merged_windows_sec,
    item?.mergedFrom,
    item?.sourceSignalIds,
  ];
  for (const candidate of candidateLists) {
    if (!Array.isArray(candidate)) continue;
    for (const entry of candidate) {
      if (typeof entry === "number" && Number.isFinite(entry) && entry > 0) {
        values.add(Math.round(entry));
        continue;
      }
      const text = String(entry || "");
      const parsed = Number.parseInt(text.split(":")[2], 10);
      if (Number.isFinite(parsed) && parsed > 0) {
        values.add(parsed);
      }
    }
  }
  const fallback = Number(fallbackWindowSec);
  if (!values.size && Number.isFinite(fallback) && fallback > 0) {
    values.add(Math.round(fallback));
  }
  return Array.from(values).sort((left, right) => left - right);
}

function normalizeVolumeDisplayMeta(item, context, fallbackWindowSec = null) {
  const rawTotal =
    numberOrNull(item?.displayVolumeBtc ?? item?.display_volume_btc) ??
    numberOrNull(item?.totalVolumeBtc ?? item?.total_volume_btc) ??
    numberOrNull(item?.volume ?? item?.volume_btc) ??
    0;
  const rawNet =
    numberOrNull(item?.netVolumeBtc ?? item?.net_volume_btc) ??
    numberOrNull(item?.netVolume ?? item?.net_volume) ??
    0;
  const explicitBuy = numberOrNull(item?.buyVolumeBtc ?? item?.buy_volume_btc);
  const explicitSell = numberOrNull(item?.sellVolumeBtc ?? item?.sell_volume_btc);
  const derived = deriveBuySellFromTotalNet(rawTotal, rawNet);
  const sourceExchanges = uniqueExchangeNames(
    item?.sourceExchanges ?? item?.source_exchanges ?? item?.exchanges ?? item?.sourceSignal?.exchanges,
  );
  const sourceExchangeCount = numberOrNull(item?.sourceExchangeCount ?? item?.source_exchange_count);
  return {
    displayVolumeBtc: rawTotal,
    displayVolumeLabel: item?.displayVolumeLabel ?? item?.display_volume_label ?? volumeDisplayLabelForContext(context),
    volumeSemantics: item?.volumeSemantics ?? item?.volume_semantics ?? volumeSemanticsForContext(context),
    isBidirectionalVolume: Boolean(item?.isBidirectionalVolume ?? item?.is_bidirectional_volume ?? true),
    isCrossExchangeAggregated: Boolean(
      item?.isCrossExchangeAggregated ?? item?.is_cross_exchange_aggregated ?? sourceExchanges.length > 1,
    ),
    isLifecycleAccumulated: Boolean(
      item?.isLifecycleAccumulated ?? item?.is_lifecycle_accumulated ?? context !== VOLUME_DISPLAY_CONTEXT.SINGLE_WINDOW,
    ),
    mergedSignalCount: Math.max(
      1,
      Math.round(
        numberOrNull(item?.mergedSignalCount ?? item?.merged_signal_count) ||
          numberOrNull(item?.sourceSignal?.eventLifecycle?.updateCount) ||
          numberOrNull(item?.eventLifecycle?.updateCount) ||
          sourceExchanges.length ||
          1,
      ),
    ),
    sourceExchangeCount: sourceExchangeCount === null ? (sourceExchanges.length ? sourceExchanges.length : null) : sourceExchangeCount,
    sourceExchanges,
    mergedWindowsSec: mergeWindowsSec(item, fallbackWindowSec ?? item?.windowSec ?? item?.window_sec ?? 0),
    buyVolumeBtc: explicitBuy ?? derived.buy,
    sellVolumeBtc: explicitSell ?? derived.sell,
  };
}

function impactLevelFromLegacySignals(dynamicThresholdLevel, percentileLevel, impactScore) {
  const percentile = numberOrNull(percentileLevel);
  const score = numberOrNull(impactScore);
  const threshold = String(dynamicThresholdLevel || "").toLowerCase();

  if (percentile !== null) {
    if (percentile > 97) return "S";
    if (percentile >= 90) return "A";
    if (percentile >= 80) return "B";
  }

  if (score !== null) {
    if (score > 5) return "S";
    if (score >= 3) return "A";
    if (score >= 1.8) return "B";
  }

  if (threshold === "s") return "S";
  if (threshold === "critical") return "A";
  if (threshold === "high") return "B";
  return "C";
}

function impactLevelToSignalLevel(impactLevel) {
  if (impactLevel === "S") return "S";
  if (impactLevel === "A") return "L3";
  if (impactLevel === "B") return "L2";
  return "L1";
}

function impactLevelToSignalLabel(impactLevel) {
  if (impactLevel === "S") return "SHOCK IMPACT EVENT";
  if (impactLevel === "A") return "HIGH IMPACT EVENT";
  if (impactLevel === "B") return "MEDIUM IMPACT EVENT";
  return "LOW IMPACT EVENT";
}

function impactLevelToNormalizedStrength(impactLevel) {
  if (impactLevel === "S") return "EXTREME";
  if (impactLevel === "A") return "HIGH";
  if (impactLevel === "B") return "MEDIUM";
  return "LOW";
}

function resolveImpactNormalization(item, { dynamicThresholdLevel = null, impactScoreFallback = null, percentileFallback = null } = {}) {
  const impactScore =
    numberOrNull(item?.impactScore ?? item?.impact_score) ??
    numberOrNull(impactScoreFallback);
  const zScore = numberOrNull(item?.zScore ?? item?.z_score);
  const percentile =
    numberOrNull(item?.percentile ?? item?.percentile_level) ??
    numberOrNull(percentileFallback);
  const explicitImpactLevel = item?.impactLevel ?? item?.impact_level;
  const impactLevel = explicitImpactLevel
    ? String(explicitImpactLevel).toUpperCase()
    : impactLevelFromLegacySignals(
        dynamicThresholdLevel ?? item?.dynamicThresholdLevel ?? item?.dynamic_threshold_level,
        percentile,
        impactScore,
      );
  const explicitSignalLevel = item?.signalLevel ?? item?.signal_level;
  const explicitSignalLabel = item?.signalLabel ?? item?.signal_label;
  const explicitNormalizedStrength = item?.normalizedStrength ?? item?.normalized_strength;

  return {
    impactScore,
    zScore,
    percentile,
    normalizedScore: clampRatio(numberOrNull(item?.normalizedScore ?? item?.normalized_score) ?? 0),
    normalizedStrength: explicitNormalizedStrength
      ? String(explicitNormalizedStrength).toUpperCase()
      : impactLevelToNormalizedStrength(impactLevel),
    impactLevel,
    signalLevel: explicitSignalLevel
      ? String(explicitSignalLevel).toUpperCase()
      : impactLevelToSignalLevel(impactLevel),
    signalLabel: explicitSignalLabel
      ? String(explicitSignalLabel).toUpperCase()
      : impactLevelToSignalLabel(impactLevel),
  };
}

export function normalizeFinalEvent(item, fallbackSymbol = "BTC") {
  const sourceSignal = item?.sourceSignal && typeof item.sourceSignal === "object" ? item.sourceSignal : {};
  const eventSymbol = item?.symbol || sourceSignal.symbol || fallbackSymbol || "BTC";
  const signal = normalizeContractWhaleSignal(sourceSignal, eventSymbol);
  const sourceSignalIds = normalizeRawStringArray(item?.sourceSignalIds);
  const eventId = item?.eventId ? String(item.eventId) : (signal.eventLifecycle?.eventId || signal.id);
  const eventType = item?.eventType ? String(item.eventType) : signal.signalType;
  const status = String(item?.status || signal.eventLifecycle?.status || "active").toLowerCase() === "closed" ? "closed" : "active";
  const volumeMeta = normalizeVolumeDisplayMeta(
    {
      ...item,
      sourceSignal: signal,
      sourceSignalIds,
    },
    VOLUME_DISPLAY_CONTEXT.FINAL_LIFECYCLE_EVENT,
    item?.windowSec ?? signal.windowSec,
  );
  const volume = numberOrNull(item?.volume) ?? signal.totalVolumeBtc;
  const netVolume = numberOrNull(item?.netVolume) ?? signal.netVolumeBtc;
  const notional = numberOrNull(item?.notional) ?? signal.totalNotionalUsd;
  const price = numberOrNull(item?.price) ?? signal.orderPriceUsd ?? signal.triggerPriceUsd;
  const priceMovePct = numberOrNull(item?.priceMovePct) ?? signal.priceMovePct;
  const dominance = clampRatio(numberOrNull(item?.dominance) ?? signal.dominance);
  const rawVolume = numberOrNull(item?.rawVolume) ?? volume;
  const impact = resolveImpactNormalization(item, {
    dynamicThresholdLevel: signal.dynamicThresholdLevel,
    impactScoreFallback: signal.dynamicMultiple,
    percentileFallback: signal.percentileLevel,
  });
  const falseEventFlags = normalizeStringArray(item?.falseEventFlags);
  const mergedFrom = sourceSignalIds.length > 1 ? sourceSignalIds.slice(1) : signal.mergedFrom;
  const finalEvent = {
    eventId,
    symbol: eventSymbol,
    eventType,
    startTime: numberOrNull(item?.startTime),
    endTime: numberOrNull(item?.endTime),
    status,
    windowSec: numberOrNull(item?.windowSec),
    rawVolume,
    impactScore: impact.impactScore,
    zScore: impact.zScore,
    percentile: impact.percentile,
    normalizedScore: impact.normalizedScore,
    normalizedStrength: impact.normalizedStrength,
    impactLevel: impact.impactLevel,
    signalLevel: impact.signalLevel,
    signalLabel: impact.signalLabel,
    volume,
    totalVolumeBtc: volume,
    netVolume,
    netVolumeBtc: netVolume,
    notional,
    totalNotionalUsd: notional,
    buyVolumeBtc: volumeMeta.buyVolumeBtc,
    sellVolumeBtc: volumeMeta.sellVolumeBtc,
    displayVolumeBtc: volumeMeta.displayVolumeBtc,
    displayVolumeLabel: volumeMeta.displayVolumeLabel,
    volumeSemantics: volumeMeta.volumeSemantics,
    isBidirectionalVolume: volumeMeta.isBidirectionalVolume,
    isCrossExchangeAggregated: volumeMeta.isCrossExchangeAggregated,
    isLifecycleAccumulated: volumeMeta.isLifecycleAccumulated,
    mergedSignalCount: volumeMeta.mergedSignalCount,
    sourceExchangeCount: volumeMeta.sourceExchangeCount,
    sourceExchanges: volumeMeta.sourceExchanges,
    mergedWindowsSec: volumeMeta.mergedWindowsSec,
    price,
    priceMovePct,
    directionBias: item?.directionBias || signal.direction,
    dominance,
    qualityScore: clampRatio(numberOrNull(item?.qualityScore) ?? signal.eventQuality?.qualityScore ?? 1),
    mergeSimilarityScore: clampRatio(
      numberOrNull(item?.mergeSimilarityScore) ?? signal.eventQuality?.mergeSimilarityScore ?? 1,
    ),
    falseEventFlags,
    sourceSignalIds,
  };

  return {
    ...signal,
    id: eventId,
    eventId,
    finalEventId: eventId,
    sourceSignalId: item?.sourceSignalId || signal.id,
    rawVolume,
    impactScore: impact.impactScore,
    zScore: impact.zScore,
    percentile: impact.percentile,
    normalizedScore: impact.normalizedScore,
    normalizedStrength: impact.normalizedStrength,
    impactLevel: impact.impactLevel,
    signalLevel: impact.signalLevel,
    signalLabel: impact.signalLabel,
    ts: numberOrNull(item?.endTime) ?? signal.ts,
    symbol: eventSymbol,
    baseAsset: eventSymbol,
    quantityUnit: eventSymbol,
    signalType: eventType,
    direction: item?.directionBias || signal.direction,
    windowSec: numberOrNull(item?.windowSec) || signal.windowSec,
    totalVolumeBtc: volume,
    netVolumeBtc: netVolume,
    totalNotionalUsd: notional,
    dominance,
    triggerPriceUsd: price ?? signal.triggerPriceUsd,
    orderPriceUsd: price ?? signal.orderPriceUsd,
    priceMovePct,
    mergedFrom,
    eventLifecycle: {
      ...signal.eventLifecycle,
      eventId,
      startTime: numberOrNull(item?.startTime) ?? signal.eventLifecycle?.startTime,
      lastUpdateTime: numberOrNull(item?.endTime) ?? signal.eventLifecycle?.lastUpdateTime,
      status,
      volumeAccumulated: volume,
      updateCount: Math.max(1, sourceSignalIds.length || signal.eventLifecycle?.updateCount || 1),
    },
    eventQuality: {
      ...signal.eventQuality,
      qualityScore: clampRatio(numberOrNull(item?.qualityScore) ?? signal.eventQuality?.qualityScore ?? 1),
      mergeSimilarityScore: clampRatio(
        numberOrNull(item?.mergeSimilarityScore) ?? signal.eventQuality?.mergeSimilarityScore ?? 1,
      ),
      valid: falseEventFlags.length === 0 && (signal.eventQuality?.valid ?? true),
      falseEventFlags,
    },
    finalEvent,
  };
}

export function normalizeContractEvent(item, fallbackSymbol = "BTC") {
  const normalized = normalizeFinalEvent(item, fallbackSymbol);
  const finalEvent = normalized.finalEvent || {};
  const rawWindowSec = numberOrNull(item?.windowSec ?? item?.window_sec);
  const rawVolumeBtc = numberOrNull(item?.volumeBtc ?? item?.volume_btc);
  const rawNotionalUsd = numberOrNull(item?.notionalUsd ?? item?.notional_usd);
  const rawNetVolumeBtc = numberOrNull(item?.netVolumeBtc ?? item?.net_volume_btc);
  const rawPrice = numberOrNull(item?.price);
  const rawTs = numberOrNull(item?.ts);
  const volumeMeta = normalizeVolumeDisplayMeta(item, VOLUME_DISPLAY_CONTEXT.CONTRACT_EVENT_STREAM, rawWindowSec ?? normalized.windowSec);
  return {
    ...normalized,
    id: item?.eventId || item?.event_id || normalized.id,
    eventId: item?.eventId || item?.event_id || normalized.eventId || normalized.id,
    finalEventId: item?.eventId || item?.event_id || normalized.finalEventId || normalized.id,
    sourceSignalId: item?.sourceSignalId || item?.source_signal_id || normalized.sourceSignalId,
    ts: rawTs ?? normalized.ts,
    status: String(item?.status || finalEvent.status || normalized.eventLifecycle?.status || "unknown").toLowerCase(),
    signalType: item?.signalType || item?.signal_type || normalized.signalType,
    severity: item?.severity || normalized.severity,
    windowSec: rawWindowSec ?? normalized.windowSec,
    score: numberOrNull(item?.score) ?? normalized.score,
    totalVolumeBtc: rawVolumeBtc ?? normalized.totalVolumeBtc,
    volumeBtc: rawVolumeBtc ?? normalized.totalVolumeBtc,
    totalNotionalUsd: rawNotionalUsd ?? normalized.totalNotionalUsd,
    notionalUsd: rawNotionalUsd ?? normalized.totalNotionalUsd,
    netVolumeBtc: rawNetVolumeBtc ?? normalized.netVolumeBtc,
    buyVolumeBtc: numberOrNull(item?.buyVolumeBtc ?? item?.buy_volume_btc) ?? volumeMeta.buyVolumeBtc,
    sellVolumeBtc: numberOrNull(item?.sellVolumeBtc ?? item?.sell_volume_btc) ?? volumeMeta.sellVolumeBtc,
    displayVolumeBtc: numberOrNull(item?.displayVolumeBtc ?? item?.display_volume_btc) ?? volumeMeta.displayVolumeBtc,
    displayVolumeLabel: item?.displayVolumeLabel ?? item?.display_volume_label ?? volumeMeta.displayVolumeLabel,
    volumeSemantics: item?.volumeSemantics ?? item?.volume_semantics ?? volumeMeta.volumeSemantics,
    isBidirectionalVolume: Boolean(item?.isBidirectionalVolume ?? item?.is_bidirectional_volume ?? volumeMeta.isBidirectionalVolume),
    isCrossExchangeAggregated: Boolean(
      item?.isCrossExchangeAggregated ?? item?.is_cross_exchange_aggregated ?? volumeMeta.isCrossExchangeAggregated,
    ),
    isLifecycleAccumulated: Boolean(item?.isLifecycleAccumulated ?? item?.is_lifecycle_accumulated ?? volumeMeta.isLifecycleAccumulated),
    mergedSignalCount: Math.max(1, Math.round(numberOrNull(item?.mergedSignalCount ?? item?.merged_signal_count) || volumeMeta.mergedSignalCount)),
    sourceExchangeCount: numberOrNull(item?.sourceExchangeCount ?? item?.source_exchange_count) ?? volumeMeta.sourceExchangeCount,
    sourceExchanges: normalizeStringArray(item?.sourceExchanges ?? item?.source_exchanges).length
      ? normalizeStringArray(item?.sourceExchanges ?? item?.source_exchanges)
      : volumeMeta.sourceExchanges,
    mergedWindowsSec: volumeMeta.mergedWindowsSec,
    finalEvent: {
      ...finalEvent,
      displayVolumeBtc: volumeMeta.displayVolumeBtc,
      displayVolumeLabel: volumeMeta.displayVolumeLabel,
      volumeSemantics: volumeMeta.volumeSemantics,
      isBidirectionalVolume: volumeMeta.isBidirectionalVolume,
      isCrossExchangeAggregated: volumeMeta.isCrossExchangeAggregated,
      isLifecycleAccumulated: volumeMeta.isLifecycleAccumulated,
      mergedSignalCount: volumeMeta.mergedSignalCount,
      sourceExchangeCount: volumeMeta.sourceExchangeCount,
      sourceExchanges: volumeMeta.sourceExchanges,
      mergedWindowsSec: volumeMeta.mergedWindowsSec,
      buyVolumeBtc: volumeMeta.buyVolumeBtc,
      sellVolumeBtc: volumeMeta.sellVolumeBtc,
    },
    direction: item?.direction || normalized.direction,
    dominance: numberOrNull(item?.dominance) ?? normalized.dominance,
    mainForceScore: numberOrNull(item?.mainForceScore ?? item?.main_force_score) ?? normalized.mainForceScore,
    spotScore: numberOrNull(item?.spotScore ?? item?.spot_score) ?? normalized.spotScore,
    contractScore: numberOrNull(item?.contractScore ?? item?.contract_score) ?? normalized.contractScore,
    orderPriceUsd: rawPrice ?? normalized.orderPriceUsd,
    triggerPriceUsd: rawPrice ?? normalized.triggerPriceUsd,
    priceDeviationPct: numberOrNull(item?.priceDeviationPct ?? item?.price_deviation_pct) ?? normalized.priceDeviationPct,
    priceMovePct: numberOrNull(item?.priceMovePct ?? item?.price_move_pct) ?? normalized.priceMovePct,
    dynamicMultiple: numberOrNull(item?.dynamicMultiple ?? item?.dynamic_multiple) ?? normalized.dynamicMultiple,
    percentileLevel: numberOrNull(item?.percentileLevel ?? item?.percentile_level) ?? normalized.percentileLevel,
    mainExchange: item?.mainExchange || item?.main_exchange || normalized.mainExchange,
    liquidationSuspected: Boolean(item?.liquidationSuspected ?? item?.liquidation_suspected ?? normalized.liquidationSuspected),
    liquidationLongBtc: numberOrNull(item?.liquidationLongBtc ?? item?.liquidation_long_btc) ?? normalized.liquidationLongBtc,
    liquidationRatio: numberOrNull(item?.liquidationRatio ?? item?.liquidation_ratio) ?? normalized.liquidationRatio,
    oiChange1mBtc: numberOrNull(item?.oiChange1mBtc ?? item?.oi_change_1m_btc) ?? normalized.oiChange1mBtc,
    oiChangePct: numberOrNull(item?.oiChangePct ?? item?.oi_change_pct) ?? normalized.oiChangePct,
    oiBias: item?.oiBias || item?.oi_bias || normalized.oiBias,
    fundingRate: numberOrNull(item?.fundingRate ?? item?.funding_rate) ?? normalized.fundingRate,
    fundingBias: item?.fundingBias || item?.funding_bias || normalized.fundingBias,
    source: item?.source ? String(item.source) : "contract_whale_signals",
    isRetentionProtected: Boolean(item?.isRetentionProtected ?? item?.is_retention_protected),
    retentionReason: item?.retentionReason ?? item?.retention_reason ?? null,
    isVisible:
      item?.isVisible ??
      item?.is_visible ??
      !(
        Boolean(item?.priceDeviationFiltered ?? item?.price_deviation_filtered ?? normalized.priceDeviationFiltered) ||
        Boolean(item?.hiddenReason ?? item?.hidden_reason)
      ),
    hiddenReason:
      item?.hiddenReason ??
      item?.hidden_reason ??
      (Boolean(item?.priceDeviationFiltered ?? item?.price_deviation_filtered ?? normalized.priceDeviationFiltered)
        ? "price_deviation_gt_5pct"
        : null),
    hiddenDetail: item?.hiddenDetail ?? item?.hidden_detail ?? null,
  };
}

export function normalizeContractWhaleSignal(item, fallbackSymbol = "BTC") {
  const totalVolumeBtc = numberOrNull(item.totalVolume) ?? numberOrNull(item.totalVolumeBtc) ?? 0;
  const totalNotionalUsd = numberOrNull(item.totalNotionalUsd) || 0;
  const activeSources = normalizeActiveSources(item.activeSources);
  const triggerPriceUsd = normalizeTriggerPrice(item, totalVolumeBtc, totalNotionalUsd);
  const volumeMeta = normalizeVolumeDisplayMeta(
    item,
    VOLUME_DISPLAY_CONTEXT.SINGLE_WINDOW,
    numberOrNull(item.windowSec) || 0,
  );
  const orderPriceUsd =
    numberOrNull(item.orderPriceUsd) ??
    numberOrNull(item.orderPrice) ??
    numberOrNull(item.signalPriceUsd) ??
    triggerPriceUsd;
  const currentMarketPriceUsd =
    numberOrNull(item.currentMarketPriceUsd) ??
    numberOrNull(item.currentMarketPrice) ??
    numberOrNull(item.marketPriceUsd);
  const priceDeviationPct =
    numberOrNull(item.priceDeviationPct) ?? computePriceDeviationPct(orderPriceUsd, currentMarketPriceUsd);
  const priceDeviationFiltered =
    Boolean(item.priceDeviationFiltered) ||
    (priceDeviationPct !== null && priceDeviationPct > CWM_MAX_PRICE_DEVIATION_PCT);
  const impact = resolveImpactNormalization(item, {
    dynamicThresholdLevel: item?.dynamicThresholdLevel ?? item?.dynamic_threshold_level,
    impactScoreFallback: item?.dynamicMultiple ?? item?.dynamic_multiple,
    percentileFallback: item?.percentileLevel ?? item?.percentile_level,
  });
  return {
    id: item.id || `${item.symbol || fallbackSymbol || "BTC"}-${item.windowSec || 0}-${item.ts || Date.now()}`,
    ts: numberOrNull(item.ts),
    symbol: item.symbol || item.quantityUnit || item.baseAsset || fallbackSymbol || "BTC",
    baseAsset: item.baseAsset || item.quantityUnit || item.symbol || fallbackSymbol || "BTC",
    quantityUnit: item.quantityUnit || item.baseAsset || item.symbol || fallbackSymbol || "BTC",
    windowSec: numberOrNull(item.windowSec) || 0,
    signalType: item.signalType || "unknown",
    direction: item.direction || "neutral",
    severity: item.severity || "medium",
    score: numberOrNull(item.score) || 0,
    totalVolumeBtc,
    netVolumeBtc: numberOrNull(item.netVolume) ?? numberOrNull(item.netVolumeBtc) ?? 0,
    totalNotionalUsd,
    buyVolumeBtc: volumeMeta.buyVolumeBtc,
    sellVolumeBtc: volumeMeta.sellVolumeBtc,
    displayVolumeBtc: volumeMeta.displayVolumeBtc,
    displayVolumeLabel: item.displayVolumeLabel ?? item.display_volume_label ?? volumeMeta.displayVolumeLabel,
    volumeSemantics: item.volumeSemantics ?? item.volume_semantics ?? volumeMeta.volumeSemantics,
    isBidirectionalVolume: volumeMeta.isBidirectionalVolume,
    isCrossExchangeAggregated: volumeMeta.isCrossExchangeAggregated,
    isLifecycleAccumulated: volumeMeta.isLifecycleAccumulated,
    mergedSignalCount: volumeMeta.mergedSignalCount,
    sourceExchangeCount: volumeMeta.sourceExchangeCount,
    sourceExchanges: volumeMeta.sourceExchanges,
    mergedWindowsSec: volumeMeta.mergedWindowsSec,
    dominance: numberOrNull(item.dominance) || 0,
    priceMovePct: numberOrNull(item.priceMovePct),
    priceMove5sPct: numberOrNull(item.priceMove5sPct),
    priceMove15sPct: numberOrNull(item.priceMove15sPct),
    priceMove30sPct: numberOrNull(item.priceMove30sPct),
    priceResponseType: item.priceResponseType ? String(item.priceResponseType).toLowerCase() : "no_clear_response",
    triggerPriceUsd,
    orderPriceUsd,
    currentMarketPriceUsd,
    priceDeviationPct,
    priceDeviationFiltered,
    mainExchange: item.mainExchange || "Multi",
    marketType: item.marketType ? String(item.marketType).toLowerCase() : "perp",
    sourceRole: item.sourceRole ? String(item.sourceRole).toLowerCase() : "optional",
    dominantVenueNetContributionShare: numberOrNull(item.dominantVenueNetContributionShare),
    dynamicMultiple: numberOrNull(item.dynamicMultiple),
    dynamicBaselineBtc: numberOrNull(item.dynamicBaselineBtc),
    dynamicThresholdLevel: item.dynamicThresholdLevel ? String(item.dynamicThresholdLevel).toLowerCase() : "normal",
    percentileLevel: numberOrNull(item.percentileLevel),
    impactScore: impact.impactScore,
    zScore: impact.zScore,
    percentile: impact.percentile,
    normalizedScore: impact.normalizedScore,
    normalizedStrength: impact.normalizedStrength,
    impactLevel: impact.impactLevel,
    signalLevel: impact.signalLevel,
    signalLabel: impact.signalLabel,
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
    mainForceScore: numberOrNull(item.mainForceScore),
    spotScore: numberOrNull(item.spotScore ?? item.spotConfirmation?.score),
    contractScore: numberOrNull(item.contractScore ?? item.score),
    scoreBreakdown: normalizeScoreBreakdown(item.scoreBreakdown),
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
    spotConfirmation: normalizeSpotConfirmation(item.spotConfirmation),
    discordEligible: Boolean(item.discordEligible),
    discordSent: Boolean(item.discordSent),
    discordSentAt: numberOrNull(item.discordSentAt),
    discordReason: item.discordReason || "not_sent",
    discordWouldSend: Boolean(item.discordWouldSend),
    finalResult: item.finalResult || "contract whale flow candidate",
    ageSec: numberOrNull(item.ageSec ?? item.age_sec),
    isStale: Boolean(item.isStale ?? item.is_stale),
    staleReason: item.staleReason ?? item.stale_reason ?? null,
    mergedFrom: Array.isArray(item.mergedFrom) ? item.mergedFrom.filter(Boolean).map(String) : [],
    cluster: normalizeSignalCluster(item.cluster),
    persistence: normalizePersistenceState(item.persistence),
    whaleAction: normalizeWhaleAction(item.whaleAction),
    trajectory: normalizeWhaleTrajectory(item.trajectory),
    liquidationForce: normalizeLiquidationForce(item.liquidationForce),
    marketDriver: normalizeMarketDriver(item.marketDriver),
    eventLifecycle: normalizeEventLifecycle(item.eventLifecycle),
    eventQuality: normalizeEventQuality(item.eventQuality),
  };
}

function isVisibleContractWhaleSignal(signal) {
  if (signal?.isVisible === false) return false;
  if (signal?.hiddenReason) return false;
  return !signal.priceDeviationFiltered;
}

function normalizeSignalCluster(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    clusterId: source.clusterId ? String(source.clusterId) : "",
    signalCount: Math.max(1, Math.round(numberOrNull(source.signalCount) || 1)),
    dominantIntent: source.dominantIntent ? String(source.dominantIntent) : "single_signal",
    startedAt: numberOrNull(source.startedAt),
    updatedAt: numberOrNull(source.updatedAt),
    durationMs: Math.max(0, Math.round(numberOrNull(source.durationMs) || 0)),
    intensity: clampRatio(numberOrNull(source.intensity) ?? 0),
    priceRangePct: numberOrNull(source.priceRangePct),
  };
}

function normalizePersistenceState(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    persistenceScore: clampRatio(numberOrNull(source.persistenceScore) ?? 0),
    signalHalfLifeMs: Math.max(0, Math.round(numberOrNull(source.signalHalfLifeMs) || 0)),
    regimeStability: clampRatio(numberOrNull(source.regimeStability) ?? 0),
    redundantWithPrevious: Boolean(source.redundantWithPrevious),
    redundantReason: source.redundantReason ? String(source.redundantReason) : "",
  };
}

function normalizeEventLifecycle(value) {
  const source = value && typeof value === "object" ? value : {};
  const status = source.status ? String(source.status).toLowerCase() : "active";
  return {
    eventId: source.eventId ? String(source.eventId) : "",
    startTime: numberOrNull(source.startTime),
    lastUpdateTime: numberOrNull(source.lastUpdateTime),
    status: status === "closed" ? "closed" : "active",
    volumeAccumulated: numberOrNull(source.volumeAccumulated) || 0,
    oiAccumulated: numberOrNull(source.oiAccumulated) || 0,
    updateCount: Math.max(1, Math.round(numberOrNull(source.updateCount) || 1)),
  };
}

function normalizeEventQuality(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    qualityScore: clampRatio(numberOrNull(source.qualityScore) ?? 1),
    mergeSimilarityScore: clampRatio(numberOrNull(source.mergeSimilarityScore) ?? 1),
    valid: source.valid === undefined ? true : Boolean(source.valid),
    falseEventFlags: normalizeStringArray(source.falseEventFlags),
  };
}

function normalizeWhaleAction(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    ts: numberOrNull(source.ts),
    symbol: source.symbol ? String(source.symbol) : "",
    actionType: source.actionType ? String(source.actionType) : "unknown",
    volume: numberOrNull(source.volume) ?? 0,
    priceImpact: numberOrNull(source.priceImpact) ?? 0,
    exchange: source.exchange ? String(source.exchange) : "unknown",
  };
}

function normalizeWhaleTrajectory(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    trajectoryId: source.trajectoryId ? String(source.trajectoryId) : "",
    startTs: numberOrNull(source.startTs),
    endTs: numberOrNull(source.endTs),
    durationMs: Math.max(0, Math.round(numberOrNull(source.durationMs) || 0)),
    actions: Array.isArray(source.actions) ? source.actions.map(normalizeWhaleAction) : [],
    intent: source.intent ? String(source.intent) : "unknown",
    regimePath: Array.isArray(source.regimePath) ? source.regimePath.filter(Boolean).map(String) : [],
    stealthProfile: normalizeStealthProfile(source.stealthProfile),
    aggressivenessCurve: Array.isArray(source.aggressivenessCurve)
      ? source.aggressivenessCurve.map((value) => clampRatio(numberOrNull(value) ?? 0))
      : [],
    conclusion: source.conclusion ? String(source.conclusion) : "",
  };
}

function normalizeStealthProfile(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    gamma: clampRatio(numberOrNull(source.gamma) ?? 0),
    fragmentation: clampRatio(numberOrNull(source.fragmentation) ?? 0),
    entropy: clampRatio(numberOrNull(source.entropy) ?? 0),
    crossExchangeDispersion: clampRatio(numberOrNull(source.crossExchangeDispersion) ?? 0),
  };
}

function normalizeLiquidationForce(value) {
  const source = value && typeof value === "object" ? value : {};
  const flow = source.flowAttribution && typeof source.flowAttribution === "object" ? source.flowAttribution : {};
  const impact = source.priceImpact && typeof source.priceImpact === "object" ? source.priceImpact : {};
  return {
    activeZone: source.activeZone ? String(source.activeZone) : "neutral",
    primaryDriver: source.primaryDriver ? String(source.primaryDriver) : (flow.dominantDriver ? String(flow.dominantDriver) : "whale_initiated_flow"),
    longLiquidationPressure: clampScore(numberOrNull(source.longLiquidationPressure) ?? 0),
    shortSqueezePressure: clampScore(numberOrNull(source.shortSqueezePressure) ?? 0),
    stopHuntProbability: clampScore(numberOrNull(source.stopHuntProbability) ?? 0),
    cascadeIntensity: clampScore(numberOrNull(source.cascadeIntensity) ?? 0),
    estimatedForcedSizeUsd: numberOrNull(source.estimatedForcedSizeUsd) ?? 0,
    zones: Array.isArray(source.zones) ? source.zones.map(normalizeLiquidationZone) : [],
    flowAttribution: {
      whalePct: clampRatio(numberOrNull(flow.whalePct) ?? 1),
      retailPct: clampRatio(numberOrNull(flow.retailPct) ?? 0),
      liquidationPct: clampRatio(numberOrNull(flow.liquidationPct) ?? 0),
      dominantDriver: flow.dominantDriver ? String(flow.dominantDriver) : "whale_initiated_flow",
    },
    priceImpact: {
      whaleImpact: numberOrNull(impact.whaleImpact) ?? 0,
      liquidationCascade: numberOrNull(impact.liquidationCascade) ?? 0,
      stopLossSweep: numberOrNull(impact.stopLossSweep) ?? 0,
      passiveAbsorption: numberOrNull(impact.passiveAbsorption) ?? 0,
    },
  };
}

function normalizeLiquidationZone(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    side: source.side ? String(source.side) : "neutral",
    lowPriceUsd: numberOrNull(source.lowPriceUsd),
    highPriceUsd: numberOrNull(source.highPriceUsd),
    estimatedSizeUsd: numberOrNull(source.estimatedSizeUsd) ?? 0,
    intensity: clampScore(numberOrNull(source.intensity) ?? 0),
    reason: source.reason ? String(source.reason) : "",
  };
}

function normalizeMarketDriver(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    primaryDriver: source.primaryDriver ? String(source.primaryDriver) : "whale_intent",
    marketState: source.marketState ? String(source.marketState) : "whale_led_expansion",
    whaleIntentPct: clampRatio(numberOrNull(source.whaleIntentPct) ?? 1),
    liquidityForcingPct: clampRatio(numberOrNull(source.liquidityForcingPct) ?? 0),
    derivativesPressurePct: clampRatio(numberOrNull(source.derivativesPressurePct) ?? 0),
    reflexivityPct: clampRatio(numberOrNull(source.reflexivityPct) ?? 0),
    components: Array.isArray(source.components) ? source.components.map(normalizeMarketDriverComponent) : [],
    interpretation: source.interpretation ? String(source.interpretation) : "价格主要由主动资金流驱动。",
  };
}

function normalizeMarketDriverComponent(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    key: source.key ? String(source.key) : "whale_intent",
    score: clampScore(numberOrNull(source.score) ?? 0),
    weightPct: clampRatio(numberOrNull(source.weightPct) ?? 0),
  };
}

function clampScore(value) {
  const number = Number(value || 0);
  if (!Number.isFinite(number)) return 0;
  return Math.max(0, Math.min(100, Math.round(number)));
}

function clampRatio(value) {
  const number = Number(value || 0);
  if (!Number.isFinite(number)) return 0;
  return Math.max(0, Math.min(1, number));
}

function computePriceDeviationPct(orderPrice, currentMarketPrice) {
  if (
    orderPrice === null ||
    currentMarketPrice === null ||
    !Number.isFinite(orderPrice) ||
    !Number.isFinite(currentMarketPrice) ||
    orderPrice <= 0 ||
    currentMarketPrice <= 0
  ) {
    return null;
  }
  return Math.abs(orderPrice - currentMarketPrice) / currentMarketPrice * 100;
}

function normalizeSpotConfirmation(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    status: source.status ? String(source.status).toLowerCase() : "unavailable",
    confirmationType: source.confirmationType ? String(source.confirmationType).toLowerCase() : "unavailable",
    direction: source.direction ? String(source.direction).toLowerCase() : "neutral",
    score: numberOrNull(source.score) || 0,
    latestSignalId: source.latestSignalId ? String(source.latestSignalId) : null,
    latestSignalAt: numberOrNull(source.latestSignalAt),
    signalType: source.signalType ? String(source.signalType).toLowerCase() : null,
    severity: source.severity ? String(source.severity).toLowerCase() : null,
    totalVolumeBtc: numberOrNull(source.totalVolumeBtc),
    netVolumeBtc: numberOrNull(source.netVolumeBtc),
    dominance: numberOrNull(source.dominance),
    coinbasePremiumPct: numberOrNull(source.coinbasePremiumPct),
    finalResult: source.finalResult ? String(source.finalResult) : "",
  };
}

function normalizeScoreBreakdown(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    volumeScore: numberOrNull(source.volumeScore) || 0,
    notionalScore: numberOrNull(source.notionalScore) || 0,
    dynamicAnomalyScore: numberOrNull(source.dynamicAnomalyScore) || 0,
    directionalStrengthScore: numberOrNull(source.directionalStrengthScore) || 0,
    priceResponseScore: numberOrNull(source.priceResponseScore) || 0,
    multiSourceScore: numberOrNull(source.multiSourceScore) || 0,
    dataQualityScore: numberOrNull(source.dataQualityScore) || 0,
    dominantVenueScore: numberOrNull(source.dominantVenueScore) || 0,
    oiContextScore: numberOrNull(source.oiContextScore) || 0,
    penaltyScore: numberOrNull(source.penaltyScore) || 0,
    finalScore: numberOrNull(source.finalScore) || 0,
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

function signalMatchesRequestedSymbol(item, requestedSymbol = "BTC") {
  const requested = baseAssetKey(requestedSymbol);
  if (!requested || requested === "ALL") return true;
  const itemSymbol = item?.symbol || item?.quantityUnit || item?.baseAsset;
  if (!itemSymbol) return true;
  return baseAssetKey(itemSymbol) === requested;
}

function baseAssetKey(symbol = "") {
  return String(symbol || "")
    .trim()
    .toUpperCase()
    .replace(/[-_/]?(USDT|USD|PERP|SWAP)$/i, "");
}

export function normalizeMainForceEvent(item, fallbackSymbol = "BTC") {
  return {
    id: numberOrNull(item.id) || 0,
    symbol: item.symbol || fallbackSymbol || "BTC",
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

function normalizeSummary(summary, fallbackSymbol = "BTC") {
  if (!summary || typeof summary !== "object") {
    return fallbackSummary(fallbackSymbol);
  }
  return {
    status: summary.status || calmSummary.status,
    healthStatus: summary.healthStatus || calmSummary.healthStatus,
    healthReason: summary.healthReason || calmSummary.healthReason,
    symbol: summary.symbol || summary.quantityUnit || summary.baseAsset || fallbackSymbol,
    baseAsset: summary.baseAsset || summary.quantityUnit || summary.symbol || fallbackSymbol,
    quantityUnit: summary.quantityUnit || summary.baseAsset || summary.symbol || fallbackSymbol,
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
    discordDryRunStats: normalizeDiscordDryRunStats(summary.discordDryRunStats),
    marketStructureLite: normalizeMarketStructureLite(summary.marketStructureLite),
    trend60s: normalizeTrend60s(
      summary.trend60s,
      summary.symbol || summary.selectedSymbol || summary.quantityUnit || summary.baseAsset || fallbackSymbol,
    ),
    exchanges: normalizeExchanges(summary.exchanges),
    platforms: normalizePlatforms(summary.platforms),
  };
}

function fallbackSummary(symbol = "BTC") {
  const fallbackSymbol = symbol || "BTC";
  return {
    ...calmSummary,
    trend60s: {
      ...normalizeTrend60s(calmSummary.trend60s, fallbackSymbol),
      symbol: fallbackSymbol,
    },
  };
}

function normalizeDiscordDryRunStats(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    signals1h: numberOrNull(source.signals1h) || 0,
    high1h: numberOrNull(source.high1h) || 0,
    critical1h: numberOrNull(source.critical1h) || 0,
    s1h: numberOrNull(source.s1h) || 0,
    wouldSend1h: numberOrNull(source.wouldSend1h) || 0,
    skippedLowScore1h: numberOrNull(source.skippedLowScore1h) || 0,
    skippedCooldown1h: numberOrNull(source.skippedCooldown1h) || 0,
    skippedDataQuality1h: numberOrNull(source.skippedDataQuality1h) || 0,
    skippedWarmup1h: numberOrNull(source.skippedWarmup1h) || 0,
    skippedDisplayOnly1h: numberOrNull(source.skippedDisplayOnly1h) || 0,
  };
}

function normalizeMarketStructureLite(value) {
  const source = value && typeof value === "object" ? value : {};
  return {
    status: source.status ? String(source.status).toLowerCase() : "calm",
    regimeType: source.regimeType ? String(source.regimeType).toLowerCase() : "unclear",
    mainForceScore: numberOrNull(source.mainForceScore) || 0,
    extremeImpactScore: numberOrNull(source.extremeImpactScore) || 0,
    structureBias: numberOrNull(source.structureBias) || 0,
    confidence: numberOrNull(source.confidence) || 0,
    dataQuality: numberOrNull(source.dataQuality) || 0,
    spotScore: numberOrNull(source.spotScore) || 0,
    contractScore: numberOrNull(source.contractScore) || 0,
    crossConfirmScore: numberOrNull(source.crossConfirmScore) || 0,
    mainForceConfirmed: Boolean(source.mainForceConfirmed),
    extremeImpactConfirmed: Boolean(source.extremeImpactConfirmed),
    reason: source.reason ? String(source.reason) : "",
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

function normalizeCanonicalTimeline(meta) {
  if (!meta || typeof meta !== "object") return null;
  return {
    source: meta.source ? String(meta.source) : "none",
    eventTs: numberOrNull(meta.eventTs ?? meta.event_ts),
    processedTs: numberOrNull(meta.processedTs ?? meta.processed_ts),
    persistedTs: numberOrNull(meta.persistedTs ?? meta.persisted_ts),
    servedTs: numberOrNull(meta.servedTs ?? meta.served_ts),
    timelineLagSec: numberOrNull(meta.timelineLagSec ?? meta.timeline_lag_sec) ?? 0,
  };
}

function normalizeCanonicalTimelineView(view) {
  const source = view && typeof view === "object" ? view : {};
  return {
    count: numberOrNull(source.count) ?? 0,
    maxEventTs: numberOrNull(source.maxEventTs ?? source.max_event_ts),
    driftVsCanonicalSec: numberOrNull(source.driftVsCanonicalSec ?? source.drift_vs_canonical_sec) ?? 0,
    cacheAgeSec: numberOrNull(source.cacheAgeSec ?? source.cache_age_sec),
    cacheTtlSec: numberOrNull(source.cacheTtlSec ?? source.cache_ttl_sec),
    generatedAt: numberOrNull(source.generatedAt ?? source.generated_at),
  };
}

function normalizeCanonicalTimelineFlowView(view) {
  const source = view && typeof view === "object" ? view : {};
  return {
    updatedAt: numberOrNull(source.updatedAt ?? source.updated_at),
    driftVsCanonicalSec: numberOrNull(source.driftVsCanonicalSec ?? source.drift_vs_canonical_sec) ?? 0,
  };
}

function normalizeCanonicalTimelineResponse(payload, filters = {}, fallbackError = null) {
  const source = payload && typeof payload === "object" ? payload : {};
  return {
    symbol: String(source.symbol || filters.symbol || "BTC"),
    range: String(source.range || filters.range || "24h"),
    source: source.source ? String(source.source) : "none",
    eventTs: numberOrNull(source.eventTs ?? source.event_ts),
    processedTs: numberOrNull(source.processedTs ?? source.processed_ts),
    persistedTs: numberOrNull(source.persistedTs ?? source.persisted_ts),
    servedTs: numberOrNull(source.servedTs ?? source.served_ts),
    timelineLagSec: numberOrNull(source.timelineLagSec ?? source.timeline_lag_sec) ?? 0,
    views: {
      latest: normalizeCanonicalTimelineView(source.views?.latest),
      history: normalizeCanonicalTimelineView(source.views?.history),
      finalEventsV2: normalizeCanonicalTimelineView(source.views?.finalEventsV2 ?? source.views?.final_events_v2),
      flow: normalizeCanonicalTimelineFlowView(source.views?.flow),
    },
    error: fallbackError || source.error || null,
  };
}

function normalizeStringArray(value) {
  if (!Array.isArray(value)) return [];
  return value.map((item) => String(item || "").toLowerCase()).filter(Boolean);
}

function normalizeRawStringArray(value) {
  if (!Array.isArray(value)) return [];
  return value.map((item) => String(item || "")).filter(Boolean);
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

function normalizeTrend60s(trend, symbol = "BTC") {
  const source = trend && typeof trend === "object" ? trend : {};
  return {
    symbol: source.symbol || source.quantityUnit || source.baseAsset || symbol || "BTC",
    baseAsset: source.baseAsset || source.quantityUnit || source.symbol || symbol || "BTC",
    quantityUnit: source.quantityUnit || source.baseAsset || source.symbol || symbol || "BTC",
    buyVolumeBtc: numberOrNull(source.buyVolume) ?? numberOrNull(source.buyVolumeBtc) ?? 0,
    sellVolumeBtc: numberOrNull(source.sellVolume) ?? numberOrNull(source.sellVolumeBtc) ?? 0,
    totalVolumeBtc: numberOrNull(source.totalVolume) ?? numberOrNull(source.totalVolumeBtc) ?? 0,
    netVolumeBtc: numberOrNull(source.netVolume) ?? numberOrNull(source.netVolumeBtc) ?? 0,
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
