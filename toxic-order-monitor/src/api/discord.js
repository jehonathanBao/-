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
          score: signal.score,
          dataQuality: signal.dataQuality ?? 100,
          reason: finalResultDescription(signal),
          time: signal.time,
        };
  const response = await axios.post(`${baseURL}/api/discord/push`, {
    ...body,
  });
  return response.data;
}

export async function sendDiscordTestMessage() {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  const response = await axios.post(`${baseURL}/api/discord/push`, {
    test: true,
  });
  return response.data;
}
