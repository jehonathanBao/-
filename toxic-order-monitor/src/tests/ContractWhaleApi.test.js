import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchContractWhaleEvents,
  fetchContractWhaleHistory,
  fetchContractWhaleLatest,
  fetchContractWhaleSummary,
  normalizeContractWhaleSignal,
} from "../api/contractWhale.js";

vi.mock("axios", () => ({
  default: {
    get: vi.fn(),
  },
}));

describe("contract whale api", () => {
  beforeEach(() => {
    axios.get.mockReset();
    vi.stubEnv("VITE_API_BASE_URL", "");
  });

  it("maps latest contract whale response into dashboard shape", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: {
          status: "strong",
          healthStatus: "healthy",
          healthReason: "primary_sources_recent",
          thresholdProfile: "binance_bitfinex",
          thresholdProfileReason: "active_contract_sources=binance,bitfinex",
          configuredContractSources: ["binance", "bitfinex"],
          eligibleContractSources: ["binance", "bitfinex"],
          activeExchangeCount: 2,
          enabledExchanges: ["binance", "bitfinex"],
          disabledExchanges: ["okx"],
          activeContractExchanges: ["binance", "bitfinex"],
          direction: "buy",
          latestDirection: "buy",
          latestSeverity: "s",
          latestPushedAtMs: 1_700_000_000_000,
          lastDiscordSentAt: 1_700_000_000_100,
          signalCount: 1,
          readOnly: true,
          enabled: true,
          dryRun: true,
          contractDataQuality: 95,
          spotDataQuality: 78,
          overallDataQuality: 88,
          trend60s: {
            buyVolumeBtc: 6200,
            sellVolumeBtc: 3800,
            totalVolumeBtc: 10000,
            netVolumeBtc: 2400,
            dominance: 0.24,
            buyRatio: 0.62,
            sellRatio: 0.38,
            updatedAtMs: 1_700_000_000_000,
          },
          exchanges: {
            binance: { connected: true, lastTradeAt: 1_700_000_000_000, reconnectCount: 0, platformEnabled: true, contractEnabled: true, enabledMarkets: ["spot", "perp"], marketRoles: { spot: "primary", perp: "primary" } },
            okx: { connected: true, lastTradeAt: 1_700_000_000_000, reconnectCount: 1, platformEnabled: false, contractEnabled: false, enabledMarkets: [], marketRoles: {} },
            bitfinex: { connected: false, lastTradeAt: null, reconnectCount: 3, platformEnabled: true, contractEnabled: true, enabledMarkets: ["spot", "perp"], marketRoles: { spot: "confirmation", perp: "confirmation" } },
            coinbase: { connected: false, status: "spot_only", lastTradeAt: null, reconnectCount: 0, platformEnabled: true, contractEnabled: false, enabledMarkets: ["spot"], marketRoles: { spot: "primary" } },
          },
          platforms: {
            binance: { platformEnabled: true, status: "active", markets: { spot: { enabled: true, status: "enabled", role: "primary" }, perp: { enabled: true, status: "active", role: "primary" } } },
            bitfinex: { platformEnabled: true, status: "active", markets: { spot: { enabled: true, status: "enabled", role: "confirmation" }, perp: { enabled: true, status: "active", role: "confirmation" } } },
            coinbase: { platformEnabled: true, status: "spot_only", markets: { spot: { enabled: true, status: "enabled", role: "primary" }, perp: { enabled: false, status: "disabled", role: "optional" } } },
            okx: { platformEnabled: false, status: "disabled", markets: {} },
          },
        },
        items: [contractWhaleItem()],
      },
    });

    const payload = await fetchContractWhaleLatest(20);

    expect(axios.get).toHaveBeenCalledWith("/api/contract-whale/latest?limit=20&symbol=BTC");
    expect(payload.summary).toMatchObject({
      status: "strong",
      healthStatus: "healthy",
      healthReason: "primary_sources_recent",
      thresholdProfile: "binance_bitfinex",
      thresholdProfileReason: "active_contract_sources=binance,bitfinex",
      configuredContractSources: ["binance", "bitfinex"],
      eligibleContractSources: ["binance", "bitfinex"],
      activeExchangeCount: 2,
      enabledExchanges: ["binance", "bitfinex"],
      disabledExchanges: ["okx"],
      activeContractExchanges: ["binance", "bitfinex"],
      contractDataQuality: 95,
      spotDataQuality: 78,
      overallDataQuality: 88,
      marketType: "perp",
      direction: "buy",
      latestDirection: "buy",
      latestSeverity: "s",
      signalCount: 1,
      enabled: true,
      dryRun: true,
      lastDiscordSentAt: 1_700_000_000_100,
      trend60s: {
        buyVolumeBtc: 6200,
        sellVolumeBtc: 3800,
        totalVolumeBtc: 10000,
        buyRatio: 0.62,
        sellRatio: 0.38,
      },
      exchanges: {
        binance: { connected: true, lastTradeAt: 1_700_000_000_000, reconnectCount: 0 },
        okx: { connected: true, lastTradeAt: 1_700_000_000_000, reconnectCount: 1 },
        bitfinex: { connected: false, lastTradeAt: null, reconnectCount: 3 },
        coinbase: { status: "spot_only", contractEnabled: false },
      },
      platforms: {
        coinbase: { status: "spot_only" },
      },
    });
    expect(payload.items[0]).toMatchObject({
      symbol: "BTC",
      signalType: "aggressive_buy",
      severity: "s",
      score: 94,
      totalVolumeBtc: 4820,
      totalNotionalUsd: 337_000_000,
      dominantVenueNetContributionShare: 0.986,
      dynamicMultiple: 9.4,
      percentileLevel: 99.9,
      multiExchangeConfirmed: true,
      liquidationSuspected: true,
      liquidationLongBtc: 420,
      liquidationRatio: 0.087,
      oiChange5mBtc: 900,
      oiChangePct: 1.2,
      oiBias: "rising",
      fundingRate: 0.00018,
      fundingBias: "long",
      marketType: "perp",
      sourceRole: "primary",
      thresholdProfile: "binance_bitfinex",
      thresholdProfileReason: "active_contract_sources=binance,bitfinex",
      configuredContractSources: ["binance", "bitfinex"],
      eligibleContractSources: ["binance", "bitfinex"],
      activeContractSources: ["binance", "bitfinex"],
      activeSources: {
        contract: expect.arrayContaining([
          expect.objectContaining({ exchange: "binance", marketType: "perp", status: "active" }),
        ]),
        spot: expect.arrayContaining([
          expect.objectContaining({ exchange: "coinbase", marketType: "spot", status: "spot_only" }),
        ]),
      },
      discordEligible: true,
      discordSent: true,
      discordSentAt: 1_700_000_000_050,
      mergedFrom: ["contract-whale:BTC:5:1700000000000:buy"],
      triggerPriceUsd: 337_000_000 / 4_820,
    });
    expect(payload.items[0].activeSources.contract).not.toEqual(
      expect.arrayContaining([
        expect.objectContaining({ exchange: "coinbase", marketType: "perp" }),
      ]),
    );
    expect(payload.items[0].activeSources.contract).not.toEqual(
      expect.arrayContaining([
        expect.objectContaining({ exchange: "okx", marketType: "perp" }),
      ]),
    );
  });

  it("fetches summary health for polling", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        status: "active",
        healthStatus: "degraded",
        healthReason: "partial_sources_recent",
        thresholdProfile: "binance_bitfinex",
        activeExchangeCount: 2,
        enabledExchanges: ["binance", "bitfinex"],
        disabledExchanges: ["okx"],
        activeContractExchanges: ["binance", "bitfinex"],
        direction: "buy",
        latestDirection: "buy",
        latestSeverity: "high",
        enabled: true,
        dryRun: true,
        contractDataQuality: 72,
        spotDataQuality: 84,
        overallDataQuality: 77,
        exchanges: {
          binance: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_000, latencyMs: 120, reconnectCount: 0 },
          okx: { connected: true, status: "connected", lastTradeAt: 1_700_000_000_100, latencyMs: 80, reconnectCount: 1 },
          bitfinex: { connected: false, status: "reconnecting", lastTradeAt: 1_699_999_900_000, latencyMs: null, reconnectCount: 3 },
          coinbase: { connected: false, status: "spot_only", lastTradeAt: null, latencyMs: null, reconnectCount: 0, platformEnabled: true, contractEnabled: false, enabledMarkets: ["spot"], marketRoles: { spot: "primary" } },
        },
        platforms: {
          coinbase: { platformEnabled: true, status: "spot_only", markets: { spot: { enabled: true, status: "enabled", role: "primary" } } },
        },
      },
    });

    const payload = await fetchContractWhaleSummary();

    expect(axios.get).toHaveBeenCalledWith("/api/contract-whale/summary?symbol=BTC");
    expect(payload.summary.exchanges.binance).toMatchObject({
      connected: true,
      status: "connected",
      latencyMs: 120,
    });
    expect(payload.summary.exchanges.bitfinex.status).toBe("reconnecting");
    expect(payload.summary.healthStatus).toBe("degraded");
    expect(payload.summary.thresholdProfile).toBe("binance_bitfinex");
    expect(payload.summary.enabledExchanges).toEqual(["binance", "bitfinex"]);
    expect(payload.summary.disabledExchanges).toEqual(["okx"]);
    expect(payload.summary.activeContractExchanges).toEqual(["binance", "bitfinex"]);
    expect(payload.summary.contractDataQuality).toBe(72);
    expect(payload.summary.spotDataQuality).toBe(84);
    expect(payload.summary.overallDataQuality).toBe(77);
    expect(payload.summary.exchanges.coinbase.status).toBe("spot_only");
    expect(payload.summary.platforms.coinbase.status).toBe("spot_only");
    expect(payload.summary.marketType).toBe("perp");
  });

  it("fetches history with server-side filters", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: {},
        items: [contractWhaleItem()],
        meta: {
          exchange: "coinbase",
          marketType: "perp",
          exchangeStatus: "spot_only",
          reason: "coinbase_perp_disabled",
        },
      },
    });

    const payload = await fetchContractWhaleHistory({
      symbol: "BTC",
      severity: "critical",
      signal_type: "aggressive_buy",
      direction: "buy",
      discord_sent: "true",
      window_sec: "15",
      exchange: "binance",
      limit: 50,
    });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/history?symbol=BTC&severity=critical&signal_type=aggressive_buy&direction=buy&discord_sent=true&window_sec=15&exchange=binance&limit=50",
    );
    expect(payload.items).toHaveLength(1);
    expect(payload.meta).toMatchObject({
      exchange: "coinbase",
      marketType: "perp",
      exchangeStatus: "spot_only",
      reason: "coinbase_perp_disabled",
    });
  });

  it("fetches main-force event history for timeline markers", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [
          {
            id: 7,
            symbol: "BTC",
            startedAt: 1_700_000_000_000,
            endedAt: 1_700_000_900_000,
            peakAt: 1_700_000_300_000,
            regimeType: "main_force_long_build",
            severity: "Major",
            peakMainForceScore: 88,
            peakExtremeImpactScore: 61,
            peakStructureBias: 64,
            confidence: 76,
            spotScore: 71,
            contractScore: 86,
            crossConfirmScore: 74,
            cwmScore: 89,
            oiScore: 82,
            liquidationScore: 31,
            fundingCrowdingScore: 24,
            mainForceConfirmed: true,
            extremeImpactConfirmed: false,
            liquidationDriven: false,
            reasonsJson: {
              finalResult: "高概率主力建多，不是单纯清算推动。",
            },
          },
        ],
      },
    });

    const payload = await fetchContractWhaleEvents({ symbol: "BTC", limit: 12 });

    expect(axios.get).toHaveBeenCalledWith("/api/contract-whale/events?symbol=BTC&limit=12");
    expect(payload.items[0]).toMatchObject({
      id: 7,
      symbol: "BTC",
      regimeType: "main_force_long_build",
      severity: "Major",
      peakMainForceScore: 88,
      mainForceConfirmed: true,
      reasons: {
        finalResult: "高概率主力建多，不是单纯清算推动。",
      },
    });
  });

  it("uses latest limit 50 by default", async () => {
    axios.get.mockResolvedValueOnce({ data: { summary: {}, items: [] } });

    await fetchContractWhaleLatest();

    expect(axios.get).toHaveBeenCalledWith("/api/contract-whale/latest?limit=50&symbol=BTC");
  });

  it("fetches ETH summary with symbol query", async () => {
    axios.get.mockResolvedValueOnce({ data: { status: "calm", enabled: true } });

    await fetchContractWhaleSummary("ETH");

    expect(axios.get).toHaveBeenCalledWith("/api/contract-whale/summary?symbol=ETH");
  });

  it("falls back to calm state on network failure", async () => {
    axios.get.mockRejectedValueOnce(new Error("network"));

    const payload = await fetchContractWhaleLatest();

    expect(payload.summary.status).toBe("calm");
    expect(payload.summary.enabled).toBe(false);
    expect(payload.summary.dryRun).toBe(true);
    expect(payload.items).toEqual([]);
  });

  it("normalizes missing fields without exposing raw internals", () => {
    const signal = normalizeContractWhaleSignal({
      id: "contract-whale-test",
      symbol: "BTC",
      rawPayload: "must not map",
      webhook: "must not map",
      token: "must not map",
    });

    expect(signal.id).toBe("contract-whale-test");
    expect(signal.rawPayload).toBeUndefined();
    expect(signal.webhook).toBeUndefined();
    expect(signal.token).toBeUndefined();
  });
});

function contractWhaleItem() {
  return {
    id: "contract-whale:BTC:15:1700000000000:buy",
    ts: 1_700_000_000_000,
    symbol: "BTC",
    windowSec: 15,
    signalType: "aggressive_buy",
    direction: "buy",
    severity: "s",
    score: 94,
    totalVolumeBtc: 4820,
    netVolumeBtc: 3260,
    totalNotionalUsd: 337_000_000,
    dominance: 0.676,
    priceMovePct: 0.31,
    mainExchange: "binance",
    dominantVenueNetContributionShare: 0.986,
    dynamicMultiple: 9.4,
    percentileLevel: 99.9,
    multiExchangeConfirmed: true,
    liquidationSuspected: true,
    liquidationLongBtc: 420,
    liquidationShortBtc: 0,
    liquidationNotionalUsd: 29_400_000,
    liquidationRatio: 0.087,
    oiChange1mBtc: 250,
    oiChange5mBtc: 900,
    oiChangePct: 1.2,
    oiBias: "rising",
    fundingRate: 0.00018,
    fundingBias: "long",
    marketType: "perp",
    sourceRole: "primary",
    thresholdProfile: "binance_bitfinex",
    thresholdProfileReason: "active_contract_sources=binance,bitfinex",
    configuredContractSources: ["binance", "bitfinex"],
    eligibleContractSources: ["binance", "bitfinex"],
    activeContractSources: ["binance", "bitfinex"],
    activeSources: {
      contract: [
        { exchange: "binance", marketType: "perp", sourceRole: "primary", enabled: true, status: "active" },
        { exchange: "bitfinex", marketType: "perp", sourceRole: "confirmation", enabled: true, status: "configured" },
      ],
      spot: [
        { exchange: "binance", marketType: "spot", sourceRole: "primary", enabled: true, status: "configured" },
        { exchange: "coinbase", marketType: "spot", sourceRole: "primary", enabled: true, status: "spot_only" },
      ],
    },
    exchanges: [
      {
        exchange: "binance",
        buyVolumeBtc: 2610,
        sellVolumeBtc: 200,
        totalVolumeBtc: 2810,
        buyShare: 0.929,
        sellShare: 0.071,
        netVolumeBtc: 2410,
        dominance: 0.858,
        netContributionShare: 0.986,
      },
    ],
    dataQuality: 91,
    discordEligible: true,
    discordSent: true,
    discordSentAt: 1_700_000_000_050,
    discordReason: "critical_or_s_gate",
    finalResult: "多平台主动买入爆发，疑似主力合约拉盘",
    mergedFrom: ["contract-whale:BTC:5:1700000000000:buy"],
  };
}
