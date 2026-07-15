import axios from "axios";

const DEFAULT_SYMBOL = "BTCUSDT";
const lastSuccessByRequest = new Map();

export async function fetchLiquidationCascade(symbol = DEFAULT_SYMBOL) {
  return fetchWithState(
    "/api/liquidation/cascade",
    { symbol },
    (payload) => normalizeCascade(payload, symbol),
    isValidCascade,
  );
}

export async function fetchLiquidationLeverageMap(symbol = DEFAULT_SYMBOL) {
  return fetchWithState(
    "/api/liquidation/leverage-map",
    { symbol },
    (payload) => normalizeLeverageMap(payload, symbol),
    isValidLeverageMap,
  );
}

export async function fetchLiquidationLiquidityGap(symbol = DEFAULT_SYMBOL) {
  return fetchWithState(
    "/api/liquidation/liquidity-gap",
    { symbol },
    (payload) => normalizeLiquidityGap(payload, symbol),
    isValidLiquidityGap,
  );
}

export async function fetchMarketRegime(symbol = DEFAULT_SYMBOL) {
  return fetchWithState(
    "/api/regime/latest",
    { symbol },
    (payload) => normalizeRegime(payload, symbol),
    isValidRegime,
  );
}

export async function fetchBtcStructure(symbol = DEFAULT_SYMBOL) {
  return fetchWithState(
    "/api/btc/structure",
    { symbol },
    (payload) => normalizeBtcStructure(payload, symbol),
    isValidBtcStructure,
  );
}

async function fetchWithState(path, params, normalize, validate) {
  const fetchedAtMs = Date.now();
  const requestKey = `${path}:${normalizeSymbol(params?.symbol)}`;
  try {
    const response = await axios.get(path, { params });
    if (!validate(response.data)) {
      return unavailableResult(requestKey, fetchedAtMs, "MALFORMED_RESPONSE");
    }
    const data = normalize(response.data);
    if (!symbolsMatch(data?.symbol, params?.symbol)) {
      return unavailableResult(requestKey, fetchedAtMs, "SYMBOL_MISMATCH");
    }
    lastSuccessByRequest.set(requestKey, fetchedAtMs);
    return {
      data,
      error: null,
      state: {
        phase: "ready",
        source: "backend",
        fetchedAtMs,
        lastSuccessAtMs: fetchedAtMs,
      },
    };
  } catch (error) {
    return unavailableResult(
      requestKey,
      fetchedAtMs,
      error?.response?.data?.reason || error?.message || "NETWORK_ERROR",
    );
  }
}

function unavailableResult(requestKey, fetchedAtMs, error) {
  return {
    data: null,
    error,
    state: {
      phase: "unavailable",
      source: null,
      fetchedAtMs,
      lastSuccessAtMs: lastSuccessByRequest.get(requestKey) ?? null,
    },
  };
}

export function normalizeCascade(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  const symbol = normalizeSymbol(payload.symbol || requestedSymbol);
  return {
    symbol,
    cascadeProbability: clamp01(payload.cascadeProbability),
    status: stringOrNull(payload.status)?.toUpperCase() ?? null,
    direction: stringOrNull(payload.direction)?.toUpperCase() ?? null,
    estimatedMove: stringOrNull(payload.estimatedMove),
    timeWindow: stringOrNull(payload.timeWindow),
    riskZone: Array.isArray(payload.riskZone) ? payload.riskZone.map(numberOrNull) : null,
    signals: normalizeStringArray(payload.signals),
    components: normalizeCascadeComponents(payload.components),
    readOnly: booleanOrNull(payload.readOnly),
    runtimeModified: booleanOrNull(payload.runtimeModified),
  };
}

export function normalizeLeverageMap(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    heatmap: Array.isArray(payload.heatmap) ? payload.heatmap.map(normalizeLeverageLevel) : [],
    highRiskZones: Array.isArray(payload.highRiskZones)
      ? payload.highRiskZones.map(normalizePriceZone)
      : [],
    readOnly: booleanOrNull(payload.readOnly),
    runtimeModified: booleanOrNull(payload.runtimeModified),
  };
}

export function normalizeLiquidityGap(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    belowPrice: clamp01(payload.belowPrice),
    abovePrice: clamp01(payload.abovePrice),
    dominantGap: stringOrNull(payload.dominantGap)?.toUpperCase() ?? null,
    signals: normalizeStringArray(payload.signals),
    readOnly: booleanOrNull(payload.readOnly),
    runtimeModified: booleanOrNull(payload.runtimeModified),
  };
}

export function normalizeRegime(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    regime: stringOrNull(payload.regime)?.toUpperCase() ?? null,
    confidence: clamp01(payload.confidence),
    directionBias: stringOrNull(payload.directionBias)?.toUpperCase() ?? null,
    signals: normalizeStringArray(payload.signals),
    metrics: normalizeMetricMap(payload.metrics),
    readOnly: booleanOrNull(payload.readOnly),
    runtimeModified: booleanOrNull(payload.runtimeModified),
  };
}

export function normalizeBtcStructure(payload = {}, requestedSymbol = DEFAULT_SYMBOL) {
  return {
    symbol: normalizeSymbol(payload.symbol || requestedSymbol),
    regime: stringOrNull(payload.regime)?.toUpperCase() ?? null,
    bias: stringOrNull(payload.bias)?.toUpperCase() ?? null,
    confidence: clamp01(payload.confidence),
    structureScore: clamp01(payload.structureScore),
    liquidationCascadeProbability: clamp01(payload.liquidationCascadeProbability),
    gammaPressure: clamp01(payload.gammaPressure),
    signals: normalizeStringArray(payload.signals),
    metrics: normalizeMetricMap(payload.metrics),
    readOnly: booleanOrNull(payload.readOnly),
    runtimeModified: booleanOrNull(payload.runtimeModified),
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
    price: numberOrNull(level.price),
    side: stringOrNull(level.side),
    intensity: clamp01(level.intensity),
    notionalUsd: numberOrNull(level.notionalUsd),
    distanceBps: numberOrNull(level.distanceBps),
  };
}

function normalizePriceZone(zone = {}) {
  return {
    low: numberOrNull(zone.low),
    high: numberOrNull(zone.high),
    strength: clamp01(zone.strength),
    side: stringOrNull(zone.side),
  };
}

function normalizeMetricMap(metrics = {}) {
  return Object.fromEntries(
    Object.entries(metrics || {}).map(([key, value]) => [key, numberOrNull(value)]),
  );
}

function normalizeSymbol(value) {
  return String(value || DEFAULT_SYMBOL).toUpperCase();
}

function symbolsMatch(observedSymbol, requestedSymbol) {
  const observed = canonicalSymbol(observedSymbol);
  const requested = canonicalSymbol(requestedSymbol);
  return Boolean(observed && requested && observed === requested);
}

function canonicalSymbol(value) {
  let symbol = String(value || "").trim().toUpperCase();
  if (!symbol) return null;
  if (symbol.startsWith("T") && symbol.includes("F0")) symbol = symbol.slice(1);
  symbol = symbol.split(/[-_/:]/)[0];
  for (const suffix of ["PERP", "SWAP", "USDT", "USDC", "USD", "F0"]) {
    symbol = symbol.replace(new RegExp(`${suffix}$`), "");
  }
  return symbol || null;
}

function normalizeStringArray(value) {
  return Array.isArray(value) ? value.map((item) => String(item)).filter(Boolean) : [];
}

function numberOrNull(value) {
  if (value === null || value === undefined || value === "") {
    return null;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function clamp01(value) {
  const number = numberOrNull(value);
  return number === null ? null : Math.min(1, Math.max(0, number));
}

function booleanOrNull(value) {
  return typeof value === "boolean" ? value : null;
}

function stringOrNull(value) {
  return typeof value === "string" && value.trim() ? value : null;
}

function isValidCascade(payload) {
  return isRecord(payload) && numberOrNull(payload.cascadeProbability) !== null && stringOrNull(payload.status) !== null && stringOrNull(payload.direction) !== null;
}

function isValidLeverageMap(payload) {
  return isRecord(payload) && Array.isArray(payload.heatmap) && Array.isArray(payload.highRiskZones);
}

function isValidLiquidityGap(payload) {
  return isRecord(payload) && numberOrNull(payload.belowPrice) !== null && numberOrNull(payload.abovePrice) !== null && stringOrNull(payload.dominantGap) !== null;
}

function isValidRegime(payload) {
  return isRecord(payload) && stringOrNull(payload.regime) !== null && stringOrNull(payload.directionBias) !== null;
}

function isValidBtcStructure(payload) {
  return isRecord(payload) && stringOrNull(payload.regime) !== null && stringOrNull(payload.bias) !== null;
}

function isRecord(value) {
  return Boolean(value && typeof value === "object" && !Array.isArray(value));
}
