import { memo, useEffect, useMemo, useState } from "react";
import {
  CWM_MAX_PRICE_DEVIATION_PCT,
  fetchContractEventDebugCounts,
  fetchContractEvents,
  fetchContractWhaleIntelligenceTerminal,
  fetchContractWhaleLatencyDebug,
  fetchContractWhaleEvents,
  fetchContractWhaleLatest,
  fetchContractWhaleRawFlowDebug,
  fetchContractWhaleSummary,
  fetchFinalEventsV2,
} from "../api/contractWhale.js";

const STATUS_REFRESH_MS = 5_000;
const EVENT_REFRESH_MS = 15_000;
const EVENT_RECOVERY_RETRY_MS = 2_000;
const EVENTS_SYNC_LAG_MS = 15_000;
const DEFAULT_CONTRACT_EVENT_LIMIT = 50;
const INITIAL_CONTRACT_EVENT_LIMIT = 20;
/** Contract event tape query/retention window: keep at least 7 days of full history. */
const CONTRACT_EVENT_RANGE = "7d";
const BTC_MIN_VISIBLE_TOTAL_VOLUME_BTC = 500;
const EVENT_FEED_SESSION_CACHE_VERSION = 2;
const EVENT_FEED_SESSION_CACHE_TTL_MS = 10 * 60 * 1_000;
const EVENT_FEED_SESSION_CACHE_PREFIX = "contract-whale:event-feed";
const STATUS_SESSION_CACHE_VERSION = 1;
const STATUS_SESSION_CACHE_TTL_MS = 2 * 60 * 1_000;
const STATUS_SESSION_CACHE_PREFIX = "contract-whale:status";
const OPERATOR_DIAGNOSTICS_ENABLED =
  import.meta.env.MODE === "test" || import.meta.env.VITE_ENABLE_OPERATOR_DIAGNOSTICS === "true";
const DEFAULT_FILTERS = {
  symbol: "BTC",
  severity: "all",
  signal_type: "all",
  direction: "all",
  net_direction: "all",
  impact_level: "all",
  discord_sent: "all",
  window_sec: "all",
  exchange: "all",
};

const DATA_SLICE_LABELS = {
  status: "状态快照",
  historical: "历史事件",
  lifecycle: "生命周期",
  intelligence: "智能分析",
};

function createDataSlice(overrides = {}) {
  return {
    state: "loading",
    errorCode: null,
    lastSuccessAt: null,
    nextRetryAt: null,
    cacheAgeSec: null,
    cacheTtlSec: null,
    ...overrides,
  };
}

function createDataSlices() {
  return {
    status: createDataSlice(),
    historical: createDataSlice(),
    lifecycle: createDataSlice(),
    intelligence: createDataSlice(),
  };
}

function eventFeedSessionCacheKey(filters) {
  const identity = [
    filters.symbol || "BTC",
    filters.severity || "all",
    filters.signal_type || "all",
    filters.direction || "all",
    filters.net_direction || "all",
    filters.impact_level || "all",
    filters.discord_sent || "all",
    filters.window_sec || "all",
    filters.exchange || "all",
  ]
    .map((value) => String(value).trim().toLowerCase())
    .join(":");
  return `${EVENT_FEED_SESSION_CACHE_PREFIX}:v${EVENT_FEED_SESSION_CACHE_VERSION}:${identity}`;
}

function readEventFeedSessionCache(filters) {
  if (typeof window === "undefined") return null;
  const key = eventFeedSessionCacheKey(filters);
  try {
    const raw = window.sessionStorage.getItem(key);
    if (!raw) return null;
    const cached = JSON.parse(raw);
    const savedAt = Number(cached?.savedAt);
    const cacheAgeMs = Date.now() - savedAt;
    if (
      cached?.version !== EVENT_FEED_SESSION_CACHE_VERSION
      || !Array.isArray(cached?.items)
      || cached.items.length === 0
      || !Number.isFinite(savedAt)
      || cacheAgeMs < 0
      || cacheAgeMs > EVENT_FEED_SESSION_CACHE_TTL_MS
    ) {
      window.sessionStorage.removeItem(key);
      return null;
    }
    return { ...cached, cacheAgeMs };
  } catch {
    return null;
  }
}

function writeEventFeedSessionCache(filters, payload) {
  if (typeof window === "undefined" || !Array.isArray(payload?.items)) {
    return;
  }
  try {
    window.sessionStorage.setItem(
      eventFeedSessionCacheKey(filters),
      JSON.stringify({
        version: EVENT_FEED_SESSION_CACHE_VERSION,
        savedAt: Date.now(),
        items: payload.items.slice(0, DEFAULT_CONTRACT_EVENT_LIMIT),
        nextCursor: payload.nextCursor ?? null,
        hasMore: Boolean(payload.hasMore),
        serverTime: payload.serverTime ?? null,
        lastEventTs: payload.lastEventTs ?? null,
        maxEventTs: payload.maxEventTs ?? null,
        historyLagSec: payload.historyLagSec ?? null,
        latestLagSec: payload.latestLagSec ?? null,
        cacheAgeSec: payload.cacheAgeSec ?? null,
        cacheTtlSec: payload.cacheTtlSec ?? null,
        timeline: payload.timeline ?? null,
      }),
    );
  } catch {
    // Session storage is a best-effort first-paint cache; network refresh remains authoritative.
  }
}

function statusSessionCacheKey(symbol) {
  return `${STATUS_SESSION_CACHE_PREFIX}:v${STATUS_SESSION_CACHE_VERSION}:${String(symbol || "BTC").trim().toLowerCase()}`;
}

function readStatusSessionCache(symbol) {
  if (typeof window === "undefined") return null;
  try {
    const cached = JSON.parse(window.sessionStorage.getItem(statusSessionCacheKey(symbol)) || "null");
    const savedAt = Number(cached?.savedAt);
    const cacheAgeMs = Date.now() - savedAt;
    if (
      cached?.version !== STATUS_SESSION_CACHE_VERSION
      || !Number.isFinite(savedAt)
      || cacheAgeMs < 0
      || cacheAgeMs > STATUS_SESSION_CACHE_TTL_MS
      || (cached.summary == null && !Array.isArray(cached.items))
    ) {
      window.sessionStorage.removeItem(statusSessionCacheKey(symbol));
      return null;
    }
    return { ...cached, cacheAgeMs };
  } catch {
    return null;
  }
}

function writeStatusSessionCache(symbol, patch) {
  if (typeof window === "undefined" || !patch || typeof patch !== "object") return;
  try {
    const previous = readStatusSessionCache(symbol) || {};
    window.sessionStorage.setItem(
      statusSessionCacheKey(symbol),
      JSON.stringify({
        version: STATUS_SESSION_CACHE_VERSION,
        savedAt: Date.now(),
        summary: patch.summary !== undefined ? patch.summary : previous.summary ?? null,
        items: patch.items !== undefined ? patch.items : previous.items ?? [],
        serverTime: patch.serverTime !== undefined ? patch.serverTime : previous.serverTime ?? null,
        maxTs: patch.maxTs !== undefined ? patch.maxTs : previous.maxTs ?? null,
        maxAgeSec: patch.maxAgeSec !== undefined ? patch.maxAgeSec : previous.maxAgeSec ?? null,
        staleCount: patch.staleCount !== undefined ? patch.staleCount : previous.staleCount ?? null,
        timeline: patch.timeline !== undefined ? patch.timeline : previous.timeline ?? null,
        meta: patch.meta !== undefined ? patch.meta : previous.meta ?? null,
      }),
    );
  } catch {
    // Session storage is a best-effort first-paint cache; network refresh remains authoritative.
  }
}

function isUsableDataPayload(payload) {
  if (!payload || payload.error) return false;
  const dataState = String(payload.dataState || "fresh").toLowerCase();
  if (dataState === "unavailable") return false;
  if (dataState === "degraded" && payload.lastKnownDataAvailable !== true) return false;
  return true;
}

function deriveDataSlice(previous, payload, hasPreviousData, retryIntervalMs) {
  const now = Date.now();
  const dataState = String(payload?.dataState || (payload?.error ? "unavailable" : "fresh")).toLowerCase();
  const failed = !isUsableDataPayload(payload);
  const stale = dataState === "stale" || dataState === "degraded" || (failed && hasPreviousData);
  const nextState = failed ? (hasPreviousData ? "stale" : "unavailable") : (stale ? "stale" : "fresh");
  const retryAfterMs = Number(payload?.retryAfterMs);

  return createDataSlice({
    state: nextState,
    errorCode: nextState === "fresh" ? null : payload?.errorCode || payload?.error || "data_refresh_unavailable",
    lastSuccessAt: nextState === "fresh" ? now : previous?.lastSuccessAt ?? null,
    nextRetryAt:
      nextState === "fresh"
        ? null
        : now + (Number.isFinite(retryAfterMs) && retryAfterMs > 0 ? retryAfterMs : retryIntervalMs),
    cacheAgeSec: Number.isFinite(Number(payload?.cacheAgeSec)) ? Number(payload.cacheAgeSec) : previous?.cacheAgeSec ?? null,
    cacheTtlSec: Number.isFinite(Number(payload?.cacheTtlSec)) ? Number(payload.cacheTtlSec) : previous?.cacheTtlSec ?? null,
  });
}

function recoveryRetryDelay(payloads, refreshIntervalMs) {
  const affected = payloads.filter((payload) => {
    const dataState = String(payload?.dataState || (payload?.error ? "unavailable" : "fresh")).toLowerCase();
    return payload?.error || ["stale", "degraded", "unavailable"].includes(dataState);
  });
  if (affected.length === 0) return null;

  const hintedDelays = affected
    .map((payload) => Number(payload?.retryAfterMs))
    .filter((delay) => Number.isFinite(delay) && delay > 0);
  return Math.min(
    refreshIntervalMs,
    hintedDelays.length > 0 ? Math.min(...hintedDelays) : EVENT_RECOVERY_RETRY_MS,
  );
}

export default function ContractWhaleMonitor({ lockedSymbol = "BTC" }) {
  const assetSymbol = normalizeMainstreamSymbol(lockedSymbol);
  const [state, setState] = useState(() => {
    const cachedEventFeed = readEventFeedSessionCache({ ...DEFAULT_FILTERS, symbol: assetSymbol });
    const cachedStatus = readStatusSessionCache(assetSymbol);
    const dataSlices = createDataSlices();
    if (cachedEventFeed) {
      dataSlices.historical = createDataSlice({
        state: "stale",
        errorCode: "event_feed_session_cache",
        lastSuccessAt: cachedEventFeed.savedAt,
        nextRetryAt: Date.now(),
        cacheAgeSec: Math.floor(cachedEventFeed.cacheAgeMs / 1_000),
        cacheTtlSec: Math.floor(EVENT_FEED_SESSION_CACHE_TTL_MS / 1_000),
      });
    }
    if (cachedStatus) {
      dataSlices.status = createDataSlice({
        state: "stale",
        errorCode: "status_session_cache",
        lastSuccessAt: cachedStatus.savedAt,
        nextRetryAt: Date.now(),
        cacheAgeSec: Math.floor(cachedStatus.cacheAgeMs / 1_000),
        cacheTtlSec: Math.floor(STATUS_SESSION_CACHE_TTL_MS / 1_000),
      });
    }
    return {
      loading: true,
      contractEventsLoading: !cachedEventFeed,
      dataSlices,
      summary: cachedStatus?.summary || null,
      items: cachedStatus?.items || [],
      contractEvents: cachedEventFeed?.items || [],
      contractEventsCursor: cachedEventFeed?.nextCursor ?? null,
      contractEventsHasMore: cachedEventFeed?.hasMore ?? false,
      contractEventsServerTime: cachedEventFeed?.serverTime ?? null,
      contractEventsLastEventTs: cachedEventFeed?.lastEventTs ?? null,
      contractEventsMaxEventTs: cachedEventFeed?.maxEventTs ?? null,
      contractEventsHistoryLagSec: cachedEventFeed?.historyLagSec ?? null,
      contractEventsLatestLagSec: cachedEventFeed?.latestLagSec ?? null,
      contractEventsCacheAgeSec: cachedEventFeed?.cacheAgeSec ?? null,
      contractEventsCacheTtlSec: cachedEventFeed?.cacheTtlSec ?? null,
      contractEventsTimeline: cachedEventFeed?.timeline ?? null,
      contractEventDebugCounts: null,
      rawFlowDebug: null,
      latencyDebug: null,
      finalEvents: { active: [], closed: [] },
      finalEventsCursor: null,
      finalEventsHasMore: false,
      finalEventsServerTime: null,
      finalEventsLastEventTs: null,
      finalEventsMaxEventTs: null,
      finalEventsGeneratedAt: null,
      finalEventsProjectionLagSec: null,
      finalEventsCacheAgeSec: null,
      finalEventsCacheTtlSec: null,
      finalEventsTimeline: null,
      intelligenceTerminal: null,
      events: [],
      hiddenContractEvents: [],
      hiddenContractEventsLoaded: false,
      hiddenContractEventsExpanded: false,
      hiddenContractEventsLoading: false,
      retentionStatus: null,
      latestServerTime: cachedStatus?.serverTime ?? null,
      latestMaxTs: cachedStatus?.maxTs ?? null,
      latestMaxAgeSec: cachedStatus?.maxAgeSec ?? null,
      latestStaleCount: cachedStatus?.staleCount ?? null,
      latestTimeline: cachedStatus?.timeline ?? null,
      meta: cachedStatus?.meta || null,
    };
  });
  const [selectedSignalId, setSelectedSignalId] = useState(null);
  const [selectedWhaleId, setSelectedWhaleId] = useState(null);
  const [filters, setFilters] = useState(() => ({ ...DEFAULT_FILTERS, symbol: assetSymbol }));

  useEffect(() => {
    setSelectedSignalId(null);
    setSelectedWhaleId(null);
    setFilters((previous) => (
      previous.symbol === assetSymbol ? previous : { ...previous, symbol: assetSymbol }
    ));
  }, [assetSymbol]);

  useEffect(() => {
    let cancelled = false;
    let statusTimer = null;
    let statusRetryTimer = null;
    let eventTimer = null;
    let eventRetryTimer = null;
    let statusRefreshInFlight = false;
    let eventRefreshInFlight = false;
    let initialEventViewPending = true;

    const updateState = (updater) => {
      if (cancelled) return;
      setState((previous) => updater(previous));
    };

    updateState((previous) => {
      const dataSlices = createDataSlices();
      if (previous.contractEvents.length > 0) {
        dataSlices.historical = previous.dataSlices.historical;
      }
      return {
        ...previous,
        loading: true,
        contractEventsLoading: previous.contractEvents.length === 0,
        dataSlices,
        hiddenContractEvents: [],
        hiddenContractEventsLoaded: false,
        hiddenContractEventsExpanded: false,
        hiddenContractEventsLoading: false,
      };
    });

    const refreshSummary = () => fetchContractWhaleSummary(filters.symbol);

    const refreshLatest = () => fetchContractWhaleLatest(50, filters.symbol);

    const refreshContractEvents = async (limit = 50) => {
      const payload = await fetchContractEvents({ ...filters, range: CONTRACT_EVENT_RANGE, limit });
      const usable = isUsableDataPayload(payload);
      if (usable) {
        writeEventFeedSessionCache(filters, payload);
      }
      updateState((previous) => {
        return {
          ...previous,
          loading: false,
          contractEvents: usable
            ? reuseEventList(previous.contractEvents, payload.items)
            : previous.contractEvents,
          contractEventsLoading: false,
          contractEventsCursor: usable ? payload.nextCursor : previous.contractEventsCursor,
          contractEventsHasMore: usable ? payload.hasMore : previous.contractEventsHasMore,
          contractEventsServerTime: usable ? payload.serverTime : previous.contractEventsServerTime,
          contractEventsLastEventTs: usable ? payload.lastEventTs : previous.contractEventsLastEventTs,
          contractEventsMaxEventTs: usable ? payload.maxEventTs : previous.contractEventsMaxEventTs,
          contractEventsHistoryLagSec: usable ? payload.historyLagSec : previous.contractEventsHistoryLagSec,
          contractEventsLatestLagSec: usable ? payload.latestLagSec : previous.contractEventsLatestLagSec,
          contractEventsCacheAgeSec: usable ? payload.cacheAgeSec : previous.contractEventsCacheAgeSec,
          contractEventsCacheTtlSec: usable ? payload.cacheTtlSec : previous.contractEventsCacheTtlSec,
          contractEventsTimeline: usable ? payload.timeline : previous.contractEventsTimeline,
          dataSlices: {
            ...previous.dataSlices,
            historical: deriveDataSlice(
              previous.dataSlices.historical,
              payload,
              previous.contractEvents.length > 0,
              EVENT_REFRESH_MS,
            ),
          },
        };
      });
      return payload;
    };

    const refreshContractEventDebugCounts = async () => {
      const payload = await fetchContractEventDebugCounts({
        symbol: filters.symbol,
        range: CONTRACT_EVENT_RANGE,
        includeHidden: true,
      });
      updateState((previous) => ({
        ...previous,
        contractEventDebugCounts: payload.error ? previous.contractEventDebugCounts : payload,
      }));
    };

    const refreshRawFlowDebug = async () => {
      const payload = await fetchContractWhaleRawFlowDebug({
        symbol: filters.symbol,
        range: "24h",
      });
      updateState((previous) => ({
        ...previous,
        rawFlowDebug: payload.error ? previous.rawFlowDebug : payload,
      }));
    };

    const refreshLatencyDebug = async () => {
      const payload = await fetchContractWhaleLatencyDebug({
        symbol: filters.symbol,
        range: "24h",
      });
      updateState((previous) => ({
        ...previous,
        latencyDebug: payload.error ? previous.latencyDebug : payload,
      }));
    };

    const refreshFinalEvents = async (limit = 30) => {
      const payload = await fetchFinalEventsV2({ symbol: filters.symbol, range: "24h", limit });
      const usable = isUsableDataPayload(payload);
      updateState((previous) => ({
        ...previous,
        loading: false,
        finalEvents: usable
          ? {
              active: reuseEventList(previous.finalEvents.active, payload.active),
              closed: reuseEventList(previous.finalEvents.closed, payload.closed),
            }
          : previous.finalEvents,
        finalEventsCursor: usable ? payload.nextCursor : previous.finalEventsCursor,
        finalEventsHasMore: usable ? payload.hasMore : previous.finalEventsHasMore,
        finalEventsServerTime: usable ? payload.serverTime : previous.finalEventsServerTime,
        finalEventsLastEventTs: usable ? payload.lastEventTs : previous.finalEventsLastEventTs,
        finalEventsMaxEventTs: usable ? payload.maxEventTs : previous.finalEventsMaxEventTs,
        finalEventsGeneratedAt: usable ? payload.generatedAt : previous.finalEventsGeneratedAt,
        finalEventsProjectionLagSec: usable ? payload.projectionLagSec : previous.finalEventsProjectionLagSec,
        finalEventsCacheAgeSec: usable ? payload.cacheAgeSec : previous.finalEventsCacheAgeSec,
        finalEventsCacheTtlSec: usable ? payload.cacheTtlSec : previous.finalEventsCacheTtlSec,
        finalEventsTimeline: usable ? payload.timeline : previous.finalEventsTimeline,
        dataSlices: {
          ...previous.dataSlices,
          lifecycle: deriveDataSlice(
            previous.dataSlices.lifecycle,
            payload,
            previous.finalEvents.active.length > 0 || previous.finalEvents.closed.length > 0,
            EVENT_REFRESH_MS,
          ),
        },
      }));
      return payload;
    };

    const refreshIntelligenceTerminal = async () => {
      const payload = await fetchContractWhaleIntelligenceTerminal({
        symbol: filters.symbol,
        range: "24h",
      });
      updateState((previous) => ({
        ...previous,
        intelligenceTerminal: isUsableDataPayload(payload) ? payload : previous.intelligenceTerminal,
        dataSlices: {
          ...previous.dataSlices,
          intelligence: deriveDataSlice(
            previous.dataSlices.intelligence,
            payload,
            Boolean(previous.intelligenceTerminal),
            EVENT_REFRESH_MS,
          ),
        },
      }));
      return payload;
    };

    const refreshWhaleEvents = async () => {
      const payload = await fetchContractWhaleEvents({ symbol: filters.symbol, limit: 12 });
      updateState((previous) => ({
        ...previous,
        loading: false,
        events: payload.error ? previous.events : payload.items,
      }));
    };

    const refreshStatusViews = async ({ allowRecoveryRetry = true } = {}) => {
      if (statusRefreshInFlight) return;
      statusRefreshInFlight = true;
      try {
        const summaryRequest = refreshSummary().then((payload) => {
          if (isUsableDataPayload(payload)) {
            writeStatusSessionCache(filters.symbol, {
              summary: payload.summary,
              meta: payload.meta,
            });
            updateState((previous) => ({
              ...previous,
              summary: payload.summary || previous.summary,
              meta: payload.meta || previous.meta,
            }));
          }
          return payload;
        });
        const latestRequest = refreshLatest();
        latestRequest.then((payload) => {
          if (isUsableDataPayload(payload)) {
            writeStatusSessionCache(filters.symbol, {
              items: payload.items,
              serverTime: payload.serverTime,
              maxTs: payload.maxTs,
              maxAgeSec: payload.maxAgeSec,
              staleCount: payload.staleCount,
              timeline: payload.timeline,
              summary: payload.summary,
              meta: payload.meta,
            });
          }
          return payload;
        });
        const [summaryPayload, latestPayload] = await Promise.all([
          summaryRequest,
          latestRequest,
        ]);
        const summaryUsable = Boolean(summaryPayload && !summaryPayload.error);
        const latestUsable = isUsableDataPayload(latestPayload);
        const statusError = summaryPayload?.error || latestPayload?.error ||
          (latestPayload?.dataState === "degraded" ? latestPayload.errorCode || "latest_degraded" : null);
        const statusPayload = statusError
          ? {
              ...latestPayload,
              dataState: "degraded",
              degraded: true,
              errorCode: statusError,
              error: statusError,
            }
          : latestPayload;
        updateState((previous) => ({
          ...previous,
          loading: false,
          summary: latestUsable
            ? latestPayload.summary
            : (summaryUsable ? summaryPayload.summary : previous.summary),
          items: latestUsable ? latestPayload.items : previous.items,
          latestServerTime: latestUsable ? latestPayload.serverTime : previous.latestServerTime,
          latestMaxTs: latestUsable ? latestPayload.maxTs : previous.latestMaxTs,
          latestMaxAgeSec: latestUsable ? latestPayload.maxAgeSec : previous.latestMaxAgeSec,
          latestStaleCount: latestUsable ? latestPayload.staleCount : previous.latestStaleCount,
          latestTimeline: latestUsable ? latestPayload.timeline : previous.latestTimeline,
          meta: latestUsable
            ? (latestPayload.meta || previous.meta)
            : (summaryUsable ? summaryPayload.meta || previous.meta : previous.meta),
          dataSlices: {
            ...previous.dataSlices,
            status: deriveDataSlice(
              previous.dataSlices.status,
              statusPayload,
              Boolean(previous.summary) || previous.items.length > 0 || summaryUsable || latestUsable,
              STATUS_REFRESH_MS,
            ),
          },
        }));
        const retryDelay = allowRecoveryRetry
          ? recoveryRetryDelay([statusPayload], STATUS_REFRESH_MS)
          : null;
        if (statusRetryTimer) {
          window.clearTimeout(statusRetryTimer);
          statusRetryTimer = null;
        }
        if (retryDelay !== null && document.visibilityState !== "hidden") {
          statusRetryTimer = window.setTimeout(() => {
            statusRetryTimer = null;
            void refreshStatusViews({ allowRecoveryRetry: false });
          }, retryDelay);
        }
      } finally {
        statusRefreshInFlight = false;
      }
    };

    const refreshEventViews = async ({ allowRecoveryRetry = true } = {}) => {
      if (eventRefreshInFlight) return;
      eventRefreshInFlight = true;
      try {
        const contractEventLimit = initialEventViewPending
          ? INITIAL_CONTRACT_EVENT_LIMIT
          : DEFAULT_CONTRACT_EVENT_LIMIT;
        const contractEventsPayload = await refreshContractEvents(contractEventLimit);
        if (isUsableDataPayload(contractEventsPayload)) {
          initialEventViewPending = false;
        }
        const secondaryResults = await Promise.allSettled([
          refreshFinalEvents(30),
          refreshIntelligenceTerminal(),
        ]);
        const projectionPayloads = [
          contractEventsPayload,
          ...secondaryResults
            .filter((result) => result.status === "fulfilled")
            .map((result) => result.value),
        ];
        const retryDelay = allowRecoveryRetry
          ? recoveryRetryDelay(projectionPayloads, EVENT_REFRESH_MS)
          : null;
        if (eventRetryTimer) {
          window.clearTimeout(eventRetryTimer);
          eventRetryTimer = null;
        }
        if (retryDelay !== null && document.visibilityState !== "hidden") {
          eventRetryTimer = window.setTimeout(() => {
            eventRetryTimer = null;
            void refreshEventViews({ allowRecoveryRetry: false });
          }, retryDelay);
        }
      } finally {
        eventRefreshInFlight = false;
      }
    };

    const clearTimers = () => {
      if (statusTimer) window.clearInterval(statusTimer);
      if (statusRetryTimer) window.clearTimeout(statusRetryTimer);
      if (eventTimer) window.clearInterval(eventTimer);
      if (eventRetryTimer) window.clearTimeout(eventRetryTimer);
      statusTimer = null;
      statusRetryTimer = null;
      eventTimer = null;
      eventRetryTimer = null;
    };

    const configurePolling = () => {
      clearTimers();
      if (document.visibilityState === "hidden") return;
      statusTimer = window.setInterval(() => {
        void refreshStatusViews();
      }, STATUS_REFRESH_MS);
      eventTimer = window.setInterval(() => {
        void refreshEventViews();
      }, EVENT_REFRESH_MS);
    };

    const handleVisibilityChange = () => {
      configurePolling();
      if (document.visibilityState !== "hidden") {
        void refreshStatusViews();
        void refreshEventViews();
      }
    };

    void refreshStatusViews();
    void refreshEventViews();
    if (OPERATOR_DIAGNOSTICS_ENABLED) {
      void refreshContractEventDebugCounts();
      void refreshRawFlowDebug();
      void refreshLatencyDebug();
    }
    void refreshWhaleEvents();
    configurePolling();
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      cancelled = true;
      clearTimers();
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [filters]);

  useEffect(() => {
    const scopedItems = [
      ...filterContractItemsBySymbol(state.items, assetSymbol),
      ...filterContractItemsBySymbol(state.contractEvents, assetSymbol),
      ...filterContractItemsBySymbol(state.finalEvents.active, assetSymbol),
      ...filterContractItemsBySymbol(state.finalEvents.closed, assetSymbol),
    ];
    if (selectedSignalId && !scopedItems.some((item) => matchesSignalIdentity(item, selectedSignalId))) {
      setSelectedSignalId(null);
    }
  }, [
    assetSymbol,
    selectedSignalId,
    state.contractEvents,
    state.finalEvents.active,
    state.finalEvents.closed,
    state.items,
  ]);

  useEffect(() => {
    const scopedItems = filterContractItemsBySymbol(state.items, assetSymbol);
    if (scopedItems.length === 0) {
      if (selectedWhaleId) setSelectedWhaleId(null);
      return;
    }
    const entities = buildWhaleEntities(scopedItems);
    if (!selectedWhaleId || !entities.some((entity) => entity.id === selectedWhaleId)) {
      setSelectedWhaleId(entities[0]?.id || null);
    }
  }, [assetSymbol, selectedWhaleId, state.items]);

  const summary = state.summary || {
    status: "calm",
    healthStatus: "disabled",
    healthReason: "contract_whale_monitor_disabled",
    thresholdProfile: "binance_bitfinex",
    thresholdProfileReason: "active_contract_sources=binance,bitfinex",
    configuredContractSources: ["binance", "bitfinex"],
    eligibleContractSources: ["binance", "bitfinex"],
    activeExchangeCount: 0,
    enabledExchanges: [],
    disabledExchanges: ["binance", "okx", "bitfinex"],
        activeContractExchanges: [],
        direction: "neutral",
    latestDirection: "neutral",
    latestSeverity: "calm",
    latestPushedAtMs: null,
    lastDiscordSentAt: null,
    signalCount: 0,
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
  const platformCapabilities = summary.platforms || {};
  const latestItems = useMemo(
    () => filterContractItemsBySymbol(state.items, assetSymbol),
    [assetSymbol, state.items],
  );
  const contractEvents = useMemo(
    () => filterContractItemsBySymbol(state.contractEvents, assetSymbol),
    [assetSymbol, state.contractEvents],
  );
  const finalActiveEvents = useMemo(
    () => filterContractItemsBySymbol(state.finalEvents.active, assetSymbol),
    [assetSymbol, state.finalEvents.active],
  );
  const finalClosedEvents = useMemo(
    () => filterContractItemsBySymbol(state.finalEvents.closed, assetSymbol),
    [assetSymbol, state.finalEvents.closed],
  );
  const finalEvents = useMemo(
    () => ({ active: finalActiveEvents, closed: finalClosedEvents }),
    [finalActiveEvents, finalClosedEvents],
  );
  const whaleEvents = useMemo(
    () => filterContractItemsBySymbol(state.events, assetSymbol),
    [assetSymbol, state.events],
  );
  const hiddenContractEvents = useMemo(
    () => filterContractItemsBySymbol(state.hiddenContractEvents, assetSymbol),
    [assetSymbol, state.hiddenContractEvents],
  );
  const lifecycleItems = useMemo(
    () => [...finalEvents.active, ...finalEvents.closed],
    [finalEvents],
  );
  const detailItems = useMemo(
    () => dedupeSignalsById([...latestItems, ...contractEvents, ...lifecycleItems]),
    [contractEvents, latestItems, lifecycleItems],
  );
  const displayFilterLabel = contractWhaleDisplayFilterLabel(assetSymbol);
  const visibleContractEvents = useMemo(
    () => contractEvents.filter((item) => passesContractWhaleVisibleDisplayFilter(item, assetSymbol)),
    [assetSymbol, contractEvents],
  );
  const hiddenDisplayFilteredCount = Math.max(0, contractEvents.length - visibleContractEvents.length);
  const shouldApplyNotionalDisplayFilter = contractEvents.length > 0;
  const visibleSignalIds = buildVisibleSignalIdSet(visibleContractEvents);
  const displayIntelligence = shouldApplyNotionalDisplayFilter
    ? filterIntelligenceByVisibleSignals(state.intelligenceTerminal, visibleSignalIds)
    : state.intelligenceTerminal;
  const intelligenceSlice = state.dataSlices.intelligence;
  const currentDisplayIntelligence = intelligenceSlice.state === "fresh" ? displayIntelligence : null;
  const displaySummary = shouldApplyNotionalDisplayFilter
    ? filterSummaryTradeOpportunitiesByVisibleSignals(summary, visibleSignalIds)
    : summary;
  const latestSignalTs = Math.max(
    0,
    ...latestItems.map((item) => Number(item?.ts) || 0),
  );
  const eventsSyncLag = latestSignalTs > 0 &&
    Number.isFinite(state.contractEventsLastEventTs) &&
    latestSignalTs - state.contractEventsLastEventTs > EVENTS_SYNC_LAG_MS;
  const selectedSignal = detailItems.find((item) => matchesSignalIdentity(item, selectedSignalId)) || null;
  const whaleEntities = buildWhaleEntities(latestItems);
  const showCoinbaseSpotOnlyNotice =
    filters.exchange === "coinbase" || state.meta?.reason === "coinbase_perp_disabled";

  useEffect(() => {
    if (!selectedSignalId || !selectedSignal || hasRichContractEventDetail(selectedSignal)) {
      return undefined;
    }
    let cancelled = false;
    void (async () => {
      const payload = await fetchContractEvents({
        ...filters,
        range: CONTRACT_EVENT_RANGE,
        limit: DEFAULT_CONTRACT_EVENT_LIMIT,
        includeSourceSignal: true,
      });
      if (cancelled || !isUsableDataPayload(payload)) return;
      const enriched = payload.items.find((item) => matchesSignalIdentity(item, selectedSignalId));
      if (!enriched) return;
      setState((previous) => ({
        ...previous,
        contractEvents: previous.contractEvents.map((item) => (
          matchesSignalIdentity(item, selectedSignalId) ? { ...item, ...enriched } : item
        )),
        finalEvents: {
          active: previous.finalEvents.active.map((item) => (
            matchesSignalIdentity(item, selectedSignalId) ? { ...item, ...enriched } : item
          )),
          closed: previous.finalEvents.closed.map((item) => (
            matchesSignalIdentity(item, selectedSignalId) ? { ...item, ...enriched } : item
          )),
        },
        items: previous.items.map((item) => (
          matchesSignalIdentity(item, selectedSignalId) ? { ...item, ...enriched } : item
        )),
      }));
    })();
    return () => {
      cancelled = true;
    };
  }, [filters, selectedSignal, selectedSignalId]);

  async function loadMoreContractEvents() {
    if (!state.contractEventsHasMore || !state.contractEventsCursor) return;
    const payload = await fetchContractEvents({
      ...filters,
      range: CONTRACT_EVENT_RANGE,
      limit: 100,
      cursor: state.contractEventsCursor,
    });
    if (payload.error) return;
    setState((previous) => ({
      ...previous,
      contractEvents: mergeUniqueById(previous.contractEvents, payload.items),
      contractEventsCursor: payload.nextCursor,
      contractEventsHasMore: payload.hasMore,
      contractEventsServerTime: payload.serverTime,
      contractEventsLastEventTs: payload.lastEventTs ?? previous.contractEventsLastEventTs,
    }));
  }

  async function toggleHiddenContractEvents() {
    if (state.hiddenContractEventsExpanded) {
      setState((previous) => ({
        ...previous,
        hiddenContractEventsExpanded: false,
      }));
      return;
    }
    if (state.hiddenContractEventsLoaded) {
      setState((previous) => ({
        ...previous,
        hiddenContractEventsExpanded: true,
      }));
      return;
    }
    setState((previous) => ({
      ...previous,
      hiddenContractEventsLoading: true,
    }));
    const payload = await fetchContractEvents({
      ...filters,
      range: CONTRACT_EVENT_RANGE,
      limit: 100,
      includeHidden: true,
    });
    setState((previous) => ({
      ...previous,
      hiddenContractEvents: payload.error
        ? previous.hiddenContractEvents
        : payload.items.filter((item) => item.isVisible === false),
      hiddenContractEventsLoaded: !payload.error,
      hiddenContractEventsExpanded: !payload.error,
      hiddenContractEventsLoading: false,
    }));
  }

  async function loadMoreFinalEvents() {
    if (!state.finalEventsHasMore || !state.finalEventsCursor) return;
    const payload = await fetchFinalEventsV2({
      symbol: filters.symbol,
      range: "24h",
      limit: 100,
      cursor: state.finalEventsCursor,
    });
    if (payload.error) return;
    setState((previous) => ({
      ...previous,
      finalEvents: {
        active: mergeUniqueById(previous.finalEvents.active, payload.active),
        closed: mergeUniqueById(previous.finalEvents.closed, payload.closed),
      },
      finalEventsCursor: payload.nextCursor,
      finalEventsHasMore: payload.hasMore,
      finalEventsServerTime: payload.serverTime,
      finalEventsLastEventTs: payload.lastEventTs ?? previous.finalEventsLastEventTs,
    }));
  }

  return (
    <section className="contract-workspace overflow-x-hidden" data-testid="contract-workspace">
      <ContractWorkspaceCommandBar
        contractEvents={contractEvents}
        latestItems={latestItems}
        summary={summary}
        symbol={assetSymbol}
      />
      <ContractWorkspaceStatusRibbon
        displayFilterLabel={displayFilterLabel}
        intelligence={currentDisplayIntelligence}
        summary={summary}
      />

      <div className="contract-filter-dock">
        <ContractWhaleFilters
          filters={filters}
          lockedSymbol={assetSymbol}
          onChange={(nextFilters) => {
            setSelectedSignalId(null);
            setSelectedWhaleId(null);
            setFilters({ ...nextFilters, symbol: assetSymbol });
          }}
        />
        <p className="contract-filter-note">
          <span className="text-slate-300">VISIBLE GATE</span>
          <span>{displayFilterLabel}</span>
          <span>价格偏离 ≤ {CWM_MAX_PRICE_DEVIATION_PCT}%</span>
          <span>保留：合约市场事件完整数据 ≥ 7 天 / B 3 个月 / A·S 永久</span>
        </p>
        <DataHealthBanner dataSlices={state.dataSlices} />
      </div>

      <section
        className="contract-primary-grid"
        data-testid="primary-analysis-grid"
      >
        <div className="min-w-0">
          <HistoricalEventStreamPanel
            contractEvents={contractEvents}
            visibleContractEvents={visibleContractEvents}
            hiddenDisplayFilteredCount={hiddenDisplayFilteredCount}
            debugCounts={state.contractEventDebugCounts}
            rawFlowDebug={state.rawFlowDebug}
            enabled={summary.enabled}
            loading={state.contractEventsLoading}
            error={state.dataSlices.historical.state === "unavailable"}
            onLoadMoreContractEvents={loadMoreContractEvents}
            onOpenSignal={setSelectedSignalId}
            eventsSyncLag={eventsSyncLag}
            latestSignalTs={latestSignalTs}
            contractEventsLastEventTs={state.contractEventsLastEventTs}
            latestMaxTs={state.latestMaxTs}
            contractEventsMaxEventTs={state.contractEventsMaxEventTs}
            contractEventsLatestLagSec={state.contractEventsLatestLagSec}
            contractEventsHistoryLagSec={state.contractEventsHistoryLagSec}
            contractEventsHasMore={state.contractEventsHasMore}
            displayFilterLabel={displayFilterLabel}
            symbol={filters.symbol}
          />

          <ProDeskOverviewBar
            contractEventsLastEventTs={state.contractEventsLastEventTs}
            intelligence={currentDisplayIntelligence}
            intelligenceSlice={intelligenceSlice}
            latestSignalTs={latestSignalTs}
            previousIntelligence={displayIntelligence}
            summary={summary}
          />
        </div>

        <ContractDeskInsightRail
          intelligence={currentDisplayIntelligence}
          latestItems={latestItems}
          summary={summary}
        />
      </section>

      <EventFirstJumpNavigation />

      <section
        className="mt-4 grid gap-4 2xl:grid-cols-[minmax(280px,0.75fr)_minmax(280px,0.75fr)_minmax(0,1.15fr)] 2xl:items-start"
        data-testid="secondary-analysis-grid"
      >
        <MarketStructureDeskPanel intelligence={currentDisplayIntelligence} summary={summary} />
        <LiquidityMapDeskPanel intelligence={currentDisplayIntelligence} />
        <TradeSetupsDeskPanel
          intelligence={currentDisplayIntelligence}
          onSelectSignal={(signalId) => {
            setSelectedSignalId(signalId);
            document.getElementById("contract-whale-events")?.scrollIntoView({ behavior: "smooth", block: "start" });
          }}
          selectedSignalId={selectedSignalId}
          summary={displaySummary}
        />
      </section>

      <section
        className="mt-4 grid gap-4 2xl:grid-cols-[minmax(0,1.35fr)_minmax(320px,0.65fr)] 2xl:items-start"
        data-testid="lifecycle-risk-grid"
      >
        <LifecycleEventSections
          finalEvents={finalEvents}
          finalEventsHasMore={state.finalEventsHasMore}
          onLoadMoreFinalEvents={loadMoreFinalEvents}
          onOpenSignal={setSelectedSignalId}
          symbol={filters.symbol}
        />
        <RiskContextDeskPanel intelligence={currentDisplayIntelligence} summary={summary} />
      </section>

      <ContractWhaleSystemStatusPanel
        contractEvents={contractEvents}
        debugCounts={state.contractEventDebugCounts}
        rawFlowDebug={state.rawFlowDebug}
        latencyDebug={state.latencyDebug}
        enabled={summary.enabled}
        latestItems={latestItems}
        finalEvents={finalEvents}
        finalEventsHasMore={state.finalEventsHasMore}
        hiddenContractEvents={hiddenContractEvents}
        hiddenContractEventsExpanded={state.hiddenContractEventsExpanded}
        hiddenContractEventsLoading={state.hiddenContractEventsLoading}
        loading={state.loading}
        onLoadMoreFinalEvents={loadMoreFinalEvents}
        onOpenSignal={setSelectedSignalId}
        onToggleHiddenContractEvents={toggleHiddenContractEvents}
        retentionStatus={state.retentionStatus}
        eventsSyncLag={eventsSyncLag}
        latestSignalTs={latestSignalTs}
        latestTimeline={state.latestTimeline}
        contractEventsLastEventTs={state.contractEventsLastEventTs}
        latestMaxTs={state.latestMaxTs}
        contractEventsMaxEventTs={state.contractEventsMaxEventTs}
        contractEventsLatestLagSec={state.contractEventsLatestLagSec}
        contractEventsHistoryLagSec={state.contractEventsHistoryLagSec}
        contractEventsTimeline={state.contractEventsTimeline}
        finalEventsMaxEventTs={state.finalEventsMaxEventTs}
        finalEventsProjectionLagSec={state.finalEventsProjectionLagSec}
        finalEventsTimeline={state.finalEventsTimeline}
        symbol={filters.symbol}
        summary={summary}
        platformCapabilities={platformCapabilities}
        showCoinbaseSpotOnlyNotice={showCoinbaseSpotOnlyNotice}
      />

      <WhaleTrajectoryDashboard
        enabled={summary.enabled}
        loading={state.loading}
        onOpenSignal={setSelectedSignalId}
        onSelectWhale={setSelectedWhaleId}
        selectedWhaleId={selectedWhaleId}
        symbol={filters.symbol}
        whales={whaleEntities}
      />

      <MainForceEventsSection events={whaleEvents} symbol={filters.symbol} />

      {selectedSignal ? (
        <ContractWhaleDetailModal
          summary={summary}
          signal={selectedSignal}
          relatedSignals={detailItems}
          onClose={() => setSelectedSignalId(null)}
        />
      ) : null}
    </section>
  );
}

function ContractWorkspaceCommandBar({ contractEvents, latestItems, summary, symbol }) {
  const [now, setNow] = useState(() => new Date());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 1_000);
    return () => window.clearInterval(timer);
  }, []);

  const latest = latestItems[0] || contractEvents[0] || null;
  const price = signalTriggerPrice(latest);
  const priceMovePct = numberOrNull(latest?.priceMovePct);
  const fundingRate = numberOrNull(latest?.fundingRate ?? latest?.sourceSignal?.fundingRate);
  const oiDeltaBtc = numberOrNull(
    latest?.oiDelta
      ?? latest?.sourceSignal?.oiDelta
      ?? latest?.eventLifecycle?.netOiDeltaBtc,
  );
  const notional24h = contractEvents.reduce(
    (sum, item) => sum + finiteNumber(item?.notionalUsd ?? item?.totalNotionalUsd ?? item?.notional, 0),
    0,
  );
  const sourceCount = contractSourceLabels(summary).length;
  const healthy = !["unavailable", "offline", "error"].includes(String(summary?.healthStatus || "healthy").toLowerCase());

  return (
    <header className="contract-command-bar" data-testid="contract-workspace-command-bar">
      <div className="contract-market-selector">
        <span className="contract-market-symbol">₿</span>
        <div>
          <p className="contract-market-title">{symbol} / PERP</p>
          <p className="contract-market-subtitle">{symbol} CONTRACT WHALE FLOW</p>
        </div>
      </div>

      <div className="contract-price-block">
        <p className="contract-price-value">{price === null ? "N/A" : formatPrice(price)}</p>
        <p className={priceMovePct === null ? "text-slate-500" : signedMetricClass(priceMovePct)}>
          {priceMovePct === null ? "PRICE N/A" : formatSignedPct(priceMovePct)}
        </p>
      </div>

      <div className="contract-command-metrics">
        <ContractWorkspaceMetric label="资金费率" value={fundingRate === null ? "N/A" : formatFundingPercent(fundingRate)} tone={fundingRate} />
        <ContractWorkspaceMetric label="持仓量 Δ" value={oiDeltaBtc === null ? "N/A" : formatSignedBaseVolume(oiDeltaBtc, symbol)} tone={oiDeltaBtc} />
        <ContractWorkspaceMetric label="事件名义价值 (24H)" value={notional24h > 0 ? formatUsd(notional24h) : "N/A"} />
        <ContractWorkspaceMetric label="覆盖交易所" value={sourceCount > 0 ? `${sourceCount}` : "N/A"} />
      </div>

      <div className="contract-command-status">
        <div className="text-right">
          <p className="font-mono text-[11px] text-slate-400">UTC+8 · {now.toLocaleTimeString("zh-CN", { hour12: false })}</p>
          <p className="mt-1 text-[10px] uppercase tracking-[0.16em] text-slate-600">主力合约监控</p>
        </div>
        <div className={healthy ? "contract-live-state text-emerald-300" : "contract-live-state text-amber-300"}>
          <span className={healthy ? "bg-emerald-400" : "bg-amber-400"} />
          {healthy ? "LIVE" : "RECOVERING"}
        </div>
        <span className="contract-readonly-badge">只读监控</span>
        <span className="sr-only">只读提醒 · 不下单 · CWM Discord gate 独立</span>
      </div>
    </header>
  );
}

function ContractWorkspaceMetric({ label, value, tone = null }) {
  return (
    <div className="contract-command-metric">
      <p>{label}</p>
      <strong className={tone === null ? "text-slate-200" : signedMetricClass(tone)}>{value}</strong>
    </div>
  );
}

function ContractWorkspaceStatusRibbon({ displayFilterLabel, intelligence, summary }) {
  const regime = intelligence?.marketRegime?.regime || summary?.marketStructureLite?.regimeType || "UNKNOWN";
  const regimeConfidence = numberOrNull(intelligence?.marketRegime?.confidence ?? summary?.marketStructureLite?.confidence);
  return (
    <section className="contract-status-ribbon" data-testid="contract-workspace-status-ribbon">
      <WorkspaceStatusItem label="REGIME" value={String(regime).toUpperCase()} detail={regimeConfidence === null ? "未确认" : `${Math.round(regimeConfidence)}% confidence`} tone="amber" />
      <WorkspaceStatusItem label="DIRECTION" value={directionLabel(summary.latestDirection || summary.direction)} detail={biasText(summary?.marketStructureLite?.structureBias)} tone="emerald" />
      <WorkspaceStatusItem label="SIGNAL" value={severityLabel(summary.latestSeverity)} detail={statusLabel(summary.status)} tone="cyan" />
      <WorkspaceStatusItem label="HEALTH" value={healthStatusLabel(summary.healthStatus)} detail={modeLabel(summary)} tone="emerald" />
      <WorkspaceStatusItem label="THRESHOLD" value={thresholdProfileLabel(summary.thresholdProfile)} detail={displayFilterLabel} tone="slate" wide />
      <WorkspaceStatusItem label="LAST PUSH" value={summary.lastDiscordSentAt ? relativeAge(summary.lastDiscordSentAt) : "暂无"} detail="Discord gate 独立" tone="slate" />
    </section>
  );
}

function WorkspaceStatusItem({ detail, label, tone, value, wide = false }) {
  const toneClass = {
    amber: "text-amber-300",
    cyan: "text-cyan-200",
    emerald: "text-emerald-300",
    slate: "text-slate-200",
  }[tone] || "text-slate-200";
  return (
    <div className={`contract-status-item ${wide ? "contract-status-item-wide" : ""}`}>
      <p>{label}</p>
      <strong className={toneClass}>{value}</strong>
      <span>{detail}</span>
    </div>
  );
}

function ContractDeskInsightRail({ intelligence, latestItems, summary }) {
  const regime = intelligence?.marketRegime || {};
  const liquidityBehaviors = Array.isArray(intelligence?.liquidityBehaviors) ? intelligence.liquidityBehaviors : [];
  const riskContext = intelligence?.riskContext || {};
  const noTradeZones = Array.isArray(riskContext?.noTradeZones) ? riskContext.noTradeZones : [];
  const setup = deriveDeskTradeIdeas(intelligence, summary)[0] || null;
  const latest = latestItems[0] || null;
  const oiDelta = numberOrNull(latest?.oiDelta ?? latest?.sourceSignal?.oiDelta ?? latest?.eventLifecycle?.netOiDeltaBtc);
  const structureBias = summary?.marketStructureLite?.structureBias;
  const strength = Math.max(0, Math.min(100, Math.round(numberOrNull(regime.confidence) ?? 0)));

  return (
    <aside className="contract-insight-rail" data-testid="contract-insight-rail">
      <section className="contract-insight-panel" id="contract-whale-structure-snapshot">
        <div className="contract-insight-heading">
          <div>
            <p>MARKET STRUCTURE</p>
            <h4>市场结构</h4>
          </div>
          <span>{strength}%</span>
        </div>
        <div className="contract-insight-two-col">
          <WorkspaceMiniMetric label="Regime 当前" value={String(regime.regime || "UNKNOWN").toUpperCase()} tone="amber" />
          <WorkspaceMiniMetric label="方向偏置" value={biasText(structureBias)} tone={finiteNumber(structureBias, 0) >= 0 ? "emerald" : "red"} />
        </div>
        <DeskStrengthBar value={strength} />
        <p className="contract-insight-copy">{regime.reason || "当前结构证据不足，保持只读观察。"}</p>
      </section>

      <section className="contract-insight-panel">
        <div className="contract-insight-heading">
          <div>
            <p>LIQUIDITY &amp; OI</p>
            <h4>流动性与 OI</h4>
          </div>
          <span>{liquidityBehaviors.length} signals</span>
        </div>
        <div className="contract-liquidity-balance">
          <span style={{ width: `${Math.max(16, Math.min(84, 50 + finiteNumber(structureBias, 0) / 2))}%` }} />
        </div>
        <div className="contract-insight-two-col">
          <WorkspaceMiniMetric
            label="主导行为"
            value={liquidityBehaviors[0]?.label ? `主导 · ${liquidityBehaviors[0].label}` : "暂无明确结构"}
            tone="cyan"
          />
          <WorkspaceMiniMetric label="持仓量 Δ" value={oiDelta === null ? "N/A" : formatSignedBaseVolume(oiDelta, latest?.symbol || "BTC")} tone={oiDelta !== null && oiDelta < 0 ? "red" : "emerald"} />
        </div>
        <p className="contract-insight-copy">{latest ? oiStatus(latest) : "等待 OI 证据"}</p>
      </section>

      <section className="contract-insight-panel">
        <div className="contract-insight-heading">
          <div>
            <p>OPPORTUNITY &amp; RISK</p>
            <h4>交易机会 / 风险</h4>
          </div>
          <span className={riskBadgeClass(riskContext.fakeBreakoutRisk)}>{riskLabel(riskContext.fakeBreakoutRisk)}</span>
        </div>
        <div className="contract-opportunity-card">
          <p>首选结构</p>
          <strong>{setup?.setupType ? `#1 · ${setup.setupType}` : "暂无明确结构"}</strong>
          <span>{setup ? `${setup.directionLabel} · ${setup.score}/100 · ${setup.confidence}%` : "等待事件确认"}</span>
        </div>
        <div className="contract-risk-card">
          <p>失效 / No-trade</p>
          <strong>{noTradeZones[0]?.label || "暂无明确风险区"}</strong>
          <span>{noTradeZones[0]?.rangeLabel || riskContext.summary || "结构风险保持受控"}</span>
        </div>
      </section>
    </aside>
  );
}

function WorkspaceMiniMetric({ label, tone, value }) {
  const toneClass = {
    amber: "text-amber-300",
    cyan: "text-cyan-200",
    emerald: "text-emerald-300",
    red: "text-rose-300",
  }[tone] || "text-slate-200";
  return (
    <div className="contract-insight-metric">
      <p>{label}</p>
      <strong className={toneClass}>{value}</strong>
    </div>
  );
}

function DeskStrengthBar({ value }) {
  return (
    <div className="contract-strength-bar" aria-label={`结构强度 ${value}%`} role="img">
      <span className="bg-rose-400" />
      <span className="bg-amber-400" />
      <span className="bg-slate-500" />
      <span className="bg-lime-400" />
      <span className="bg-emerald-300" />
      <i style={{ left: `${Math.max(2, Math.min(98, value))}%` }} />
    </div>
  );
}

function signedMetricClass(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number === 0) return "text-slate-300";
  return number > 0 ? "text-emerald-300" : "text-rose-300";
}

function formatFundingPercent(value) {
  const number = numberOrNull(value);
  if (number === null) return "N/A";
  return `${number >= 0 ? "+" : ""}${(number * 100).toFixed(4)}%`;
}

function WhaleTrajectoryDashboard({
  enabled,
  loading,
  onOpenSignal,
  onSelectWhale,
  selectedWhaleId,
  symbol,
  whales,
}) {
  const selectedWhale = whales.find((whale) => whale.id === selectedWhaleId) || whales[0] || null;
  return (
    <section className="mt-4 rounded-2xl border border-cyan-500/20 bg-slate-950/35 p-4">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <p className="console-label text-cyan-300">Whale Behavior Timeline</p>
          <h4 className="mt-1 text-base font-bold text-white">主力行为轨迹（辅助）</h4>
          <p className="mt-1 max-w-3xl text-xs leading-5 text-slate-400">
            按同一 symbol、方向、价格区间和时间连续性把事件继续合并成 whale entity，用于复盘连续主力意图；上方事件表展示已压缩的市场事件。
          </p>
        </div>
        <div className="grid grid-cols-3 gap-2 text-xs text-slate-300">
          <MiniInfoCard label="Whale Entities" value={`${whales.length}`} detail={`当前筛选 ${symbol}`} />
          <MiniInfoCard label="Merged Signals" value={`${whales.reduce((sum, whale) => sum + whale.signalCount, 0)}`} detail="去重后的主力投影" />
          <MiniInfoCard label="Focus" value={selectedWhale ? shortWhaleId(selectedWhale.id) : "N/A"} detail={selectedWhale ? trajectoryIntentLabel(selectedWhale.intent) : "等待数据"} />
        </div>
      </div>

      {loading ? (
        <p className="mt-4 rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
          主力轨迹载入中...
        </p>
      ) : whales.length === 0 ? (
        <p className="mt-4 rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
          {enabled ? `暂无 ${symbol} 主力轨迹` : "主力合约监控未启用"}
        </p>
      ) : (
        <div className="mt-4 grid gap-4 xl:grid-cols-[minmax(240px,0.38fr)_minmax(0,1fr)]">
          <aside className="rounded-xl border border-slate-800 bg-slate-950/45 p-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="console-label">Whale Entity List</p>
                <h5 className="mt-1 text-sm font-bold text-white">主力实体</h5>
              </div>
              <span className="rounded-full border border-cyan-500/30 px-2 py-1 text-[11px] font-semibold text-cyan-100">
                {whales.length} active
              </span>
            </div>
            <div className="mt-3 space-y-2">
              {whales.map((whale) => (
                <WhaleEntityCard
                  key={whale.id}
                  onSelect={() => onSelectWhale(whale.id)}
                  selected={whale.id === selectedWhale?.id}
                  whale={whale}
                />
              ))}
            </div>
          </aside>

          <TrajectoryFocusPanel onOpenSignal={onOpenSignal} whale={selectedWhale} />
        </div>
      )}
    </section>
  );
}

function WhaleEntityCard({ onSelect, selected, whale }) {
  return (
    <button
      className={`w-full rounded-xl border px-3 py-3 text-left outline-none transition focus-visible:ring-2 focus-visible:ring-cyan-500/35 ${
        selected
          ? "border-cyan-400/70 bg-cyan-500/10 shadow-glow"
          : "border-slate-800 bg-slate-900/55 hover:border-cyan-500/40 hover:bg-slate-900"
      }`}
      data-testid={`whale-entity-card-${whale.id}`}
      onClick={onSelect}
      type="button"
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-bold text-slate-100">{shortWhaleId(whale.id)}</p>
          <p className="mt-1 text-xs text-cyan-100">{trajectoryIntentLabel(whale.intent)}</p>
        </div>
        <span className={`rounded-full px-2 py-1 text-[11px] font-bold ${severityBadgeClass(whale.severity)}`}>
          {severityLabel(whale.severity)}
        </span>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2 text-[11px] text-slate-400">
        <p>signals {whale.signalCount}</p>
        <p>duration {formatMsDuration(whale.durationMs)}</p>
        <p>stealth {formatPct(whale.stealthGamma * 100)}</p>
        <p>λ proxy {formatPct(whale.hazardPeak * 100)}</p>
      </div>
      <p className="mt-2 truncate text-[11px] text-slate-500" title={regimePathLabel(whale.regimePath)}>
        {regimePathLabel(whale.regimePath)}
      </p>
    </button>
  );
}

function TrajectoryFocusPanel({ onOpenSignal, whale }) {
  if (!whale) return null;
  const primarySignal = whale.signals[0];
  return (
    <article className="rounded-xl border border-slate-800 bg-slate-950/45 p-4">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <p className="console-label">Trajectory Overview</p>
          <h5 className="mt-1 text-base font-bold text-white">
            {primarySignal.symbol} · {trajectoryIntentLabel(whale.intent)}
          </h5>
          <p className="mt-1 max-w-3xl text-xs leading-5 text-slate-400">
            {whale.conclusion || "轨迹证据不足，保持观察。"}
          </p>
        </div>
        <button
          className="rounded-lg border border-cyan-500/40 px-3 py-2 text-xs font-semibold text-cyan-100 outline-none transition hover:border-cyan-300 hover:bg-cyan-500/10 focus-visible:ring-2 focus-visible:ring-cyan-500/35"
          onClick={() => onOpenSignal(primarySignal.id)}
          type="button"
        >
          查看代表信号
        </button>
      </div>

      <div className="mt-4 grid gap-2 text-xs md:grid-cols-2 xl:grid-cols-4">
        <MiniInfoCard label="Dominant Intent" value={trajectoryIntentLabel(whale.intent)} detail={clusterIntentLabel(whale.clusterIntent)} />
        <MiniInfoCard label="Regime Path" value={regimePathLabel(whale.regimePath)} detail="phase path" />
        <MiniInfoCard label="Persistence" value={formatPct(whale.persistenceScore * 100)} detail={`stability ${formatPct(whale.regimeStability * 100)}`} />
        <MiniInfoCard label="Duration" value={formatMsDuration(whale.durationMs)} detail={`${whale.signalCount} signals merged`} />
      </div>

      <TrajectoryTimeline phases={whale.phases} />

      <div className="mt-4 grid gap-3 lg:grid-cols-2">
        <PhaseBreakdown phases={whale.phases} />
        <div className="grid gap-3">
          <CurvePanel label="Stealth Curve (gamma)" points={whale.stealthCurve} tone="cyan" />
          <CurvePanel label="Hazard Curve (lambda proxy)" points={whale.hazardCurve} tone="amber" />
        </div>
      </div>

      <details className="mt-4 rounded-xl border border-slate-800 bg-slate-950/45 px-3 py-2 text-xs text-slate-400">
        <summary className="cursor-pointer select-none text-slate-300 outline-none transition hover:text-cyan-200 focus-visible:ring-2 focus-visible:ring-cyan-500/35">
          Signals collapsed debug · {whale.signalCount} 条
        </summary>
        <div className="mt-3 grid gap-2 md:grid-cols-2 xl:grid-cols-3">
          {whale.signals.map((signal) => (
            <button
              className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2 text-left outline-none transition hover:border-cyan-500/40 hover:bg-cyan-500/5 focus-visible:ring-2 focus-visible:ring-cyan-500/35"
              key={signal.id}
              onClick={() => onOpenSignal(signal.id)}
              type="button"
            >
              <p className="font-semibold text-slate-100">{formatTime(signal.ts)} · {signalDisplayType(signal)}</p>
              <p className="mt-1 text-slate-400">
                {formatBaseVolume(signal.totalVolumeBtc, signal.symbol)} · {netDirection(signal.netVolumeBtc, signal.symbol)}
              </p>
            </button>
          ))}
        </div>
      </details>
    </article>
  );
}

function TrajectoryTimeline({ phases }) {
  return (
    <section className="mt-4">
      <p className="console-label">Trajectory Timeline</p>
      <div className="mt-3 grid gap-2 md:grid-cols-3">
        {phases.map((phase, index) => (
          <div className={`rounded-xl border px-3 py-3 ${phaseToneClass(phase.type)}`} key={`${phase.type}-${index}`}>
            <div className="flex items-center justify-between gap-2">
              <p className="text-xs font-bold text-slate-100">{index + 1}. {phaseLabel(phase.type)}</p>
              <span className="text-[11px] text-slate-400">{formatTime(phase.ts)}</span>
            </div>
            <div className="mt-3 h-2 overflow-hidden rounded-full bg-slate-800">
              <div
                className={phaseBarClass(phase.type)}
                style={{ width: `${Math.max(8, Math.min(100, phase.intensity * 100))}%` }}
              />
            </div>
            <p className="mt-2 text-[11px] leading-5 text-slate-400">{phase.detail}</p>
          </div>
        ))}
      </div>
    </section>
  );
}

function PhaseBreakdown({ phases }) {
  return (
    <section className="rounded-xl border border-slate-800 bg-slate-950/45 p-3">
      <p className="console-label">Phase Breakdown</p>
      <div className="mt-3 space-y-2">
        {phases.map((phase, index) => (
          <div className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2" key={`${phase.type}-breakdown-${index}`}>
            <div className="flex items-center justify-between gap-3">
              <p className="text-xs font-semibold text-slate-100">{phaseLabel(phase.type)}</p>
              <span className="text-[11px] text-cyan-100">{formatPct(phase.intensity * 100)}</span>
            </div>
            <p className="mt-1 text-[11px] leading-5 text-slate-400">{phase.detail}</p>
          </div>
        ))}
      </div>
    </section>
  );
}

function CurvePanel({ label, points, tone }) {
  return (
    <section className="rounded-xl border border-slate-800 bg-slate-950/45 p-3">
      <div className="flex items-center justify-between gap-3">
        <p className="console-label">{label}</p>
        <span className="text-xs font-semibold text-slate-100">{formatPct(Math.max(...points, 0) * 100)}</span>
      </div>
      <div className="mt-3 flex h-16 items-end gap-1">
        {points.map((point, index) => (
          <span
            className={`flex-1 rounded-t ${curveBarClass(tone)}`}
            key={`${label}-${index}`}
            style={{ height: `${Math.max(8, Math.min(100, point * 100))}%` }}
            title={formatPct(point * 100)}
          />
        ))}
      </div>
    </section>
  );
}

function dedupeSignalsById(items) {
  const seen = new Set();
  const result = [];
  for (const item of items) {
    if (!item?.id || seen.has(item.id)) continue;
    seen.add(item.id);
    result.push(item);
  }
  return result;
}

function mergeUniqueById(previousItems, nextItems) {
  const map = new Map();
  [...(previousItems || []), ...(nextItems || [])].forEach((item) => {
    const key = item?.eventId || item?.finalEventId || item?.id;
    if (!key) return;
    map.set(key, item);
  });
  return Array.from(map.values());
}

function EventFirstJumpNavigation() {
  const items = [
    { href: "#contract-whale-events", label: "Events" },
    { href: "#contract-whale-structure", label: "Structure" },
    { href: "#contract-whale-liquidity", label: "Liquidity" },
    { href: "#contract-whale-setups", label: "Setups" },
    { href: "#contract-whale-risk", label: "Risk" },
    { href: "#contract-whale-status", label: "Status" },
  ];

  return (
    <nav
      aria-label="Contract whale section navigation"
      className="contract-workspace-tabs"
    >
      {items.map((item) => (
        <a
          className="contract-workspace-tab"
          href={item.href}
          key={item.href}
        >
          {item.label}
        </a>
      ))}
    </nav>
  );
}

function deriveEventFeedDiagnostics({
  contractEvents,
  debugCounts,
  rawFlowDebug,
  latencyDebug,
  latestItems,
  latestTimeline,
  finalEvents,
  finalEventsTimeline,
  retentionStatus,
  eventsSyncLag,
  latestSignalTs,
  contractEventsLastEventTs,
  latestMaxTs,
  contractEventsMaxEventTs,
  contractEventsLatestLagSec,
  contractEventsHistoryLagSec,
  contractEventsTimeline,
  finalEventsMaxEventTs,
  finalEventsProjectionLagSec,
}) {
  const activeItems = finalEvents.active || [];
  const closedItems = finalEvents.closed || [];
  const visibleCount = Number(debugCounts?.visibility?.visibleCount ?? contractEvents.length ?? 0);
  const hiddenCount = Number(debugCounts?.visibility?.hiddenCount ?? 0);
  const backendReturnedCount = Number(debugCounts?.apiQuery?.returnedItems ?? visibleCount);
  const latestCount = Number(debugCounts?.latest?.latestCount ?? 0);
  const latestStaleCount = Number(
    debugCounts?.latest?.staleCount ?? latestItems?.filter((item) => item?.isStale).length ?? 0,
  );
  const finalActiveCount = Number(debugCounts?.finalEventsV2?.activeCount ?? activeItems.length ?? 0);
  const finalClosedCount = Number(debugCounts?.finalEventsV2?.closedCount ?? closedItems.length ?? 0);
  const rawDbCount = Number(debugCounts?.db?.contractWhaleSignalsBtc24h ?? 0);
  const hiddenReasons = debugCounts?.visibility?.hiddenReasons || {};
  const dominantHiddenReason = hiddenCount > 0 ? summarizeHiddenReasons(hiddenReasons) : null;
  const showLatestHistoryDriftHint = latestCount > visibleCount;
  const showStaleLatestOnlyWarning = latestCount > 0 && latestStaleCount === latestCount && visibleCount === 0;
  const showRawFlowDiagnosis = showStaleLatestOnlyWarning && rawFlowDebug?.diagnosis?.primaryReason;
  const layeredLatestTs = Number(latestMaxTs ?? latestSignalTs) || null;
  const layeredHistoryTs = Number(contractEventsMaxEventTs ?? contractEventsLastEventTs) || null;
  const layeredHistoryLagSec = Number(contractEventsLatestLagSec ?? 0);
  const layeredHistoryAgeSec = Number(contractEventsHistoryLagSec ?? 0);
  const layeredFinalTs = Number(finalEventsMaxEventTs ?? 0) || null;
  const layeredProjectionLagSec = Number(finalEventsProjectionLagSec ?? 0);
  const canonicalTimeline =
    latencyDebug?.timeline || contractEventsTimeline || finalEventsTimeline || latestTimeline || null;
  const canonicalViews = canonicalTimeline?.views || {};
  const showHistorySyncWarning = layeredHistoryLagSec > 15;
  const showProjectionSyncWarning = layeredProjectionLagSec > 10;
  const latencySummary = {
    diagnosisLayer: latencyDebug?.diagnosis?.layer || "ok",
    diagnosisReason: latencyDebug?.diagnosis?.reason || "within_target",
    canonicalSource: canonicalTimeline?.source || "none",
    marketTimeTs: Number(canonicalTimeline?.eventTs ?? 0) || null,
    processedTs: Number(canonicalTimeline?.processedTs ?? 0) || null,
    persistedTs: Number(canonicalTimeline?.persistedTs ?? 0) || null,
    servedTs: Number(canonicalTimeline?.servedTs ?? latencyDebug?.serverTime ?? 0) || null,
    timelineLagSec: Number(canonicalTimeline?.timelineLagSec ?? 0),
    latestDriftSec: Number(canonicalViews?.latest?.driftVsCanonicalSec ?? 0),
    historyDriftSec: Number(canonicalViews?.history?.driftVsCanonicalSec ?? layeredHistoryLagSec ?? 0),
    finalDriftSec: Number(canonicalViews?.finalEventsV2?.driftVsCanonicalSec ?? layeredProjectionLagSec ?? 0),
    flowDriftSec: Number(canonicalViews?.flow?.driftVsCanonicalSec ?? 0),
    historyCacheAgeSec: latencyDebug?.contractEvents?.cacheAgeSec,
    finalCacheAgeSec: latencyDebug?.finalEventsV2?.cacheAgeSec,
  };
  return {
    activeItems,
    closedItems,
    visibleCount,
    hiddenCount,
    backendReturnedCount,
    latestCount,
    latestStaleCount,
    finalActiveCount,
    finalClosedCount,
    rawDbCount,
    hiddenReasons,
    dominantHiddenReason,
    showLatestHistoryDriftHint,
    showStaleLatestOnlyWarning,
    showRawFlowDiagnosis,
    layeredLatestTs,
    layeredHistoryTs,
    layeredHistoryLagSec,
    layeredHistoryAgeSec,
    layeredFinalTs,
    layeredProjectionLagSec,
    showHistorySyncWarning,
    showProjectionSyncWarning,
    latencySummary,
    retentionStatus,
    eventsSyncLag,
  };
}

function DataHealthBanner({ dataSlices }) {
  const affected = Object.entries(dataSlices || {}).filter(([, slice]) =>
    (slice?.state === "stale" || slice?.state === "unavailable")
      && !(slice?.state === "stale" && slice?.errorCode === "contract_projection_refresh_in_progress"),
  );
  if (affected.length === 0) return null;

  const hasStatusIssue = affected.some(([key]) => key === "status");
  const historicalUnavailable = dataSlices?.historical?.state === "unavailable";
  const affectedLabel = affected
    .map(([key, slice]) => `${DATA_SLICE_LABELS[key]}（${slice.state === "stale" ? "陈旧" : "不可用"}）`)
    .join("、");

  return (
    <aside
      className="contract-data-health"
      data-testid="data-health-banner"
      role="status"
    >
      <p className="font-semibold">部分数据正在自动恢复：{affectedLabel}</p>
      {hasStatusIssue ? <p className="mt-1">主力合约监控数据暂时不可用，已保留上一次结果。</p> : null}
      {historicalUnavailable ? <p className="mt-1">事件流暂时不可用，系统将在下一轮自动重试。</p> : null}
      <p className="mt-1 text-amber-200/80">页面会先做一次短间隔重试，再按原轮询节奏继续恢复，无需手动操作。</p>
    </aside>
  );
}

function HistoricalEventStreamPanel({
  contractEvents,
  visibleContractEvents,
  hiddenDisplayFilteredCount,
  debugCounts,
  rawFlowDebug,
  enabled,
  loading,
  error,
  onLoadMoreContractEvents,
  onOpenSignal,
  eventsSyncLag,
  latestSignalTs,
  contractEventsLastEventTs,
  latestMaxTs,
  contractEventsMaxEventTs,
  contractEventsLatestLagSec,
  contractEventsHistoryLagSec,
  contractEventsHasMore,
  displayFilterLabel,
  symbol,
}) {
  const historicalVolumeLabel = `窗口总流量 ${baseAssetSymbol(symbol)}`;
  const volumeTooltip =
    "总流量 = 主动买量 + 主动卖量；历史事件会跨已启用交易所聚合。ACTIVE/CLOSED 优先显示真实换手，原始 1s 数据不足时显示生命周期峰值窗口，不对重复窗口相加。";
  const diagnostics = deriveEventFeedDiagnostics({
    contractEvents,
    debugCounts,
    rawFlowDebug,
    latestItems: [],
    latestTimeline: null,
    finalEvents: { active: [], closed: [] },
    finalEventsTimeline: null,
    retentionStatus: null,
    eventsSyncLag,
    latestSignalTs,
    contractEventsLastEventTs,
    latestMaxTs,
    contractEventsMaxEventTs,
    contractEventsLatestLagSec,
    contractEventsHistoryLagSec,
    contractEventsTimeline: null,
    finalEventsMaxEventTs: null,
    finalEventsProjectionLagSec: null,
  });

  return (
    <section
      className={`contract-event-panel ${visibleContractEvents.length > 0 ? "min-h-[50vh]" : ""}`}
      data-testid="historical-events-primary"
      id="contract-whale-events"
    >
      <div className="contract-event-tape" data-testid="contract-event-tape">
      <div className="contract-event-header">
        <div className="min-w-0">
          <p className="contract-event-kicker">CONTRACT EVENT TAPE</p>
          <div className="mt-1 flex flex-wrap items-baseline gap-x-3 gap-y-1">
            <h4 className="text-sm font-semibold text-slate-100">合约市场事件</h4>
            <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-slate-600">HISTORICAL EVENTS (7d stream)</span>
          </div>
          <p className="sr-only">
            当前列表为历史事件流，不是 latest 快照。latest 只用于顶部实时状态；历史事件来自 contract_whale_signals，默认保留并查询最近 7 天完整数据，ACTIVE/CLOSED 是生命周期投影视图。
          </p>
        </div>
        <div className="contract-event-controls">
          <span className="contract-live-state text-emerald-300"><i className="bg-emerald-400" /> LIVE</span>
          <span>{contractEvents.length} EVENTS</span>
          <span>7D</span>
          <span>已加载 {contractEvents.length} 条</span>
        </div>
      </div>
      <div className="contract-event-body">
        <div className="contract-event-meta">
          <span>当前过滤：{displayFilterLabel}</span>
          <span>
            {hiddenDisplayFilteredCount > 0 ? ` 本页额外隐藏 ${hiddenDisplayFilteredCount} 条未达展示门槛事件。` : " 低于阈值的事件不会进入默认事件流。"}
          </span>
          {debugCounts && !debugCounts.error ? (
            <span>
              历史可见 {diagnostics.visibleCount} 条 / 后端返回 {diagnostics.backendReturnedCount} 条。
              {diagnostics.showLatestHistoryDriftHint ? " 当前页面已改为 canonical timeline，对比时请以 Market Time 为准。" : ""}
            </span>
          ) : null}
          {diagnostics.showStaleLatestOnlyWarning ? (
            <span className="contract-event-warning">
              {symbol} latest 为旧快照，最近 24h 没有新的 {symbol} 主力历史信号。
            </span>
          ) : null}
          {diagnostics.showRawFlowDiagnosis ? (
            <span className="contract-event-warning">
              上游诊断：{rawFlowDebug.diagnosis.primaryReason}
            </span>
          ) : null}
          {eventsSyncLag ? (
            <span className="contract-event-warning">
              数据延迟：latest 已更新到 {formatDateTime(latestSignalTs)}，历史事件流当前只同步到 {formatDateTime(contractEventsLastEventTs)}。
            </span>
          ) : null}
        </div>
        {loading ? (
          <p className="px-4 py-5 text-sm text-slate-400">主力合约监控载入中...</p>
        ) : error && contractEvents.length === 0 ? (
          <p className="px-4 py-5 text-sm text-slate-400">暂无可用的历史事件缓存。</p>
        ) : contractEvents.length === 0 ? (
          <p className="px-4 py-5 text-sm text-slate-400">{enabled ? "暂无主力合约异动" : "主力合约监控未启用"}</p>
        ) : visibleContractEvents.length === 0 ? (
          <p className="px-4 py-5 text-sm text-slate-400">
            当前历史事件已接入，但都低于 {displayFilterLabel} 展示阈值。
          </p>
        ) : (
          <div className="contract-event-table-shell" title={volumeTooltip}>
            <div className="max-h-[62vh] overflow-auto">
              <ContractEventTapeTable
                items={visibleContractEvents}
                onOpenSignal={onOpenSignal}
                testId="raw-contract-whale-signals"
                volumeLabel={historicalVolumeLabel}
                volumeTooltip={volumeTooltip}
              />
            </div>
            {contractEventsHasMore ? (
              <div className="border-t border-slate-800 px-3 py-2 text-right">
                <button
                  className="rounded-lg border border-cyan-500/30 px-3 py-1 text-xs font-semibold text-cyan-100 transition hover:bg-cyan-500/10"
                  onClick={onLoadMoreContractEvents}
                  type="button"
                >
                  加载更多历史事件
                </button>
              </div>
            ) : null}
          </div>
        )}
      </div>
      </div>
    </section>
  );
}

function LifecycleEventSections({
  finalEvents,
  finalEventsHasMore,
  onLoadMoreFinalEvents,
  onOpenSignal,
  symbol,
}) {
  const lifecycleVolumeLabel = `峰值窗口流量 ${baseAssetSymbol(symbol)}`;
  const volumeTooltip =
    "总流量 = 主动买量 + 主动卖量；ACTIVE/CLOSED 优先显示事件真实换手，raw 1s bucket 缺失时显示生命周期内的峰值窗口流量。";

  return (
    <section className="mt-4 space-y-4" id="contract-whale-lifecycle">
      <div className="rounded-xl border border-slate-800 bg-slate-950/35 px-4 py-3">
        <p className="console-label text-cyan-300">Lifecycle Event Views</p>
        <h4 className="mt-1 text-sm font-bold text-white">生命周期事件视图</h4>
        <p className="mt-1 text-xs leading-5 text-slate-400">
          ACTIVE / CLOSED 用来复盘同一事件在生命周期里的状态迁移，放在历史事件流之后查看更顺手。
        </p>
      </div>
      <EventLifecycleFeedGroup
        emptyText="暂无活跃合约事件"
        hasMore={finalEventsHasMore}
        items={finalEvents.active || []}
        onLoadMore={onLoadMoreFinalEvents}
        onOpenSignal={onOpenSignal}
        testId="raw-contract-whale-signals-active"
        title="ACTIVE EVENTS (updated)"
        volumeLabel={lifecycleVolumeLabel}
        volumeTooltip={volumeTooltip}
      />
      <EventLifecycleFeedGroup
        emptyText="暂无已结束合约事件"
        hasMore={finalEventsHasMore}
        items={finalEvents.closed || []}
        onLoadMore={onLoadMoreFinalEvents}
        onOpenSignal={onOpenSignal}
        testId="raw-contract-whale-signals-closed"
        title="CLOSED EVENTS (finalized)"
        volumeLabel={lifecycleVolumeLabel}
        volumeTooltip={volumeTooltip}
      />
    </section>
  );
}

function ContractWhaleSystemStatusPanel({
  contractEvents,
  debugCounts,
  rawFlowDebug,
  latencyDebug,
  enabled,
  latestItems,
  finalEvents,
  hiddenContractEvents,
  hiddenContractEventsExpanded,
  hiddenContractEventsLoading,
  onOpenSignal,
  onToggleHiddenContractEvents,
  retentionStatus,
  eventsSyncLag,
  latestSignalTs,
  latestTimeline,
  contractEventsLastEventTs,
  latestMaxTs,
  contractEventsMaxEventTs,
  contractEventsLatestLagSec,
  contractEventsHistoryLagSec,
  contractEventsTimeline,
  finalEventsMaxEventTs,
  finalEventsProjectionLagSec,
  finalEventsTimeline,
  symbol,
  summary,
  platformCapabilities,
  showCoinbaseSpotOnlyNotice,
}) {
  const [expanded, setExpanded] = useState(true);
  const diagnostics = deriveEventFeedDiagnostics({
    contractEvents,
    debugCounts,
    rawFlowDebug,
    latencyDebug,
    latestItems,
    latestTimeline,
    finalEvents,
    finalEventsTimeline,
    retentionStatus,
    eventsSyncLag,
    latestSignalTs,
    contractEventsLastEventTs,
    latestMaxTs,
    contractEventsMaxEventTs,
    contractEventsLatestLagSec,
    contractEventsHistoryLagSec,
    contractEventsTimeline,
    finalEventsMaxEventTs,
    finalEventsProjectionLagSec,
  });

  return (
    <section className="mt-4 rounded-2xl border border-slate-800 bg-slate-950/35" id="contract-whale-status">
      <button
        aria-expanded={expanded}
        className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left"
        onClick={() => setExpanded((previous) => !previous)}
        type="button"
      >
        <div>
          <p className="console-label text-fuchsia-200">System Status / Latency / Retention</p>
          <h4 className="mt-1 text-sm font-bold text-white">系统状态与诊断</h4>
        </div>
        <span className="rounded-full border border-slate-700 px-3 py-1 text-xs font-semibold text-slate-300">
          {expanded ? "收起" : "展开"}
        </span>
      </button>
      {expanded ? (
        <div className="space-y-4 border-t border-slate-800 p-3">
          <ContractWhaleTrendBar trend={summary.trend60s} symbol={symbol} />

          <p className="rounded-lg border border-slate-800/80 bg-slate-950/35 px-3 py-2 text-xs leading-5 text-slate-400">
            合约数据质量 {formatScore(summary.contractDataQuality)} · 现货数据质量 {formatScore(summary.spotDataQuality)} · 总体 {formatScore(summary.overallDataQuality)} · {summary.thresholdProfileReason}
          </p>

          <MarketStructureLitePanel summary={summary} />

          <PlatformCapabilitySection
            exchanges={summary.exchanges || {}}
            platforms={platformCapabilities}
            summary={summary}
          />

          {showCoinbaseSpotOnlyNotice ? (
            <p className="rounded-lg border border-cyan-500/30 bg-cyan-500/10 px-3 py-2 text-xs text-cyan-100">
              Coinbase 当前仅启用现货，未启用合约；本页只统计 perp 合约成交，因此不会返回 Coinbase 合约信号。
            </p>
          ) : null}

          <div className="rounded-xl border border-slate-800 bg-slate-950/40 p-3 text-xs leading-5 text-slate-400">
            <p>这里保留延迟、防抖和保留策略诊断，不再抢占首屏交易事件区域。</p>
            {latencyDebug && !latencyDebug.error ? (
              <LatencyGuardPanel summary={diagnostics.latencySummary} />
            ) : null}
            {(diagnostics.layeredLatestTs || diagnostics.layeredHistoryTs || diagnostics.layeredFinalTs) ? (
              <div className="mt-2 grid gap-2 md:grid-cols-3">
                <div className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2">
                  <p className="console-label">实时快照</p>
                  <p className="mt-1 text-xs text-slate-100">{formatDateTime(diagnostics.layeredLatestTs)}</p>
                </div>
                <div className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2">
                  <p className="console-label">历史事件流</p>
                  <p className="mt-1 text-xs text-slate-100">
                    {formatDateTime(diagnostics.layeredHistoryTs)}，落后 latest {diagnostics.layeredHistoryLagSec} 秒
                  </p>
                  <p className="mt-1 text-[11px] text-slate-400">历史最新事件距今 {diagnostics.layeredHistoryAgeSec} 秒</p>
                </div>
                <div className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2">
                  <p className="console-label">生命周期视图</p>
                  <p className="mt-1 text-xs text-slate-100">
                    {formatDateTime(diagnostics.layeredFinalTs)}，落后历史 {diagnostics.layeredProjectionLagSec} 秒
                  </p>
                </div>
              </div>
            ) : null}
            {debugCounts && !debugCounts.error ? (
              <div className="mt-2 space-y-2 rounded-lg border border-cyan-500/20 bg-cyan-500/5 px-3 py-2 text-cyan-100">
                <p>
                  24h {symbol} 历史事件：后端返回 {diagnostics.backendReturnedCount} 条，可见 {diagnostics.visibleCount} 条，隐藏 {diagnostics.hiddenCount} 条；latest 快照 {diagnostics.latestCount} 条。
                </p>
                <p className="text-[11px] text-cyan-200/90">
                  DB 原始 {diagnostics.rawDbCount} 条 · final-events active {diagnostics.finalActiveCount} 条 / closed {diagnostics.finalClosedCount} 条。
                  {diagnostics.dominantHiddenReason ? ` 隐藏主因：${diagnostics.dominantHiddenReason}。` : ""}
                </p>
                {diagnostics.showLatestHistoryDriftHint ? (
                  <p className="text-[11px] text-cyan-200/90">
                    latest 是实时快照，history 是持久化历史事件流；两者不是同一数据源，latest 里的信号可能尚未持久化、被过滤或被合并。
                  </p>
                ) : null}
                {diagnostics.showHistorySyncWarning ? (
                  <p className="text-[11px] text-amber-200/90">
                    历史事件流同步中：落后 latest {diagnostics.layeredHistoryLagSec} 秒，已自动触发刷新。
                  </p>
                ) : null}
                {diagnostics.showProjectionSyncWarning ? (
                  <p className="text-[11px] text-amber-200/90">
                    生命周期视图同步中：落后历史事件流 {diagnostics.layeredProjectionLagSec} 秒，不代表数据丢失。
                  </p>
                ) : null}
                {diagnostics.hiddenCount > 0 ? (
                  <div className="flex flex-wrap items-center gap-2">
                    <button
                      className="rounded-lg border border-cyan-500/30 px-3 py-1 text-xs font-semibold text-cyan-100 transition hover:bg-cyan-500/10"
                      onClick={onToggleHiddenContractEvents}
                      type="button"
                    >
                      {hiddenContractEventsExpanded ? "收起隐藏事件" : "查看隐藏事件"}
                    </button>
                    <span className="text-[11px] text-cyan-200/80">
                      价格偏离&gt;5% {Number(diagnostics.hiddenReasons.priceDeviationGt5pct ?? 0)} 条 · 坏质量 {Number(diagnostics.hiddenReasons.badQuality ?? 0)} 条
                    </span>
                  </div>
                ) : null}
              </div>
            ) : null}
            {retentionStatus ? (
              <p className="mt-2 text-cyan-100">
                retention: flow 保留 {retentionStatus.flowRetentionDays} 天 · signal 默认 {retentionStatus.signalRetentionDays} 天 · B 级 {retentionStatus.impactBRetentionDays ?? 90} 天 · A·S 永久{retentionStatus.signalProtectImpactAS === false ? "（未启用）" : ""} · |净量| &gt;= {retentionStatus.signalProtectNetVolumeBtc} BTC 永久保留
              </p>
            ) : (
              <p className="mt-2 text-slate-500">retention: 后台维护中；详细统计不在页面加载链路执行。</p>
            )}
          </div>

          {hiddenContractEventsExpanded ? (
            <HiddenContractEventsPanel
              items={hiddenContractEvents}
              loading={hiddenContractEventsLoading}
            />
          ) : null}
        </div>
      ) : null}
    </section>
  );
}

function ProDeskOverviewBar({
  contractEventsLastEventTs,
  intelligence,
  intelligenceSlice,
  latestSignalTs,
  previousIntelligence,
  summary,
}) {
  const regime = intelligence?.marketRegime || {};
  const riskContext = intelligence?.riskContext || {};
  const intelligenceState = intelligenceSlice?.state || "loading";
  const intelligenceFresh = intelligenceState === "fresh";
  const previousRegime = previousIntelligence?.marketRegime?.regime;
  const previousRisk = previousIntelligence?.riskContext?.fakeBreakoutRisk;
  const freshTs = Number(contractEventsLastEventTs ?? latestSignalTs) || null;
  const freshness = freshTs ? relativeAge(freshTs) : "暂无";
  const noTradeZones = Array.isArray(riskContext?.noTradeZones) ? riskContext.noTradeZones.length : 0;
  const intelligenceFreshnessLabel = intelligenceState === "stale"
    ? "STALE"
    : intelligenceState === "unavailable"
      ? "UNAVAILABLE"
      : intelligenceState === "fresh"
        ? "FRESH"
        : "LOADING";

  return (
    <section className="mt-4 rounded-2xl border border-cyan-500/20 bg-slate-950/35 p-4">
      <div className="flex flex-col gap-3 xl:flex-row xl:items-end xl:justify-between">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <p className="text-[11px] uppercase tracking-[0.2em] text-cyan-300">Pro Trading Desk Layout v2</p>
            <span
              className={`rounded-full border px-2 py-0.5 text-[10px] font-semibold ${intelligenceFresh ? "border-emerald-500/30 text-emerald-200" : "border-amber-500/30 text-amber-200"}`}
              data-testid="intelligence-freshness"
            >
              {intelligenceFreshnessLabel}
            </span>
          </div>
          <h4 className="mt-1 text-base font-bold text-white">事件驱动交易台总览</h4>
          <p className="mt-1 text-xs leading-5 text-slate-400">
            首屏先看市场发生了什么，再看结构、流动性、机会和风险，不让分析层抢走事件流的主视角。
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-5">
          <TradeSummaryPill
            label="Regime"
            testId="current-market-regime"
            tone={intelligenceFresh ? "cyan" : "slate"}
            value={intelligenceFresh ? regime.regime || "UNKNOWN" : "UNKNOWN"}
          />
          <TradeSummaryPill
            label="当前风险"
            testId="current-risk-state"
            tone={intelligenceFresh ? riskPillTone(riskContext.fakeBreakoutRisk) : "slate"}
            value={intelligenceFresh ? riskLabel(riskContext.fakeBreakoutRisk) : "UNKNOWN"}
          />
          <TradeSummaryPill label="历史新鲜度" tone="slate" value={freshness} />
          <TradeSummaryPill label="No-trade Zones" tone="yellow" value={`${noTradeZones}`} />
          <TradeSummaryPill label="Run Mode" tone={summary.enabled ? (summary.dryRun ? "yellow" : "cyan") : "slate"} value={modeLabel(summary)} />
        </div>
      </div>
      {!intelligenceFresh && previousRegime ? (
        <p
          className="mt-3 rounded-lg border border-slate-800 bg-slate-950/45 px-3 py-2 text-xs text-slate-400"
          data-testid="previous-intelligence-context"
        >
          上一版分析（仅供对照）：{previousRegime} / {previousRisk ? riskLabel(previousRisk) : "UNKNOWN"}
        </p>
      ) : null}
    </section>
  );
}

function MarketStructureDeskPanel({ intelligence, summary }) {
  const regime = intelligence?.marketRegime || {
    regime: "UNKNOWN",
    confidence: 0,
    reason: "当前分析数据不可用或仍在刷新，不沿用上一版结论作为当前判断。",
  };
  const rankedEvents = Array.isArray(intelligence?.rankedEvents) ? intelligence.rankedEvents : [];
  const opportunityMap = Array.isArray(intelligence?.opportunityMap) ? intelligence.opportunityMap : [];
  const compression = intelligence?.signalCompression || {};

  return (
    <section className="rounded-2xl border border-cyan-500/20 bg-slate-950/35 p-4" id="contract-whale-structure">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-cyan-300">Market Structure</p>
          <h4 className="mt-1 text-base font-bold text-white">结构分析</h4>
          <p className="mt-1 text-xs leading-5 text-slate-400">
            用来解释事件流背后的主导市场状态、方向偏置和当前最重要的结构机会。
          </p>
        </div>
        <span className="rounded-full border border-cyan-500/30 bg-cyan-500/10 px-2 py-1 text-xs font-semibold text-cyan-100">
          Regime {regime.confidence || 0}%
        </span>
      </div>

      <div className="mt-4 grid gap-3">
        <article className="rounded-xl border border-slate-800 bg-slate-950/55 p-4">
          <div className="flex items-start justify-between gap-3">
            <div>
              <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Market Regime</p>
              <h5 className="mt-1 text-lg font-bold text-white">{regime.regime}</h5>
            </div>
            <span className="rounded-full border border-slate-700 px-2 py-1 text-[11px] text-slate-200">
              Bias {biasText(summary?.marketStructureLite?.structureBias)}
            </span>
          </div>
          <p className="mt-3 text-sm leading-6 text-slate-300">{regime.reason}</p>
        </article>

        <article className="rounded-xl border border-slate-800 bg-slate-950/55 p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Signal Strength Ranking</p>
              <h5 className="mt-1 text-base font-bold text-white">强度排序</h5>
            </div>
            <span className="text-xs text-slate-500">{rankedEvents.length} ranked</span>
          </div>
          {rankedEvents.length === 0 ? (
            <p className="mt-3 rounded-xl border border-slate-800 bg-slate-950/50 px-3 py-3 text-sm text-slate-400">
              当前没有通过结构排序门槛的主导事件。
            </p>
          ) : (
            <div className="mt-3 space-y-3">
              {rankedEvents.slice(0, 3).map((event) => (
                <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-3" key={event.signalId}>
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Rank #{event.rank}</p>
                      <p className="mt-1 text-sm font-semibold text-white">{event.eventType}</p>
                    </div>
                    <div className="text-right text-xs text-slate-300">
                      <p>{event.strengthLabel}</p>
                      <p className="mt-1 text-cyan-200">{event.strengthScore}/100</p>
                    </div>
                  </div>
                  <p className="mt-2 text-xs leading-5 text-slate-400">{event.rationale}</p>
                </div>
              ))}
            </div>
          )}
        </article>

        <article className="rounded-xl border border-slate-800 bg-slate-950/55 p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Opportunity Map</p>
              <h5 className="mt-1 text-base font-bold text-white">机会分布</h5>
            </div>
            <span className="text-xs text-slate-500">{compression.qualityScore || 0}% quality</span>
          </div>
          {opportunityMap.length === 0 ? (
            <p className="mt-3 rounded-xl border border-slate-800 bg-slate-950/50 px-3 py-3 text-sm text-slate-400">
              当前没有明确的结构机会区域。
            </p>
          ) : (
            <div className="mt-3 grid gap-3">
              {opportunityMap.slice(0, 3).map((zone) => (
                <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-3" key={`${zone.zoneType}-${zone.rangeLabel}`}>
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <p className="text-sm font-semibold text-white">{zone.label}</p>
                      <p className="mt-1 text-xs text-cyan-200">{zone.rangeLabel}</p>
                    </div>
                    <span className="rounded-full border border-slate-700 px-2 py-1 text-[11px] text-slate-200">
                      {zone.strengthScore}/100
                    </span>
                  </div>
                  <p className="mt-2 text-xs leading-5 text-slate-400">{zone.description}</p>
                </div>
              ))}
            </div>
          )}
        </article>
      </div>
    </section>
  );
}

function LiquidityMapDeskPanel({ intelligence }) {
  const liquidityBehaviors = Array.isArray(intelligence?.liquidityBehaviors) ? intelligence.liquidityBehaviors : [];
  const opportunityMap = Array.isArray(intelligence?.opportunityMap) ? intelligence.opportunityMap : [];

  return (
    <section className="rounded-2xl border border-cyan-500/20 bg-slate-950/35 p-4" id="contract-whale-liquidity">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-cyan-300">Liquidity Map</p>
          <h4 className="mt-1 text-base font-bold text-white">流动性地图</h4>
          <p className="mt-1 text-xs leading-5 text-slate-400">
            把吸收、扫流动性、假突破和失衡簇从事件流里抽出来，单独作为交易员的结构观察区。
          </p>
        </div>
        <span className="rounded-full border border-slate-700 px-2 py-1 text-[11px] text-slate-200">
          {liquidityBehaviors.length} patterns
        </span>
      </div>

      <article className="mt-4 rounded-xl border border-slate-800 bg-slate-950/55 p-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Liquidity Behavior</p>
            <h5 className="mt-1 text-base font-bold text-white">Liquidity Behavior</h5>
          </div>
          <span className="text-xs text-slate-500">heatmap style</span>
        </div>
        {liquidityBehaviors.length === 0 ? (
          <p className="mt-3 rounded-xl border border-slate-800 bg-slate-950/50 px-3 py-3 text-sm text-slate-400">
            当前没有明确主导的流动性行为。
          </p>
        ) : (
          <div className="mt-3 grid gap-3">
            {liquidityBehaviors.map((behavior) => (
              <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-3" key={`${behavior.behavior}-${behavior.rangeLabel}`}>
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <p className="text-sm font-semibold text-white">{behavior.label}</p>
                    <p className="mt-1 text-xs text-cyan-200">{behavior.rangeLabel}</p>
                  </div>
                  <div className="text-right text-xs text-slate-300">
                    <p>{behavior.strengthScore}/100</p>
                    <p className="mt-1 text-slate-500">Conf {behavior.confidence}%</p>
                  </div>
                </div>
                <p className="mt-2 text-xs leading-5 text-slate-400">{behavior.reason}</p>
              </div>
            ))}
          </div>
        )}
      </article>

      <article className="mt-4 rounded-xl border border-slate-800 bg-slate-950/55 p-4">
        <div className="flex items-center justify-between gap-3">
          <div>
            <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Zone Overlay</p>
            <h5 className="mt-1 text-base font-bold text-white">关键区间覆盖</h5>
          </div>
          <span className="text-xs text-slate-500">{opportunityMap.length} zones</span>
        </div>
        <div className="mt-3 space-y-3">
          {opportunityMap.slice(0, 3).map((zone) => (
            <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-3" key={`${zone.zoneType}-${zone.rangeLabel}`}>
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-sm font-semibold text-white">{zone.label}</p>
                  <p className="mt-1 text-xs text-cyan-200">{zone.rangeLabel}</p>
                </div>
                <span className="rounded-full border border-slate-700 px-2 py-1 text-[11px] text-slate-200">
                  {zone.strengthScore}/100
                </span>
              </div>
              <p className="mt-2 text-xs leading-5 text-slate-400">{zone.description}</p>
            </div>
          ))}
        </div>
      </article>
    </section>
  );
}

function TradeSetupsDeskPanel({ intelligence, onSelectSignal, selectedSignalId, summary }) {
  const ideas = deriveDeskTradeIdeas(intelligence, summary);
  const regime = intelligence?.marketRegime?.regime || "RANGING";
  const dimForRegime = ["RANGING", "CHOP", "HIGH_VOLATILITY"].includes(regime);

  return (
    <section className="rounded-2xl border border-cyan-500/20 bg-slate-950/35 p-4" id="contract-whale-setups">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-cyan-300">Structure Setups</p>
          <h4 className="mt-1 text-base font-bold text-white">结构机会</h4>
          <p className="mt-1 text-xs leading-5 text-slate-400">
            只展示 Top 3 结构机会，和事件流分区显示；点击卡片会回到对应事件来源。
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-3">
          <TradeSummaryPill label="Top Structures" tone="emerald" value={`${ideas.length}`} />
          <TradeSummaryPill label="当前 Regime" tone="cyan" value={regime} />
          <TradeSummaryPill label="Desk Mode" tone={dimForRegime ? "yellow" : "cyan"} value={dimForRegime ? "Dimmed" : "Active"} />
        </div>
      </div>

      {dimForRegime ? (
        <p className="mt-3 rounded-xl border border-amber-500/25 bg-amber-500/10 px-3 py-2 text-xs text-amber-100">
          当前处于 {regime}，结构机会已自动降亮处理，优先把它当结构参考。
        </p>
      ) : null}

      {ideas.length === 0 ? (
        <p className="mt-4 rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-4 text-sm text-slate-400">
          当前没有通过 desk 压缩门槛的 setup。
        </p>
      ) : (
        <div className="mt-4 grid gap-3 xl:grid-cols-3">
          {ideas.map((idea) => {
            const selected = selectedSignalId === idea.signalId;
            return (
              <button
                className={`rounded-xl border p-4 text-left outline-none transition focus-visible:ring-2 focus-visible:ring-cyan-500/35 ${
                  selected
                    ? "border-cyan-400/70 bg-cyan-500/10 shadow-glow"
                    : `border-slate-800 bg-slate-950/50 hover:border-cyan-500/40 ${dimForRegime ? "opacity-70" : ""}`
                }`}
                key={idea.signalId}
                onClick={() => onSelectSignal(idea.signalId)}
                type="button"
              >
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Rank #{idea.rank}</p>
                    <h5 className="mt-1 text-base font-bold text-white">{idea.setupType}</h5>
                  </div>
                  <div className="text-right">
                    <span className={`rounded-full px-2 py-1 text-xs font-bold ${tradeActionClass(idea.actionTone)}`}>
                      {idea.directionLabel}
                    </span>
                    <p className="mt-2 text-[11px] font-semibold text-cyan-100">{idea.confidenceText}</p>
                  </div>
                </div>
                <div className="mt-3 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
                  <TradeMetric label="Score" value={`${idea.score}/100`} />
                  <TradeMetric label="Confidence" value={`${idea.confidence}%`} />
                  <TradeMetric label="Reason" value={idea.reasonTag} />
                  <TradeMetric label="Window" value={`${idea.windowSec}s`} />
                </div>
                {idea.pressureZoneLabel ? (
                  <p className="mt-3 text-xs text-cyan-200">压力区 {idea.pressureZoneLabel}</p>
                ) : null}
                <p className="mt-3 text-sm leading-6 text-slate-300">{idea.reason}</p>
                {idea.riskBoundaryReason ? (
                  <p className="mt-2 text-xs leading-5 text-slate-400">风险边界：{idea.riskBoundaryReason}</p>
                ) : null}
                <p className="mt-2 text-xs text-slate-500">点击后将回溯到来源 event，并高亮对应信号。</p>
              </button>
            );
          })}
        </div>
      )}
    </section>
  );
}

function RiskContextDeskPanel({ intelligence, summary }) {
  const riskContext = intelligence?.riskContext || {};
  const noTradeZones = Array.isArray(riskContext?.noTradeZones) ? riskContext.noTradeZones : [];

  return (
    <section className="rounded-2xl border border-cyan-500/20 bg-slate-950/35 p-4" id="contract-whale-risk">
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-cyan-300">Risk Context</p>
          <h4 className="mt-1 text-base font-bold text-white">风险语境</h4>
          <p className="mt-1 text-xs leading-5 text-slate-400">
            风险常驻可见，避免因为首屏事件太强把 no-trade 区和假突破风险忽略掉。
          </p>
        </div>
        <span className={`rounded-full px-2 py-1 text-xs font-bold ${riskBadgeClass(riskContext.fakeBreakoutRisk)}`}>
          {riskLabel(riskContext.fakeBreakoutRisk)}
        </span>
      </div>

      <div className="mt-4 grid gap-3">
        <article className="rounded-xl border border-slate-800 bg-slate-950/55 p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">当前风险</p>
              <h5 className="mt-1 text-base font-bold text-white">{riskLabel(riskContext.fakeBreakoutRisk)}</h5>
            </div>
            <span className="rounded-full border border-slate-700 px-2 py-1 text-[11px] text-slate-200">
              {noTradeZones.length} no-trade
            </span>
          </div>
          <p className="mt-3 text-sm leading-6 text-slate-300">
            {riskContext.summary || "当前没有显著 no-trade 风险。"}
          </p>
        </article>

        <article className="rounded-xl border border-slate-800 bg-slate-950/55 p-4">
          <div className="flex items-center justify-between gap-3">
            <div>
              <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">No-Trade Zones</p>
              <h5 className="mt-1 text-base font-bold text-white">风险区间</h5>
            </div>
            <span className="text-xs text-slate-500">{summary?.healthStatus || "healthy"}</span>
          </div>
          {noTradeZones.length === 0 ? (
            <p className="mt-3 rounded-xl border border-slate-800 bg-slate-950/50 px-3 py-3 text-sm text-slate-400">
              当前没有明确的禁做区间。
            </p>
          ) : (
            <div className="mt-3 space-y-3">
              {noTradeZones.map((zone, index) => (
                <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-3" key={`${zone.rangeLabel}-${index}`}>
                  <p className="text-sm font-semibold text-white">{zone.rangeLabel || "N/A"}</p>
                  <p className="mt-2 text-xs leading-5 text-slate-400">{zone.reason || "暂无说明"}</p>
                </div>
              ))}
            </div>
          )}
        </article>
      </div>
    </section>
  );
}

function LatencyGuardPanel({ summary }) {
  return (
    <div className="mt-2 rounded-lg border border-fuchsia-500/20 bg-fuchsia-500/5 px-3 py-3 text-fuchsia-100">
      <div className="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
        <div>
          <p className="console-label text-fuchsia-200">LATENCY GUARD</p>
          <p className="mt-1 text-xs text-fuchsia-100/90">
            当前 canonical timeline：{summary.canonicalSource} · 瓶颈层：{summary.diagnosisLayer} · {summary.diagnosisReason}
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 md:grid-cols-4">
          <MiniInfoCard
            label="Market Time"
            value={summary.marketTimeTs ? formatDateTime(summary.marketTimeTs) : "N/A"}
            detail="canonical event_ts"
          />
          <MiniInfoCard
            label="System Lag"
            value={`lag ${formatLatencySeconds(summary.timelineLagSec)}`}
            detail={summary.persistedTs ? `persisted ${formatDateTime(summary.persistedTs)}` : "served_ts - event_ts"}
          />
          <MiniInfoCard
            label="Latest Drift"
            value={`latest ${formatLatencySeconds(summary.latestDriftSec)}`}
            detail="vs canonical"
          />
          <MiniInfoCard
            label="History Drift"
            value={`history ${formatLatencySeconds(summary.historyDriftSec)}`}
            detail={summary.historyCacheAgeSec != null ? `cache ${formatLatencySeconds(summary.historyCacheAgeSec)}` : "vs canonical"}
          />
          <MiniInfoCard
            label="Final Drift"
            value={`final ${formatLatencySeconds(summary.finalDriftSec)}`}
            detail={summary.finalCacheAgeSec != null ? `cache ${formatLatencySeconds(summary.finalCacheAgeSec)}` : "vs canonical"}
          />
          <MiniInfoCard
            label="Flow Drift"
            value={`flow ${formatLatencySeconds(summary.flowDriftSec)}`}
            detail="vs canonical"
          />
        </div>
      </div>
    </div>
  );
}

function summarizeHiddenReasons(hiddenReasons) {
  const buckets = [
    ["priceDeviationGt5pct", "价格偏离 > 5%"],
    ["badQuality", "质量过滤"],
    ["missingPrice", "缺少价格"],
    ["disabledMonitor", "监控关闭"],
    ["unknown", "其他"],
  ]
    .map(([key, label]) => ({ label, count: Number(hiddenReasons?.[key] ?? 0) }))
    .filter((item) => item.count > 0)
    .sort((left, right) => right.count - left.count);
  return buckets[0] ? `${buckets[0].label} ${buckets[0].count} 条` : null;
}

function HiddenContractEventsPanel({ items, loading }) {
  return (
    <div className="rounded-xl border border-amber-500/20 bg-slate-950/40">
      <div className="flex items-center justify-between border-b border-slate-800 px-3 py-2">
        <p className="text-xs font-bold tracking-[0.18em] text-amber-200">隐藏事件</p>
        <span className="rounded-full border border-slate-700 px-2 py-0.5 text-[11px] text-slate-300">
          已加载 {items.length} 条
        </span>
      </div>
      {loading ? (
        <p className="px-3 py-4 text-xs text-slate-500">隐藏事件加载中...</p>
      ) : items.length === 0 ? (
        <p className="px-3 py-4 text-xs text-slate-500">当前没有可展开的隐藏事件。</p>
      ) : (
        <div className="overflow-x-auto">
          <table className="min-w-full table-fixed text-left text-xs">
            <thead className="bg-slate-950/80 text-slate-400">
              <tr>
                <HeaderCell>隐藏原因</HeaderCell>
                <HeaderCell>时间</HeaderCell>
                <HeaderCell>价格</HeaderCell>
                <HeaderCell>偏离比例</HeaderCell>
                <HeaderCell title={CONTRACT_CLASSIFICATION_TOOLTIP}>类型</HeaderCell>
                <HeaderCell>等级</HeaderCell>
                <HeaderCell>说明</HeaderCell>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800 text-slate-300">
              {items.map((item) => (
                <tr key={item.eventId || item.id}>
                  <Cell>{item.hiddenReason || "unknown"}</Cell>
                  <Cell>
                    <span className="block whitespace-nowrap text-slate-200">{formatDate(item.ts)}</span>
                    <span className="block whitespace-nowrap text-slate-500">{formatTime(item.ts)}</span>
                  </Cell>
                  <Cell>{formatPrice(signalTriggerPrice(item))}</Cell>
                  <Cell>{formatDeviation(item.priceDeviationPct)}</Cell>
                  <Cell>
                    <SignalTypeSummary item={item} />
                  </Cell>
                  <Cell>{severityLabel(item.severity)}</Cell>
                  <Cell>{item.hiddenDetail || "后端标记为隐藏事件"}</Cell>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function EventLifecycleFeedGroup({ emptyText, hasMore, items, onLoadMore, onOpenSignal, testId, title, volumeLabel, volumeTooltip }) {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/40">
      <div className="flex items-center justify-between border-b border-slate-800 px-3 py-2">
        <p className="text-xs font-bold tracking-[0.18em] text-cyan-200">{title}</p>
        <span className="rounded-full border border-slate-700 px-2 py-0.5 text-[11px] text-slate-300">
          已加载 {items.length} 条
        </span>
      </div>
      <p className="border-b border-slate-800 px-3 py-2 text-[11px] leading-5 text-slate-500" title={volumeTooltip}>
        口径：{volumeLabel}。{volumeTooltip}
      </p>
      {items.length === 0 ? (
        <p className="px-3 py-4 text-xs text-slate-500">{emptyText}</p>
      ) : (
        <>
          <div className="overflow-x-auto">
            <RawSignalDebugTable
              items={items}
              onOpenSignal={onOpenSignal}
              testId={testId}
              volumeLabel={volumeLabel}
              volumeTooltip={volumeTooltip}
            />
          </div>
          {hasMore ? (
            <div className="border-t border-slate-800 px-3 py-2 text-right">
              <button
                className="rounded-lg border border-cyan-500/30 px-3 py-1 text-xs font-semibold text-cyan-100 transition hover:bg-cyan-500/10"
                onClick={onLoadMore}
                type="button"
              >
                加载更多
              </button>
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}

const ContractEventTapeTable = memo(function ContractEventTapeTable({
  items,
  onOpenSignal,
  testId = "raw-contract-whale-signals",
  volumeLabel = "总流量",
  volumeTooltip,
}) {
  return (
    <table className="contract-event-table" data-testid={testId}>
      <thead>
        <tr>
          <TapeHeaderCell>时间</TapeHeaderCell>
          <TapeHeaderCell>市场 / 事件</TapeHeaderCell>
          <TapeHeaderCell>方向</TapeHeaderCell>
          <TapeHeaderCell>等级</TapeHeaderCell>
          <TapeHeaderCell title={volumeTooltip}>{volumeLabel}</TapeHeaderCell>
          <TapeHeaderCell>净流量</TapeHeaderCell>
          <TapeHeaderCell>名义价值</TapeHeaderCell>
          <TapeHeaderCell>价格 Δ</TapeHeaderCell>
          <TapeHeaderCell title={OI_CONTEXT_TOOLTIP}>OI 背景</TapeHeaderCell>
          <TapeHeaderCell>来源 / Discord</TapeHeaderCell>
        </tr>
      </thead>
      <tbody>
        {items.map((item, index) => {
          const netVolume = finiteNumber(item.netVolumeBtc, 0);
          const priceMove = numberOrNull(item.priceMovePct);
          const rowTone = netVolume > 0 ? "contract-event-row-buy" : netVolume < 0 ? "contract-event-row-sell" : "contract-event-row-neutral";
          return (
            <tr
              className={`${rowTone} ${index === 0 ? "contract-event-row-latest" : ""}`}
              data-testid={`contract-whale-row-${item.eventId || item.finalEventId || item.id}`}
              key={item.eventId || item.finalEventId || item.id}
              onClick={() => onOpenSignal(signalDetailTargetId(item))}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onOpenSignal(signalDetailTargetId(item));
                }
              }}
              tabIndex={0}
            >
              <TapeCell>
                <span className="block font-mono text-[11px] text-slate-200">{formatTime(item.ts)}</span>
                <span className="mt-1 block text-[10px] text-slate-600">{formatDate(item.ts)}</span>
                <span className="mt-1 block text-[10px] text-slate-500">{relativeAge(item.ts)}</span>
              </TapeCell>
              <TapeCell>
                <span className="mb-1 flex items-center gap-1.5 font-mono text-[11px] font-semibold text-slate-100">
                  <span>{item.symbol}</span>
                  <span className="text-slate-600">·</span>
                  <span className="text-cyan-200">{formatUsd(item.totalNotionalUsd ?? item.notionalUsd ?? item.notional)}</span>
                </span>
                <SignalTypeSummary item={item} />
                <span className="mt-1 block font-mono text-[11px] text-slate-300">{formatPrice(signalTriggerPrice(item))}</span>
              </TapeCell>
              <TapeCell>
                <span className={`contract-direction-mark ${signedMetricClass(netVolume)}`}>
                  {netVolume > 0 ? "买入 →" : netVolume < 0 ? "卖出 →" : "中性 —"}
                </span>
                <span className="mt-1 block text-[10px] text-slate-500">{netDirection(item.netVolumeBtc, item.symbol)}</span>
              </TapeCell>
              <TapeCell>
                {impactNormalizationBadge(item)}
                <span className="mt-1 block text-[10px]">{eventQualityBadge(item)}</span>
              </TapeCell>
              <TapeCell>
                <LifecycleVolumeCell item={item} volumeLabel={volumeLabel} />
              </TapeCell>
              <TapeCell>
                <span className={`font-mono font-semibold ${signedMetricClass(netVolume)}`}>
                  {formatSignedBaseVolume(netVolume, item.symbol)}
                </span>
                <span className="mt-1 block text-[10px] text-slate-500">{formatPct(finiteNumber(item.dominance, 0) * 100)} dominance</span>
              </TapeCell>
              <TapeCell>
                <span className="font-mono font-semibold text-slate-200">
                  TOTAL {formatUsd(item.totalNotionalUsd ?? item.notionalUsd ?? item.notional)}
                </span>
                <span className="mt-1 block text-[10px] text-slate-500">{item.windowSec || "—"}s window</span>
              </TapeCell>
              <TapeCell>
                <EventImpactTrace value={priceMove} />
                <span className={`mt-1 block font-mono text-[11px] ${priceMove === null ? "text-slate-500" : signedMetricClass(priceMove)}`}>
                  {priceMove === null ? "N/A" : formatSignedPct(priceMove)}
                </span>
                <span className="mt-1 block text-[10px] text-slate-500">{liquidationStatus(item)}</span>
              </TapeCell>
              <TapeCell>
                <LifecycleOiCell item={item} />
                <span className="mt-1 block text-[10px] text-slate-500">{fundingStatus(item)}</span>
                {oiEvidenceSummary(item) ? <span className="mt-1 block text-[10px] text-amber-300">{oiEvidenceSummary(item)}</span> : null}
              </TapeCell>
              <TapeCell>
                <span className="block text-slate-200">{item.mainExchange || item.source || "N/A"}</span>
                <span className="mt-1 block text-[10px] text-slate-500">{discordStatus(item)}</span>
                <button
                  aria-label={`查看主力合约信号详情 ${signalDetailTargetId(item)} ${testId}`}
                  className="contract-detail-link"
                  onClick={(event) => {
                    event.stopPropagation();
                    onOpenSignal(signalDetailTargetId(item));
                  }}
                  type="button"
                >
                  详情 ↗
                </button>
              </TapeCell>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
});

function TapeHeaderCell({ children, title }) {
  return <th title={title}>{children}</th>;
}

function TapeCell({ children }) {
  return <td>{children}</td>;
}

function EventImpactTrace({ value }) {
  const numeric = finiteNumber(value, 0);
  const magnitude = Math.min(1, Math.abs(numeric) / 1.5);
  const rising = numeric >= 0;
  const samples = [0.26, 0.48, 0.36, 0.62, 0.52, 0.84, 0.68, 0.94, 0.74, 1];
  return (
    <span className={`contract-impact-trace ${rising ? "contract-impact-trace-up" : "contract-impact-trace-down"}`} aria-hidden="true">
      {samples.map((sample, index) => {
        const normalized = rising ? sample : samples[samples.length - index - 1];
        return <i key={index} style={{ height: `${Math.max(18, normalized * (40 + magnitude * 55))}%` }} />;
      })}
    </span>
  );
}

const RawSignalDebugTable = memo(function RawSignalDebugTable({ items, onOpenSignal, testId = "raw-contract-whale-signals", volumeLabel = "总流量", volumeTooltip }) {
  return (
    <table className="min-w-full table-fixed text-left text-xs" data-testid={testId}>
      <thead className="bg-slate-950/80 text-slate-400">
        <tr>
          <HeaderCell>时间</HeaderCell>
          <HeaderCell>币种 / 名义金额 / 价格</HeaderCell>
          <HeaderCell title={CONTRACT_CLASSIFICATION_TOOLTIP}>类型</HeaderCell>
          <HeaderCell>等级</HeaderCell>
          <HeaderCell>事件窗口</HeaderCell>
          <HeaderCell>质量</HeaderCell>
          <HeaderCell>市场冲击等级</HeaderCell>
          <HeaderCell title={volumeTooltip}>{volumeLabel}</HeaderCell>
          <HeaderCell>价格</HeaderCell>
          <HeaderCell>价格偏离</HeaderCell>
          <HeaderCell title="主力行为判断与市场冲击等级分离">主力行为</HeaderCell>
          <HeaderCell>轨迹</HeaderCell>
          <HeaderCell>现货 / 合约</HeaderCell>
          <HeaderCell>净方向</HeaderCell>
          <HeaderCell>方向占比</HeaderCell>
          <HeaderCell>异常倍数</HeaderCell>
          <HeaderCell>历史分位</HeaderCell>
          <HeaderCell>主导平台</HeaderCell>
          <HeaderCell>价格变化</HeaderCell>
          <HeaderCell>市场驱动</HeaderCell>
          <HeaderCell>清算</HeaderCell>
          <HeaderCell>驱动力</HeaderCell>
          <HeaderCell>OI</HeaderCell>
          <HeaderCell>资金费率</HeaderCell>
          <HeaderCell>Discord</HeaderCell>
          <HeaderCell>详情</HeaderCell>
        </tr>
      </thead>
      <tbody className="divide-y divide-slate-800 text-slate-300">
        {items.map((item) => (
          <tr
            className="console-row"
            data-testid={`contract-whale-row-${item.eventId || item.finalEventId || item.id}`}
            key={item.eventId || item.finalEventId || item.id}
            onClick={() => onOpenSignal(signalDetailTargetId(item))}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onOpenSignal(signalDetailTargetId(item));
              }
            }}
            tabIndex={0}
          >
            <Cell>
              <span className="block whitespace-nowrap text-slate-200">{formatDate(item.ts)}</span>
              <span className="block whitespace-nowrap text-slate-500">{formatTime(item.ts)}</span>
            </Cell>
            <Cell>
              <SymbolWithPrice item={item} />
            </Cell>
            <Cell>
              <SignalTypeSummary item={item} />
            </Cell>
            <Cell>
              <span className="block whitespace-nowrap">
                <span className={`rounded-full px-2 py-1 font-bold ${severityBadgeClass(item.severity)}`}>
                  {severityLabel(item.severity)}
                </span>
                <span
                  className={`mt-1 block text-[10px] font-bold ${signalLevelClass(resolveImpactDisplay(item).signalLevel)}`}
                  title={`${resolveImpactDisplay(item).signalLabel} · ${impactMetricSummary(resolveImpactDisplay(item))}`}
                >
                  {resolveImpactDisplay(item).signalLevel} / {resolveImpactDisplay(item).impactLevel}
                </span>
              </span>
            </Cell>
            <Cell>
              <span className="block whitespace-nowrap">{item.windowSec}s</span>
              {item.mergedFrom?.length ? (
                <span className="block whitespace-nowrap text-[10px] uppercase tracking-wide text-cyan-300">
                  {mergedWindowLabel(item)}
                </span>
              ) : null}
            </Cell>
            <Cell>{eventQualityBadge(item)}</Cell>
            <Cell>{impactNormalizationBadge(item)}</Cell>
            <Cell>
              <LifecycleVolumeCell item={item} volumeLabel={volumeLabel} />
            </Cell>
            <Cell>{formatPrice(signalTriggerPrice(item))}</Cell>
            <Cell>{formatDeviation(item.priceDeviationPct)}</Cell>
            <Cell><BehaviorAssessmentCell item={item} /></Cell>
            <Cell>{clusterTableLabel(item)}</Cell>
            <Cell>{formatScorePair(item.spotScore, item.contractScore)}</Cell>
            <Cell>{netDirection(item.netVolumeBtc, item.symbol)}</Cell>
            <Cell>{formatPct(item.dominance * 100)}</Cell>
            <Cell>{formatMultiple(item.dynamicMultiple)}</Cell>
            <Cell>{formatPercentile(item.percentileLevel)}</Cell>
            <Cell>{item.mainExchange}</Cell>
            <Cell>{formatSignedPct(item.priceMovePct)}</Cell>
            <Cell>{marketDriverLabel(item.marketDriver?.primaryDriver)}</Cell>
            <Cell>{liquidationStatus(item)}</Cell>
            <Cell>{liquidationDriverLabel(item.liquidationForce?.primaryDriver)}</Cell>
            <Cell>
              <LifecycleOiCell item={item} />
              {oiEvidenceSummary(item) ? (
                <span className="mt-1 block text-[10px] text-amber-300" title={oiEvidenceSummary(item)}>
                  {oiEvidenceSummary(item)}
                </span>
              ) : null}
            </Cell>
            <Cell>{fundingStatus(item)}</Cell>
            <Cell>{discordStatus(item)}</Cell>
            <Cell>
              <button
                aria-label={`查看主力合约信号详情 ${signalDetailTargetId(item)} ${testId}`}
                className="rounded-lg border border-cyan-500/40 px-2 py-1 text-cyan-100 outline-none transition hover:border-cyan-300 hover:bg-cyan-500/10 focus-visible:ring-2 focus-visible:ring-cyan-500/35"
                onClick={(event) => {
                  event.stopPropagation();
                  onOpenSignal(signalDetailTargetId(item));
                }}
                type="button"
              >
                详情
              </button>
            </Cell>
          </tr>
        ))}
      </tbody>
    </table>
  );
});

function reuseEventList(previous = [], next = []) {
  if (previous === next) return previous;
  if (previous.length !== next.length) return next;
  for (let index = 0; index < previous.length; index += 1) {
    if (eventRevisionKey(previous[index]) !== eventRevisionKey(next[index])) return next;
  }
  return previous;
}

function eventRevisionKey(item = {}) {
  return JSON.stringify(item);
}

function LifecycleVolumeCell({ item, volumeLabel }) {
  const lifecycle = item?.eventLifecycle || {};
  const hasLifecycleStats =
    lifecycle.latestWindowVolumeBtc !== null && lifecycle.latestWindowVolumeBtc !== undefined
    || lifecycle.peakWindowVolumeBtc !== null && lifecycle.peakWindowVolumeBtc !== undefined
    || lifecycle.uniqueTurnoverBtc !== null && lifecycle.uniqueTurnoverBtc !== undefined;
  if (!hasLifecycleStats) {
    return formatOptionalBaseVolume(item.displayVolumeBtc ?? item.totalVolumeBtc, item.symbol);
  }
  return (
    <span className="block min-w-[150px]" title={`${volumeLabel} · 生命周期窗口统计`}>
      <span className="block text-slate-200">
        最新 {formatOptionalBaseVolume(lifecycle.latestWindowVolumeBtc, item.symbol)}
      </span>
      <span className="block text-slate-400">
        峰值 {formatOptionalBaseVolume(lifecycle.peakWindowVolumeBtc, item.symbol)}
      </span>
      <span className="block text-cyan-300">
        {lifecycle.uniqueTurnoverAvailable && lifecycle.uniqueTurnoverBtc !== null
          ? `真实换手 ${formatOptionalBaseVolume(lifecycle.uniqueTurnoverBtc, item.symbol)}`
          : "真实换手 N/A · 峰值窗口为回退"}
      </span>
    </span>
  );
}

function LifecycleOiCell({ item }) {
  const lifecycle = item?.eventLifecycle || {};
  const hasLifecycleOi =
    lifecycle.netOiDeltaBtc !== null && lifecycle.netOiDeltaBtc !== undefined
    || lifecycle.peakAbsOiDeltaBtc !== null && lifecycle.peakAbsOiDeltaBtc !== undefined;
  if (!hasLifecycleOi) return <span className="block">{oiStatus(item)}</span>;
  return (
    <span className="block min-w-[140px]">
      <span className="block">{oiStatus(item)}</span>
      <span className="block text-[10px] text-slate-400">
        净 OI {formatSignedBaseVolume(lifecycle.netOiDeltaBtc, item.symbol)}
      </span>
      <span className="block text-[10px] text-cyan-300">
        峰值 OI {formatSignedBaseVolume(lifecycle.peakAbsOiDeltaBtc, item.symbol)}
      </span>
    </span>
  );
}

function signalDetailTargetId(item) {
  return item?.sourceSignalId || item?.id;
}

function hasRichContractEventDetail(item) {
  return Boolean(
    item?.scoreBreakdown
    || item?.trajectory
    || item?.activeSources
    || item?.sourceSignal?.id
    || item?.exchanges?.length,
  );
}

function matchesSignalIdentity(item, signalId) {
  if (!item || !signalId) return false;
  const target = String(signalId);
  return [item.id, item.sourceSignalId, item.eventId, item.finalEventId]
    .filter(Boolean)
    .some((value) => String(value) === target);
}

function MainForceEventsSection({ events, symbol }) {
  return (
    <section className="mt-5">
      <div className="mb-3 flex items-end justify-between gap-3">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-slate-500">Main Force Events</p>
          <h4 className="mt-1 text-sm font-bold text-white">主力结构事件历史</h4>
        </div>
        <p className="text-xs text-slate-500">让你知道这里发生过什么主力行为</p>
      </div>
      {events.length === 0 ? (
        <p className="rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-4 text-sm text-slate-400">
          暂无 {symbol} 主力结构事件
        </p>
      ) : (
        <div className="grid gap-3 xl:grid-cols-2">
          {events.map((event) => (
            <article
              className="rounded-xl border border-slate-800 bg-slate-950/50 p-4"
              data-testid={`main-force-event-${event.id}`}
              key={event.id}
            >
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <p className="text-sm font-bold text-slate-100">
                    {regimeTypeLabel(event.regimeType)}
                  </p>
                  <p className="mt-1 text-xs text-slate-500">
                    {formatEventRange(event.startedAt, event.endedAt)}
                  </p>
                </div>
                <span className={`rounded-full px-2 py-1 text-xs font-bold ${marketSeverityBadgeClass(event.severity)}`}>
                  {event.severity}
                </span>
              </div>
              <div className="mt-3 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
                <EventMetric label="峰值主力评分" value={`${Math.round(event.peakMainForceScore)}`} />
                <EventMetric label="峰值极端冲击" value={`${Math.round(event.peakExtremeImpactScore)}`} />
                <EventMetric label="峰值结构方向" value={`${biasText(event.peakStructureBias)}`} />
                <EventMetric label="置信度" value={`${Math.round(event.confidence)}`} />
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {event.mainForceConfirmed ? <EventTag label="主力确认" tone="emerald" /> : null}
                {event.extremeImpactConfirmed ? <EventTag label="极端冲击" tone="amber" /> : null}
                <EventTag label={event.liquidationDriven ? "清算驱动" : "非清算驱动"} tone={event.liquidationDriven ? "red" : "cyan"} />
                {event.endedAt ? <EventTag label="已结束" tone="slate" /> : <EventTag label="进行中" tone="emerald" />}
              </div>
              <p className="mt-3 text-sm text-slate-300">
                {event.reasons.finalResult || event.reasons.coreReason || "主力结构事件已记录，可用于后续复盘。"}
              </p>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function TradeOpportunitiesPanel({ summary }) {
  const opportunities = Array.isArray(summary?.tradeOpportunities) ? summary.tradeOpportunities : [];
  const suppression = summary?.noiseSuppression || {};
  return (
    <section className="mt-4 rounded-2xl border border-cyan-500/20 bg-slate-950/35 p-4">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-cyan-300">Structure Opportunities</p>
          <h4 className="mt-1 text-sm font-bold text-white">结构机会排序</h4>
          <p className="mt-1 text-xs leading-5 text-slate-400">
            先把重复窗口与生命周期噪声压平，再给出当前最值得盯的主力合约结构。
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-5">
          <TradeSummaryPill label="原始候选" value={`${suppression.rawCandidates || 0}`} />
          <TradeSummaryPill label="合并后" value={`${suppression.mergedEvents || 0}`} />
          <TradeSummaryPill label="降噪后事件" value={`${suppression.filteredEvents || 0}`} />
          <TradeSummaryPill label="结构候选" value={`${suppression.tradeableSetups || 0}`} tone="emerald" />
          <TradeSummaryPill label="降噪比例" value={`${suppression.noiseReductionPct || 0}%`} tone="cyan" />
        </div>
      </div>

      {opportunities.length === 0 ? (
        <p className="mt-4 rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-4 text-sm text-slate-400">
          当前没有通过排序门槛的结构机会，系统保留结构观察。
        </p>
      ) : (
        <div className="mt-4 grid gap-3 xl:grid-cols-3">
          {opportunities.map((opportunity) => (
            <article
              className="rounded-xl border border-slate-800 bg-slate-950/60 p-4"
              data-testid={`trade-opportunity-${opportunity.signalId}`}
              key={opportunity.signalId}
            >
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-xs uppercase tracking-[0.18em] text-slate-500">Rank #{opportunity.rank}</p>
                  <h5 className="mt-1 text-base font-bold text-white">{opportunity.setupType}</h5>
                </div>
                <span className={`rounded-full px-2 py-1 text-xs font-bold ${tradeActionClass(opportunity.action)}`}>
                  {opportunity.action}
                </span>
              </div>
              <div className="mt-3 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
                <TradeMetric label="结构评分" value={`${opportunity.tradeScore}/100`} />
                <TradeMetric label="置信度" value={`${opportunity.confidence}%`} />
                <TradeMetric label="方向偏置" value={directionLabel(opportunity.directionBias)} />
                <TradeMetric label="事件窗口" value={`${opportunity.windowSec}s`} />
                <TradeMetric label="结构上下文" value={regimeTypeLabel(opportunity.regimeContext)} />
                <TradeMetric label="事件等级" value={severityLabel(opportunity.severity)} />
              </div>
              <p className="mt-3 text-sm leading-6 text-slate-300">{opportunity.rationale}</p>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function InstitutionalAnalysisTerminalPanel({ intelligence }) {
  const [activeTab, setActiveTab] = useState("market-intelligence");
  const regime = intelligence?.marketRegime || {
    regime: "RANGING",
    confidence: 0,
    reason: "当前缺少足够的主力历史信号。",
  };
  const liquidityBehaviors = Array.isArray(intelligence?.liquidityBehaviors)
    ? intelligence.liquidityBehaviors
    : [];
  const rankedEvents = Array.isArray(intelligence?.rankedEvents) ? intelligence.rankedEvents : [];
  const opportunityMap = Array.isArray(intelligence?.opportunityMap) ? intelligence.opportunityMap : [];
  const suppression = intelligence?.noiseSuppression || {};
  const signalCompression = intelligence?.signalCompression || {};
  const tradeIdeas = Array.isArray(intelligence?.tradeIdeas) ? intelligence.tradeIdeas : [];
  const riskContext = intelligence?.riskContext || {};
  const noTradeZones = Array.isArray(riskContext?.noTradeZones) ? riskContext.noTradeZones : [];
  const tabs = [
    { id: "market-intelligence", label: "Market Intelligence" },
    { id: "trade-ideas", label: "Structure Ideas" },
    { id: "risk-no-trade", label: "Risk / No-Trade" },
  ];

  return (
    <section className="mt-4 rounded-2xl border border-cyan-500/20 bg-slate-950/35 p-4">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-cyan-300">Institutional Analysis Terminal</p>
          <h4 className="mt-1 text-sm font-bold text-white">半机构级分析终端</h4>
          <p className="mt-1 text-xs leading-5 text-slate-400">
            只展示市场状态、流动性行为、强度排序和结构机会，决策层保持只读语义。
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-5">
          <TradeSummaryPill label="原始候选" value={`${suppression.rawCandidates || 0}`} />
          <TradeSummaryPill label="合并后" value={`${suppression.mergedEvents || 0}`} />
          <TradeSummaryPill label="降噪后事件" value={`${suppression.filteredEvents || 0}`} tone="cyan" />
          <TradeSummaryPill label="结构机会" value={`${opportunityMap.length}`} tone="emerald" />
          <TradeSummaryPill label="压缩质量" value={`${signalCompression.qualityScore || 0}%`} tone="cyan" />
          <TradeSummaryPill label="保留信号" value={`${signalCompression.topSignalCount || 0}`} tone="emerald" />
          <TradeSummaryPill label="丢弃信号" value={`${signalCompression.discardedCount || 0}`} tone="red" />
          <TradeSummaryPill label="降噪比例" value={`${suppression.noiseReductionPct || 0}%`} tone="yellow" />
        </div>
      </div>

      <div className="mt-4 flex flex-wrap gap-2 border-b border-slate-800 pb-3" role="tablist" aria-label="Institutional terminal views">
        {tabs.map((tab) => {
          const selected = activeTab === tab.id;
          return (
            <button
              key={tab.id}
              id={`institutional-terminal-tab-${tab.id}`}
              role="tab"
              type="button"
              aria-selected={selected}
              aria-controls={`institutional-terminal-panel-${tab.id}`}
              className={`rounded-full border px-3 py-2 text-xs font-semibold transition ${
                selected
                  ? "border-cyan-400/70 bg-cyan-500/10 text-cyan-100"
                  : "border-slate-700 bg-slate-950/50 text-slate-300 hover:border-cyan-500/40 hover:text-cyan-100"
              }`}
              onClick={() => setActiveTab(tab.id)}
            >
              {tab.label}
            </button>
          );
        })}
      </div>

      {activeTab === "market-intelligence" ? (
        <div
          className="mt-4"
          id="institutional-terminal-panel-market-intelligence"
          role="tabpanel"
          aria-labelledby="institutional-terminal-tab-market-intelligence"
        >
          <div className="grid gap-3 xl:grid-cols-[1.05fr_1fr]">
            <article className="rounded-xl border border-slate-800 bg-slate-950/60 p-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Market Regime</p>
                  <h5 className="mt-1 text-lg font-bold text-white">{regime.regime}</h5>
                </div>
                <span className="rounded-full border border-cyan-500/30 bg-cyan-500/10 px-2 py-1 text-xs font-semibold text-cyan-100">
                  Regime {regime.confidence}%
                </span>
              </div>
              <p className="mt-3 text-sm leading-6 text-slate-300">{regime.reason}</p>
              <p className="mt-3 text-[11px] text-slate-500">
                {signalCompression.compressionReason || "cross-window dedup + quality gating"}
              </p>
            </article>

            <article className="rounded-xl border border-slate-800 bg-slate-950/60 p-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Opportunity Map</p>
                  <h5 className="mt-1 text-base font-bold text-white">结构机会分布</h5>
                </div>
                <span className="text-xs text-slate-500">{opportunityMap.length} zones</span>
              </div>
              {opportunityMap.length === 0 ? (
                <p className="mt-3 rounded-xl border border-slate-800 bg-slate-950/50 px-3 py-3 text-sm text-slate-400">
                  当前没有明确的结构机会区域，保留观察。
                </p>
              ) : (
                <div className="mt-3 grid gap-3 md:grid-cols-2">
                  {opportunityMap.map((zone) => (
                    <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-3" key={`${zone.zoneType}-${zone.rangeLabel}`}>
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <p className="text-sm font-semibold text-white">{zone.label}</p>
                          <p className="mt-1 text-xs text-cyan-200">{zone.rangeLabel}</p>
                        </div>
                        <span className="rounded-full border border-slate-700 px-2 py-1 text-[11px] text-slate-200">
                          {zone.strengthScore}/100
                        </span>
                      </div>
                      <p className="mt-2 text-xs leading-5 text-slate-400">{zone.description}</p>
                    </div>
                  ))}
                </div>
              )}
            </article>
          </div>

          <div className="mt-4 grid gap-3 xl:grid-cols-2">
            <article className="rounded-xl border border-slate-800 bg-slate-950/60 p-4">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Liquidity Behavior</p>
                  <h5 className="mt-1 text-base font-bold text-white">流动性行为</h5>
                </div>
                <span className="text-xs text-slate-500">{liquidityBehaviors.length} patterns</span>
              </div>
              {liquidityBehaviors.length === 0 ? (
                <p className="mt-3 rounded-xl border border-slate-800 bg-slate-950/50 px-3 py-3 text-sm text-slate-400">
                  当前没有可解释的主导流动性行为。
                </p>
              ) : (
                <div className="mt-3 grid gap-3">
                  {liquidityBehaviors.map((behavior) => (
                    <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-3" key={`${behavior.behavior}-${behavior.rangeLabel}`}>
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <p className="text-sm font-semibold text-white">{behavior.label}</p>
                          <p className="mt-1 text-xs text-cyan-200">{behavior.rangeLabel}</p>
                        </div>
                        <div className="text-right text-xs text-slate-300">
                          <p>{behavior.strengthScore}/100</p>
                          <p className="mt-1 text-slate-500">Conf {behavior.confidence}%</p>
                        </div>
                      </div>
                      <p className="mt-2 text-xs leading-5 text-slate-400">{behavior.reason}</p>
                    </div>
                  ))}
                </div>
              )}
            </article>

            <article className="rounded-xl border border-slate-800 bg-slate-950/60 p-4">
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Signal Strength Ranking</p>
                  <h5 className="mt-1 text-base font-bold text-white">强度排序</h5>
                </div>
                <span className="text-xs text-slate-500">{rankedEvents.length} ranked</span>
              </div>
              {rankedEvents.length === 0 ? (
                <p className="mt-3 rounded-xl border border-slate-800 bg-slate-950/50 px-3 py-3 text-sm text-slate-400">
                  当前没有通过排序门槛的结构事件。
                </p>
              ) : (
                <div className="mt-3 grid gap-3">
                  {rankedEvents.map((event) => (
                    <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-3" key={event.signalId}>
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Rank #{event.rank}</p>
                          <p className="mt-1 text-sm font-semibold text-white">{event.eventType}</p>
                        </div>
                        <div className="text-right text-xs text-slate-300">
                          <p>{event.strengthLabel}</p>
                          <p className="mt-1 text-cyan-200">{event.strengthScore}/100</p>
                        </div>
                      </div>
                      <div className="mt-3 grid gap-2 text-xs text-slate-300 sm:grid-cols-3">
                        <TradeMetric label="方向" value={event.directionBias} />
                        <TradeMetric label="窗口" value={`${event.windowSec}s`} />
                        <TradeMetric label="Regime" value={event.regimeAlignment} />
                      </div>
                      <p className="mt-2 text-xs leading-5 text-slate-400">{event.rationale}</p>
                    </div>
                  ))}
                </div>
              )}
            </article>
          </div>
        </div>
      ) : null}

      {activeTab === "trade-ideas" ? (
        <div
          className="mt-4"
          id="institutional-terminal-panel-trade-ideas"
          role="tabpanel"
          aria-labelledby="institutional-terminal-tab-trade-ideas"
        >
          <div className="rounded-xl border border-slate-800 bg-slate-950/60 p-4">
            <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
              <div>
                <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Signal Compression View</p>
                <h5 className="mt-1 text-base font-bold text-white">结构机会压缩视图</h5>
                <p className="mt-2 text-xs leading-5 text-slate-400">
                  这里只提供结构化参考，不生成自动执行指令；所有区间都属于分析终端的只读投影。
                </p>
              </div>
              <div className="grid gap-2 text-xs text-slate-300 md:grid-cols-3">
                <TradeSummaryPill label="压缩质量" value={`${signalCompression.qualityScore || 0}%`} tone="cyan" />
                <TradeSummaryPill label="Top Ideas" value={`${tradeIdeas.length}`} tone="emerald" />
                <TradeSummaryPill label="Discarded" value={`${signalCompression.discardedCount || 0}`} tone="yellow" />
              </div>
            </div>

            {tradeIdeas.length === 0 ? (
              <p className="mt-4 rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-4 text-sm text-slate-400">
                当前没有通过压缩门槛的结构机会，系统只保留市场解释层。
              </p>
            ) : (
              <div className="mt-4 grid gap-3 xl:grid-cols-3">
                {tradeIdeas.map((idea) => (
                  <article className="rounded-xl border border-slate-800 bg-slate-950/50 p-4" key={idea.signalId}>
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Rank #{idea.rank}</p>
                        <h6 className="mt-1 text-sm font-semibold text-white">{idea.setupType}</h6>
                      </div>
                      <span className="rounded-full border border-cyan-500/30 bg-cyan-500/10 px-2 py-1 text-[11px] font-semibold text-cyan-100">
                        {idea.confidenceLabel}
                      </span>
                    </div>
                    <div className="mt-3 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
                      <TradeMetric label="方向偏置" value={idea.directionBias} />
                      <TradeMetric label="强度评分" value={`${idea.score}/100`} />
                      <TradeMetric label="压力区" value={idea.pressureZone?.label || "N/A"} />
                      <TradeMetric label="风险边界" value={formatPrice(idea.riskBoundary?.priceLevel)} />
                      <TradeMetric label="Regime" value={idea.regimeContext || "N/A"} />
                      <TradeMetric label="事件窗口" value={`${idea.windowSec || 0}s`} />
                    </div>
                    <p className="mt-3 text-xs leading-5 text-slate-400">{idea.structureContext || "暂无结构备注"}</p>
                    <p className="mt-2 text-xs leading-5 text-slate-500">{idea.riskBoundary?.reason || "暂无风险边界说明"}</p>
                  </article>
                ))}
              </div>
            )}
          </div>
        </div>
      ) : null}

      {activeTab === "risk-no-trade" ? (
        <div
          className="mt-4"
          id="institutional-terminal-panel-risk-no-trade"
          role="tabpanel"
          aria-labelledby="institutional-terminal-tab-risk-no-trade"
        >
          <div className="grid gap-3 xl:grid-cols-[0.95fr_1.05fr]">
            <article className="rounded-xl border border-slate-800 bg-slate-950/60 p-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Risk / No-Trade</p>
                  <h5 className="mt-1 text-base font-bold text-white">No-trade Zones</h5>
                </div>
                <span className="rounded-full border border-amber-500/30 bg-amber-500/10 px-2 py-1 text-xs font-semibold text-amber-100">
                  {riskContext.fakeBreakoutRisk || "LOW"}
                </span>
              </div>
              <p className="mt-3 text-sm leading-6 text-slate-300">
                {riskContext.summary || "当前没有显著 no-trade 风险。"}
              </p>
              {noTradeZones.length === 0 ? (
                <p className="mt-4 rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-4 text-sm text-slate-400">
                  当前没有明确的禁做区间，保留常规结构观察即可。
                </p>
              ) : (
                <div className="mt-4 grid gap-3">
                  {noTradeZones.map((zone, index) => (
                    <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-3" key={`${zone.rangeLabel}-${index}`}>
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <p className="text-sm font-semibold text-white">{zone.rangeLabel || "N/A"}</p>
                          <p className="mt-1 text-xs text-cyan-200">{zone.reason || "暂无说明"}</p>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </article>

            <article className="rounded-xl border border-slate-800 bg-slate-950/60 p-4">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">Risk Summary</p>
                  <h5 className="mt-1 text-base font-bold text-white">风险抑制视图</h5>
                </div>
                <span className="text-xs text-slate-500">{noTradeZones.length} zones</span>
              </div>
              <div className="mt-4 grid gap-2 text-xs text-slate-300 md:grid-cols-2">
                <TradeMetric label="假突破风险" value={riskContext.fakeBreakoutRisk ? `${riskContext.fakeBreakoutRisk} RISK` : "LOW RISK"} />
                <TradeMetric label="No-trade 区数量" value={`${noTradeZones.length}`} />
                <TradeMetric label="保留信号" value={`${signalCompression.topSignalCount || 0}`} />
                <TradeMetric label="被压缩候选" value={`${signalCompression.discardedCount || 0}`} />
              </div>
              <p className="mt-4 text-xs leading-6 text-slate-400">
                这一页只负责告诉你哪里不该着急下判断。系统将高噪声、低响应和假突破风险明确隔离，避免污染主事件流。
              </p>
            </article>
          </div>
        </div>
      ) : null}
    </section>
  );
}

function TradeSummaryPill({ label, value, tone = "slate", testId }) {
  return (
    <div
      className={`rounded-xl border px-3 py-2 ${tradeSummaryPillClass(tone)}`}
      data-testid={testId}
    >
      <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">{label}</p>
      <p className="mt-1 text-sm font-bold text-white">{value}</p>
    </div>
  );
}

function TradeMetric({ label, value }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950/50 px-3 py-2">
      <p className="text-[11px] uppercase tracking-[0.18em] text-slate-500">{label}</p>
      <p className="mt-1 text-sm font-semibold text-slate-100">{value}</p>
    </div>
  );
}

function ContractWhaleDetailModal({ signal, relatedSignals, summary, onClose }) {
  const quantityUnit = baseAssetSymbol(signal.symbol);
  const signalExchanges = Array.isArray(signal.exchanges) ? signal.exchanges : [];
  const windowRows = [5, 15, 60].map((windowSec) => {
    const match = relatedSignals.find(
      (item) =>
        item.symbol === signal.symbol &&
        item.signalType === signal.signalType &&
        item.direction === signal.direction &&
        item.windowSec === windowSec,
    );
    return match || (signal.windowSec === windowSec ? signal : null);
  });
  const scoringRows = scoringBreakdown(signal);

  return (
    <div className="contract-detail-backdrop fixed inset-0 z-50 flex items-center justify-center px-4 py-6">
      <div
        aria-label="主力合约信号详情"
        aria-modal="true"
        className="workspace-dialog contract-detail-inspector"
        role="dialog"
      >
        <div className="contract-detail-header" data-testid="contract-detail-header">
          <div className="contract-detail-heading">
            <p>
              <span>Contract Whale Detail</span>
              <span aria-hidden="true"> / EVENT INSPECTOR</span>
            </p>
            <div className="contract-detail-title-row">
              <h3>{signal.symbol} 主力合约信号详情</h3>
              <span>{signalDisplayType(signal)}</span>
              <span>{directionLabel(signal.direction)}</span>
            </div>
            <p className="contract-detail-conclusion">{signal.finalResult}</p>
          </div>
          <div className="contract-detail-actions">
            <span className="contract-detail-readonly">READ ONLY</span>
            <button
              aria-label="关闭主力合约信号详情"
              className="contract-detail-close"
              onClick={onClose}
              type="button"
            >
              <span aria-hidden="true">×</span>
              关闭
            </button>
          </div>
        </div>

        <div className="contract-detail-summary" data-testid="contract-detail-summary">
          <ContractDetailMetric label="SEVERITY" value={severityLabel(signal.severity)} />
          <ContractDetailMetric label="事件状态" value={eventLifecycleStatus(signal) === "closed" ? "CLOSED" : "ACTIVE"} />
          <ContractDetailMetric
            label={signal.displayVolumeLabel || signal.finalEvent?.displayVolumeLabel || `总流量 ${quantityUnit}`}
            value={formatOptionalBaseVolume(signal.displayVolumeBtc ?? signal.finalEvent?.displayVolumeBtc ?? signal.totalVolumeBtc, signal.symbol)}
          />
          <ContractDetailMetric
            label={`净方向 ${quantityUnit}`}
            value={signal.netVolumeBtc === null || signal.netVolumeBtc === undefined ? "—" : netDirection(signal.netVolumeBtc, signal.symbol)}
          />
          <ContractDetailMetric label="名义金额" value={formatUsd(signal.totalNotionalUsd)} />
          <ContractDetailMetric label="触发价格" value={formatPrice(signalTriggerPrice(signal))} />
        </div>

        <div className="contract-detail-layout">
          <main className="contract-detail-body" data-testid="contract-detail-body">
        <DetailSection title="基础信息">
          <DetailGrid
            rows={[
              ["Symbol", signal.symbol],
              ["类型", signalDisplayType(signal)],
              ["方向", directionLabel(signal.direction)],
              ["价格响应", priceResponseLabel(signal.priceResponseTypeV2 || signal.priceResponseType)],
              ["v2 流向", flowDirectionLabel(signal.flowDirection)],
              ["OI 语境", formatOiContextSummary(signal)],
              ["意图置信", `${signal.intentConfidence || 0}/100`],
              ["强主力意图", yesNoLabel(signal.isStrongMainForceIntent)],
              ["分类版本", signal.classificationVersion || "legacy"],
              ["分类原因", signal.classificationReasons?.length ? signal.classificationReasons.join(" · ") : "N/A"],
              ["等级", severityLabel(signal.severity)],
              ["事件窗口", signal.mergedFrom?.length ? `${signal.windowSec}s · ${mergedWindowLabel(signal)}` : `${signal.windowSec}s`],
              ["事件状态", eventLifecycleStatus(signal) === "closed" ? "CLOSED" : "ACTIVE"],
              ["事件开始", formatTime(signal.eventLifecycle?.startTime)],
              ["最近更新", formatTime(signal.eventLifecycle?.lastUpdateTime)],
              ["事件更新次数", `${signal.eventLifecycle?.updateCount || 1}`],
              ["流量口径", signal.displayVolumeLabel || signal.finalEvent?.displayVolumeLabel || `总流量 ${quantityUnit}`],
              [`总流量 ${quantityUnit}`, formatOptionalBaseVolume(signal.displayVolumeBtc ?? signal.finalEvent?.displayVolumeBtc ?? signal.totalVolumeBtc, signal.symbol)],
              ["原始窗口流量", formatOptionalBaseVolume(signal.rawVolume ?? signal.finalEvent?.rawVolume ?? signal.totalVolumeBtc, signal.symbol)],
              [`主动买 ${quantityUnit}`, formatOptionalBaseVolume(signal.buyVolumeBtc ?? signal.finalEvent?.buyVolumeBtc, signal.symbol)],
              [`主动卖 ${quantityUnit}`, formatOptionalBaseVolume(signal.sellVolumeBtc ?? signal.finalEvent?.sellVolumeBtc, signal.symbol)],
              [`净方向 ${quantityUnit}`, signal.netVolumeBtc === null || signal.netVolumeBtc === undefined ? "—" : netDirection(signal.netVolumeBtc, signal.symbol)],
              ["来源交易所", sourceListLabel(signal.sourceExchanges || signal.finalEvent?.sourceExchanges || signal.activeSources?.contract?.map((entry) => entry.exchange))],
              ["来源交易所数", signal.sourceExchangeCount === null || signal.sourceExchangeCount === undefined ? "—" : `${signal.sourceExchangeCount}`],
              ["合并窗口", formatWindowList(signal.mergedWindowsSec || signal.finalEvent?.mergedWindowsSec || [signal.windowSec])],
              [
                "累计信号数",
                `${signal.mergedSignalCount ?? signal.finalEvent?.mergedSignalCount ?? signal.eventLifecycle?.updateCount ?? 1}`,
              ],
              ["跨交易所聚合", yesNoLabel(signal.isCrossExchangeAggregated ?? signal.finalEvent?.isCrossExchangeAggregated)],
              ["生命周期累计", yesNoLabel(signal.isLifecycleAccumulated ?? signal.finalEvent?.isLifecycleAccumulated)],
              ["Impact Score", impactScoreLabel(signal)],
              ["Z-score", impactZScoreLabel(signal)],
              ["Percentile", impactPercentileLabel(signal)],
              ["Impact Level", resolveImpactDisplay(signal).impactLevel],
                ["Signal Level", resolveImpactDisplay(signal).signalLevel],
                ["Signal Label", resolveImpactDisplay(signal).signalLabel],
                ["Normalized Strength", resolveImpactDisplay(signal).normalizedStrength],
                ["事件质量", eventQualityLabel(signal)],
                ["合并相似度", formatPct(Number(signal.eventQuality?.mergeSimilarityScore || 0) * 100)],
                ["假事件标记", eventQualityFlagsLabel(signal)],
                ["触发时间", formatTime(signal.ts)],
                ["触发价格", formatPrice(signalTriggerPrice(signal))],
                ["信号价格", formatPrice(signal.orderPriceUsd ?? signalTriggerPrice(signal))],
                ["当前价格", formatPrice(signal.currentMarketPriceUsd)],
                ["价格偏离", formatDeviation(signal.priceDeviationPct)],
                ["偏离过滤", signal.priceDeviationFiltered ? "已过滤" : `未过滤（阈值 ${CWM_MAX_PRICE_DEVIATION_PCT}%）`],
                ["Market Type", marketLabel(signal.marketType)],
                ["Source Role", sourceRoleLabel(signal.sourceRole)],
                ["Risk Score", `${signal.score}/100`],
                ["Main Force Score", formatScore(signal.mainForceScore)],
                ["Spot Score", formatScore(signal.spotScore)],
                ["Contract Score", formatScore(signal.contractScore)],
                ["Data Quality", `${signal.dataQuality}/100`],
                ["Threshold Profile", thresholdProfileLabel(signal.thresholdProfile || summary?.thresholdProfile)],
                ["Profile Reason", signal.thresholdProfileReason || signal.activeSources?.thresholdProfileReason || summary?.thresholdProfileReason || "N/A"],
                ["Configured Sources", sourceListLabel(signal.configuredContractSources || signal.activeSources?.configuredContractSources || summary?.configuredContractSources)],
                ["Eligible Sources", sourceListLabel(signal.eligibleContractSources || signal.activeSources?.eligibleContractSources || summary?.eligibleContractSources)],
                ["Active Sources", sourceListLabel(signal.activeContractSources || signal.activeSources?.activeContractSources || summary?.activeContractExchanges)],
              ]}
            />
          </DetailSection>

        <DetailSection title="Full Market Driver Engine" className="mt-4">
          <MarketDriverPanel signal={signal} />
        </DetailSection>

        <DetailSection title="Liquidity Force Layer" className="mt-4">
          <LiquidationForcePanel signal={signal} />
        </DetailSection>

        <DetailSection title="Signal Cluster / Persistence" className="mt-4">
          <DetailGrid
            rows={[
              ["Cluster ID", signal.cluster?.clusterId || "N/A"],
              ["Dominant Intent", clusterIntentLabel(signal.cluster?.dominantIntent)],
              ["Cluster Signals", `${signal.cluster?.signalCount || 1}`],
              ["Cluster Duration", formatMsDuration(signal.cluster?.durationMs)],
              ["Cluster Intensity", formatPct(Number(signal.cluster?.intensity || 0) * 100)],
              ["Price Range", formatOptionalPct(signal.cluster?.priceRangePct)],
              ["Persistence Score", formatPct(Number(signal.persistence?.persistenceScore || 0) * 100)],
              ["Half Life", formatMsDuration(signal.persistence?.signalHalfLifeMs)],
              ["Regime Stability", formatPct(Number(signal.persistence?.regimeStability || 0) * 100)],
              ["Redundant Projection", signal.persistence?.redundantWithPrevious ? repetitionReasonLabel(signal.persistence?.redundantReason) : "否"],
            ]}
          />
          <p className="mt-2 rounded-xl border border-cyan-500/20 bg-cyan-500/5 px-3 py-2 text-xs leading-6 text-cyan-100">
            Cluster 表示同 symbol、同方向、120 秒内且价格区间小于 0.3% 的连续信号；它更像同一主力意图轨迹，不等同于多个独立机会。
          </p>
        </DetailSection>

        <DetailSection title="Whale Trajectory" className="mt-4">
          <DetailGrid
            rows={[
              ["Trajectory ID", signal.trajectory?.trajectoryId || "N/A"],
              ["Intent", trajectoryIntentLabel(signal.trajectory?.intent)],
              ["Duration", formatMsDuration(signal.trajectory?.durationMs)],
              ["Regime Path", regimePathLabel(signal.trajectory?.regimePath)],
              ["Stealth Gamma", formatPct(Number(signal.trajectory?.stealthProfile?.gamma || 0) * 100)],
              ["Fragmentation", formatPct(Number(signal.trajectory?.stealthProfile?.fragmentation || 0) * 100)],
              ["Entropy", formatPct(Number(signal.trajectory?.stealthProfile?.entropy || 0) * 100)],
              ["Cross Exchange", formatPct(Number(signal.trajectory?.stealthProfile?.crossExchangeDispersion || 0) * 100)],
            ]}
          />
          <div className="mt-2 rounded-xl border border-slate-800 bg-slate-900/60 p-3">
            <p className="text-xs leading-6 text-cyan-100">
              {signal.trajectory?.conclusion || "轨迹证据不足，保持观察。"}
            </p>
            <div className="mt-3 grid gap-2 md:grid-cols-2">
              {(signal.trajectory?.actions || []).map((action, index) => (
                <div className="rounded-lg border border-slate-800 bg-slate-950/50 p-2 text-xs text-slate-300" key={`${action.ts}-${index}`}>
                  <p className="font-semibold text-slate-100">
                    {index + 1}. {actionTypeLabel(action.actionType)}
                  </p>
                  <p className="mt-1">
                    {formatTime(action.ts)} · {exchangeLabel(action.exchange)} · {formatBaseVolume(action.volume, action.symbol || signal.symbol)}
                  </p>
                  <p className="mt-1 text-slate-500">价格冲击 {formatSignedPct(action.priceImpact)}</p>
                </div>
              ))}
            </div>
          </div>
        </DetailSection>

        <DetailSection title="现货确认" className="mt-4">
          <DetailGrid
            rows={[
              ["状态", spotConfirmationStatusLabel(signal.spotConfirmation?.status)],
              ["确认类型", spotConfirmationTypeLabel(signal.spotConfirmation?.confirmationType)],
              ["现货方向", directionLabel(signal.spotConfirmation?.direction)],
              ["现货评分", `${Number(signal.spotConfirmation?.score || 0)}/100`],
              ["现货类型", signal.spotConfirmation?.signalType ? spotSignalTypeLabel(signal.spotConfirmation.signalType) : "N/A"],
              ["现货等级", signal.spotConfirmation?.severity ? severityLabel(signal.spotConfirmation.severity) : "N/A"],
              ["现货成交量", signal.spotConfirmation?.totalVolumeBtc === null || signal.spotConfirmation?.totalVolumeBtc === undefined ? "N/A" : formatBaseVolume(signal.spotConfirmation.totalVolumeBtc, signal.symbol)],
              ["现货净方向", signal.spotConfirmation?.netVolumeBtc === null || signal.spotConfirmation?.netVolumeBtc === undefined ? "N/A" : netDirection(signal.spotConfirmation.netVolumeBtc, signal.symbol)],
              ["Coinbase 溢价", signal.spotConfirmation?.coinbasePremiumPct === null || signal.spotConfirmation?.coinbasePremiumPct === undefined ? "N/A" : formatSignedPct(signal.spotConfirmation.coinbasePremiumPct)],
              ["现货结论", signal.spotConfirmation?.finalResult || "N/A"],
            ]}
          />
        </DetailSection>

        <DetailSection title="Active Source Snapshot" className="mt-4">
          <div className="grid gap-4 lg:grid-cols-2">
            <SourceSnapshotCard entries={signal.activeSources?.contract} title="合约源" />
            <SourceSnapshotCard entries={signal.activeSources?.spot} title="现货源" />
          </div>
        </DetailSection>

        <DetailSection title="5s / 15s / 60s 窗口数据" className="mt-4">
          <div className="grid gap-2 md:grid-cols-3">
            {[5, 15, 60].map((windowSec, index) => {
              const item = windowRows[index];
              return (
                <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-3" key={windowSec}>
                  <p className="font-bold text-slate-100">{windowSec}s</p>
                  {item ? (
                    <div className="mt-2 space-y-1 text-xs text-slate-300">
                      <p>{signal.displayVolumeLabel || `总流量 ${quantityUnit}`}：{formatOptionalBaseVolume(item.displayVolumeBtc ?? item.totalVolumeBtc, signal.symbol)}</p>
                      <p>主动买 {quantityUnit}：{formatOptionalBaseVolume(item.buyVolumeBtc, signal.symbol)}</p>
                      <p>主动卖 {quantityUnit}：{formatOptionalBaseVolume(item.sellVolumeBtc, signal.symbol)}</p>
                      <p>名义金额：{formatUsd(item.totalNotionalUsd)}</p>
                      <p>价格：{formatPrice(signalTriggerPrice(item))}</p>
                      <p>净方向 {quantityUnit}：{item.netVolumeBtc === null || item.netVolumeBtc === undefined ? "—" : netDirection(item.netVolumeBtc, signal.symbol)}</p>
                      <p>价格变化：{formatSignedPct(item.priceMovePct)}</p>
                      <p>异常倍数：{formatMultiple(item.dynamicMultiple)}</p>
                    </div>
                  ) : (
                    <p className="mt-2 text-xs text-slate-500">未触发或已被代表信号合并</p>
                  )}
                </div>
              );
            })}
          </div>
        </DetailSection>

        <DetailSection title="平台拆分" className="mt-4">
          <div className="grid gap-2 md:grid-cols-3">
            {signalExchanges.length ? (
              signalExchanges.map((exchange) => (
                <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-3" key={exchange.exchange}>
                  <p className="font-bold text-slate-100">{exchangeLabel(exchange.exchange)}</p>
                  <div className="mt-2 space-y-1 text-xs text-slate-300">
                    <p>主动买入：{formatBaseVolume(exchange.buyVolumeBtc, signal.symbol)}</p>
                    <p>主动卖出：{formatBaseVolume(exchange.sellVolumeBtc, signal.symbol)}</p>
                    <p>总量：{formatBaseVolume(exchange.totalVolumeBtc, signal.symbol)}</p>
                    <p>买/卖占比：{formatPct(Number(exchange.buyShare || 0) * 100)} / {formatPct(Number(exchange.sellShare || 0) * 100)}</p>
                    <p>净方向：{netDirection(exchange.netVolumeBtc, signal.symbol)}</p>
                    <p>方向强度：{formatPct(exchange.dominance * 100)}</p>
                    <p>净流贡献：{formatPct(Number(exchange.netContributionShare || 0) * 100)}</p>
                  </div>
                </div>
              ))
            ) : (
              <p className="text-sm text-slate-500">暂无平台拆分</p>
            )}
          </div>
        </DetailSection>

        <div className="mt-4 grid gap-4 lg:grid-cols-2">
          <DetailSection title="上下文指标">
            <DetailGrid
              rows={[
                ["Dynamic Multiple", formatMultiple(signal.dynamicMultiple)],
                ["Dynamic Baseline", signal.dynamicBaselineBtc === null || signal.dynamicBaselineBtc === undefined ? "N/A" : formatBaseVolume(signal.dynamicBaselineBtc, signal.symbol)],
                ["Dynamic Level", dynamicThresholdLevelLabel(signal.dynamicThresholdLevel)],
                ["Volatility Source", microVolatilityLabel(signal)],
                ["Micro Volatility", microVolatilityValueLabel(signal)],
                ["Price Efficiency", priceEfficiencyLabel(signal)],
                ["Semantic Shadow", signal.semanticMismatch ? `legacy ${signal.legacySignalType || signal.signalType} != v2` : "一致"],
                ["Percentile", formatPercentile(signal.percentileLevel)],
                ["Price Move", formatSignedPct(signal.priceMovePct)],
                ["5s Price Move", formatSignedPct(signal.priceMove5sPct)],
                ["15s Price Move", formatSignedPct(signal.priceMove15sPct)],
                ["30s Price Move", formatSignedPct(signal.priceMove30sPct)],
                ["Price Response", priceResponseLabel(signal.priceResponseType)],
                ["Price Reversal", signal.priceReversalRatio === null || signal.priceReversalRatio === undefined ? "N/A" : formatPct(signal.priceReversalRatio * 100)],
                ["Dominant Net Flow", formatPct(dominantNetFlowShare(signal) * 100)],
                ["Liquidation", liquidationStatus(signal)],
                ["Liquidation Evidence", liquidationEvidenceLabel(signal)],
                ["OI", oiStatus(signal)],
                ["OI Evidence", evidenceStateLabel(signal.oiEvidenceState, signal.oiReason)],
                ["Funding", fundingStatus(signal)],
                ["Funding Evidence", evidenceStateLabel(signal.fundingEvidenceState)],
              ]}
            />
          </DetailSection>

          <DetailSection title="Score Breakdown">
            <DetailGrid rows={scoringRows} />
          </DetailSection>
        </div>

        <DetailSection title="口径说明" className="mt-4">
          <div className="rounded-xl border border-cyan-500/20 bg-cyan-500/5 p-3 text-xs leading-6 text-cyan-50">
            <p>总流量 = 主动买量 + 主动卖量；历史视图会跨已启用交易所聚合。ACTIVE/CLOSED 优先显示真实换手，原始 1s 数据不足时显示生命周期峰值窗口，不做重复窗口相加。</p>
            <p>主动买 / 主动卖 / 净方向 是流向拆分，不是新的算法结果；净方向只表示买卖差值，不代表总量变化。</p>
            <p>买入/卖出占比 = 单个平台内部的主动买卖比例，只说明该平台自己的流向结构。</p>
            <p>净流贡献 = 该平台对本轮信号同方向净流的贡献比例，用来判断主导平台。</p>
          </div>
        </DetailSection>
          </main>

          <aside className="contract-detail-rail" data-testid="contract-detail-rail">
            <div className="contract-detail-rail-block contract-detail-decision">
              <p className="contract-detail-rail-eyebrow">DESK DECISION</p>
              <h4>核心判断</h4>
              <strong>{signal.finalResult}</strong>
              <p>{priceResponseNarrative(signal)}</p>
              {signal.cluster?.signalCount > 1 ? <p>{clusterTrajectoryNarrative(signal)}</p> : null}
            </div>

            <DetailSection title="Discord Gate">
              <DetailGrid
                rows={[
                  ["信号等级", severityLabel(signal.severity)],
                  ["市场冲击", discordImpactLabel(signal)],
                  ["推送原因", discordReasonLabel(signal)],
                  ["Gate Result", signal.discordEligible ? "可进入推送判断" : "仅展示"],
                  ["Would Send", signal.discordWouldSend ? "dry-run 会推送" : "不会推送"],
                  ["Discord Sent", signal.discordSent ? "已推送" : "未推送"],
                  ["Skip Reason", signal.discordSent ? "sent" : signal.discordReason],
                  ["多平台确认", signal.multiExchangeConfirmed ? "是" : "否"],
                  ["疑似强平", signal.liquidationSuspected ? "是" : "否"],
                  ["合并来源", signal.mergedFrom?.length ? signal.mergedFrom.join(", ") : "无"],
                ]}
              />
            </DetailSection>

            <div className="contract-detail-rail-block contract-detail-boundary">
              <p className="contract-detail-rail-eyebrow">EXECUTION BOUNDARY</p>
              <h4>只读事件审查</h4>
              <p>该详情仅解释监控证据，不执行交易、不签名，也不会改变 Discord 推送状态。</p>
            </div>
          </aside>
        </div>
      </div>
    </div>
  );
}

function ContractDetailMetric({ label, value }) {
  return (
    <div className="contract-detail-summary-cell">
      <p>{label}</p>
      <strong>{value ?? "N/A"}</strong>
    </div>
  );
}

function DetailSection({ title, children, className = "" }) {
  return (
    <section className={`contract-detail-section ${className}`.trim()}>
      <p className="contract-detail-section-title">{title}</p>
      {children}
    </section>
  );
}

function DetailGrid({ rows }) {
  return (
    <div className="contract-detail-grid">
      {rows.map(([label, value]) => (
        <div className="contract-detail-field" key={label}>
          <p>{label}</p>
          <strong>{value ?? "N/A"}</strong>
        </div>
      ))}
    </div>
  );
}

function MarketDriverPanel({ signal }) {
  const driver = signal.marketDriver || {};
  const rows = [
    ["Whale Intent", driver.whaleIntentPct, "主动鲸鱼资金"],
    ["Liquidity Force", driver.liquidityForcingPct, "流动性真空 / 风险单"],
    ["Derivatives", driver.derivativesPressurePct, "清算 / OI / Funding"],
    ["Reflexivity", driver.reflexivityPct, "趋势反馈放大"],
  ];
  return (
    <div className="rounded-xl border border-cyan-400/20 bg-cyan-400/5 p-3">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <p className="console-label text-cyan-200">Market Forcing Function</p>
          <h5 className="mt-1 text-sm font-bold text-white">
            Primary Driver: {marketDriverLabel(driver.primaryDriver)}
          </h5>
          <p className="mt-1 text-xs text-slate-400">{driver.interpretation || "价格主要由主动资金流驱动。"}</p>
        </div>
        <span className="rounded-full border border-cyan-300/25 px-2 py-1 text-[11px] font-bold text-cyan-100">
          {marketDriverStateLabel(driver.marketState)}
        </span>
      </div>
      <div className="mt-3 grid gap-2 sm:grid-cols-2">
        <MetricStack label="Dominant Driver" value={marketDriverLabel(driver.primaryDriver)} detail="final driver fusion" />
        <MetricStack label="Market State" value={marketDriverStateLabel(driver.marketState)} detail="final classifier" />
      </div>
      <div className="mt-3 space-y-2">
        {rows.map(([label, value, detail]) => (
          <ProgressRow key={label} label={`${label} · ${detail}`} value={Number(value || 0) * 100} />
        ))}
      </div>
    </div>
  );
}

function LiquidationForcePanel({ signal }) {
  const force = signal.liquidationForce || {};
  const flow = force.flowAttribution || {};
  const impact = force.priceImpact || {};
  const zones = Array.isArray(force.zones) ? force.zones : [];
  return (
    <div className="grid gap-3 lg:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
      <div className="rounded-xl border border-red-500/20 bg-red-500/5 p-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <p className="console-label text-red-200">Forced Liquidity</p>
            <h5 className="mt-1 text-sm font-bold text-white">
              {activeLiquidationZoneLabel(force.activeZone)}
            </h5>
          </div>
          <span className="rounded-full border border-red-400/30 px-2 py-1 text-[11px] font-bold text-red-100">
            {liquidationDriverLabel(force.primaryDriver)}
          </span>
        </div>
        <div className="mt-3 grid gap-2 sm:grid-cols-2">
          <MetricStack label="Long Liq Pressure" value={formatScore(force.longLiquidationPressure)} detail="longs forced sell" />
          <MetricStack label="Short Squeeze" value={formatScore(force.shortSqueezePressure)} detail="shorts forced buy" />
          <MetricStack label="Stop Hunt" value={formatScore(force.stopHuntProbability)} detail="wick / reversal risk" />
          <MetricStack label="Cascade" value={formatScore(force.cascadeIntensity)} detail={formatUsd(force.estimatedForcedSizeUsd)} />
        </div>
        <div className="mt-3 space-y-2">
          <ProgressRow label="Whale initiated" value={Number(flow.whalePct || 0) * 100} />
          <ProgressRow label="Retail follow" value={Number(flow.retailPct || 0) * 100} />
          <ProgressRow label="Forced liquidation" value={Number(flow.liquidationPct || 0) * 100} />
        </div>
      </div>
      <div className="rounded-xl border border-slate-800 bg-slate-950/45 p-3">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <p className="console-label">Price Impact Attribution</p>
            <h5 className="mt-1 text-sm font-bold text-white">价格驱动力拆解</h5>
          </div>
          <span className="text-xs text-slate-500">alert-only · read-only</span>
        </div>
        <div className="mt-3 grid gap-2 sm:grid-cols-2">
          <MetricStack label="Whale flow" value={formatSignedPct(impact.whaleImpact)} detail="主动资金影响" />
          <MetricStack label="Liquidation" value={formatSignedPct(impact.liquidationCascade)} detail="强制平仓影响" />
          <MetricStack label="Stop-loss sweep" value={formatSignedPct(impact.stopLossSweep)} detail="扫损影响" />
          <MetricStack label="Absorption" value={formatSignedPct(impact.passiveAbsorption)} detail="被动吸收抵消" />
        </div>
        <div className="mt-3 space-y-2">
          {zones.length ? zones.map((zone, index) => (
            <div className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2" key={`${zone.side}-${index}`}>
              <div className="flex items-center justify-between gap-3">
                <p className="text-xs font-semibold text-slate-100">{liquidationZoneSideLabel(zone.side)}</p>
                <span className="text-[11px] font-bold text-cyan-100">{formatScore(zone.intensity)}</span>
              </div>
              <p className="mt-1 text-xs text-slate-400">
                {formatPriceRange(zone.lowPriceUsd, zone.highPriceUsd)} · {formatUsd(zone.estimatedSizeUsd)}
              </p>
              <p className="mt-1 text-[11px] text-slate-500">{liquidationForceReasonLabel(zone.reason)}</p>
            </div>
          )) : (
            <p className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-3 text-xs text-slate-500">
              当前窗口没有清算区证据，按主动资金流观察。
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

function ContractWhaleTrendBar({ trend, symbol }) {
  const item = trend || {};
  const baseSymbol = baseAssetSymbol(item.symbol || symbol);
  const total = Number(item.totalVolumeBtc || 0);
  const buyRatio = total > 0 ? clampRatio(item.buyRatio) : 0;
  const sellRatio = total > 0 ? clampRatio(item.sellRatio || (1 - buyRatio)) : 0;
  const netDirectionLabel = netDirection(Number(item.netVolumeBtc || 0), baseSymbol);
  return (
    <div className="console-panel-muted mt-4 px-4 py-3">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <p className="console-label">60s Contract Flow</p>
          <p className="mt-1 text-sm font-semibold text-slate-100">
            Buy {formatPct(buyRatio * 100)} / Sell {formatPct(sellRatio * 100)}
          </p>
        </div>
        <div className="text-xs text-slate-400 md:text-right">
          <p>{netDirectionLabel}</p>
          <p>总量 {formatBaseVolume(total, baseSymbol)} · dominance {formatPct(Number(item.dominance || 0) * 100)}</p>
        </div>
      </div>
      <div className="mt-3 h-2 overflow-hidden rounded-full bg-red-500/20">
        <div
          aria-label="最近 60 秒主动买入占比"
          className="h-full rounded-full bg-emerald-400"
          style={{ width: total > 0 ? `${Math.max(3, buyRatio * 100)}%` : "0%" }}
        />
      </div>
      <div className="mt-2 flex justify-between text-[11px] text-slate-400">
        <span>主动买入 {formatBaseVolume(item.buyVolumeBtc, baseSymbol)}</span>
        <span>主动卖出 {formatBaseVolume(item.sellVolumeBtc, baseSymbol)}</span>
      </div>
      <p className="mt-2 text-[11px] text-slate-500">
        最近 60 秒主动成交流只表示 flow，不用于判断平台在线 / 离线状态。
      </p>
    </div>
  );
}

function MarketStructureLitePanel({ summary }) {
  const lite = summary.marketStructureLite || {};
  const stats = summary.discordDryRunStats || {};
  return (
    <div className="mt-3 grid gap-2 text-xs md:grid-cols-2 xl:grid-cols-4">
      <MiniInfoCard
        label="结构判断"
        value={`${regimeTypeLabel(lite.regimeType || "unclear")} · ${marketStructureStatusLabel(lite.status)}`}
        detail={lite.reason || "等待现货与合约确认"}
      />
      <MiniInfoCard
        label="主力评分"
        value={`${Math.round(Number(lite.mainForceScore || 0))}/100`}
        detail={`方向 ${biasText(lite.structureBias || 0)} · 置信 ${Math.round(Number(lite.confidence || 0))}`}
      />
      <MiniInfoCard
        label="现货确认"
        value={`Spot ${Math.round(Number(lite.spotScore || 0))} / Contract ${Math.round(Number(lite.contractScore || 0))}`}
        detail={`Cross ${Math.round(Number(lite.crossConfirmScore || 0))} · ${lite.mainForceConfirmed ? "已确认" : "待确认"}`}
      />
      <MiniInfoCard
        label="Dry-run 1h"
        value={`would-send ${Number(stats.wouldSend1h || 0)}`}
        detail={`signals ${Number(stats.signals1h || 0)} · C/S ${Number(stats.critical1h || 0)}/${Number(stats.s1h || 0)}`}
      />
    </div>
  );
}

function MiniInfoCard({ label, value, detail }) {
  return (
    <div className="console-panel-muted px-3 py-2">
      <p className="console-label">{label}</p>
      <p className="mt-1 font-bold text-slate-100">{value}</p>
      <p className="mt-1 truncate text-slate-400" title={detail}>{detail}</p>
    </div>
  );
}

function MetricStack({ label, value, detail }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-950/45 px-3 py-2">
      <p className="text-[11px] uppercase tracking-[0.14em] text-slate-500">{label}</p>
      <p className="mt-1 text-sm font-black text-slate-100">{value}</p>
      <p className="mt-1 truncate text-[11px] text-slate-500" title={detail}>{detail}</p>
    </div>
  );
}

function ProgressRow({ label, value }) {
  const percent = Math.max(0, Math.min(100, Number(value || 0)));
  return (
    <div>
      <div className="mb-1 flex items-center justify-between gap-3 text-[11px] text-slate-400">
        <span>{label}</span>
        <span className="font-semibold text-slate-200">{formatPct(percent)}</span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-slate-800">
        <div className="h-full rounded-full bg-cyan-400/80" style={{ width: `${percent}%` }} />
      </div>
    </div>
  );
}

function ContractWhaleFilters({ filters, lockedSymbol, onChange }) {
  const update = (key, value) => onChange({ ...filters, [key]: value });
  return (
    <div className="contract-filter-grid">
      <LockedAssetField symbol={lockedSymbol || filters.symbol} />
      <FilterSelect label="等级" value={filters.severity} onChange={(value) => update("severity", value)}>
        <option value="all">全部</option>
        <option value="s">S</option>
        <option value="critical">Critical</option>
        <option value="high">High</option>
        <option value="medium">Medium</option>
      </FilterSelect>
      <FilterSelect label="类型" value={filters.signal_type} onChange={(value) => update("signal_type", value)}>
        <option value="all">全部</option>
        <option value="aggressive_buy">主力拉盘</option>
        <option value="aggressive_sell">主力砸盘</option>
        <option value="downside_absorption">下方吸收</option>
        <option value="upside_suppression">上方压制</option>
      </FilterSelect>
      <FilterSelect label="方向" value={filters.direction} onChange={(value) => update("direction", value)}>
        <option value="all">全部</option>
        <option value="buy">主动买入</option>
        <option value="sell">主动卖出</option>
        <option value="absorption">吸收</option>
        <option value="suppression">压制</option>
      </FilterSelect>
      <FilterSelect label="净方向" value={filters.net_direction} onChange={(value) => update("net_direction", value)}>
        <option value="all">全部</option>
        <option value="abs500">大于 500（正负）</option>
        <option value="abs1000">大于 1000（正负）</option>
      </FilterSelect>
      <FilterSelect label="冲击等级" value={filters.impact_level || "all"} onChange={(value) => update("impact_level", value)}>
        <option value="all">全部</option>
        <option value="A">A</option>
        <option value="B">B</option>
        <option value="S">S</option>
      </FilterSelect>
      <FilterSelect label="Discord" value={filters.discord_sent} onChange={(value) => update("discord_sent", value)}>
        <option value="all">全部</option>
        <option value="true">已推送</option>
        <option value="false">未推送</option>
      </FilterSelect>
      <FilterSelect label="窗口" value={filters.window_sec} onChange={(value) => update("window_sec", value)}>
        <option value="all">全部</option>
        <option value="5">5s</option>
        <option value="15">15s</option>
        <option value="60">60s</option>
      </FilterSelect>
      <FilterSelect label="交易所" value={filters.exchange} onChange={(value) => update("exchange", value)}>
        <option value="all">全部</option>
        <option value="binance">Binance</option>
        <option value="bitfinex">Bitfinex</option>
        <option value="coinbase">Coinbase</option>
      </FilterSelect>
    </div>
  );
}

function LockedAssetField({ symbol }) {
  const asset = normalizeMainstreamSymbol(symbol);
  return (
    <div className="contract-filter-field">
      <span>币种</span>
      <div className="contract-filter-locked">
        币种：{asset}（当前页面固定）
      </div>
    </div>
  );
}

function FilterSelect({ label, value, onChange, children }) {
  return (
    <label className="contract-filter-field">
      <span>{label}</span>
      <select
        className="contract-filter-select"
        onChange={(event) => onChange(event.target.value)}
        value={value}
      >
        {children}
      </select>
    </label>
  );
}

function StatusPill({ label, value, tone }) {
  return (
    <div className="rounded-xl border border-slate-700/70 bg-slate-950/45 px-3 py-2">
      <p className="text-[11px] text-slate-400">{label}</p>
      <p className={`mt-1 text-base font-bold ${toneClass(tone)}`}>{value}</p>
    </div>
  );
}

function EventMetric({ label, value }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2">
      <p className="text-[11px] uppercase tracking-[0.12em] text-slate-500">{label}</p>
      <p className="mt-1 font-semibold text-slate-100">{value}</p>
    </div>
  );
}

function EventTag({ label, tone }) {
  return <span className={`rounded-full px-2 py-1 text-[11px] font-semibold ${eventTagClass(tone)}`}>{label}</span>;
}

function PlatformCapabilitySection({ exchanges, platforms, summary }) {
  const contractSources = contractSourceLabels(summary);
  const spotSources = spotSourceLabels(platforms);
  const platformStatuses = compactPlatformStatuses(exchanges, platforms);
  return (
    <section className="mt-4 rounded-xl border border-slate-800 bg-slate-950/35 px-3 py-3" data-testid="platform-status-strip">
      <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
        <div className="min-w-0">
          <p className="console-label">Platform Status</p>
          <h4 className="mt-1 text-sm font-bold text-white">平台状态</h4>
          <p className="mt-1 truncate text-xs text-slate-400" title={`合约源 ${contractSources.length ? contractSources.join(", ") : "无"} · 现货确认 ${spotSources.length ? spotSources.join(", ") : "无"} · 阈值 ${thresholdProfileLabel(summary?.thresholdProfile)}`}>
            合约源 {contractSources.length ? contractSources.join(", ") : "无"} · 现货确认 {spotSources.length ? spotSources.join(", ") : "无"} · 阈值 {thresholdProfileLabel(summary?.thresholdProfile)}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          {platformStatuses.map((entry) => (
            <PlatformStatusChip entry={entry} key={entry.exchange} />
          ))}
        </div>
      </div>
      <details className="mt-2 text-[11px] text-slate-400">
        <summary className="cursor-pointer select-none text-slate-300 outline-none transition hover:text-cyan-200 focus-visible:ring-2 focus-visible:ring-cyan-500/35">
          平台口径
        </summary>
        <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-slate-500">
          <span>profile: {summary?.thresholdProfileReason || "N/A"}</span>
          <span>Coinbase 仅现货确认，不参与 CWM 合约成交量、阈值和 Discord gate。</span>
        </div>
      </details>
    </section>
  );
}

function PlatformStatusChip({ entry }) {
  return (
    <span
      className={`inline-flex items-center gap-2 rounded-full border px-3 py-1 text-xs font-semibold ${compactPlatformStatusClass(entry.tone)}`}
      data-testid={`platform-status-chip-${entry.exchange}`}
    >
      <span className={`h-2 w-2 rounded-full ${compactPlatformDotClass(entry.tone)}`} aria-hidden="true" />
      <span>{exchangeLabel(entry.exchange)}</span>
      <span className="text-slate-400">·</span>
      <span>{entry.label}</span>
    </span>
  );
}

function SourceSnapshotCard({ entries, title }) {
  const items = Array.isArray(entries) ? entries : [];
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-semibold text-slate-100">{title}</p>
        <span className="text-xs text-slate-500">{items.length} 个来源</span>
      </div>
      {items.length === 0 ? (
        <p className="mt-3 text-sm text-slate-500">暂无快照</p>
      ) : (
        <div className="mt-3 space-y-2">
          {items.map((entry) => (
            <div className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2" key={`${title}-${entry.exchange}-${entry.marketType}`}>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <p className="text-sm font-medium text-slate-100">
                  {exchangeLabel(entry.exchange)} · {marketLabel(entry.marketType)}
                </p>
                <span className={`text-[11px] font-semibold ${snapshotStatusClass(entry.status)}`}>
                  {snapshotStatusLabel(entry.status)}
                </span>
              </div>
              <p className="mt-1 text-xs text-slate-400">
                {sourceRoleLabel(entry.sourceRole)}{entry.productId ? ` · ${entry.productId}` : ""}
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function HeaderCell({ children, title }) {
  return (
    <th className="whitespace-nowrap px-3 py-3 text-[11px] font-semibold uppercase tracking-[0.08em]" title={title}>
      {children}
    </th>
  );
}

function Cell({ children }) {
  return <td className="whitespace-nowrap px-3 py-3">{children}</td>;
}

function SymbolWithPrice({ item }) {
  return (
    <span className="flex min-w-[112px] flex-col leading-tight">
      <span className="font-semibold text-slate-100">{item.symbol}</span>
      <span className="mt-1 text-[11px] font-semibold text-amber-200">{formatUsd(item.totalNotionalUsd)}</span>
      <span className="mt-1 text-[11px] font-semibold text-cyan-200">{formatPrice(signalTriggerPrice(item))}</span>
    </span>
  );
}

const CONTRACT_CLASSIFICATION_TOOLTIP =
  "主力拉盘/砸盘仅在主动流方向、价格跟随、多窗口确认同时满足时显示。否则显示主动买压/主动卖压；吸收/压制要求主动方向占优，同时价格不跟随或出现有效回收/回落。";
const OI_CONTEXT_TOOLTIP =
  "OI 标签用于解释该窗口内未平仓量变化：\n- OI 上升：新仓增加\n- OI 下降：平仓/止损占比更高\n- 结合主动买卖方向与价格响应判断新多、新空、空头回补或多头平仓。";

function SignalTypeSummary({ item }) {
  return (
    <span className="flex min-w-[128px] flex-col leading-tight" title={signalClassificationTooltip(item)}>
      <span className="inline-flex items-center gap-1">
        <span className={signalTypeIconClass(item.signalType)} aria-hidden="true">
          {signalTypeIcon(item.signalType)}
        </span>
        <span>{signalDisplayType(item)}</span>
      </span>
      <span className="mt-1 whitespace-normal text-[10px] leading-4 text-slate-500">
        {signalClassificationMeta(item)}
      </span>
    </span>
  );
}

function BehaviorAssessmentCell({ item }) {
  const state = String(item?.behaviorState || "insufficient").toLowerCase();
  const type = String(item?.behaviorType || "insufficient_evidence").toLowerCase();
  const stateLabel = {
    confirmed: "已确认",
    provisional: "候选",
    insufficient: "证据不足",
    invalidated: "已失效",
  }[state] || "证据不足";
  const typeLabel = {
    new_long_build: "新多建仓",
    new_short_build: "新空建仓",
    short_covering: "空头回补",
    long_unwind: "多头平仓",
    downside_absorption: "下方吸收",
    upside_suppression: "上方压制",
    liquidation_sweep: "清算驱动",
    insufficient_evidence: "普通成交流",
  }[type] || "普通成交流";
  const confidence = Math.round(Number(item?.behaviorConfidence || 0));
  const tone = state === "confirmed" ? "text-emerald-300" : state === "provisional" ? "text-amber-300" : "text-slate-500";
  return (
    <span className="flex min-w-[112px] flex-col leading-tight" title={`${item?.behaviorRationale || ""}\n支持：${(item?.behaviorSupportingEvidence || []).join(" · ")}\n反证：${(item?.behaviorCounterEvidence || []).join(" · ")}`}>
      <span className={`font-semibold ${tone}`}>{typeLabel}</span>
      <span className="mt-1 text-[10px] text-slate-500">{stateLabel} · {confidence}/100</span>
    </span>
  );
}

function signalTypeLabel(type) {
  const labels = {
    aggressive_buy: "主力拉盘",
    aggressive_sell: "主力砸盘",
    downside_absorption: "下方吸收",
    upside_suppression: "上方压制",
  };
  return labels[type] || type || "未知";
}

function signalDisplayType(signal) {
  if (signal && typeof signal === "object") {
    const display = String(signal.displaySignalType || "").trim();
    if (display) return display;
    return signalTypeLabel(signal.signalType);
  }
  return signalTypeLabel(signal);
}

function signalClassificationMeta(signal) {
  const flow = signal?.flowDirection ? flowDirectionLabel(signal.flowDirection) : "—";
  const priceType = signal?.priceResponseTypeV2 || signal?.priceResponseType;
  const price = priceType ? priceResponseLabel(priceType) : "—";
  const oi = formatOiContextSummary(signal);
  return `主动流：${flow} · 价格：${price} · OI：${oi}`;
}

function microVolatilityLabel(signal) {
  const source = String(signal?.dynamicThresholds?.volatilitySource || "fallback").trim();
  if (source === "flow_1s_vwap") return "1s VWAP EWMA";
  if (source === "disabled") return "已关闭";
  return "基准缺失 / 降级";
}

function microVolatilityValueLabel(signal) {
  const thresholds = signal?.dynamicThresholds || {};
  const value = Number(thresholds.microVolatilityPct);
  if (!Number.isFinite(value)) return "N/A";
  const samples = Number(thresholds.volatilitySampleCount || 0);
  return `${value.toFixed(4)}% · ${samples} samples${thresholds.volatilityStale ? " · stale" : ""}`;
}

function priceEfficiencyLabel(signal) {
  const value = Number(signal?.priceEfficiency);
  if (!Number.isFinite(value)) return "N/A";
  const version = String(signal?.priceEfficiencyVersion || "legacy_btc_volume_v0");
  const unit = version === "notional_bps_per_usd_million_v1" ? "bps / $1M" : "legacy";
  return `${value.toFixed(4)} ${unit}`;
}

function signalClassificationTooltip(signal) {
  const reasons = Array.isArray(signal?.classificationReasons) && signal.classificationReasons.length
    ? `\n分类原因：${signal.classificationReasons.join(" · ")}`
    : "";
  return `${CONTRACT_CLASSIFICATION_TOOLTIP}\n${signalClassificationMeta(signal)}\n${OI_CONTEXT_TOOLTIP}${reasons}`;
}

function resolvedOiContextLabel(item) {
  const explicit = String(item?.oiContextLabel || "").trim();
  if (explicit) return explicit;
  return oiContextLabel(item?.oiContext);
}

function formatOiContextSummary(item) {
  // Only treat explicit false as unavailable; missing flags should still render context labels.
  if (item?.oiAvailable === false) return "OI 不可用";
  const label = resolvedOiContextLabel(item);
  const deltaPct = Number(item?.oiDeltaPct ?? item?.oiChangePct);
  if (!Number.isFinite(deltaPct)) {
    return label || "OI 不确认";
  }
  const sign = deltaPct > 0 ? "+" : "";
  return `${label || "OI 不确认"} ${sign}${deltaPct.toFixed(2)}%`;
}

function oiEvidenceSummary(item) {
  const consistent = Array.isArray(item?.oiConsistentSources) ? item.oiConsistentSources : [];
  const excluded = Array.isArray(item?.oiExcludedSources) ? item.oiExcludedSources : [];
  const parts = [];
  if (consistent.length) parts.push(`覆盖 ${consistent.join("+")}`);
  if (item?.oiCrossExchangeConsensus === true) parts.push("跨所共识");
  if (item?.oiCrossExchangeConsensus === false) parts.push("跨所冲突");
  if (item?.oiSourceCoverageChanged) parts.push("来源变化");
  if (item?.oiEvidenceDegraded) parts.push(item?.oiEvidenceReason || "证据降级");
  if (excluded.length) parts.push(`排除 ${excluded.length}`);
  return parts.join(" · ");
}

function flowDirectionLabel(value) {
  const labels = {
    buy_dominant: "主动买占优",
    sell_dominant: "主动卖占优",
    balanced: "多空均衡",
    unknown: "未知",
  };
  return labels[String(value || "unknown").toLowerCase()] || value || "未知";
}

function oiContextLabel(value) {
  const labels = {
    new_long_build: "新多开仓",
    new_short_build: "新空开仓",
    short_covering: "空头回补",
    long_unwind: "多头平仓",
    oi_not_confirmed: "OI 不确认",
    oi_unavailable: "OI 不可用",
  };
  return labels[String(value || "oi_unavailable").toLowerCase()] || value || "OI 不可用";
}

function priceResponseLabel(type) {
  const labels = {
    trend_follow_up: "买盘推动上涨",
    trend_follow_down: "卖盘推动下跌",
    downside_absorption: "卖出被承接",
    upside_resistance: "买入被压制",
    no_clear_response: "价格响应不明确",
  };
  return labels[String(type || "no_clear_response").toLowerCase()] || "价格响应不明确";
}

function priceResponseNarrative(signal) {
  const move = formatSignedPct(signal.priceMovePct);
  const responseType = signal.priceResponseTypeV2 || signal.priceResponseType;
  const response = priceResponseLabel(responseType);
  const base = `价格响应：${response}，当前窗口价格变化 ${move}。`;
  const value = String(responseType || "").toLowerCase();
  if (value === "downside_absorption") {
    return `${base} 主动卖出放大但没有有效打穿价格，优先按下方承接观察。`;
  }
  if (value === "upside_resistance") {
    return `${base} 主动买入放大但没有有效推升价格，优先按上方压制观察。`;
  }
  if (value === "trend_follow_up" || value === "trend_follow_down") {
    return `${base} 成交流和价格方向一致，说明短线冲击更直接。`;
  }
  return `${base} 缺少明确价格配合时，只作为成交流异常观察，不单独确认趋势。`;
}

function regimeTypeLabel(value) {
  const labels = {
    main_force_long_build: "主力建多",
    main_force_short_build: "主力建空",
    contract_flow_shock: "合约冲击",
    spot_accumulation: "现货吸筹",
    spot_distribution: "现货派发",
    contract_short_squeeze: "空头挤压",
    long_liquidation_cascade: "多头清算瀑布",
    downside_absorption: "下方吸收",
    upside_resistance: "上方压制",
    range_rotation: "高换手震荡",
  };
  return labels[value] || value || "结构未明";
}

function marketStructureStatusLabel(value) {
  const status = String(value || "calm").toLowerCase();
  if (status === "confirmed") return "已确认";
  if (status === "watch") return "观察";
  return "平静";
}

function dynamicThresholdLevelLabel(value) {
  const level = String(value || "normal").toLowerCase();
  if (level === "s") return "S 级动态异常";
  if (level === "critical") return "Critical 动态异常";
  if (level === "high") return "High 动态异常";
  if (level === "watch") return "Watch 动态异常";
  return "正常";
}

function spotConfirmationStatusLabel(value) {
  const status = String(value || "unavailable").toLowerCase();
  if (status === "confirmed") return "现货确认";
  if (status === "divergent") return "现货分歧";
  if (status === "context") return "仅作上下文";
  if (status === "disabled") return "现货监控未启用";
  if (status === "no_spot_sample") return "暂无现货样本";
  return "不可用";
}

function spotConfirmationTypeLabel(value) {
  const type = String(value || "unavailable").toLowerCase();
  const labels = {
    confirms_contract_direction: "现货与合约同向",
    spot_absorption_against_contract_sell: "合约卖压被现货承接",
    spot_resistance_against_contract_buy: "合约买盘遇现货压制",
    spot_divergence: "现货与合约分歧",
    spot_context_only: "现货上下文",
    spot_monitor_disabled: "现货监控未启用",
    unavailable: "不可用",
  };
  return labels[type] || labels.unavailable;
}

function spotSignalTypeLabel(type) {
  const labels = {
    spot_aggressive_buy: "现货主动买入",
    spot_aggressive_sell: "现货主动卖出",
    spot_downside_absorption: "现货下方吸收",
    spot_upside_suppression: "现货上方压制",
    spot_exchange_dislocation: "现货跨所错位",
  };
  return labels[String(type || "").toLowerCase()] || type || "N/A";
}

function signalTypeIcon(type) {
  const icons = {
    aggressive_buy: "▲",
    aggressive_sell: "▼",
    downside_absorption: "▣",
    upside_suppression: "⊣",
  };
  return icons[type] || "•";
}

function signalTypeIconClass(type) {
  const value = String(type || "").toLowerCase();
  if (value === "aggressive_buy") return "text-emerald-300";
  if (value === "aggressive_sell") return "text-red-300";
  if (value === "downside_absorption") return "text-cyan-300";
  if (value === "upside_suppression") return "text-yellow-300";
  return "text-slate-400";
}

function severityLabel(severity) {
  const value = String(severity || "calm").toLowerCase();
  if (value === "s") return "S";
  if (value === "critical") return "Critical";
  if (value === "high") return "High";
  if (value === "medium") return "Medium";
  return "平静";
}

function healthStatusLabel(status) {
  const value = String(status || "disabled").toLowerCase();
  if (value === "healthy") return "健康";
  if (value === "degraded") return "降级";
  if (value === "unhealthy") return "异常";
  if (value === "warming_up") return "预热";
  return "未启用";
}

function healthStatusTone(status) {
  const value = String(status || "disabled").toLowerCase();
  if (value === "healthy") return "cyan";
  if (value === "degraded" || value === "warming_up") return "yellow";
  if (value === "unhealthy") return "red";
  return "slate";
}

function marketSeverityBadgeClass(severity) {
  const value = String(severity || "").toLowerCase();
  if (value === "extreme") return "border border-fuchsia-500/40 bg-fuchsia-500/15 text-fuchsia-200";
  if (value === "major") return "border border-red-500/40 bg-red-500/15 text-red-200";
  if (value === "confirmed") return "border border-amber-500/40 bg-amber-500/15 text-amber-200";
  return "border border-slate-700/80 bg-slate-900/70 text-slate-300";
}

function eventTagClass(tone) {
  if (tone === "emerald") return "border border-emerald-500/40 bg-emerald-500/15 text-emerald-200";
  if (tone === "amber") return "border border-amber-500/40 bg-amber-500/15 text-amber-200";
  if (tone === "red") return "border border-red-500/40 bg-red-500/15 text-red-200";
  if (tone === "cyan") return "border border-cyan-500/40 bg-cyan-500/15 text-cyan-200";
  return "border border-slate-700/80 bg-slate-900/70 text-slate-300";
}

function directionLabel(direction) {
  const value = String(direction || "neutral").toLowerCase();
  if (value.includes("disabled")) return "未启用";
  if (value.includes("buy")) return "多";
  if (value.includes("sell")) return "空";
  if (value.includes("absorption")) return "吸收";
  if (value.includes("suppression")) return "压制";
  return "平静";
}

function shouldUseHistory(filters) {
  return ["severity", "signal_type", "direction", "net_direction", "impact_level", "discord_sent", "window_sec", "exchange"].some(
    (key) => filters[key] && filters[key] !== "all",
  );
}

function modeLabel(summary) {
  if (!summary.enabled) return "未启用";
  return summary.dryRun ? "Dry-run" : "实时提醒";
}

function thresholdProfileLabel(profile) {
  const value = String(profile || "").toLowerCase();
  if (value === "no_contract_sources") return "无合约源";
  if (value === "binance_bitfinex") return "Binance+Bitfinex";
  if (value === "binance_bitfinex_coinbase") return "Binance+Bitfinex+Coinbase";
  if (value === "three_exchange") return "三平台";
  return "默认";
}

function exchangeLabel(exchange) {
  const labels = {
    binance: "Binance",
    okx: "OKX",
    bitfinex: "Bitfinex",
    coinbase: "Coinbase",
  };
  return labels[exchange] || exchange;
}

function contractSourceLabels(summary) {
  const sources = Array.isArray(summary?.activeContractSources) && summary.activeContractSources.length
    ? summary.activeContractSources
    : Array.isArray(summary?.activeContractExchanges) && summary.activeContractExchanges.length
      ? summary.activeContractExchanges
      : Array.isArray(summary?.eligibleContractSources)
        ? summary.eligibleContractSources
        : [];
  return sources.map((exchange) => `${exchangeLabel(exchange)} Perp`);
}

function spotSourceLabels(platforms) {
  const source = platforms && typeof platforms === "object" ? platforms : {};
  return ["coinbase", "binance", "bitfinex", "okx"]
    .filter((exchange) => {
      const spot = source[exchange]?.markets?.spot;
      return Boolean(spot?.enabled);
    })
    .map((exchange) => `${exchangeLabel(exchange)} Spot`);
}

function compactPlatformStatuses(exchanges, platforms) {
  const platformSource = platforms && typeof platforms === "object" ? platforms : {};
  return ["binance", "bitfinex", "coinbase", "okx"].map((exchange) => {
    const platform = platformSource[exchange] || { platformEnabled: false, status: "disabled", markets: {} };
    const runtime = exchanges?.[exchange] || {};
    return {
      exchange,
      ...compactPlatformStatus(platform, runtime),
    };
  });
}

function compactPlatformStatus(platform, runtime) {
  const platformStatus = String(platform?.status || runtime?.status || "disabled").toLowerCase();
  const runtimeStatus = String(runtime?.status || "").toLowerCase();
  const platformEnabled = Boolean(platform?.platformEnabled ?? platform?.enabled);
  const connected = Boolean(runtime?.connected) || runtimeStatus === "connected";
  if (!platformEnabled || platformStatus === "disabled") {
    return { label: "未启用", tone: "slate" };
  }
  if (platformStatus === "spot_only") {
    return { label: "仅现货", tone: "cyan" };
  }
  if (runtimeStatus === "reconnecting" || platformStatus === "reconnecting") {
    return { label: "重连中", tone: "yellow" };
  }
  if (connected) {
    return { label: "运行中", tone: "emerald" };
  }
  if (runtimeStatus === "stale" || platformStatus === "stale") {
    return { label: "数据延迟", tone: "yellow" };
  }
  if (
    runtimeStatus === "initializing" ||
    runtimeStatus === "waiting_for_data" ||
    platformStatus === "initializing" ||
    platformStatus === "waiting_for_data"
  ) {
    return { label: "等待数据", tone: "cyan" };
  }
  if (runtimeStatus === "disconnected" || platformStatus === "disconnected") {
    return { label: "离线", tone: "red" };
  }
  if (platformStatus === "active" || platformStatus === "enabled") {
    return { label: "等待数据", tone: "cyan" };
  }
  return { label: "等待数据", tone: "cyan" };
}

function compactPlatformStatusClass(tone) {
  if (tone === "emerald") return "border-emerald-500/40 bg-emerald-500/10 text-emerald-200";
  if (tone === "cyan") return "border-cyan-500/40 bg-cyan-500/10 text-cyan-200";
  if (tone === "yellow") return "border-yellow-500/40 bg-yellow-500/10 text-yellow-100";
  if (tone === "red") return "border-red-500/40 bg-red-500/10 text-red-100";
  return "border-slate-700/80 bg-slate-900/70 text-slate-300";
}

function compactPlatformDotClass(tone) {
  if (tone === "emerald") return "bg-emerald-300";
  if (tone === "cyan") return "bg-cyan-300";
  if (tone === "yellow") return "bg-yellow-300";
  if (tone === "red") return "bg-red-300";
  return "bg-slate-500";
}

function tradeSummaryPillClass(tone) {
  if (tone === "emerald") return "border-emerald-500/30 bg-emerald-500/10";
  if (tone === "cyan") return "border-cyan-500/30 bg-cyan-500/10";
  if (tone === "yellow") return "border-yellow-500/30 bg-yellow-500/10";
  if (tone === "red") return "border-red-500/30 bg-red-500/10";
  return "border-slate-800 bg-slate-950/50";
}

function tradeActionClass(action) {
  const value = String(action || "").toUpperCase();
  if (value.includes("BULL") || value === "LONG") return "border border-emerald-500/40 bg-emerald-500/15 text-emerald-200";
  if (value.includes("BEAR") || value === "SHORT") return "border border-red-500/40 bg-red-500/15 text-red-200";
  return "border border-slate-700/80 bg-slate-900/70 text-slate-300";
}

function deriveDeskTradeIdeas(intelligence, summary) {
  const structuredIdeas = Array.isArray(intelligence?.tradeIdeas) ? intelligence.tradeIdeas : [];
  if (structuredIdeas.length > 0) {
    return structuredIdeas.slice(0, 3).map((idea, index) => ({
      signalId: idea.signalId || `desk-idea-${index}`,
      rank: idea.rank || index + 1,
      setupType: idea.setupType || idea.label || "结构机会",
      directionLabel: humanizeDeskDirection(idea.directionBias),
      actionTone: idea.directionBias,
      score: Math.round(Number(idea.score ?? 0)),
      confidence: Math.round(Number(idea.confidence ?? 0)),
      confidenceText: `${String(idea.confidenceLabel || tradeConfidenceLabel(idea.confidence)).toUpperCase()} CONF`,
      reasonTag: regimeTypeLabel(idea.regimeContext),
      windowSec: Number(idea.windowSec || 0),
      reason: idea.structureContext || idea.reason || "暂无结构备注",
      pressureZoneLabel: idea.pressureZone?.label || null,
      riskBoundaryReason: idea.riskBoundary?.reason || null,
    }));
  }

  const summaryIdeas = Array.isArray(summary?.tradeOpportunities) ? summary.tradeOpportunities : [];
  return summaryIdeas.slice(0, 3).map((idea, index) => ({
    signalId: idea.signalId || `summary-idea-${index}`,
    rank: idea.rank || index + 1,
    setupType: idea.setupType || idea.label || "结构机会",
    directionLabel: humanizeDeskDirection(idea.directionBias),
    actionTone: idea.action || idea.directionBias,
    score: Math.round(Number(idea.tradeScore ?? 0)),
    confidence: Math.round(Number(idea.confidence ?? 0)),
    confidenceText: `${tradeConfidenceLabel(idea.confidence)} CONF`,
    reasonTag: regimeTypeLabel(idea.regimeContext),
    windowSec: Number(idea.windowSec || 0),
    reason: idea.rationale || "暂无结构备注",
    pressureZoneLabel: idea.pressureZone?.label || idea.entryZone?.label || null,
    riskBoundaryReason: idea.riskBoundary?.reason || idea.invalidation?.reason || null,
  }));
}

function passesContractWhaleVisibleDisplayFilter(item, symbol) {
  if (item?.isVisible === false) return false;
  if (normalizeMainstreamSymbol(symbol) !== "BTC") return true;
  return resolveContractWhaleVolumeBtc(item) >= BTC_MIN_VISIBLE_TOTAL_VOLUME_BTC;
}

function resolveContractWhaleVolumeBtc(item) {
  const rawValue =
    item?.volumeBtc ??
    item?.totalVolumeBtc ??
    item?.volume_btc ??
    item?.total_volume_btc ??
    item?.windowVolumeBtc ??
    item?.window_volume_btc ??
    0;
  const normalized = Number(rawValue);
  return Number.isFinite(normalized) ? normalized : 0;
}

function contractWhaleDisplayFilterLabel(symbol) {
  if (normalizeMainstreamSymbol(symbol) === "BTC") {
    return `窗口总流量 ≥ ${BTC_MIN_VISIBLE_TOTAL_VOLUME_BTC} BTC`;
  }
  return "后端可见性过滤";
}

function buildVisibleSignalIdSet(items) {
  const ids = new Set();
  (Array.isArray(items) ? items : []).forEach((item) => {
    [signalDetailTargetId(item), item?.sourceSignalId, item?.eventId, item?.finalEventId, item?.id]
      .filter(Boolean)
      .forEach((value) => ids.add(String(value)));
  });
  return ids;
}

function filterIntelligenceByVisibleSignals(intelligence, visibleSignalIds) {
  if (!intelligence || typeof intelligence !== "object") return intelligence;
  return {
    ...intelligence,
    rankedEvents: filterItemsByVisibleSignalId(intelligence.rankedEvents, visibleSignalIds),
    tradeIdeas: filterItemsByVisibleSignalId(intelligence.tradeIdeas, visibleSignalIds),
  };
}

function filterSummaryTradeOpportunitiesByVisibleSignals(summary, visibleSignalIds) {
  if (!summary || typeof summary !== "object") return summary;
  return {
    ...summary,
    tradeOpportunities: filterItemsByVisibleSignalId(summary.tradeOpportunities, visibleSignalIds),
  };
}

function filterItemsByVisibleSignalId(items, visibleSignalIds) {
  if (!Array.isArray(items)) return [];
  if (!(visibleSignalIds instanceof Set) || visibleSignalIds.size === 0) return [];
  return items.filter((item) => visibleSignalIds.has(String(item?.signalId || "")));
}

function humanizeDeskDirection(value) {
  const normalized = String(value || "").toUpperCase();
  if (normalized.includes("BULL") || normalized.includes("BUY") || normalized.includes("LONG")) return "Bullish bias";
  if (normalized.includes("BEAR") || normalized.includes("SELL") || normalized.includes("SHORT")) return "Bearish bias";
  if (normalized.includes("ABSORPTION")) return "ABSORPTION";
  if (normalized.includes("SUPPRESSION")) return "SUPPRESSION";
  return normalized || "NEUTRAL";
}

function tradeConfidenceLabel(value) {
  const numeric = Number(value || 0);
  if (numeric >= 80) return "HIGH";
  if (numeric >= 60) return "MEDIUM";
  return "LOW";
}

function riskLabel(value) {
  const normalized = String(value || "").toUpperCase();
  if (!normalized || normalized === "LOW") return "LOW RISK";
  if (normalized.includes("HIGH")) return "HIGH RISK";
  if (normalized.includes("MEDIUM")) return "MEDIUM RISK";
  return `${normalized} RISK`;
}

function riskPillTone(value) {
  const normalized = String(value || "").toUpperCase();
  if (normalized.includes("HIGH")) return "red";
  if (normalized.includes("MEDIUM")) return "yellow";
  return "cyan";
}

function riskBadgeClass(value) {
  const normalized = String(value || "").toUpperCase();
  if (normalized.includes("HIGH")) return "border border-red-500/30 bg-red-500/10 text-red-100";
  if (normalized.includes("MEDIUM")) return "border border-amber-500/30 bg-amber-500/10 text-amber-100";
  return "border border-emerald-500/30 bg-emerald-500/10 text-emerald-100";
}

function snapshotStatusLabel(status) {
  const value = String(status || "configured").toLowerCase();
  if (value === "active") return "已参与";
  if (value === "spot_only") return "仅现货";
  if (value === "configured") return "已配置";
  if (value === "disabled") return "未启用";
  return value;
}

function snapshotStatusClass(status) {
  const value = String(status || "configured").toLowerCase();
  if (value === "active") return "text-emerald-300";
  if (value === "spot_only") return "text-cyan-300";
  if (value === "disabled") return "text-slate-500";
  return "text-yellow-300";
}

function sourceRoleLabel(role) {
  const value = String(role || "disabled").toLowerCase();
  if (value === "primary" || value === "primary_liquidity") return "主流动性源";
  if (value === "confirmation") return "确认源";
  if (value === "spot_confirmation") return "现货确认源";
  if (value === "optional") return "可选源";
  return "未参与";
}

function marketLabel(value) {
  const labels = {
    spot: "Spot",
    perp: "Perp",
    funding: "Funding",
    oi: "OI",
    liquidation: "Liquidation",
    level2: "Level2",
  };
  return labels[value] || value;
}

function sourceListLabel(value) {
  if (!Array.isArray(value) || value.length === 0) return "无";
  return value.map(exchangeLabel).join(", ");
}

function buildWhaleEntities(items) {
  const groups = new Map();
  for (const item of items || []) {
    const id = trajectoryKey(item);
    const existing = groups.get(id) || {
      id,
      signals: [],
      severity: item.severity,
      score: 0,
      startTs: item.ts,
      endTs: item.ts,
    };
    existing.signals.push(item);
    existing.severity = strongestSeverity(existing.severity, item.severity);
    existing.score = Math.max(existing.score, Number(item.mainForceScore ?? item.score ?? 0));
    existing.startTs = Math.min(existing.startTs, Number(item.ts || existing.startTs));
    existing.endTs = Math.max(existing.endTs, Number(item.ts || existing.endTs));
    groups.set(id, existing);
  }

  return Array.from(groups.values())
    .map((group) => {
      const signals = group.signals.sort((a, b) => Number(a.ts || 0) - Number(b.ts || 0));
      const lead = signals[signals.length - 1] || signals[0];
      const trajectory = lead?.trajectory || {};
      const cluster = lead?.cluster || {};
      const stealthProfile = trajectory.stealthProfile || {};
      const actions = Array.isArray(trajectory.actions) ? trajectory.actions : [];
      const signalCount = Math.max(
        signals.length,
        Number(cluster.signalCount || trajectory.signalCount || 0),
        1,
      );
      const durationMs = Math.max(
        Number(trajectory.durationMs || 0),
        Number(cluster.durationMs || 0),
        Math.max(0, group.endTs - group.startTs),
      );
      const regimePath = Array.isArray(trajectory.regimePath) && trajectory.regimePath.length
        ? trajectory.regimePath
        : inferRegimePath(lead);
      const stealthGamma = clampRatio(stealthProfile.gamma || inferStealthGamma(signals));
      const hazardCurve = buildHazardCurve(signals, actions);
      return {
        ...group,
        actions,
        clusterIntent: cluster.dominantIntent,
        conclusion: trajectory.conclusion || clusterTrajectoryNarrativeSafe(lead),
        durationMs,
        hazardCurve,
        hazardPeak: Math.max(...hazardCurve, 0),
        intent: trajectory.intent || inferTrajectoryIntent(lead),
        persistenceScore: clampRatio(lead?.persistence?.persistenceScore || signalCount / 6),
        phases: deriveTrajectoryPhases(signals, actions, regimePath),
        regimePath,
        regimeStability: clampRatio(lead?.persistence?.regimeStability || lead?.cluster?.intensity || 0),
        signalCount,
        stealthCurve: buildStealthCurve(signals, stealthProfile),
        stealthGamma,
      };
    })
    .sort((a, b) => {
      const severityDelta = severityRank(b.severity) - severityRank(a.severity);
      if (severityDelta !== 0) return severityDelta;
      return Number(b.endTs || 0) - Number(a.endTs || 0);
    });
}

function trajectoryKey(item) {
  if (item?.trajectory?.trajectoryId) return item.trajectory.trajectoryId;
  if (item?.cluster?.clusterId) return `trajectory:${item.cluster.clusterId}`;
  return `trajectory:${item?.symbol || "unknown"}:${item?.direction || "neutral"}:${Math.floor(Number(item?.ts || 0) / 120_000)}`;
}

function shortWhaleId(id) {
  const text = String(id || "whale");
  const suffix = text.split(":").filter(Boolean).pop() || text;
  return `Whale #${suffix.slice(-6).toUpperCase()}`;
}

function strongestSeverity(a, b) {
  return severityRank(b) > severityRank(a) ? b : a;
}

function severityRank(value) {
  const ranks = { calm: 0, low: 1, medium: 2, high: 3, critical: 4, s: 5 };
  return ranks[String(value || "calm").toLowerCase()] || 0;
}

function inferRegimePath(item) {
  const type = String(item?.signalType || "");
  if (type.includes("absorption")) return ["manipulation", "accumulation"];
  if (type.includes("suppression")) return ["manipulation", "distribution"];
  if (String(item?.direction || "") === "buy") return ["accumulation"];
  if (String(item?.direction || "") === "sell") return ["distribution"];
  return ["unclear"];
}

function inferTrajectoryIntent(item) {
  const type = String(item?.signalType || "");
  if (type.includes("absorption")) return "accumulation";
  if (type.includes("suppression")) return "distribution";
  if (String(item?.direction || "") === "buy") return "accumulation";
  if (String(item?.direction || "") === "sell") return "distribution";
  return "unknown";
}

function inferStealthGamma(signals) {
  if (!Array.isArray(signals) || signals.length === 0) return 0;
  const averagePersistence = signals.reduce((sum, item) => sum + Number(item?.persistence?.persistenceScore || 0), 0) / signals.length;
  const averageIntensity = signals.reduce((sum, item) => sum + Number(item?.cluster?.intensity || 0), 0) / signals.length;
  return Math.max(averagePersistence, averageIntensity);
}

function buildStealthCurve(signals, stealthProfile) {
  const base = [
    Number(stealthProfile.fragmentation || 0),
    Number(stealthProfile.entropy || 0),
    Number(stealthProfile.crossExchangeDispersion || 0),
    Number(stealthProfile.gamma || 0),
  ].map(clampRatio);
  const signalValues = (signals || []).map((signal) => clampRatio(signal?.persistence?.persistenceScore || signal?.cluster?.intensity || 0));
  const points = [...signalValues, ...base].filter((value) => value > 0);
  return points.length ? points.slice(-8) : [0.12, 0.18, 0.16, 0.2];
}

function buildHazardCurve(signals, actions) {
  const actionValues = (actions || []).map((action) => clampRatio(Math.abs(Number(action?.priceImpact || 0)) / 0.5));
  const signalValues = (signals || []).map((signal) => {
    const volume = clampRatio(Number(signal.totalVolumeBtc || 0) / 4_500);
    const dominance = clampRatio(Number(signal.dominance || 0));
    const priceMove = clampRatio(Math.abs(Number(signal.priceMovePct || 0)) / 0.35);
    return clampRatio(volume * 0.45 + dominance * 0.35 + priceMove * 0.2);
  });
  const points = [...signalValues, ...actionValues].filter((value) => value > 0);
  return points.length ? points.slice(-8) : [0.08, 0.1, 0.09, 0.12];
}

function deriveTrajectoryPhases(signals, actions, regimePath) {
  if (Array.isArray(actions) && actions.length > 0) {
    return actions.slice(0, 4).map((action) => ({
      detail: `${exchangeLabel(action.exchange)} · ${formatBaseVolume(action.volume, action.symbol || signals?.[0]?.symbol)} · price impact ${formatSignedPct(action.priceImpact)}`,
      intensity: clampRatio(Math.abs(Number(action.priceImpact || 0)) / 0.5 || Number(action.volume || 0) / 4_500),
      ts: action.ts || signals?.[0]?.ts,
      type: action.actionType || "unknown",
    }));
  }
  const source = (signals || []).slice(-4);
  if (source.length > 0) {
    return source.map((signal) => ({
      detail: `${signalDisplayType(signal)} · ${netDirection(signal.netVolumeBtc, signal.symbol)} · ${formatUsd(signal.totalNotionalUsd)}`,
      intensity: clampRatio(Math.max(Number(signal.dominance || 0), Number(signal?.cluster?.intensity || 0))),
      ts: signal.ts,
      type: signal.signalType || "unknown",
    }));
  }
  return (regimePath || ["unclear"]).map((type) => ({
    detail: "等待更多连续信号确认。",
    intensity: 0.2,
    ts: null,
    type,
  }));
}

function clusterTrajectoryNarrativeSafe(signal) {
  if (signal?.cluster?.signalCount > 1) return clusterTrajectoryNarrative(signal);
  return signal?.finalResult || "该信号暂未形成连续主力轨迹。";
}

function phaseLabel(value) {
  const labels = {
    accumulation: "吸筹阶段",
    aggressive_buy: "主动拉盘",
    aggressive_sell: "主动砸盘",
    distribution: "派发阶段",
    downside_absorption: "下方吸收",
    liquidity_probe: "流动性测试",
    manipulation: "操控试探",
    passive_absorb: "被动吸收",
    stop_hunt: "扫损/清算",
    unknown: "证据不足",
    upside_suppression: "上方压制",
  };
  return labels[value] || actionTypeLabel(value);
}

function phaseToneClass(value) {
  const text = String(value || "");
  if (text.includes("buy") || text.includes("accumulation") || text.includes("absorption")) {
    return "border-emerald-500/20 bg-emerald-500/5";
  }
  if (text.includes("sell") || text.includes("distribution") || text.includes("suppression")) {
    return "border-red-500/20 bg-red-500/5";
  }
  if (text.includes("hunt") || text.includes("manipulation")) {
    return "border-amber-500/20 bg-amber-500/5";
  }
  return "border-slate-800 bg-slate-900/60";
}

function phaseBarClass(value) {
  const text = String(value || "");
  if (text.includes("buy") || text.includes("accumulation") || text.includes("absorption")) return "h-full rounded-full bg-emerald-400";
  if (text.includes("sell") || text.includes("distribution") || text.includes("suppression")) return "h-full rounded-full bg-red-400";
  if (text.includes("hunt") || text.includes("manipulation")) return "h-full rounded-full bg-amber-300";
  return "h-full rounded-full bg-cyan-300";
}

function curveBarClass(tone) {
  if (tone === "amber") return "bg-amber-300/80";
  if (tone === "cyan") return "bg-cyan-300/80";
  return "bg-slate-400";
}

function netDirection(value, symbol = "BTC") {
  if (value > 0) return `净买入 ${formatBaseVolume(Math.abs(value), symbol)}`;
  if (value < 0) return `净卖出 ${formatBaseVolume(Math.abs(value), symbol)}`;
  return "中性";
}

function formatBtc(value) {
  return formatBaseVolume(value, "BTC");
}

function formatOptionalBaseVolume(value, symbol = "BTC") {
  if (value === null || value === undefined) return "—";
  return formatBaseVolume(value, symbol);
}

function formatBaseVolume(value, symbol = "BTC") {
  return `${Math.round(Number(value || 0)).toLocaleString("en-US")} ${baseAssetSymbol(symbol)}`;
}

function yesNoLabel(value) {
  if (value === null || value === undefined) return "—";
  return value ? "是" : "否";
}

function formatWindowList(value) {
  if (!Array.isArray(value) || value.length === 0) return "—";
  return value.join(", ") + " 秒";
}

function baseAssetSymbol(symbol = "BTC") {
  return String(symbol || "BTC")
    .toUpperCase()
    .replace(/[-_/]?(USDT|USD|PERP|SWAP)$/i, "") || "BTC";
}

function normalizeMainstreamSymbol(symbol = "BTC") {
  return baseAssetSymbol(symbol) === "ETH" ? "ETH" : "BTC";
}

function filterContractItemsBySymbol(items, symbol = "BTC") {
  const expected = normalizeMainstreamSymbol(symbol);
  return (Array.isArray(items) ? items : []).filter((item) => {
    const sourceSymbol = item?.symbol || item?.quantityUnit || item?.baseAsset || item?.asset || expected;
    return normalizeMainstreamSymbol(sourceSymbol) === expected;
  });
}

function formatUsd(value) {
  const number = Number(value || 0);
  if (number >= 1_000_000_000) return `$${(number / 1_000_000_000).toFixed(2)}B`;
  if (number >= 1_000_000) return `$${Math.round(number / 1_000_000).toLocaleString("en-US")}M`;
  return `$${Math.round(number).toLocaleString("en-US")}`;
}

function signalTriggerPrice(item) {
  const explicit = Number(
    item?.triggerPriceUsd ??
      item?.triggerPrice ??
      item?.avgPriceUsd ??
      item?.price,
  );
  if (Number.isFinite(explicit) && explicit > 0) {
    return explicit;
  }
  const totalVolumeBtc = Number(item?.totalVolumeBtc || 0);
  const totalNotionalUsd = Number(item?.totalNotionalUsd || 0);
  if (totalVolumeBtc > 0 && totalNotionalUsd > 0) {
    return totalNotionalUsd / totalVolumeBtc;
  }
  return null;
}

function formatPrice(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  if (number >= 1000) return `$${Math.round(number).toLocaleString("en-US")}`;
  if (number >= 1) return `$${number.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  return `$${number.toLocaleString("en-US", { minimumFractionDigits: 4, maximumFractionDigits: 4 })}`;
}

function formatPct(value) {
  return `${Number(value || 0).toFixed(1)}%`;
}

function formatScore(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) return "0/100";
  return `${Math.round(number)}/100`;
}

function formatScorePair(spotScore, contractScore) {
  return `S ${Math.round(Number(spotScore || 0))} / C ${Math.round(Number(contractScore || 0))}`;
}

function clusterTableLabel(item) {
  const count = Number(item?.cluster?.signalCount || 1);
  const persistence = Number(item?.persistence?.persistenceScore || 0);
  if (count <= 1 && persistence <= 0) return "单点";
  return `${count}条 · ${formatPct(persistence * 100)}`;
}

function mergedWindowLabel(item) {
  const windows = new Set();
  const currentWindow = Number(item?.windowSec || 0);
  if (currentWindow > 0) windows.add(currentWindow);
  for (const id of item?.mergedFrom || []) {
    const parts = String(id || "").split(":");
    const parsed = Number(parts[2]);
    if (Number.isFinite(parsed) && parsed > 0) windows.add(parsed);
  }
  const label = [...windows].sort((left, right) => left - right).map((windowSec) => `${windowSec}s`).join(" + ");
  return label ? `merged ${label}` : `merged +${item?.mergedFrom?.length || 0}`;
}

function eventLifecycleStatus(item) {
  return String(item?.eventLifecycle?.status || "active").toLowerCase() === "closed" ? "closed" : "active";
}

function eventQualityBadge(item) {
  const score = Math.round(Number(item?.eventQuality?.qualityScore ?? 1) * 100);
  const valid = item?.eventQuality?.valid !== false;
  const flags = item?.eventQuality?.falseEventFlags || [];
  return (
    <span className={valid ? "text-emerald-200" : "text-rose-300"}>
      Q {score}
      {flags.length ? <span className="block text-[10px] uppercase text-rose-300">{flags[0]}</span> : null}
    </span>
  );
}

function impactNormalizationBadge(item) {
  const impact = resolveImpactDisplay(item);

  return (
    <span className="block whitespace-nowrap">
      <span className={`block text-xs font-bold ${signalLevelClass(impact.signalLevel)}`}>
        {impact.signalLevel} / {impact.impactLevel}
      </span>
      <span className="block text-[10px] uppercase tracking-wide text-slate-400">{impact.signalLabel}</span>
      <span className="block text-[10px] text-slate-500">
        {impactMetricSummary(impact)}
      </span>
    </span>
  );
}

function resolveImpactDisplay(item) {
  const impactScore = numberOrNull(item?.impactScore ?? item?.finalEvent?.impactScore ?? item?.dynamicMultiple);
  const zScore = numberOrNull(
    item?.zScore ??
      item?.impactZScore ??
      item?.impact_z_score ??
      item?.finalEvent?.zScore ??
      item?.finalEvent?.impactZScore,
  );
  const percentile = numberOrNull(
    item?.percentile ??
      item?.finalEvent?.percentile ??
      item?.percentileLevel ??
      item?.finalEvent?.percentileLevel,
  );
  const dynamicThresholdLevel = String(
    item?.dynamicThresholdLevel ??
      item?.finalEvent?.dynamicThresholdLevel ??
      "normal",
  ).toLowerCase();
  const impactLevel = String(
    item?.impactLevel ??
      item?.finalEvent?.impactLevel ??
      deriveImpactLevelFromFallback(dynamicThresholdLevel, percentile, impactScore),
  ).toUpperCase();
  const signalLevel = String(
    item?.signalLevel ??
      item?.finalEvent?.signalLevel ??
      deriveSignalLevelFromImpact(impactLevel),
  ).toUpperCase();
  const signalLabel = String(
    item?.signalLabel ??
      item?.finalEvent?.signalLabel ??
      deriveSignalLabelFromImpact(impactLevel),
  ).toUpperCase();
  const normalizedStrength = String(
    item?.normalizedStrength ??
      item?.finalEvent?.normalizedStrength ??
      deriveNormalizedStrengthFromImpact(impactLevel),
  ).toUpperCase();
  return {
    impactScore,
    zScore,
    percentile,
    impactLevel,
    signalLevel,
    signalLabel,
    normalizedStrength,
  };
}

function impactMetricSummary(impact) {
  const parts = [];
  if (impact.impactScore !== null) {
    parts.push(`${impact.impactScore.toFixed(2)}x`);
  }
  if (impact.zScore !== null) {
    parts.push(`z ${impact.zScore.toFixed(2)}`);
  }
  if (impact.percentile !== null) {
    parts.push(formatPercentile(impact.percentile));
  }
  return parts.join(" · ") || "impact pending";
}

function impactScoreLabel(signal) {
  const impact = resolveImpactDisplay(signal);
  return impact.impactScore === null ? "—" : `${impact.impactScore.toFixed(2)}x`;
}

function impactZScoreLabel(signal) {
  const impact = resolveImpactDisplay(signal);
  return impact.zScore === null ? "—" : impact.zScore.toFixed(2);
}

function impactPercentileLabel(signal) {
  const impact = resolveImpactDisplay(signal);
  return impact.percentile === null ? "—" : formatPercentile(impact.percentile);
}

function deriveImpactLevelFromFallback(dynamicThresholdLevel, percentile, impactScore) {
  if (percentile !== null) {
    if (percentile > 97) return "S";
    if (percentile >= 90) return "A";
    if (percentile >= 80) return "B";
  }
  if (impactScore !== null) {
    if (impactScore > 5) return "S";
    if (impactScore >= 3) return "A";
    if (impactScore >= 1.8) return "B";
  }
  if (dynamicThresholdLevel === "s") return "S";
  if (dynamicThresholdLevel === "critical") return "A";
  if (dynamicThresholdLevel === "high") return "B";
  return "C";
}

function deriveSignalLevelFromImpact(impactLevel) {
  if (impactLevel === "S") return "S";
  if (impactLevel === "A") return "L3";
  if (impactLevel === "B") return "L2";
  return "L1";
}

function deriveSignalLabelFromImpact(impactLevel) {
  if (impactLevel === "S") return "SHOCK IMPACT EVENT";
  if (impactLevel === "A") return "HIGH IMPACT EVENT";
  if (impactLevel === "B") return "MEDIUM IMPACT EVENT";
  return "LOW IMPACT EVENT";
}

function deriveNormalizedStrengthFromImpact(impactLevel) {
  if (impactLevel === "S") return "EXTREME";
  if (impactLevel === "A") return "HIGH";
  if (impactLevel === "B") return "MEDIUM";
  return "LOW";
}

function signalLevelClass(signalLevel) {
  if (signalLevel === "S") return "text-rose-200";
  if (signalLevel === "L3") return "text-red-300";
  if (signalLevel === "L2") return "text-yellow-200";
  return "text-slate-400";
}

function numberOrNull(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function finiteNumber(value, fallback = 0) {
  const number = Number(value);
  return Number.isFinite(number) ? number : fallback;
}

function eventQualityLabel(item) {
  const score = Math.round(Number(item?.eventQuality?.qualityScore ?? 1) * 100);
  return `Q ${score} · ${item?.eventQuality?.valid === false ? "filtered" : "clean"}`;
}

function eventQualityFlagsLabel(item) {
  const flags = item?.eventQuality?.falseEventFlags || [];
  return flags.length ? flags.join(", ") : "none";
}

function clusterIntentLabel(value) {
  const labels = {
    liquidity_probe_buy: "买方流动性测试",
    liquidity_probe_sell: "卖方流动性测试",
    downside_absorption: "下方吸收",
    upside_suppression: "上方压制",
    single_signal: "单点信号",
  };
  return labels[value] || value || "N/A";
}

function clusterTrajectoryNarrative(signal) {
  return `该 cluster 共 ${signal.cluster.signalCount} 条同向信号，持续 ${formatMsDuration(signal.cluster.durationMs)}，价格区间 ${formatOptionalPct(signal.cluster.priceRangePct)}，更接近同一主力意图的连续投影。`;
}

function repetitionReasonLabel(value) {
  const labels = {
    same_intent_within_60s: "是：60 秒内同意图重复投影",
  };
  return labels[value] || "是";
}

function trajectoryIntentLabel(value) {
  const labels = {
    accumulation: "隐蔽吸筹",
    distribution: "分段派发",
    liquidity_manipulation: "流动性操控",
    stop_hunting: "扫损 / 清算猎取",
    unknown: "证据不足",
  };
  return labels[value] || value || "N/A";
}

function actionTypeLabel(value) {
  const labels = {
    aggressive_buy: "主动买入",
    aggressive_sell: "主动卖出",
    passive_absorb: "被动吸收",
    liquidity_probe: "流动性测试",
    stop_hunt: "扫损/清算",
    unknown: "未知动作",
  };
  return labels[value] || value || "N/A";
}

function regimePathLabel(path) {
  if (!Array.isArray(path) || path.length === 0) return "N/A";
  const labels = {
    accumulation: "吸筹",
    distribution: "派发",
    manipulation: "操控",
    unclear: "不明确",
  };
  return path.map((item) => labels[item] || item).join(" -> ");
}

function formatMsDuration(value) {
  const number = Number(value || 0);
  if (!Number.isFinite(number) || number <= 0) return "0s";
  if (number < 60_000) return `${Math.round(number / 1000)}s`;
  return `${Math.floor(number / 60_000)}m ${Math.round((number % 60_000) / 1000)}s`;
}

function formatOptionalPct(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) return "N/A";
  return formatPct(number);
}

function formatDeviation(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) return "N/A";
  return `${number.toFixed(2)}%`;
}

function clampRatio(value) {
  const number = Number(value || 0);
  if (!Number.isFinite(number)) return 0;
  return Math.max(0, Math.min(1, number));
}

function formatMultiple(value) {
  if (value === null || value === undefined) return "N/A";
  return `${Number(value).toFixed(1)}x`;
}

function formatPercentile(value) {
  if (value === null || value === undefined) return "N/A";
  return `P${Number(value).toFixed(1)}`;
}

function formatSignedPct(value) {
  if (value === null || value === undefined) return "N/A";
  const number = Number(value);
  return `${number >= 0 ? "+" : ""}${number.toFixed(2)}%`;
}

function formatTime(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  return new Date(number).toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    hour12: false,
    minute: "2-digit",
    second: "2-digit",
  });
}

function formatDate(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  return new Date(number).toLocaleDateString("zh-CN", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
}

function formatDateTime(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  return `${formatDate(number)} ${formatTime(number)}`;
}

function formatEventRange(startedAt, endedAt) {
  const start = formatTime(startedAt);
  if (!endedAt) {
    return `${start} - 进行中`;
  }
  return `${start} - ${formatTime(endedAt)}`;
}

function biasText(value) {
  const number = Number(value || 0);
  if (number >= 15) return `偏多 +${Math.round(number)}`;
  if (number <= -15) return `偏空 ${Math.round(number)}`;
  return `中性 ${Math.round(number)}`;
}

function discordStatus(item) {
  if (item.discordSent) return "已推";
  if (item.discordEligible) return "待推";
  return "未推";
}

function discordImpactLabel(item) {
  const impact = resolveImpactDisplay(item);
  return `${impact.impactLevel} / ${impact.signalLevel}`;
}

function discordReasonLabel(item) {
  const reason = item?.discordSent ? "sent" : item?.discordReason;
  if (reason === "impact_level_gate") {
    return `市场冲击 ${resolveImpactDisplay(item).impactLevel}`;
  }
  if (reason === "critical_or_s_gate") return "Critical / S gate";
  if (reason === "btc_high_gate") return "BTC High gate";
  if (reason === "high_score_multi_exchange") return "高分多平台确认";
  if (reason === "high_primary_source_extreme") return "主交易所极端冲击";
  if (reason === "data_quality_low" || reason === "data_quality_display_only") return "数据质量不足";
  if (reason === "warmup_collect_only") return "Warmup collect only";
  if (reason === "display_only" || reason === "medium_observe_only" || reason === "observe_only") {
    return "观察层不推送";
  }
  if (reason === "dry_run") return "dry-run 会推送";
  if (reason === "sent") return "已推送";
  return reason || "N/A";
}

function liquidationStatus(item) {
  if (!item.liquidationSuspected) return "正常";
  const total = Number(item.liquidationLongBtc || 0) + Number(item.liquidationShortBtc || 0);
  const ratio = item.liquidationRatio === null || item.liquidationRatio === undefined
    ? "N/A"
    : formatPct(Number(item.liquidationRatio) * 100);
  return `疑似强平 ${formatBaseVolume(total, item.symbol)} / ${ratio}`;
}

function liquidationEvidenceLabel(item) {
  const status = String(item?.liquidationEvidenceStatus || "unavailable").toLowerCase();
  if (status === "live") return "实时强平样本";
  if (status === "inferred") return `结构推断 · ${evidenceReasonLabel(item?.liquidationEvidenceReason)}`;
  return `不可用 · ${evidenceReasonLabel(item?.liquidationEvidenceReason)}`;
}

function evidenceStateLabel(state, reason = null) {
  const labels = {
    available: "可用",
    missing: "缺失",
    stale: "陈旧",
    insufficient_samples: "样本不足",
    query_failed: "查询失败",
  };
  const label = labels[String(state || "missing").toLowerCase()] || String(state || "missing");
  return reason ? `${label} · ${evidenceReasonLabel(reason)}` : label;
}

function evidenceReasonLabel(reason) {
  const labels = {
    price_volume_shape_only: "仅价格/流量形态",
    no_live_liquidation_samples: "无实时强平样本",
    older_than_latest_ttl: "超过实时有效期",
    stale: "超过证据有效期",
    missing: "未采集到数据",
  };
  return labels[String(reason || "").toLowerCase()] || reason || "未提供原因";
}

function activeLiquidationZoneLabel(value) {
  const labels = {
    long_liquidation_zone: "Long Liquidation Zone",
    short_squeeze_zone: "Short Squeeze Zone",
    stop_loss_sweep_zone: "Stop-loss Sweep Zone",
    neutral: "Neutral Zone",
  };
  return labels[value] || "Neutral Zone";
}

function marketDriverLabel(value) {
  const labels = {
    whale_intent: "Whale Intent",
    liquidity_forcing: "Liquidity Forcing",
    derivatives_pressure: "Derivatives Pressure",
    reflexivity_feedback: "Reflexivity Feedback",
  };
  return labels[value] || "Whale Intent";
}

function marketDriverStateLabel(value) {
  const labels = {
    whale_led_expansion: "Whale-led Expansion",
    whale_led_distribution: "Whale-led Distribution",
    liquidity_squeeze_regime: "Liquidity Squeeze",
    liquidation_cascade_regime: "Liquidation Cascade",
    short_squeeze_regime: "Short Squeeze",
    stop_hunt_regime: "Stop Hunt",
    derivatives_pressure_regime: "Derivatives Pressure",
    reflexive_trend_phase: "Reflexive Trend",
  };
  return labels[value] || "Whale-led Expansion";
}

function liquidationDriverLabel(value) {
  const labels = {
    whale_initiated_flow: "Whale Flow",
    liquidation_cascade: "Liquidation",
    retail_follow_flow: "Retail Follow",
  };
  return labels[value] || "Whale Flow";
}

function liquidationZoneSideLabel(value) {
  const labels = {
    long_liquidation: "Long liquidation cluster",
    short_liquidation: "Short liquidation cluster",
    neutral: "Neutral zone",
  };
  return labels[value] || "Liquidation cluster";
}

function liquidationForceReasonLabel(value) {
  const labels = {
    "downside stop-loss and long liquidation cluster": "下方风险单与多头强平区",
    "upside stop-loss and short liquidation cluster": "上方风险单与空头强平区",
  };
  return labels[value] || value || "清算代理区";
}

function formatPriceRange(low, high) {
  const lowText = formatPrice(low);
  const highText = formatPrice(high);
  if (lowText === "N/A" && highText === "N/A") return "price N/A";
  return `${lowText} - ${highText}`;
}

function oiStatus(item) {
  const bias = oiBiasLabel(item.oiBias);
  if (item.oiChange5mBtc === null || item.oiChange5mBtc === undefined) {
    // Compact tape may omit 5m delta; fall back to semantic window OI when available.
    if (item?.oiAvailable !== false) {
      const semantic = formatOiContextSummary(item);
      if (semantic && semantic !== "OI 不可用") return semantic;
    }
    return bias;
  }
  const pctValue = Number(item?.oiChangePct ?? item?.oiDeltaPct);
  const pct = Number.isFinite(pctValue) ? ` / ${formatSignedPct(pctValue)}` : "";
  return `${formatSignedBaseVolume(item.oiChange5mBtc, item.symbol)}${pct} ${bias}`;
}

function fundingStatus(item) {
  const bias = fundingBiasLabel(item.fundingBias);
  if (item.fundingRate === null || item.fundingRate === undefined) return bias;
  return `${formatSignedPct(Number(item.fundingRate) * 100)} ${bias}`;
}

function scoringBreakdown(item) {
  const breakdown = item?.scoreBreakdown || {};
  const hasBackendBreakdown = Number(breakdown.finalScore || 0) > 0
    || ["volumeScore", "notionalScore", "dynamicAnomalyScore", "directionalStrengthScore", "priceResponseScore"].some((key) => Number(breakdown[key] || 0) !== 0);
  if (hasBackendBreakdown) {
    return [
      ["Volume Strength", scorePart(breakdown.volumeScore)],
      ["Notional Size", scorePart(breakdown.notionalScore)],
      ["Dynamic Anomaly", scorePart(breakdown.dynamicAnomalyScore)],
      ["Directional Strength", scorePart(breakdown.directionalStrengthScore)],
      ["Price Response", scorePart(breakdown.priceResponseScore)],
      ["Multi Source", scorePart(breakdown.multiSourceScore)],
      ["Data Quality", scorePart(breakdown.dataQualityScore)],
      ["Dominant Venue", scorePart(breakdown.dominantVenueScore)],
      ["OI Context", scorePart(breakdown.oiContextScore)],
      ["Penalty", scorePart(breakdown.penaltyScore)],
      ["Final Score", `${Number(breakdown.finalScore || item.score || 0).toFixed(1)} / 100`],
    ];
  }
  const volumeScore = Math.min(35, (Number(item.totalVolumeBtc || 0) / 4_500) * 35);
  const dynamicScore = item.dynamicMultiple === null || item.dynamicMultiple === undefined
    ? 0
    : Math.min(20, (Number(item.dynamicMultiple) / 10) * 20);
  const dominanceScore = Math.max(0, Math.min(15, ((Number(item.dominance || 0) - 0.5) / 0.25) * 15));
  const priceScore = item.priceMovePct === null || item.priceMovePct === undefined
    ? 0
    : Math.min(15, (Math.abs(Number(item.priceMovePct)) / 0.25) * 15);
  const exchangeCount = Array.isArray(item.exchanges)
    ? item.exchanges.length
    : item.mainExchange
      ? 1
      : 0;
  const exchangeScore = exchangeCount >= 3 ? 10 : exchangeCount === 2 ? 8 : exchangeCount === 1 ? 4 : 0;
  const dataQualityScore = Math.min(5, (Number(item.dataQuality || 0) / 100) * 5);
  const dominantNetFlowScore = Math.max(0, Math.min(5, ((dominantNetFlowShare(item) - 0.7) / 0.3) * 5));
  return [
    ["Volume Strength", `${volumeScore.toFixed(1)} / 35`],
    ["Dynamic Multiple", `${dynamicScore.toFixed(1)} / 20`],
    ["Dominance", `${dominanceScore.toFixed(1)} / 15`],
    ["Price Impact", `${priceScore.toFixed(1)} / 15`],
    ["Multi Exchange", `${exchangeScore.toFixed(1)} / 10`],
    ["Data Quality", `${dataQualityScore.toFixed(1)} / 5`],
    ["Dominant Venue Net Flow", `${dominantNetFlowScore.toFixed(1)} / 5`],
    ["Penalty Notes", item.liquidationSuspected ? "liquidation_suspected" : "none"],
  ];
}

function scorePart(value) {
  const number = Number(value || 0);
  if (!Number.isFinite(number)) return "0.0";
  return number.toFixed(1);
}

function dominantNetFlowShare(item) {
  const explicit = Number(item?.dominantVenueNetContributionShare);
  if (Number.isFinite(explicit) && explicit > 0) return explicit;
  return Math.max(
    0,
    ...((item?.exchanges || []).map((exchange) => Number(exchange.netContributionShare || 0))),
  );
}

function oiBiasLabel(value) {
  const bias = String(value || "unknown").toLowerCase();
  if (bias === "rising") return "OI上升";
  if (bias === "falling") return "OI下降";
  if (bias === "flat") return "OI横盘";
  return "OI N/A";
}

function fundingBiasLabel(value) {
  const bias = String(value || "unknown").toLowerCase();
  if (bias === "long") return "偏多";
  if (bias === "short") return "偏空";
  if (bias === "neutral") return "中性";
  return "Funding N/A";
}

function formatSignedBtc(value) {
  return formatSignedBaseVolume(value, "BTC");
}

function formatSignedBaseVolume(value, symbol = "BTC") {
  const number = Number(value || 0);
  const sign = number >= 0 ? "+" : "-";
  return `${sign}${formatBaseVolume(Math.abs(number), symbol)}`;
}

function relativeAge(value) {
  const seconds = Math.max(0, Math.round((Date.now() - Number(value)) / 1000));
  if (seconds < 60) return `${seconds} 秒前`;
  if (seconds < 3600) return `${Math.round(seconds / 60)} 分钟前`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)} 小时前`;
  return `${Math.round(seconds / 86400)} 天前`;
}

function formatLatency(value) {
  const ms = Number(value);
  if (!Number.isFinite(ms) || ms < 0) return "N/A";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${Math.round(ms / 1000)}s`;
}

function formatLatencySeconds(value) {
  const seconds = Number(value);
  if (!Number.isFinite(seconds) || seconds < 0) return "N/A";
  return `${Math.round(seconds)} 秒`;
}

function statusTone(status) {
  const value = String(status || "calm").toLowerCase();
  if (value === "disabled" || status === "未启用") return "slate";
  if (value === "warmup" || status === "预热") return "yellow";
  if (value === "strong" || status === "强异动") return "red";
  if (value === "active" || status === "异动") return "orange";
  return "slate";
}

function statusLabel(status) {
  const value = String(status || "calm").toLowerCase();
  if (value === "disabled" || status === "未启用") return "未启用";
  if (value === "warmup" || status === "预热") return "预热";
  if (value === "strong" || status === "强异动") return "强异动";
  if (value === "active" || status === "异动") return "异动";
  return "平静";
}

function severityTone(severity) {
  const value = String(severity || "calm").toLowerCase();
  if (value === "s") return "fuchsia";
  if (value === "critical") return "red";
  if (value === "high") return "orange";
  if (value === "medium") return "yellow";
  return "slate";
}

function toneClass(tone) {
  const classes = {
    cyan: "text-cyan-200",
    fuchsia: "text-fuchsia-300",
    red: "text-red-300",
    orange: "text-orange-300",
    yellow: "text-yellow-300",
    slate: "text-slate-300",
  };
  return classes[tone] || classes.slate;
}

function severityBadgeClass(severity) {
  const value = String(severity || "calm").toLowerCase();
  if (value === "s") return "bg-fuchsia-500/15 text-fuchsia-200 ring-1 ring-fuchsia-400/40";
  if (value === "critical") return "bg-red-500/15 text-red-200 ring-1 ring-red-400/40";
  if (value === "high") return "bg-orange-500/15 text-orange-200 ring-1 ring-orange-400/40";
  if (value === "medium") return "bg-yellow-500/15 text-yellow-200 ring-1 ring-yellow-400/30";
  return "bg-slate-500/15 text-slate-200 ring-1 ring-slate-400/30";
}
