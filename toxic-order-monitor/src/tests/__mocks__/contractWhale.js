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
