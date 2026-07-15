import axios from "axios";
import { finalResultDescription } from "../utils/signalResult.js";
import { evaluateDiscordAlertGate, resolveDiscordAlertFamily } from "./alertGate.js";

export async function pushDiscordAlert(signal) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  if (typeof signal === "string") {
    return { ok: false, reason: "DISCORD_SUPPRESSED_MISSING_CLIENT_CONTEXT" };
  }
  const gate = evaluateDiscordAlertGate(signal);
  if (!gate.ok) {
    return gate;
  }
  const alertFamily = resolveDiscordAlertFamily(signal);
  const tofMetrics = alertEligibleMetrics(signal.tofMetrics) ? signal.tofMetrics : undefined;
  const body = {
    alertFamily,
    signalId: signal.id,
    dedupeKey: signal.dedupeKey,
    exchange: signal.exchange,
    symbol: signal.symbol,
    signalType: signal.type,
    level: signal.level,
    side: signal.side,
    score:
      alertFamily === "market_structure"
        ? signal.mainForceScore ??
          signal.marketStructureScore?.mainForceScore ??
          signal.riskSystems?.mainForceStructure?.mainForceScore ??
          signal.score
        : signal.authoritativeRiskScore ?? signal.riskScore,
    confidence:
      alertFamily === "market_structure"
        ? signal.marketStructureConfidence ??
          signal.marketStructureScore?.confidence ??
          signal.riskSystems?.mainForceStructure?.confidence
        : signal.detectorConfidence ?? signal.confidence,
    dataQuality:
      alertFamily === "market_structure"
        ? signal.marketStructureDataQuality ??
          signal.marketStructureScore?.dataQuality ??
          signal.riskSystems?.mainForceStructure?.dataQuality
        : signal.authoritativeDataQuality ?? signal.dataQualityScore,
    reason: finalResultDescription(signal),
    time: signal.time,
    tofMetrics,
    tofScore: tofMetrics ? signal.tofScore : undefined,
    candidateType: signal.candidateType,
    explainTags: signal.explainTags,
    directionConfidence: tofMetrics ? signal.directionConfidence : undefined,
    mainForceScore: signal.mainForceScore,
    extremeImpactScore: signal.extremeImpactScore,
    structureBias: signal.structureBias,
    marketStructureConfidence: signal.marketStructureConfidence,
    marketStructureDataQuality: signal.marketStructureDataQuality,
    marketStructureSeverity: signal.marketStructureSeverity,
    regimeType: signal.regimeType,
    spotScore: signal.spotScore,
    contractScore: signal.contractScore,
    crossConfirmScore: signal.crossConfirmScore,
    mainForceConfirmed: signal.mainForceConfirmed,
    signalAgreement: signal.signalAgreement,
    sourceCoverage: signal.sourceCoverage,
    oiScore: signal.oiScore,
    liquidationScore: signal.liquidationScore,
  };
  const response = await axios.post(`${baseURL}/api/discord/push`, compactPayload(body));
  return response.data;
}

function alertEligibleMetrics(metrics) {
  const lineage = metrics?.lineage;
  return Boolean(
    lineage &&
      lineage.alertEligible === true &&
      lineage.available === true &&
      lineage.fresh === true &&
      (lineage.provenance === "observed" || lineage.provenance === "calculated_from_observed"),
  );
}

export async function sendDiscordTestMessage(signal) {
  if (!signal || typeof signal !== "object" || !signal.id) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_MISSING_CLIENT_CONTEXT" };
  }
  const gate = evaluateDiscordAlertGate(signal);
  if (!gate.ok) {
    return gate;
  }
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  const response = await axios.post(`${baseURL}/api/discord/push`, {
    alertFamily: resolveDiscordAlertFamily(signal),
    signalId: signal.id,
    symbol: signal.symbol,
    test: true,
  });
  return response.data;
}

function compactPayload(payload) {
  return Object.fromEntries(
    Object.entries(payload).filter(([, value]) => value !== undefined),
  );
}
