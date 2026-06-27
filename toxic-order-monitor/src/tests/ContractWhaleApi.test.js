import axios from "axios";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchContractEventDebugCounts,
  fetchContractEvents,
  fetchContractRetentionStatus,
  fetchContractWhaleEvents,
  fetchContractWhaleHistory,
  fetchContractWhaleLatencyDebug,
  fetchContractWhaleLatest,
  fetchContractWhaleRawFlowDebug,
  fetchContractWhaleSummary,
  fetchFinalEvents,
  fetchFinalEventsV2,
  fetchJsonWithTimeout,
  normalizeContractWhaleSignal,
  normalizeMarketStatus,
  normalizePlatformStatus,
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
          discordDryRunStats: {
            signals1h: 4,
            high1h: 1,
            critical1h: 2,
            s1h: 1,
            wouldSend1h: 3,
            skippedLowScore1h: 1,
          },
          marketStructureLite: {
            status: "confirmed",
            regimeType: "main_force_long_build",
            mainForceScore: 84,
            extremeImpactScore: 62,
            structureBias: 64,
            confidence: 76,
            dataQuality: 88,
            spotScore: 71,
            contractScore: 94,
            crossConfirmScore: 75,
            mainForceConfirmed: true,
            extremeImpactConfirmed: false,
            reason: "合约主动买入与现货方向确认，主力建多概率提高。",
          },
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

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/latest?limit=20&symbol=BTC",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
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
      discordDryRunStats: {
        wouldSend1h: 3,
        critical1h: 2,
        s1h: 1,
      },
      marketStructureLite: {
        status: "confirmed",
        regimeType: "main_force_long_build",
        mainForceScore: 84,
        extremeImpactScore: 62,
        structureBias: 64,
        confidence: 76,
        spotScore: 71,
        contractScore: 94,
        crossConfirmScore: 75,
        mainForceConfirmed: true,
      },
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
      priceMove15sPct: 0.31,
      priceResponseType: "trend_follow_up",
      dominantVenueNetContributionShare: 0.986,
      dynamicMultiple: 9.4,
      dynamicBaselineBtc: 512,
      dynamicThresholdLevel: "critical",
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
      scoreBreakdown: expect.objectContaining({
        finalScore: 89.9,
        penaltyScore: -10,
      }),
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
      spotConfirmation: {
        status: "confirmed",
        confirmationType: "confirms_contract_direction",
        direction: "buy",
        score: 81,
      },
      discordEligible: true,
      discordSent: true,
      discordSentAt: 1_700_000_000_050,
      discordWouldSend: true,
      mergedFrom: ["contract-whale:BTC:5:1700000000000:buy"],
      triggerPriceUsd: 337_000_000 / 4_820,
      orderPriceUsd: 69_917,
      currentMarketPriceUsd: 70_000,
      priceDeviationPct: 0.1186,
      priceDeviationFiltered: false,
      mainForceScore: 87,
      spotScore: 81,
      contractScore: 94,
      cluster: {
        clusterId: "cwm-cluster:BTC:buy:14166666",
        signalCount: 3,
        dominantIntent: "liquidity_probe_buy",
        durationMs: 90_000,
        intensity: 0.91,
        priceRangePct: 0.18,
      },
      persistence: {
        persistenceScore: 0.82,
        signalHalfLifeMs: 60_000,
        regimeStability: 0.67,
        redundantWithPrevious: true,
        redundantReason: "same_intent_within_60s",
      },
      whaleAction: {
        actionType: "aggressive_buy",
        volume: 3260,
        priceImpact: 0.31,
        exchange: "binance",
      },
      trajectory: {
        trajectoryId: "whale-trajectory:cwm-cluster:BTC:buy:14166666",
        intent: "accumulation",
        durationMs: 90_000,
        regimePath: ["manipulation", "accumulation"],
        stealthProfile: {
          gamma: 0.73,
          fragmentation: 0.66,
          entropy: 0.82,
          crossExchangeDispersion: 0.33,
        },
        aggressivenessCurve: [0.41, 0.94],
        conclusion: "连续买方压力和承接行为占优，疑似主力分批吸筹。",
      },
      liquidationForce: {
        activeZone: "short_squeeze_zone",
        primaryDriver: "liquidation_cascade",
        shortSqueezePressure: 78,
        longLiquidationPressure: 12,
        stopHuntProbability: 66,
        cascadeIntensity: 73,
        estimatedForcedSizeUsd: 29_400_000,
        flowAttribution: {
          whalePct: 0.42,
          retailPct: 0.12,
          liquidationPct: 0.46,
          dominantDriver: "liquidation_cascade",
        },
        priceImpact: {
          whaleImpact: 0.12,
          liquidationCascade: 0.18,
          stopLossSweep: 0.08,
          passiveAbsorption: -0.02,
        },
        zones: [
          {
            side: "short_liquidation",
            lowPriceUsd: 70_100,
            highPriceUsd: 70_400,
            estimatedSizeUsd: 29_400_000,
            intensity: 78,
            reason: "upside stop-loss and short liquidation cluster",
          },
        ],
      },
      marketDriver: {
        primaryDriver: "derivatives_pressure",
        marketState: "short_squeeze_regime",
        whaleIntentPct: 0.28,
        liquidityForcingPct: 0.18,
        derivativesPressurePct: 0.42,
        reflexivityPct: 0.12,
        components: [
          { key: "whale_intent", score: 72, weightPct: 0.28 },
          { key: "liquidity_forcing", score: 54, weightPct: 0.18 },
          { key: "derivatives_pressure", score: 86, weightPct: 0.42 },
          { key: "reflexivity_feedback", score: 44, weightPct: 0.12 },
        ],
        interpretation: "价格主要受清算、OI/funding 或衍生品强制流推动，当前更像被迫交易而非纯主力主动流。",
      },
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

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/summary?symbol=BTC",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
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

  it("uses the requested symbol for summary trend units when the backend omits trend symbol", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        trend60s: {
          buyVolumeBtc: 100,
          sellVolumeBtc: 50,
          totalVolumeBtc: 150,
          netVolumeBtc: 50,
        },
      },
    });

    const payload = await fetchContractWhaleSummary("ETH");

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/summary?symbol=ETH",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload.summary.trend60s).toMatchObject({
      symbol: "ETH",
      baseAsset: "ETH",
      quantityUnit: "ETH",
      buyVolumeBtc: 100,
      sellVolumeBtc: 50,
      totalVolumeBtc: 150,
      netVolumeBtc: 50,
    });
    expect(payload.summary.trend60s.symbol).not.toBe("BTC");
  });

  it("filters price-deviated signals from latest response", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: {},
        items: [
          contractWhaleItem({
            id: "kept",
            orderPriceUsd: 69_000,
            currentMarketPriceUsd: 70_000,
            priceDeviationPct: undefined,
          }),
          contractWhaleItem({
            id: "filtered",
            orderPriceUsd: 60_000,
            currentMarketPriceUsd: 70_000,
            priceDeviationPct: undefined,
          }),
        ],
      },
    });

    const payload = await fetchContractWhaleLatest(20);

    expect(payload.items.map((item) => item.id)).toEqual(["kept"]);
    expect(payload.items[0]).toMatchObject({
      priceDeviationFiltered: false,
      priceDeviationPct: expect.any(Number),
    });
  });

  it("uses the requested symbol for latest signals when legacy backend rows omit symbol", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [
          contractWhaleItem({
            id: undefined,
            ts: 1_700_000_100_000,
            windowSec: 60,
            symbol: undefined,
            totalVolumeBtc: 16869,
            netVolumeBtc: 610,
            totalNotionalUsd: 28_000_000,
            orderPriceUsd: undefined,
            currentMarketPriceUsd: undefined,
            priceDeviationPct: undefined,
          }),
        ],
      },
    });

    const payload = await fetchContractWhaleLatest(20, "ETH");

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/latest?limit=20&symbol=ETH",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload.items[0]).toMatchObject({
      symbol: "ETH",
      baseAsset: "ETH",
      quantityUnit: "ETH",
      totalVolumeBtc: 16869,
      netVolumeBtc: 610,
    });
    expect(payload.items[0].id).toBe("ETH-60-1700000100000");
    expect(payload.items[0].triggerPriceUsd).toBeCloseTo(28_000_000 / 16_869, 6);
    expect(payload.items[0].orderPriceUsd).toBeCloseTo(28_000_000 / 16_869, 6);
    expect(payload.summary.trend60s.symbol).toBe("ETH");
  });

  it("drops latest rows whose explicit symbol does not match the requested symbol", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [
          contractWhaleItem({ id: "wrong-symbol", symbol: "BTC" }),
          contractWhaleItem({ id: "right-symbol", symbol: "ETH" }),
        ],
      },
    });

    const payload = await fetchContractWhaleLatest(20, "ETH");

    expect(payload.items.map((item) => item.id)).toEqual(["right-symbol"]);
  });

  it("normalizes stale latest metadata and can request hide_stale filtering", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [
          contractWhaleItem({
            id: "stale-btc-row",
            ts: 1_700_000_000_000,
            ageSec: 90_100,
            isStale: true,
            staleReason: "older_than_24h",
          }),
        ],
      },
    });

    const payload = await fetchContractWhaleLatest(20, "BTC", { hide_stale: true });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/latest?limit=20&symbol=BTC&hide_stale=true",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload.items).toHaveLength(1);
    expect(payload.items[0]).toMatchObject({
      id: "stale-btc-row",
      ageSec: 90_100,
      isStale: true,
      staleReason: "older_than_24h",
    });
  });

  it("fetches raw flow debug diagnostics for a symbol and preserves diagnosis fields", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        symbol: "BTC",
        range: "24h",
        config: {
          appRequestedSymbol: "ETH-PERP",
          querySymbol: "BTC",
        },
        normalizer: {
          connectorSymbolMismatch: true,
        },
        contractFlow1s: {
          exactSymbolRows: 0,
        },
        diagnosis: {
          status: "upstream_no_raw_flow",
          primaryReason: "connector_requested_symbol_mismatch",
          details: ["connector requested ETH-PERP while query symbol is BTC"],
        },
      },
    });

    const payload = await fetchContractWhaleRawFlowDebug({ symbol: "BTC", range: "24h" });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/raw-flow-debug?symbol=BTC&range=24h",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload).toMatchObject({
      symbol: "BTC",
      range: "24h",
      config: {
        appRequestedSymbol: "ETH-PERP",
        querySymbol: "BTC",
      },
      normalizer: {
        connectorSymbolMismatch: true,
      },
      contractFlow1s: {
        exactSymbolRows: 0,
      },
      diagnosis: {
        primaryReason: "connector_requested_symbol_mismatch",
      },
    });
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
      net_direction: "abs1000",
      discord_sent: "true",
      window_sec: "15",
      exchange: "binance",
      limit: 50,
    });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/history?symbol=BTC&severity=critical&signal_type=aggressive_buy&direction=buy&net_direction=abs1000&discord_sent=true&window_sec=15&exchange=binance&limit=50",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload.items).toHaveLength(1);
    expect(payload.meta).toMatchObject({
      exchange: "coinbase",
      marketType: "perp",
      exchangeStatus: "spot_only",
      reason: "coinbase_perp_disabled",
    });
  });

  it("uses requested symbol and drops mismatched explicit symbols in history response", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [
          contractWhaleItem({
            id: undefined,
            ts: 1_700_000_100_000,
            windowSec: 60,
            symbol: undefined,
            totalVolumeBtc: 16869,
            netVolumeBtc: 610,
            totalNotionalUsd: 28_000_000,
            orderPriceUsd: undefined,
            currentMarketPriceUsd: undefined,
            priceDeviationPct: undefined,
          }),
          contractWhaleItem({ id: "btc-row", symbol: "BTC" }),
        ],
      },
    });

    const payload = await fetchContractWhaleHistory({ symbol: "ETH", limit: 20 });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/history?symbol=ETH&limit=20",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload.summary.trend60s.symbol).toBe("ETH");
    expect(payload.items).toHaveLength(1);
    expect(payload.items[0]).toMatchObject({
      id: "ETH-60-1700000100000",
      symbol: "ETH",
      baseAsset: "ETH",
      quantityUnit: "ETH",
      totalVolumeBtc: 16869,
      netVolumeBtc: 610,
    });
    expect(payload.items[0].triggerPriceUsd).toBeCloseTo(28_000_000 / 16_869, 6);
  });

  it("rejects hanging requests after the configured timeout", async () => {
    vi.useFakeTimers();
    try {
      axios.get.mockImplementationOnce((_url, config = {}) => {
        expect(config.signal).toBeDefined();
        return new Promise(() => {});
      });

      const request = fetchJsonWithTimeout("/api/contract-whale/summary?symbol=BTC", {
        timeoutMs: 10,
      });
      const rejection = expect(request).rejects.toMatchObject({ code: "ERR_CWM_TIMEOUT" });

      await vi.advanceTimersByTimeAsync(11);

      await rejection;
    } finally {
      vi.useRealTimers();
    }
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

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/events?symbol=BTC&limit=12",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
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

  it("uses requested symbol and drops mismatched explicit symbols in main-force events", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [
          { id: 8, startedAt: 1_700_000_000_000, regimeType: "range_rotation" },
          { id: 9, symbol: "BTC", startedAt: 1_700_000_100_000, regimeType: "main_force_long_build" },
        ],
      },
    });

    const payload = await fetchContractWhaleEvents({ symbol: "ETH", limit: 12 });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/events?symbol=ETH&limit=12",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload.items).toHaveLength(1);
    expect(payload.items[0]).toMatchObject({
      id: 8,
      symbol: "ETH",
      regimeType: "range_rotation",
    });
  });

  it("fetches final events as canonical read-only event projections", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        count: 1,
        items: [
          {
            eventId: "cwm-event:BTC:downside_absorption:1700000000000",
            symbol: "BTC",
            eventType: "downside_absorption",
            startTime: 1_700_000_000_000,
            endTime: 1_700_000_015_000,
            status: "closed",
            windowSec: 15,
            rawVolume: 4_876,
            impactScore: 2.14,
            zScore: 2.14,
            percentile: 93,
            normalizedScore: 0.88,
            normalizedStrength: "EXTREME",
            impactLevel: "A",
            signalLevel: "L3",
            signalLabel: "HIGH IMPACT EVENT",
            volume: 4_876,
            netVolume: -4_619,
            notional: 313_000_000,
            price: 64_166,
            priceMovePct: 0.19,
            directionBias: "sell",
            dominance: 0.947,
            qualityScore: 0.81,
            mergeSimilarityScore: 0.84,
            falseEventFlags: [],
            sourceSignalIds: [
              "contract-whale:BTC:15:1700000000000:downside_absorption",
              "contract-whale:BTC:5:1700000000010:downside_absorption",
            ],
            sourceSignal: {
              id: "contract-whale:BTC:15:1700000000000:downside_absorption",
              ts: 1_700_000_015_000,
              symbol: "BTC",
              windowSec: 15,
              signalType: "downside_absorption",
              direction: "sell",
              severity: "medium",
              score: 51,
              mainForceScore: 51,
              spotScore: 59,
              contractScore: 47,
              totalVolumeBtc: 4_876,
              netVolumeBtc: -4_619,
              totalNotionalUsd: 313_000_000,
              dominance: 0.947,
              orderPriceUsd: 64_166,
              priceMovePct: 0.19,
              mergedFrom: ["contract-whale:BTC:5:1700000000010:downside_absorption"],
              eventLifecycle: {
                eventId: "cwm-event:BTC:downside_absorption:1700000000000",
                status: "closed",
                startTime: 1_700_000_000_000,
                lastUpdateTime: 1_700_000_015_000,
                volumeAccumulated: 4_876,
                updateCount: 2,
              },
              eventQuality: {
                qualityScore: 0.81,
                mergeSimilarityScore: 0.84,
                valid: true,
                falseEventFlags: [],
              },
            },
          },
        ],
      },
    });

    const payload = await fetchFinalEvents({ symbol: "BTC", limit: 12 });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/final-events?symbol=BTC&limit=12",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload.count).toBe(1);
    expect(payload.items[0]).toMatchObject({
      id: "cwm-event:BTC:downside_absorption:1700000000000",
      finalEventId: "cwm-event:BTC:downside_absorption:1700000000000",
      symbol: "BTC",
      signalType: "downside_absorption",
      windowSec: 15,
      totalVolumeBtc: 4_876,
      netVolumeBtc: -4_619,
      totalNotionalUsd: 313_000_000,
      dominance: 0.947,
      eventLifecycle: {
        eventId: "cwm-event:BTC:downside_absorption:1700000000000",
        status: "closed",
        startTime: 1_700_000_000_000,
        lastUpdateTime: 1_700_000_015_000,
      },
      eventQuality: {
        qualityScore: 0.81,
        mergeSimilarityScore: 0.84,
        valid: true,
      },
      finalEvent: expect.objectContaining({
        rawVolume: 4_876,
        impactScore: 2.14,
        zScore: 2.14,
        percentile: 93,
        normalizedScore: 0.88,
        normalizedStrength: "EXTREME",
        impactLevel: "A",
        signalLevel: "L3",
        signalLabel: "HIGH IMPACT EVENT",
        sourceSignalIds: [
          "contract-whale:BTC:15:1700000000000:downside_absorption",
          "contract-whale:BTC:5:1700000000010:downside_absorption",
        ],
      }),
    });
  });

  it("fetches contract events as the default historical event feed", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        items: [
          {
            eventId: "contract-event:btc:1",
            sourceSignalId: "contract-whale:BTC:15:1700000000000:buy",
            symbol: "BTC",
            price: 64_166,
            ts: 1_700_000_015_000,
            status: "active",
            signalType: "aggressive_buy",
            severity: "Medium",
            windowSec: 15,
            volumeBtc: 4_876,
            notionalUsd: 313_000_000,
            netVolumeBtc: 4_619,
            direction: "buy",
            netDirection: "net_buy",
            mainForceScore: 51,
            exchangeSpotCount: 1,
            exchangeContractCount: 2,
            source: "contract_whale_signals",
            isRetentionProtected: true,
            retentionReason: "net_volume_ge_500_btc",
          },
        ],
        nextCursor: "100",
        hasMore: true,
        limit: 100,
        range: "24h",
        serverTime: 1_700_000_100_000,
        lastEventTs: 1_700_000_000_000,
      },
    });

    const payload = await fetchContractEvents({ symbol: "BTC", range: "24h", limit: 100 });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-events?symbol=BTC&range=24h&limit=100",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload).toMatchObject({
      items: [
        expect.objectContaining({
          eventId: "contract-event:btc:1",
          sourceSignalId: "contract-whale:BTC:15:1700000000000:buy",
          symbol: "BTC",
          status: "active",
          signalType: "aggressive_buy",
          severity: "Medium",
          volumeBtc: 4_876,
          isRetentionProtected: true,
          retentionReason: "net_volume_ge_500_btc",
          isVisible: true,
          hiddenReason: null,
        }),
      ],
      nextCursor: "100",
      hasMore: true,
      limit: 100,
      range: "24h",
      serverTime: 1_700_000_100_000,
      lastEventTs: 1_700_000_000_000,
    });
  });

  it("fetches contract event debug counts for history diagnostics", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        symbol: "BTC",
        range: "24h",
        generatedAt: "2026-06-27T00:00:00Z",
        db: {
          contractWhaleSignalsTotal24h: 12,
          contractWhaleSignalsBtc24h: 8,
          oldestTs: 1_700_000_000_000,
          newestTs: 1_700_000_100_000,
        },
        apiQuery: {
          matchedBeforeFilter: 8,
          matchedAfterSymbolFilter: 8,
          matchedAfterRangeFilter: 8,
          matchedAfterSeverityFilter: null,
          matchedAfterWindowFilter: null,
          matchedAfterDirectionFilter: null,
          returnedItems: 1,
          limit: 100,
        },
        visibility: {
          visibleCount: 1,
          hiddenCount: 7,
          hiddenReasons: {
            priceDeviationGt5pct: 6,
            missingPrice: 0,
            badQuality: 1,
            disabledMonitor: 0,
            unknown: 0,
          },
        },
        latest: {
          latestCount: 2,
          latestSymbols: ["BTC", "BTC"],
        },
        finalEventsV2: {
          activeCount: 1,
          closedCount: 0,
        },
        latestVsHistory: [
          {
            latestEventId: "latest-1",
            symbol: "BTC",
            ts: 1_700_000_090_000,
            existsInHistory: true,
            historyEventId: "contract-event:btc:1",
            notInHistoryReason: null,
          },
          {
            latestEventId: "latest-2",
            symbol: "BTC",
            ts: 1_700_000_095_000,
            existsInHistory: false,
            historyEventId: null,
            notInHistoryReason: "latest_snapshot_not_persisted_yet",
          },
        ],
        finalEventsProjection: {
          source: "contract_whale_signals",
          rawSignals: 8,
          afterFilter: 1,
          mergedEvents: 1,
          active: 1,
          closed: 0,
          range: "24h",
        },
      },
    });

    const payload = await fetchContractEventDebugCounts({
      symbol: "BTC",
      range: "24h",
      includeHidden: true,
    });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-events/debug-counts?symbol=BTC&range=24h&include_hidden=true",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload).toMatchObject({
      symbol: "BTC",
      range: "24h",
      db: {
        contractWhaleSignalsBtc24h: 8,
      },
      apiQuery: {
        returnedItems: 1,
      },
      visibility: {
        visibleCount: 1,
        hiddenCount: 7,
        hiddenReasons: {
          priceDeviationGt5pct: 6,
          badQuality: 1,
        },
      },
      latest: {
        latestCount: 2,
      },
      finalEventsV2: {
        activeCount: 1,
        closedCount: 0,
      },
      latestVsHistory: expect.arrayContaining([
        expect.objectContaining({
          latestEventId: "latest-2",
          existsInHistory: false,
          notInHistoryReason: "latest_snapshot_not_persisted_yet",
        }),
      ]),
    });
  });

  it("fetches final-events-v2 with paged active and closed arrays", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        active: [{ eventId: "active-1", symbol: "BTC" }],
        closed: [{ eventId: "closed-1", symbol: "BTC" }],
        nextCursor: "200",
        hasMore: true,
        limit: 100,
        range: "24h",
        serverTime: 1_700_000_200_000,
        lastEventTs: 1_700_000_010_000,
      },
    });

    const payload = await fetchFinalEventsV2({ symbol: "BTC", limit: 100, range: "24h" });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/final-events-v2?symbol=BTC&limit=100&range=24h",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload).toMatchObject({
      active: [expect.objectContaining({ eventId: "active-1", symbol: "BTC" })],
      closed: [expect.objectContaining({ eventId: "closed-1", symbol: "BTC" })],
      nextCursor: "200",
      hasMore: true,
      limit: 100,
      range: "24h",
      serverTime: 1_700_000_200_000,
      lastEventTs: 1_700_000_010_000,
    });
  });

  it("maps latest latency metadata for contract whale snapshots", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        summary: {},
        items: [],
        serverTime: 1_700_000_300_000,
        maxTs: 1_700_000_290_000,
        maxAgeSec: 10,
        staleCount: 2,
      },
    });

    const payload = await fetchContractWhaleLatest(20, "BTC");

    expect(payload).toMatchObject({
      serverTime: 1_700_000_300_000,
      maxTs: 1_700_000_290_000,
      maxAgeSec: 10,
      staleCount: 2,
    });
  });

  it("fetches latency debug diagnostics for contract whale layers", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        symbol: "BTC",
        range: "1h",
        serverTime: 1_700_000_300_000,
        latest: { count: 2, maxTs: 1_700_000_299_000, ageSec: 1, staleCount: 0 },
        contractEvents: { count: 2, maxEventTs: 1_700_000_298_000, lagVsLatestSec: 1 },
        finalEventsV2: { activeCount: 1, closedCount: 0, projectionLagSec: 2 },
        flow: { updatedAt: 1_700_000_299_500, flowLagSec: 0 },
        diagnosis: { layer: "ok", reason: "in_sync" },
      },
    });

    const payload = await fetchContractWhaleLatencyDebug({ symbol: "BTC", range: "1h" });

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/latency-debug?symbol=BTC&range=1h",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload).toMatchObject({
      symbol: "BTC",
      range: "1h",
      serverTime: 1_700_000_300_000,
      latest: expect.objectContaining({ ageSec: 1 }),
      contractEvents: expect.objectContaining({ lagVsLatestSec: 1 }),
      finalEventsV2: expect.objectContaining({ projectionLagSec: 2 }),
      diagnosis: expect.objectContaining({ layer: "ok", reason: "in_sync" }),
    });
  });

  it("fetches retention status for contract whale cleanup diagnostics", async () => {
    axios.get.mockResolvedValueOnce({
      data: {
        flowRetentionDays: 14,
        signalRetentionDays: 365,
        signalProtectSeverityS: true,
        signalProtectNetVolumeBtc: 500,
        cleanupIntervalHours: 1,
        tables: {
          contractWhaleSignals: {
            rowCount: 99,
            protectedSCount: 3,
            protectedNetVolumeCount: 5,
          },
        },
      },
    });

    const payload = await fetchContractRetentionStatus();

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-retention-status",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
    expect(payload).toMatchObject({
      flowRetentionDays: 14,
      signalRetentionDays: 365,
      signalProtectSeverityS: true,
      signalProtectNetVolumeBtc: 500,
      cleanupIntervalHours: 1,
      tables: {
        contractWhaleSignals: {
          rowCount: 99,
          protectedSCount: 3,
          protectedNetVolumeCount: 5,
        },
      },
    });
  });

  it("uses latest limit 50 by default", async () => {
    axios.get.mockResolvedValueOnce({ data: { summary: {}, items: [] } });

    await fetchContractWhaleLatest();

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/latest?limit=50&symbol=BTC",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
  });

  it("fetches ETH summary with symbol query", async () => {
    axios.get.mockResolvedValueOnce({ data: { status: "calm", enabled: true } });

    await fetchContractWhaleSummary("ETH");

    expect(axios.get).toHaveBeenCalledWith(
      "/api/contract-whale/summary?symbol=ETH",
      expect.objectContaining({ signal: expect.any(Object) }),
    );
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

  it("maps volume display semantics without changing the underlying math", () => {
    const signal = normalizeContractWhaleSignal({
      id: "volume-semantics-signal",
      symbol: "BTC",
      windowSec: 15,
      totalVolumeBtc: 4280,
      netVolumeBtc: -620,
      exchanges: [
        { exchange: "binance" },
        { exchange: "bitfinex" },
      ],
      mergedFrom: [
        "contract-whale:BTC:5:1700000000010:buy",
        "contract-whale:BTC:15:1700000000000:buy",
      ],
    });

    expect(signal.displayVolumeLabel).toBe("窗口总流量 BTC");
    expect(signal.volumeSemantics).toBe("single_window_bidirectional_cross_exchange");
    expect(signal.displayVolumeBtc).toBe(4280);
    expect(signal.buyVolumeBtc).toBe(1830);
    expect(signal.sellVolumeBtc).toBe(2450);
    expect(signal.isBidirectionalVolume).toBe(true);
    expect(signal.isCrossExchangeAggregated).toBe(true);
    expect(signal.isLifecycleAccumulated).toBe(false);
    expect(signal.sourceExchangeCount).toBe(2);
    expect(signal.sourceExchanges).toEqual(["binance", "bitfinex"]);
    expect(signal.mergedWindowsSec).toEqual([5, 15]);
  });

  it("derives impact mapping from legacy percentile and threshold metadata when normalized fields are absent", () => {
    const signal = normalizeContractWhaleSignal({
      id: "legacy-impact-signal",
      symbol: "BTC",
      windowSec: 15,
      totalVolumeBtc: 4820,
      dynamicMultiple: 9.4,
      dynamicThresholdLevel: "critical",
      percentileLevel: 99.9,
    });

    expect(signal.impactScore).toBe(9.4);
    expect(signal.percentile).toBe(99.9);
    expect(signal.impactLevel).toBe("S");
    expect(signal.signalLevel).toBe("S");
    expect(signal.signalLabel).toBe("SHOCK IMPACT EVENT");
    expect(signal.normalizedStrength).toBe("EXTREME");
  });

  it("normalizes signal cluster and persistence metadata", () => {
    const signal = normalizeContractWhaleSignal({
      id: "clustered-signal",
      symbol: "BTC",
      cluster: {
        clusterId: "cwm-cluster:BTC:buy:14166666",
        signalCount: 4,
        dominantIntent: "liquidity_probe_buy",
        startedAt: 1_700_000_000_000,
        updatedAt: 1_700_000_090_000,
        durationMs: 90_000,
        intensity: 0.91,
        priceRangePct: 0.18,
      },
      persistence: {
        persistenceScore: 0.82,
        signalHalfLifeMs: 60_000,
        regimeStability: 0.75,
        redundantWithPrevious: true,
        redundantReason: "same_intent_within_60s",
      },
      whaleAction: {
        ts: 1_700_000_090_000,
        symbol: "BTC",
        actionType: "aggressive_buy",
        volume: 2400,
        priceImpact: 0.22,
        exchange: "binance",
      },
      trajectory: {
        trajectoryId: "whale-trajectory:cwm-cluster:BTC:buy:14166666",
        startTs: 1_700_000_000_000,
        endTs: 1_700_000_090_000,
        durationMs: 90_000,
        actions: [
          { ts: 1_700_000_000_000, symbol: "BTC", actionType: "liquidity_probe", volume: 1000, priceImpact: 0.08, exchange: "binance" },
          { ts: 1_700_000_090_000, symbol: "BTC", actionType: "aggressive_buy", volume: 2400, priceImpact: 0.22, exchange: "bitfinex" },
        ],
        intent: "accumulation",
        regimePath: ["manipulation", "accumulation"],
        stealthProfile: {
          gamma: 0.73,
          fragmentation: 0.66,
          entropy: 0.82,
          crossExchangeDispersion: 0.33,
        },
        aggressivenessCurve: [0.41, 0.94],
        conclusion: "连续买方压力和承接行为占优，疑似主力分批吸筹。",
      },
    });

    expect(signal.cluster).toMatchObject({
      clusterId: "cwm-cluster:BTC:buy:14166666",
      signalCount: 4,
      dominantIntent: "liquidity_probe_buy",
      durationMs: 90_000,
      intensity: 0.91,
      priceRangePct: 0.18,
    });
    expect(signal.persistence).toMatchObject({
      persistenceScore: 0.82,
      signalHalfLifeMs: 60_000,
      regimeStability: 0.75,
      redundantWithPrevious: true,
      redundantReason: "same_intent_within_60s",
    });
    expect(signal.whaleAction).toMatchObject({
      actionType: "aggressive_buy",
      volume: 2400,
      priceImpact: 0.22,
      exchange: "binance",
    });
    expect(signal.trajectory).toMatchObject({
      trajectoryId: "whale-trajectory:cwm-cluster:BTC:buy:14166666",
      intent: "accumulation",
      durationMs: 90_000,
      regimePath: ["manipulation", "accumulation"],
      stealthProfile: expect.objectContaining({ gamma: 0.73 }),
      aggressivenessCurve: [0.41, 0.94],
    });
  });

  it("normalizes platform and market statuses without using flow volume as health", () => {
    expect(normalizePlatformStatus({ platformEnabled: false, status: "disabled" })).toMatchObject({
      key: "disabled",
      label: "未启用",
    });
    expect(normalizePlatformStatus({ platformEnabled: true, status: "spot_only" })).toMatchObject({
      key: "spot_only",
      label: "现货专用",
    });
    expect(normalizeMarketStatus({ enabled: true, status: "active", role: "confirmation" }, "perp")).toMatchObject({
      key: "waiting_for_data",
      label: "已启用 / 等待数据",
    });
    expect(normalizeMarketStatus({ enabled: true, status: "active", role: "primary", lastTradeAt: 1 }, "perp")).toMatchObject({
      key: "active",
      label: "运行中",
    });
    expect(normalizeMarketStatus({ enabled: true, status: "enabled", role: "spot_confirmation" }, "spot")).toMatchObject({
      key: "spot_only",
      label: "现货确认源",
    });
  });
});

function contractWhaleItem(overrides = {}) {
  return {
    id: "contract-whale:BTC:15:1700000000000:buy",
    ts: 1_700_000_000_000,
    symbol: "BTC",
    windowSec: 15,
    signalType: "aggressive_buy",
    direction: "buy",
    severity: "s",
    score: 94,
    mainForceScore: 87,
    spotScore: 81,
    contractScore: 94,
    totalVolumeBtc: 4820,
    netVolumeBtc: 3260,
    totalNotionalUsd: 337_000_000,
    dominance: 0.676,
    orderPriceUsd: 69_917,
    currentMarketPriceUsd: 70_000,
    priceDeviationPct: 0.1186,
    priceDeviationFiltered: false,
    priceMovePct: 0.31,
    priceMove15sPct: 0.31,
    priceResponseType: "trend_follow_up",
    mainExchange: "binance",
    dominantVenueNetContributionShare: 0.986,
    dynamicMultiple: 9.4,
    dynamicBaselineBtc: 512,
    dynamicThresholdLevel: "critical",
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
    scoreBreakdown: {
      volumeScore: 23.6,
      notionalScore: 10.5,
      dynamicAnomalyScore: 18.8,
      directionalStrengthScore: 10.6,
      priceResponseScore: 15,
      multiSourceScore: 8,
      dataQualityScore: 4.6,
      dominantVenueScore: 4.8,
      oiContextScore: 4,
      penaltyScore: -10,
      finalScore: 89.9,
    },
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
    spotConfirmation: {
      status: "confirmed",
      confirmationType: "confirms_contract_direction",
      direction: "buy",
      score: 81,
      latestSignalId: "spot-whale:BTC:15:1700000000000:buy",
      latestSignalAt: 1_700_000_000_000,
      signalType: "spot_aggressive_buy",
      severity: "high",
      totalVolumeBtc: 820,
      netVolumeBtc: 610,
      dominance: 0.744,
      coinbasePremiumPct: 0.018,
      finalResult: "现货主动买入跟随合约方向",
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
    discordWouldSend: true,
    discordReason: "critical_or_s_gate",
    finalResult: "多平台主动买入爆发，疑似主力合约拉盘",
    mergedFrom: ["contract-whale:BTC:5:1700000000000:buy"],
    cluster: {
      clusterId: "cwm-cluster:BTC:buy:14166666",
      signalCount: 3,
      dominantIntent: "liquidity_probe_buy",
      startedAt: 1_700_000_000_000,
      updatedAt: 1_700_000_090_000,
      durationMs: 90_000,
      intensity: 0.91,
      priceRangePct: 0.18,
    },
    persistence: {
      persistenceScore: 0.82,
      signalHalfLifeMs: 60_000,
      regimeStability: 0.67,
      redundantWithPrevious: true,
      redundantReason: "same_intent_within_60s",
    },
    whaleAction: {
      ts: 1_700_000_000_000,
      symbol: "BTC",
      actionType: "aggressive_buy",
      volume: 3260,
      priceImpact: 0.31,
      exchange: "binance",
    },
    trajectory: {
      trajectoryId: "whale-trajectory:cwm-cluster:BTC:buy:14166666",
      startTs: 1_700_000_000_000,
      endTs: 1_700_000_090_000,
      durationMs: 90_000,
      actions: [
        { ts: 1_700_000_000_000, symbol: "BTC", actionType: "liquidity_probe", volume: 1000, priceImpact: 0.08, exchange: "binance" },
        { ts: 1_700_000_090_000, symbol: "BTC", actionType: "aggressive_buy", volume: 3260, priceImpact: 0.31, exchange: "bitfinex" },
      ],
      intent: "accumulation",
      regimePath: ["manipulation", "accumulation"],
      stealthProfile: {
        gamma: 0.73,
        fragmentation: 0.66,
        entropy: 0.82,
        crossExchangeDispersion: 0.33,
      },
      aggressivenessCurve: [0.41, 0.94],
      conclusion: "连续买方压力和承接行为占优，疑似主力分批吸筹。",
    },
    liquidationForce: {
      activeZone: "short_squeeze_zone",
      primaryDriver: "liquidation_cascade",
      shortSqueezePressure: 78,
      longLiquidationPressure: 12,
      stopHuntProbability: 66,
      cascadeIntensity: 73,
      estimatedForcedSizeUsd: 29_400_000,
      flowAttribution: {
        whalePct: 0.42,
        retailPct: 0.12,
        liquidationPct: 0.46,
        dominantDriver: "liquidation_cascade",
      },
      priceImpact: {
        whaleImpact: 0.12,
        liquidationCascade: 0.18,
        stopLossSweep: 0.08,
        passiveAbsorption: -0.02,
      },
      zones: [
        {
          side: "short_liquidation",
          lowPriceUsd: 70_100,
          highPriceUsd: 70_400,
          estimatedSizeUsd: 29_400_000,
          intensity: 78,
          reason: "upside stop-loss and short liquidation cluster",
        },
      ],
    },
    marketDriver: {
      primaryDriver: "derivatives_pressure",
      marketState: "short_squeeze_regime",
      whaleIntentPct: 0.28,
      liquidityForcingPct: 0.18,
      derivativesPressurePct: 0.42,
      reflexivityPct: 0.12,
      components: [
        { key: "whale_intent", score: 72, weightPct: 0.28 },
        { key: "liquidity_forcing", score: 54, weightPct: 0.18 },
        { key: "derivatives_pressure", score: 86, weightPct: 0.42 },
        { key: "reflexivity_feedback", score: 44, weightPct: 0.12 },
      ],
      interpretation: "价格主要受清算、OI/funding 或衍生品强制流推动，当前更像被迫交易而非纯主力主动流。",
    },
    ...overrides,
  };
}
