import { vi } from "vitest";

export const CWM_MAX_PRICE_DEVIATION_PCT = 5;

export const mockContractWhaleSummary = {
  status: "平静",
  healthStatus: "disabled",
  direction: "neutral",
  latestSeverity: "calm",
  latestPushedAtMs: null,
  lastDiscordSentAt: null,
  signalCount: 0,
  readOnly: true,
  enabled: false,
  dryRun: true,
  platforms: {
    binance: { platformEnabled: true, status: "active", markets: {} },
    bitfinex: { platformEnabled: true, status: "active", markets: {} },
    coinbase: { platformEnabled: true, status: "spot_only", markets: {} },
    okx: { platformEnabled: false, status: "disabled", markets: {} },
  },
  activeContractSources: ["binance", "bitfinex"],
  thresholdProfile: "binance_bitfinex",
  thresholdProfileReason: "active_contract_sources=binance,bitfinex",
  contractDataQuality: 85,
  spotDataQuality: 80,
  overallDataQuality: 82,
};

export const mockSignal = {
  symbol: "SOLUSDT",
  abnormalScore: 88,
  buildScore: 82,
  notionalUsd: 1_000_000,
  direction: "buy",
};

export const fetchContractWhaleSummary = vi.fn(() =>
  Promise.resolve({
    summary: mockContractWhaleSummary,
    error: null,
  }),
);

export const fetchContractWhaleLatest = vi.fn(() =>
  Promise.resolve({
    summary: mockContractWhaleSummary,
    items: [],
    error: null,
  }),
);

export const fetchContractWhaleHistory = vi.fn(() =>
  Promise.resolve({
    summary: null,
    items: [],
    error: null,
  }),
);

export const fetchContractWhaleEvents = vi.fn(() =>
  Promise.resolve({
    items: [],
    error: null,
  }),
);

export const fetchContractEvents = vi.fn(() =>
  Promise.resolve({
    items: [],
    nextCursor: null,
    hasMore: false,
    limit: 100,
    range: "24h",
    serverTime: Date.now(),
    lastEventTs: null,
    error: null,
  }),
);

export const fetchContractEventDebugCounts = vi.fn(() =>
  Promise.resolve({
    data: {
      symbol: "BTC",
      range: "24h",
      totalRows: 0,
      returnedItems: 0,
      visibleCount: 0,
      hiddenCount: 0,
      hiddenReasons: {},
    },
    error: null,
  }),
);

export const fetchContractWhaleRawFlowDebug = vi.fn(() =>
  Promise.resolve({
    data: {
      symbol: "BTC",
      range: "24h",
      diagnosis: {
        layer: "test",
        reason: "mock",
      },
    },
    error: null,
  }),
);

export const fetchContractWhaleLatencyDebug = vi.fn(() =>
  Promise.resolve({
    data: {
      symbol: "BTC",
      range: "24h",
      guard: {
        status: "fresh",
      },
    },
    error: null,
  }),
);

export const fetchContractWhaleIntelligenceTerminal = vi.fn(() =>
  Promise.resolve({
    data: {
      marketRegime: null,
      liquidityBehaviors: [],
      rankedEvents: [],
      opportunityMap: [],
    },
    error: null,
  }),
);

export const fetchFinalEvents = vi.fn(() =>
  Promise.resolve({
    items: [],
    error: null,
  }),
);

export const fetchFinalEventsV2 = vi.fn(() =>
  Promise.resolve({
    active: [],
    closed: [],
    nextCursor: null,
    hasMore: false,
    limit: 100,
    range: "24h",
    serverTime: Date.now(),
    lastEventTs: null,
    error: null,
  }),
);

export const fetchContractRetentionStatus = vi.fn(() =>
  Promise.resolve({
    data: {
      rows: 0,
      protectedRows: 0,
      oldestTs: null,
      newestTs: null,
      protectedRules: ["severity = 'S'", "ABS(net_volume_btc) >= 500"],
    },
    error: null,
  }),
);

export const normalizePlatformStatus = vi.fn((platform) => ({
  key: platform?.platformEnabled ? "active" : "disabled",
  label: platform?.platformEnabled ? "运行中" : "未启用",
  description: "test platform status",
  tone: platform?.platformEnabled ? "emerald" : "slate",
}));

export const normalizeMarketStatus = vi.fn((market) => ({
  key: market?.enabled ? "active" : "disabled",
  label: market?.enabled ? "运行中" : "未启用",
  detail: "test market status",
  tone: market?.enabled ? "emerald" : "slate",
}));
