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
      evidence: Array.isArray(signal.evidence) ? signal.evidence : [],
      detector: signal.detector || "new_token_flow_engine_v1",
      updatedAtMs: Number(signal.updatedAtMs || 0),
      readOnly: signal.readOnly !== false,
    },
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
