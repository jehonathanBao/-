import axios from "axios";

const ORDERFLOW_REQUEST_TIMEOUT_MS = 3_000;
const EFFECTIVE_REQUEST_TIMEOUT_MS = import.meta.env?.MODE === "test" ? 50 : ORDERFLOW_REQUEST_TIMEOUT_MS;

export async function fetchBinanceOrderflow({ symbol = "BTCUSDT", interval = "1h", limit = 50 } = {}) {
  const params = { symbol, interval, limit };
  try {
    const response = await withTimeout(
      axios.get("/api/binance/orderflow", { params }),
      EFFECTIVE_REQUEST_TIMEOUT_MS,
    );
    return normalizeBinanceOrderflow(response.data);
  } catch (error) {
    // Keep the chart useful when the monitor database is locked, compacting,
    // or has no retained flow rows. The fallback intentionally contains no
    // buy/sell fields, so the UI keeps OHLC while hiding Delta metrics.
    return fetchPublicBinanceKlines(params, error);
  }
}

async function fetchPublicBinanceKlines({ symbol, interval, limit }, originalError) {
  // Tests must remain hermetic. The public fallback is a production-only
  // resilience path; a failed mocked API request must never reach Binance.
  if (import.meta.env?.MODE === "test") {
    throw originalError;
  }
  const controller = new AbortController();
  const timer = window.setTimeout(() => controller.abort(), EFFECTIVE_REQUEST_TIMEOUT_MS);
  try {
    const query = new URLSearchParams({ symbol, interval, limit: String(limit) });
    const response = await fetch(`https://fapi.binance.com/fapi/v1/klines?${query}`, {
      signal: controller.signal,
      headers: { Accept: "application/json" },
    });
    if (!response.ok) {
      throw new Error(`Binance K线接口返回 ${response.status}`);
    }
    const rows = await response.json();
    if (!Array.isArray(rows) || rows.length === 0) {
      throw new Error("Binance K线暂无数据");
    }
    return normalizeBinanceOrderflow({
      exchange: "binance",
      symbol,
      interval,
      source: "binance_futures_kline_public_fallback",
      monitorFlowCandles: 0,
      asOfMs: Date.now(),
      candles: rows.map((row) => ({
        time: row[0],
        closeTime: row[6],
        open: row[1],
        high: row[2],
        low: row[3],
        close: row[4],
        volumeBase: row[5],
        volumeQuote: row[7],
        tradeCount: row[8],
        closed: Number(row[6]) <= Date.now(),
      })),
    });
  } catch (fallbackError) {
    fallbackError.cause = originalError;
    throw fallbackError;
  } finally {
    window.clearTimeout(timer);
  }
}

function withTimeout(promise, timeoutMs) {
  let timer;
  const timeout = new Promise((_, reject) => {
    timer = window.setTimeout(() => {
      const error = new Error(`订单流接口超过 ${timeoutMs}ms 未响应`);
      error.code = "ERR_ORDERFLOW_TIMEOUT";
      reject(error);
    }, timeoutMs);
  });
  return Promise.race([promise, timeout]).finally(() => window.clearTimeout(timer));
}

export function normalizeBinanceOrderflow(payload = {}) {
  return {
    exchange: String(payload.exchange || "binance").toLowerCase(),
    symbol: String(payload.symbol || "BTCUSDT").toUpperCase(),
    market: payload.market || "usdt_perpetual",
    interval: payload.interval || "1h",
    source: payload.source || "binance_futures_kline",
    monitorFlowCandles: Number(payload.monitorFlowCandles || 0),
    asOfMs: Number(payload.asOfMs || Date.now()),
    readOnly: payload.readOnly !== false,
    candles: Array.isArray(payload.candles) ? payload.candles.map(normalizeBinanceCandle) : [],
  };
}

function normalizeBinanceCandle(candle = {}) {
  return {
    time: Number(candle.time || 0),
    closeTime: Number(candle.closeTime || 0),
    open: Number(candle.open || 0),
    high: Number(candle.high || 0),
    low: Number(candle.low || 0),
    close: Number(candle.close || 0),
    volumeBase: Number(candle.volumeBase || 0),
    volumeQuote: Number(candle.volumeQuote || 0),
    buyBase: Number(candle.buyBase || 0),
    sellBase: Number(candle.sellBase || 0),
    buyQuote: Number(candle.buyQuote || 0),
    sellQuote: Number(candle.sellQuote || 0),
    deltaBase: Number(candle.deltaBase || 0),
    deltaQuote: Number(candle.deltaQuote || 0),
    deltaPct: Number(candle.deltaPct || 0),
    tradeCount: Number(candle.tradeCount || 0),
    closed: candle.closed !== false,
    vpin: Number.isFinite(Number(candle.vpin)) ? Number(candle.vpin) : null,
    vpinZscore: Number.isFinite(Number(candle.vpinZscore)) ? Number(candle.vpinZscore) : null,
    vpinPercentile: Number.isFinite(Number(candle.vpinPercentile)) ? Number(candle.vpinPercentile) : null,
    vpinSpike: candle.vpinSpike === true,
    vpinHigh: candle.vpinHigh === true,
    vpinExtreme: candle.vpinExtreme === true,
    tofVolumeBtc: Number.isFinite(Number(candle.tofVolumeBtc)) ? Number(candle.tofVolumeBtc) : null,
    tofSeverity: candle.tofSeverity || null,
    tofAlert: candle.tofAlert === true,
    tofReasons: Array.isArray(candle.tofReasons) ? candle.tofReasons : [],
  };
}
