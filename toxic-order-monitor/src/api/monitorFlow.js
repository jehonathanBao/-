import axios from "axios";
import { fetchBinanceOrderflow } from "./binanceOrderflow.js";
import {
  fetchContractEvents,
  fetchContractWhaleSummary,
} from "./contractWhale.js";
import { fetchScanLogs } from "./scanLogs.js";
import { fetchSpotWhaleLatest } from "./spotWhale.js";

const EMPTY_SYSTEM_STATUS = {
  alerts: null,
  marketDataQuality: null,
  runtimeControl: null,
  storage: null,
  venues: {},
  vpin: null,
};

export async function fetchMonitorFlowSnapshot() {
  const results = await Promise.allSettled([
    fetchContractWhaleSummary("BTC"),
    fetchContractWhaleSummary("ETH"),
    fetchContractEvents({ symbol: "BTC", range: "7d", limit: 24, min_notional_usd: 0 }),
    fetchContractEvents({ symbol: "ETH", range: "7d", limit: 16, min_notional_usd: 0 }),
    fetchSpotWhaleLatest(20, "BTC"),
    fetchSpotWhaleLatest(12, "ETH"),
    fetchBinanceOrderflow({ symbol: "BTCUSDT", interval: "1h", limit: 24 }),
    fetchScanLogs(36),
    fetchSystemStatus(),
  ]);

  const [contractBtc, contractEth, contractEventsBtc, contractEventsEth, spotBtc, spotEth, orderflow, scanLogs, system] =
    results.map(resultValue);

  return {
    fetchedAtMs: Date.now(),
    contract: {
      BTC: contractBtc || { summary: null, error: "summary_unavailable" },
      ETH: contractEth || { summary: null, error: "summary_unavailable" },
      events: [
        ...(contractEventsBtc?.items || []),
        ...(contractEventsEth?.items || []),
      ],
      error: contractEventsBtc?.error && contractEventsEth?.error ? "contract_events_unavailable" : null,
    },
    spot: {
      BTC: spotBtc || { summary: null, items: [], error: "latest_unavailable" },
      ETH: spotEth || { summary: null, items: [], error: "latest_unavailable" },
      error: spotBtc?.error && spotEth?.error ? "spot_events_unavailable" : null,
    },
    orderflow: orderflow || null,
    scanLogs: Array.isArray(scanLogs) ? scanLogs : [],
    system: system || EMPTY_SYSTEM_STATUS,
  };
}

export async function fetchSystemStatus() {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const response = await axios.get(`${baseURL}/api/status`, { timeout: 4_000 });
    const payload = response.data && typeof response.data === "object" ? response.data : {};
    return {
      alerts: payload.alerts || null,
      marketDataQuality: payload.marketDataQuality || null,
      runtimeControl: payload.runtimeControl || null,
      storage: payload.storage || null,
      venues: payload.venues && typeof payload.venues === "object" ? payload.venues : {},
      vpin: payload.vpin || null,
    };
  } catch {
    return EMPTY_SYSTEM_STATUS;
  }
}

function resultValue(result) {
  return result?.status === "fulfilled" ? result.value : null;
}
