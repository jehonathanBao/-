import axios from "axios";

const DEFAULT_SYMBOL = "BTCUSDT";

export async function fetchLiquidationCascade(symbol = DEFAULT_SYMBOL) {
  return fetchWithFallback(
    "/api/liquidation/cascade",
    { symbol },
    (payload) => normalizeCascade(payload, symbol),
    fallbackCascade(symbol),
  );
}

export async function fetchLiquidationLeverageMap(symbol = DEFAULT_SYMBOL) {
  return fetchWithFallback(
    "/api/liquidation/leverage-map",
    { symbol },
    (payload) => normalizeLeverageMap(payload, symbol),
    fallbackLeverageMap(symbol),
  );
}

export async function fetchLiquidationLiquidityGap(symbol = DEFAULT_SYMBOL) {
  return fetchWithFallback(
    "/api/liquidation/liquidity-gap",
    { symbol },
    (payload) => normalizeLiquidityGap(payload, symbol),
    fallbackLiquidityGap(symbol),
  );
}

export async function fetchMarketRegime(symbol = DEFAULT_SYMBOL) {
  return fetchWithFallback(
    "/api/regime/latest",
    { symbol },
    (payload) => normalizeRegime(payload, symbol),
    fallbackRegime(symbol),
  );
}

export async function fetchManipulationAssessment(symbol = DEFAULT_SYMBOL) {
  return fetchWithFallback(
    "/api/manipulation/latest",
    { symbol },
    (payload) => normalizeManipulation(payload, symbol),
    fallbackManipulation(symbol),
  );
}

export async function fetchBtcStructure(symbol = DEFAULT_SYMBOL) {
  return fetchWithFallback(
    "/api/btc/structure",
    { symbol },
    (payload) => normalizeBtcStructure(payload, symbol),
    fallbackBtcStructure(symbol),
  );
}

export async function fetchAltcoinManipulation(symbol) {
  return fetchWithFallback(
    "/api/altcoin/manipulation",
    { symbol },
    (payload) => normalizeAltcoinManipulation(payload, symbol),
    fallbackAltcoinManipulation(symbol),
  );
}

export async function fetchAltcoinSignals(symbol) {
  return fetchWithFallback(
    "/api/altcoin/signals",
    { symbol },
    (payload) => normalizeAltcoinManipulation(payload, symbol),
    fallbackAltcoinManipulation(symbol),
  );
}

export async function fetchMarketSignalAssessment(symbol = DEFAULT_SYMBOL) {
  return fetchWithFallback(
    "/api/signal/latest",
    { symbol },
    (payload) => normalizeMarketSignal(payload, symbol),
    fallbackMarketSignal(symbol),
  );
}

async function fetchWithFallback(path, params, normalize, fallback) {
  try {
    const response = await axios.get(path, { params });
    return { data: normalize(response.data || {}), error: null };
  } catch (error) {
    return {
      data: fallback,
      error: error?.response?.data?.reason || error?.message || "NETWORK_ERROR",
    };
  }
}

export function normalizeCascade(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  const symbol = normalizeSymbol(payload.symbol || requestedSymbol);
  return {
    symbol,
    cascadeProbability: clamp01(payload.cascadeProbability),
    status: String(payload.status || "CALM").toUpperCase(),
    direction: String(payload.direction || "NEUTRAL").toUpperCase(),
    estimatedMove: payload.estimatedMove || "< 0.5%",
    timeWindow: payload.timeWindow || "no active cascade window",
    riskZone: Array.isArray(payload.riskZone) ? payload.riskZone.map(numberOrZero) : null,
    signals: normalizeStringArray(payload.signals),
    components: normalizeCascadeComponents(payload.components),
    readOnly: payload.readOnly !== false,
    runtimeModified: Boolean(payload.runtimeModified),
  };
}

export function normalizeLeverageMap(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    heatmap: Array.isArray(payload.heatmap) ? payload.heatmap.map(normalizeLeverageLevel) : [],
    highRiskZones: Array.isArray(payload.highRiskZones)
      ? payload.highRiskZones.map(normalizePriceZone)
      : [],
    readOnly: payload.readOnly !== false,
    runtimeModified: Boolean(payload.runtimeModified),
  };
}

export function normalizeLiquidityGap(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    belowPrice: clamp01(payload.belowPrice),
    abovePrice: clamp01(payload.abovePrice),
    dominantGap: String(payload.dominantGap || "NEUTRAL").toUpperCase(),
    signals: normalizeStringArray(payload.signals),
    readOnly: payload.readOnly !== false,
    runtimeModified: Boolean(payload.runtimeModified),
  };
}

export function normalizeRegime(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    regime: String(payload.regime || "ACCUMULATION").toUpperCase(),
    confidence: clamp01(payload.confidence),
    directionBias: String(payload.directionBias || "NEUTRAL").toUpperCase(),
    signals: normalizeStringArray(payload.signals),
    metrics: normalizeMetricMap(payload.metrics),
    readOnly: payload.readOnly !== false,
    runtimeModified: Boolean(payload.runtimeModified),
  };
}

export function normalizeManipulation(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    score: clamp01(payload.score),
    signals: normalizeStringArray(payload.signals),
    metrics: normalizeMetricMap(payload.metrics),
    readOnly: payload.readOnly !== false,
    runtimeModified: Boolean(payload.runtimeModified),
  };
}

export function normalizeBtcStructure(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    regime: String(payload.regime || "ACCUMULATION").toUpperCase(),
    bias: String(payload.bias || "NEUTRAL").toUpperCase(),
    confidence: clamp01(payload.confidence),
    structureScore: clamp01(payload.structureScore),
    liquidationCascadeProbability: clamp01(payload.liquidationCascadeProbability),
    gammaPressure: clamp01(payload.gammaPressure),
    signals: normalizeStringArray(payload.signals),
    metrics: normalizeMetricMap(payload.metrics),
    readOnly: payload.readOnly !== false,
    runtimeModified: Boolean(payload.runtimeModified),
  };
}

export function normalizeAltcoinManipulation(payload = {}, requestedSymbol = "ETHUSDT") {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    regime: String(payload.regime || "ACCUMULATION").toUpperCase(),
    bias: String(payload.bias || "NEUTRAL").toUpperCase(),
    confidence: clamp01(payload.confidence),
    manipulationScore: clamp01(payload.manipulationScore),
    oiSignalScore: clamp01(payload.oiSignalScore),
    volumeSignalScore: clamp01(payload.volumeSignalScore),
    fundingSignalScore: clamp01(payload.fundingSignalScore),
    priceSignalScore: clamp01(payload.priceSignalScore),
    pumpDumpScore: clamp01(payload.pumpDumpScore),
    signals: normalizeStringArray(payload.signals),
    riskTags: normalizeStringArray(payload.riskTags),
    metrics: normalizeMetricMap(payload.metrics),
    readOnly: payload.readOnly !== false,
    runtimeModified: Boolean(payload.runtimeModified),
  };
}

export function normalizeMarketSignal(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    regime: String(payload.regime || "ACCUMULATION").toUpperCase(),
    confidence: clamp01(payload.confidence),
    manipulationScore: clamp01(payload.manipulationScore),
    directionBias: String(payload.directionBias || "NEUTRAL").toUpperCase(),
    signals: normalizeStringArray(payload.signals),
    adjustedSignalStrength: clamp01(payload.adjustedSignalStrength),
    allowedSignalFamily: payload.allowedSignalFamily || "monitor_only",
    riskNote: payload.riskNote || "read-only market structure output",
    metrics: normalizeMetricMap(payload.metrics),
    readOnly: payload.readOnly !== false,
    runtimeModified: Boolean(payload.runtimeModified),
  };
}

function normalizeCascadeComponents(components = {}) {
  return {
    leverageConcentration: clamp01(components.leverageConcentration),
    liquidityGap: clamp01(components.liquidityGap),
    fundingStress: clamp01(components.fundingStress),
    triggerProximity: clamp01(components.triggerProximity),
    oiStress: clamp01(components.oiStress),
  };
}

function normalizeLeverageLevel(level = {}) {
  return {
    price: numberOrZero(level.price),
    side: level.side || "neutral",
    intensity: clamp01(level.intensity),
    notionalUsd: numberOrZero(level.notionalUsd),
    distanceBps: numberOrZero(level.distanceBps),
  };
}

function normalizePriceZone(zone = {}) {
  return {
    low: numberOrZero(zone.low),
    high: numberOrZero(zone.high),
    strength: clamp01(zone.strength),
    side: zone.side || "neutral",
  };
}

function normalizeMetricMap(metrics = {}) {
  return Object.fromEntries(
    Object.entries(metrics || {}).map(([key, value]) => [key, numberOrZero(value)]),
  );
}

function fallbackCascade(symbol) {
  return normalizeCascade(
    {
      symbol,
      status: "CALM",
      direction: "NEUTRAL",
      signals: ["CASCADE_UNAVAILABLE"],
      components: {},
      readOnly: true,
    },
    symbol,
  );
}

function fallbackLeverageMap(symbol) {
  return normalizeLeverageMap({ symbol, heatmap: [], highRiskZones: [], readOnly: true }, symbol);
}

function fallbackLiquidityGap(symbol) {
  return normalizeLiquidityGap({ symbol, dominantGap: "NEUTRAL", signals: [], readOnly: true }, symbol);
}

function fallbackRegime(symbol) {
  return normalizeRegime({ symbol, regime: "ACCUMULATION", directionBias: "NEUTRAL", readOnly: true }, symbol);
}

function fallbackManipulation(symbol) {
  return normalizeManipulation({ symbol, signals: [], readOnly: true }, symbol);
}

function fallbackBtcStructure(symbol) {
  return normalizeBtcStructure({ symbol, signals: [], readOnly: true }, symbol);
}

function fallbackAltcoinManipulation(symbol) {
  return normalizeAltcoinManipulation({ symbol, signals: [], readOnly: true }, symbol);
}

function fallbackMarketSignal(symbol) {
  return normalizeMarketSignal({ symbol, regime: "ACCUMULATION", directionBias: "NEUTRAL", readOnly: true }, symbol);
}

function normalizeSymbol(value) {
  return String(value || DEFAULT_SYMBOL).toUpperCase();
}

function normalizeStringArray(value) {
  return Array.isArray(value) ? value.map((item) => String(item)).filter(Boolean) : [];
}

function numberOrZero(value) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function clamp01(value) {
  return Math.min(1, Math.max(0, numberOrZero(value)));
}
