import axios from "axios";
import { finalResultDescription } from "../utils/signalResult.js";

export async function pushDiscordAlert(signal) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  const body =
    typeof signal === "string"
      ? { signalId: signal }
      : {
          signalId: signal.id,
          dedupeKey: signal.dedupeKey,
          exchange: signal.exchange,
          symbol: signal.symbol,
          signalType: signal.type,
          level: signal.level,
          side: signal.side,
          score: signal.finalRiskScore ?? signal.score,
          dataQuality: signal.dataQuality ?? 100,
          reason: finalResultDescription(signal),
          time: signal.time,
          tofMetrics: signal.tofMetrics,
          tofScore: signal.tofScore,
          candidateType: signal.candidateType,
          explainTags: signal.explainTags,
          directionConfidence: signal.directionConfidence,
          perpTofMetrics: signal.perpTofMetrics,
          perpScore: signal.perpScore,
          perpCandidateType: signal.perpCandidateType,
          finalCandidateType: signal.finalCandidateType,
          metricsDirection: signal.metricsDirection,
          advancedTofMetrics: signal.advancedTofMetrics,
          advancedScore: signal.advancedScore,
          advancedCandidateType: signal.advancedCandidateType,
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
