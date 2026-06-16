import axios from "axios";

const fallbackDashboard = {
  ts: null,
  symbol: "BTC",
  currentPriceUsd: null,
  dataStatus: "unavailable",
  readOnly: true,
  live: false,
  marketStress: {
    stressScore: 0,
    liquidityField: 0,
    gammaField: 0,
    liquidationField: 0,
    cascadeField: 0,
    instabilityIndex: 0,
    directionalBias: "neutral",
    regime: "unknown",
    cascadeRisk: 0,
    gammaPressure: 0,
  },
  forceField: {
    ts: null,
    symbol: "BTC",
    liquidityField: 0,
    gammaField: 0,
    liquidationField: 0,
    cascadeField: 0,
    totalStress: 0,
    instabilityIndex: 0,
    nextMoveBias: "neutral",
    squeezeProbability: 0,
    cascadeProbability: 0,
    predictedRegime: "unknown",
  },
  liquidationHeatmap: [],
  gammaWalls: [],
  squeeze: {
    upProbability: 0,
    downProbability: 0,
    dominantDirection: "neutral",
    breakoutBias: "neutral",
    netLiquidationBias: 0,
    longLiquidationPressure: 0,
    shortLiquidationPressure: 0,
  },
  cascadeTimeline: [],
  liquidityMap: [],
  sources: {
    flow: "unavailable",
    liquidation: "flow_proxy",
    optionsGamma: "proxy",
    orderbook: "unavailable",
  },
  notes: ["BTC liquidation dashboard is read-only and does not execute trades."],
};

export async function fetchBtcLiquidationDashboard() {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const response = await axios.get(`${baseURL}/api/btc-liquidation/dashboard`);
    return {
      dashboard: normalizeBtcLiquidationDashboard(response.data),
      error: null,
    };
  } catch {
    return {
      dashboard: fallbackDashboard,
      error: "btc_liquidation_dashboard_unavailable",
    };
  }
}

export function normalizeBtcLiquidationDashboard(payload) {
  const data = payload && typeof payload === "object" ? payload : {};
  return {
    ...fallbackDashboard,
    ...data,
    symbol: String(data.symbol || "BTC").toUpperCase(),
    currentPriceUsd: numberOrNull(data.currentPriceUsd),
    readOnly: data.readOnly !== false,
    live: Boolean(data.live),
    marketStress: {
      ...fallbackDashboard.marketStress,
      ...(data.marketStress || {}),
    },
    forceField: normalizeForceField(data.forceField, data.marketStress),
    liquidationHeatmap: Array.isArray(data.liquidationHeatmap)
      ? data.liquidationHeatmap.map(normalizeHeatmapLevel)
      : [],
    gammaWalls: Array.isArray(data.gammaWalls) ? data.gammaWalls.map(normalizeGammaWall) : [],
    squeeze: {
      ...fallbackDashboard.squeeze,
      ...(data.squeeze || {}),
    },
    cascadeTimeline: Array.isArray(data.cascadeTimeline)
      ? data.cascadeTimeline.map(normalizeCascadePoint)
      : [],
    liquidityMap: Array.isArray(data.liquidityMap)
      ? data.liquidityMap.map(normalizeLiquidityLevel)
      : [],
    sources: {
      ...fallbackDashboard.sources,
      ...(data.sources || {}),
    },
    notes: Array.isArray(data.notes) ? data.notes.map(String) : fallbackDashboard.notes,
  };
}

function normalizeForceField(forceField, marketStress) {
  const source = forceField && typeof forceField === "object" ? forceField : {};
  const stress = marketStress && typeof marketStress === "object" ? marketStress : {};
  return {
    ...fallbackDashboard.forceField,
    ...source,
    ts: numberOrNull(source.ts),
    symbol: String(source.symbol || "BTC").toUpperCase(),
    liquidityField: Number(source.liquidityField ?? stress.liquidityField ?? 0),
    gammaField: Number(source.gammaField ?? stress.gammaField ?? 0),
    liquidationField: Number(source.liquidationField ?? stress.liquidationField ?? 0),
    cascadeField: Number(source.cascadeField ?? stress.cascadeField ?? 0),
    totalStress: Number(source.totalStress ?? stress.stressScore ?? 0),
    instabilityIndex: Number(source.instabilityIndex ?? stress.instabilityIndex ?? 0),
    nextMoveBias: String(source.nextMoveBias || stress.directionalBias || "neutral"),
    squeezeProbability: Number(source.squeezeProbability ?? 0),
    cascadeProbability: Number(source.cascadeProbability ?? stress.cascadeRisk ?? 0),
    predictedRegime: String(source.predictedRegime || stress.regime || "unknown"),
  };
}

function normalizeHeatmapLevel(item) {
  return {
    priceUsd: numberOrNull(item?.priceUsd),
    normalizedPrice: Number(item?.normalizedPrice || 0),
    side: String(item?.side || "current"),
    leverageDensity: Number(item?.leverageDensity || 0),
    liquidationVolume: Number(item?.liquidationVolume || 0),
    riskScore: Number(item?.riskScore || 0),
  };
}

function normalizeGammaWall(item) {
  return {
    strikeUsd: numberOrNull(item?.strikeUsd),
    normalizedStrike: Number(item?.normalizedStrike || 0),
    gammaExposure: Number(item?.gammaExposure || 0),
    callPutImbalance: Number(item?.callPutImbalance || 0),
    role: String(item?.role || "neutral"),
  };
}

function normalizeCascadePoint(item) {
  return {
    step: Number(item?.step || 0),
    priceUsd: numberOrNull(item?.priceUsd),
    normalizedPrice: Number(item?.normalizedPrice || 0),
    expectedLiquidation: Number(item?.expectedLiquidation || 0),
    impactAmplification: Number(item?.impactAmplification || 0),
  };
}

function normalizeLiquidityLevel(item) {
  return {
    priceUsd: numberOrNull(item?.priceUsd),
    normalizedPrice: Number(item?.normalizedPrice || 0),
    side: String(item?.side || "current"),
    pressure: Number(item?.pressure || 0),
    depthScore: Number(item?.depthScore || 0),
    label: String(item?.label || "liquidity"),
  };
}

function numberOrNull(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}
