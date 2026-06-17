import axios from "axios";

export const NEW_TOKEN_WATCH_MAX_ACTIVE = 10;

export async function fetchNewTokenWatchList() {
  const response = await axios.get("/api/new-token-watch/list");
  return normalizeNewTokenWatchList(response.data);
}

export async function addNewTokenWatch(symbol) {
  const response = await axios.post("/api/new-token-watch/add", { symbol });
  return normalizeNewTokenMutation(response.data);
}

export async function removeNewTokenWatch(symbol) {
  const response = await axios.post("/api/new-token-watch/remove", { symbol });
  return normalizeNewTokenMutation(response.data);
}

export async function fetchNewTokenReconstruction(symbol, timeframe = "15m") {
  const response = await axios.get("/api/new-token-watch/reconstruction", {
    params: { symbol, tf: timeframe },
  });
  return normalizeNewTokenReconstruction(response.data);
}

export async function fetchNewTokenChart(symbol, timeframe = "15m") {
  const response = await axios.get("/api/new-token-watch/chart", {
    params: { symbol, tf: timeframe },
  });
  return normalizeNewTokenChart(response.data);
}

export function normalizeNewTokenWatchList(payload = {}) {
  const items = Array.isArray(payload.items) ? payload.items.map(normalizeNewTokenWatchItem) : [];
  return {
    items,
    maxActiveTokens: Number(payload.maxActiveTokens ?? NEW_TOKEN_WATCH_MAX_ACTIVE),
    activeCount: Number(payload.activeCount ?? items.length),
    readOnly: payload.readOnly !== false,
  };
}

export function normalizeNewTokenMutation(payload = {}) {
  const list = normalizeNewTokenWatchList({
    items: payload.items,
    maxActiveTokens: payload.maxActiveTokens,
    activeCount: Array.isArray(payload.items) ? payload.items.length : undefined,
    readOnly: payload.readOnly,
  });
  return {
    ...list,
    ok: Boolean(payload.ok),
    item: payload.item ? normalizeNewTokenWatchItem(payload.item) : null,
    error: payload.error || null,
  };
}

export function normalizeNewTokenWatchItem(item = {}) {
  const signal = item.lastSignal || {};
  return {
    symbol: String(item.symbol || signal.symbol || "").toUpperCase(),
    addedAtMs: Number(item.addedAtMs || 0),
    streamStatus: item.streamStatus || "unknown",
    readOnly: item.readOnly !== false,
    lastSignal: {
      symbol: String(signal.symbol || item.symbol || "").toUpperCase(),
      regime: signal.regime || "neutral",
      strength: Number(signal.strength || 0),
      confidence: Number(signal.confidence || 0),
      flowPersistence: Number(signal.flowPersistence || 0),
      ofiWindows: Array.isArray(signal.ofiWindows) ? signal.ofiWindows.map(normalizeOfiWindow) : [],
      impactResponse: normalizeImpactResponse(signal.impactResponse),
      liquidityDepletion: normalizeLiquidityDepletion(signal.liquidityDepletion),
      actorDecomposition: normalizeActorDecomposition(signal.actorDecomposition),
      signalCompression: normalizeSignalCompression(signal.signalCompression),
      capitalStructure: normalizeCapitalStructure(signal.capitalStructure),
      positionReconstruction: normalizePositionReconstruction(signal.positionReconstruction),
      evidence: Array.isArray(signal.evidence) ? signal.evidence : [],
      detector: signal.detector || "new_token_flow_engine_v1",
      updatedAtMs: Number(signal.updatedAtMs || 0),
      readOnly: signal.readOnly !== false,
    },
  };
}

export function normalizeNewTokenReconstruction(payload = {}) {
  return {
    symbol: String(payload.symbol || "").toUpperCase(),
    timeframe: payload.timeframe || "15m",
    currentPhase: payload.currentPhase || "neutral",
    currentPrice: Number(payload.currentPrice || 0),
    change24hPct: optionalNumber(payload.change24hPct),
    volume24hUsd: optionalNumber(payload.volume24hUsd),
    high24h: optionalNumber(payload.high24h),
    low24h: optionalNumber(payload.low24h),
    marketCapUsd: optionalNumber(payload.marketCapUsd),
    costBasisLow: Number(payload.costBasisLow || 0),
    costBasisHigh: Number(payload.costBasisHigh || 0),
    vwapAnchor: Number(payload.vwapAnchor || 0),
    estimatedTotalPositionUsdtLow: Number(payload.estimatedTotalPositionUsdtLow || 0),
    estimatedTotalPositionUsdtHigh: Number(payload.estimatedTotalPositionUsdtHigh || 0),
    estimatedNetPositionUsdt: Number(payload.estimatedNetPositionUsdt || 0),
    floatingPnlLowPct: Number(payload.floatingPnlLowPct || 0),
    floatingPnlHighPct: Number(payload.floatingPnlHighPct || 0),
    accumulationPath: Array.isArray(payload.accumulationPath)
      ? payload.accumulationPath.map(normalizePositionPathSegment)
      : [],
    lastAccumulationNode: payload.lastAccumulationNode
      ? normalizeLastAccumulationNode(payload.lastAccumulationNode)
      : null,
    distributionPath: Array.isArray(payload.distributionPath)
      ? payload.distributionPath.map(normalizePositionPathSegment)
      : [],
    distributionCompletionPct: Number(payload.distributionCompletionPct || 0),
    distributionIntensityScore: Number(payload.distributionIntensityScore || 0),
    shortTermBehaviorProbabilities: normalizeBehaviorProbabilities(
      payload.shortTermBehaviorProbabilities
    ),
    phaseTimeline: Array.isArray(payload.phaseTimeline)
      ? payload.phaseTimeline.map(normalizePhaseTimelineSegment)
      : [],
    costDistribution: Array.isArray(payload.costDistribution)
      ? payload.costDistribution.map(normalizeCostDistributionBand)
      : [],
    smartLevels: Array.isArray(payload.smartLevels)
      ? payload.smartLevels.map(normalizeSmartLevel)
      : [],
    confidence: Number(payload.confidence || 0),
    readOnly: payload.readOnly !== false,
  };
}

export function normalizeNewTokenChart(payload = {}) {
  return {
    symbol: String(payload.symbol || "").toUpperCase(),
    timeframe: payload.timeframe || "15m",
    points: Array.isArray(payload.points) ? payload.points.map(normalizeChartPoint) : [],
    phaseSegments: Array.isArray(payload.phaseSegments)
      ? payload.phaseSegments.map(normalizePhaseTimelineSegment)
      : [],
    markers: Array.isArray(payload.markers) ? payload.markers.map(normalizeChartMarker) : [],
    readOnly: payload.readOnly !== false,
  };
}

function optionalNumber(value) {
  return value === null || value === undefined ? null : Number(value);
}

function normalizeBehaviorProbabilities(probabilities = {}) {
  return {
    continueDistribution: Number(probabilities.continueDistribution || 0),
    rangeConsolidation: Number(probabilities.rangeConsolidation || 0),
    reboundMarkup: Number(probabilities.reboundMarkup || 0),
    secondaryAccumulation: Number(probabilities.secondaryAccumulation || 0),
  };
}

function normalizePhaseTimelineSegment(segment = {}) {
  return {
    phase: segment.phase || "neutral",
    label: segment.label || segment.phase || "neutral",
    startMs: Number(segment.startMs || 0),
    endMs: Number(segment.endMs || 0),
    durationSec: Number(segment.durationSec || 0),
    lower: Number(segment.lower || 0),
    upper: Number(segment.upper || 0),
  };
}

function normalizeCostDistributionBand(band = {}) {
  return {
    label: band.label || "cost_band",
    lower: Number(band.lower || 0),
    upper: Number(band.upper || 0),
    pct: Number(band.pct || 0),
  };
}

function normalizeSmartLevel(level = {}) {
  return {
    label: level.label || "level",
    price: Number(level.price || 0),
    role: level.role || "reference",
  };
}

function normalizeChartPoint(point = {}) {
  return {
    ts: Number(point.ts || 0),
    price: Number(point.price || 0),
    volume: Number(point.volume || 0),
    netPosition: Number(point.netPosition || 0),
  };
}

function normalizeChartMarker(marker = {}) {
  return {
    ts: Number(marker.ts || 0),
    price: Number(marker.price || 0),
    label: marker.label || "marker",
    kind: marker.kind || "reference",
  };
}

function normalizePositionReconstruction(reconstruction = {}) {
  return {
    accumulationPath: Array.isArray(reconstruction.accumulationPath)
      ? reconstruction.accumulationPath.map(normalizePositionPathSegment)
      : [],
    lastAccumulationNode: reconstruction.lastAccumulationNode
      ? normalizeLastAccumulationNode(reconstruction.lastAccumulationNode)
      : null,
    distributionPath: Array.isArray(reconstruction.distributionPath)
      ? reconstruction.distributionPath.map(normalizePositionPathSegment)
      : [],
    latentPosition: Array.isArray(reconstruction.latentPosition)
      ? reconstruction.latentPosition.map(normalizeLatentPositionPoint)
      : [],
    confidence: Number(reconstruction.confidence || 0),
    regimeLabel: reconstruction.regimeLabel || "neutral",
    evidence: Array.isArray(reconstruction.evidence) ? reconstruction.evidence : [],
    readOnly: reconstruction.readOnly !== false,
  };
}

function normalizePositionPathSegment(segment = {}) {
  return {
    phase: segment.phase || "neutral",
    label: segment.label || "neutral_segment",
    startPrice: Number(segment.startPrice || 0),
    endPrice: Number(segment.endPrice || 0),
    volume: Number(segment.volume || 0),
    cumulativeDelta: Number(segment.cumulativeDelta || 0),
    impact: Number(segment.impact || 0),
    durationSec: Number(segment.durationSec || 0),
    confidence: Number(segment.confidence || 0),
    characteristics: Array.isArray(segment.characteristics) ? segment.characteristics : [],
  };
}

function normalizeLastAccumulationNode(node = {}) {
  return {
    lower: Number(node.lower || 0),
    upper: Number(node.upper || 0),
    durationSec: Number(node.durationSec || 0),
    volatilityPct: Number(node.volatilityPct || 0),
    absorptionEfficiency: Number(node.absorptionEfficiency || 0),
    confidence: Number(node.confidence || 0),
    characteristics: Array.isArray(node.characteristics) ? node.characteristics : [],
  };
}

function normalizeLatentPositionPoint(point = {}) {
  return {
    timestamp: Number(point.timestamp || 0),
    price: Number(point.price || 0),
    estimatedPosition: Number(point.estimatedPosition || 0),
    impactAdjustedPosition: Number(point.impactAdjustedPosition || 0),
  };
}

function normalizeCapitalStructure(capital = {}) {
  return {
    phase: capital.phase || "neutral",
    phaseLabel: capital.phaseLabel || capital.phase || "neutral",
    phaseConfidence: Number(capital.phaseConfidence || 0),
    behaviorWindows: Array.isArray(capital.behaviorWindows)
      ? capital.behaviorWindows.map(normalizeBehaviorWindow)
      : [],
    costBasis: normalizeCostBasis(capital.costBasis),
    estimatedPosition: normalizeEstimatedPosition(capital.estimatedPosition),
    horizon: normalizeTimeHorizon(capital.horizon),
    distributionRisk: normalizeDistributionRisk(capital.distributionRisk),
    evidence: Array.isArray(capital.evidence) ? capital.evidence : [],
    readOnly: capital.readOnly !== false,
  };
}

function normalizeBehaviorWindow(window = {}) {
  return {
    windowSec: Number(window.windowSec || 0),
    cumulativeDelta: Number(window.cumulativeDelta || 0),
    normalizedOfi: Number(window.normalizedOfi || 0),
    vwap: Number(window.vwap || 0),
    volume: Number(window.volume || 0),
    priceDriftPct: Number(window.priceDriftPct || 0),
    volatilityPct: Number(window.volatilityPct || 0),
    absorptionScore: Number(window.absorptionScore || 0),
    bidReplenishmentScore: Number(window.bidReplenishmentScore || 0),
  };
}

function normalizeCostBasis(cost = {}) {
  return {
    lower: Number(cost.lower || 0),
    upper: Number(cost.upper || 0),
    vwapAnchor: Number(cost.vwapAnchor || 0),
    confidence: Number(cost.confidence || 0),
  };
}

function normalizeEstimatedPosition(position = {}) {
  return {
    lowerUsd: Number(position.lowerUsd || 0),
    upperUsd: Number(position.upperUsd || 0),
    confidence: Number(position.confidence || 0),
  };
}

function normalizeTimeHorizon(horizon = {}) {
  return {
    minMinutes: Number(horizon.minMinutes || 0),
    maxMinutes: Number(horizon.maxMinutes || 0),
    detectedMinutes: Number(horizon.detectedMinutes || 0),
  };
}

function normalizeDistributionRisk(risk = {}) {
  return {
    score: Number(risk.score || 0),
    level: risk.level || "low",
    reasons: Array.isArray(risk.reasons) ? risk.reasons : [],
  };
}

function normalizeActorDecomposition(actor = {}) {
  return {
    liquidityProviderProbability: Number(actor.liquidityProviderProbability || 0),
    momentumChaserProbability: Number(actor.momentumChaserProbability || 0),
    smartMoneyProbability: Number(actor.smartMoneyProbability || 0),
    dominantActor: actor.dominantActor || "unknown",
    lpScore: Number(actor.lpScore || 0),
    momentumScore: Number(actor.momentumScore || 0),
    smartMoneyScore: Number(actor.smartMoneyScore || 0),
    confidence: Number(actor.confidence || 0),
    explanationTags: Array.isArray(actor.explanationTags) ? actor.explanationTags : [],
  };
}

function normalizeSignalCompression(compression = {}) {
  return {
    smartMoneyPressure: Number(compression.smartMoneyPressure || 0),
    momentumFlowExhaustion: Number(compression.momentumFlowExhaustion || 0),
    liquidityStressManipulation: Number(compression.liquidityStressManipulation || 0),
    positionValidityGate: normalizePositionValidityGate(compression.positionValidityGate),
    stabilityKernel: normalizeStabilityKernel(compression.stabilityKernel),
    explanationTags: Array.isArray(compression.explanationTags) ? compression.explanationTags : [],
    readOnly: compression.readOnly !== false,
  };
}

function normalizePositionValidityGate(gate = {}) {
  return {
    riskScore: Number(gate.riskScore || 0),
    tradePermission: Boolean(gate.tradePermission),
    positionSizeMultiplier: Number(gate.positionSizeMultiplier || 0),
    reason: gate.reason || "no_signal",
    advisoryOnly: gate.advisoryOnly !== false,
  };
}

function normalizeStabilityKernel(kernel = {}) {
  return {
    regime: kernel.regime || "neutral",
    regimeQuality: Number(kernel.regimeQuality || 0),
    tradeSignal: normalizeTradeSignalAdvisory(kernel.tradeSignal),
    positionSmoothing: normalizePositionSmoothing(kernel.positionSmoothing),
    readOnly: kernel.readOnly !== false,
  };
}

function normalizeTradeSignalAdvisory(signal = {}) {
  return {
    direction: signal.direction || "no_trade",
    confidence: Number(signal.confidence || 0),
    expectedHoldTime: signal.expectedHoldTime || "none",
    invalidationCondition: signal.invalidationCondition || "no_signal",
    reason: signal.reason || "no_trade",
    advisoryOnly: signal.advisoryOnly !== false,
  };
}

function normalizePositionSmoothing(smoothing = {}) {
  return {
    suggestedSizeMultiplier: Number(smoothing.suggestedSizeMultiplier || 0),
    volatilityAdjustment: Number(smoothing.volatilityAdjustment || 0),
    drawdownAdjustment: Number(smoothing.drawdownAdjustment ?? 1),
    reason: smoothing.reason || "no_signal",
  };
}

function normalizeOfiWindow(window = {}) {
  return {
    windowSec: Number(window.windowSec || 0),
    buyPressure: Number(window.buyPressure || 0),
    sellPressure: Number(window.sellPressure || 0),
    netOfi: Number(window.netOfi || 0),
    normalizedOfi: Number(window.normalizedOfi || 0),
    decayWeightedOfi: Number(window.decayWeightedOfi || 0),
    persistence: Number(window.persistence || 0),
  };
}

function normalizeImpactResponse(impact = {}) {
  return {
    priceMovePct: Number(impact.priceMovePct || 0),
    totalVolume: Number(impact.totalVolume || 0),
    impactPerVolume: Number(impact.impactPerVolume || 0),
    absorptionScore: Number(impact.absorptionScore || 0),
    thinLiquidityScore: Number(impact.thinLiquidityScore || 0),
    classification: impact.classification || "unknown",
  };
}

function normalizeLiquidityDepletion(depletion = {}) {
  return {
    bidDepletionRate: Number(depletion.bidDepletionRate || 0),
    askDepletionRate: Number(depletion.askDepletionRate || 0),
    replenishmentRate: Number(depletion.replenishmentRate || 0),
    depletionPressure: Number(depletion.depletionPressure || 0),
  };
}
