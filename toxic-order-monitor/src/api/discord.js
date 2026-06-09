import axios from "axios";
import { finalResultDescription } from "../utils/signalResult.js";
import { resolveDiscordAlertFamily } from "./alertGate.js";

export async function pushDiscordAlert(signal) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  const alertFamily = typeof signal === "string" ? "short_toxic_order" : resolveDiscordAlertFamily(signal);
  const body =
    typeof signal === "string"
      ? { signalId: signal, alertFamily }
      : {
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
              : signal.toxicScore ??
                signal.riskSystems?.shortTermToxic?.toxicScore ??
                signal.finalRiskScore ??
                signal.score,
          confidence:
            alertFamily === "market_structure"
              ? signal.marketStructureConfidence ??
                signal.marketStructureScore?.confidence ??
                signal.riskSystems?.mainForceStructure?.confidence ??
                signal.confidence
              : signal.confidence ??
                signal.riskSystems?.shortTermToxic?.confidence ??
                signal.toxicShortScore?.confidence,
          dataQuality:
            alertFamily === "market_structure"
              ? signal.marketStructureDataQuality ??
                signal.marketStructureScore?.dataQuality ??
                signal.riskSystems?.mainForceStructure?.dataQuality ??
                signal.dataQuality ??
                100
              : signal.dataQuality ?? 100,
          reason: finalResultDescription(signal),
          time: signal.time,
          tofMetrics: signal.tofMetrics,
          tofScore: signal.tofScore,
          candidateType: signal.candidateType,
          explainTags: signal.explainTags,
          directionConfidence: signal.directionConfidence,
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

export async function sendDiscordTestMessage() {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  const response = await axios.post(`${baseURL}/api/discord/push`, {
    test: true,
  });
  return response.data;
}

function compactPayload(payload) {
  return Object.fromEntries(
    Object.entries(payload).filter(([, value]) => value !== undefined),
  );
}
