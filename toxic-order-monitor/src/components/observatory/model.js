// Presentation transforms only. No scoring, network requests or notification decisions.
export const numeric = (value) => value === null || value === undefined || value === "" || typeof value === "boolean"
  ? null : Number.isFinite(Number(value)) ? Number(value) : null;
const validTime = (value) => numeric(value) > 0 && numeric(value) <= 8.64e15;

export function normalizeFlowSamples(rows, symbol = "BTC") {
  const unique = new Map();
  for (const row of Array.isArray(rows) ? rows : []) {
    if (row?.symbol && !String(row.symbol).toUpperCase().startsWith(symbol)) continue;
    const time = numeric(row?.time ?? row?.ts);
    let buy = numeric(row?.buyBase ?? row?.buyVolumeBtc);
    let sell = numeric(row?.sellBase ?? row?.sellVolumeBtc);
    const total = numeric(row?.totalVolumeBtc);
    const net = numeric(row?.netVolumeBtc);
    if (buy === null && sell === null && total !== null && net !== null && total >= Math.abs(net)) {
      buy = (total + net) / 2;
      sell = (total - net) / 2;
    }
    if (!validTime(time) || buy === null || sell === null || buy < 0 || sell < 0 || !Number.isFinite(buy + sell) || buy + sell <= 0) continue;
    unique.set(row.id ?? row.eventId ?? time, { time, buy, sell, delta: buy - sell, id: row.id ?? row.eventId ?? time });
  }
  return [...unique.values()].sort((a, b) => a.time - b.time).slice(-36);
}

export function normalizeObservations(rows, symbol) {
  const unique = new Map();
  for (const row of Array.isArray(rows) ? rows : []) {
    if (!row || !validTime(row.ts ?? row.time)) continue;
    if (symbol && !String(row.symbol || "").toUpperCase().startsWith(symbol)) continue;
    const id = row.id ?? row.eventId;
    if (id === null || id === undefined || id === "") continue;
    if (unique.get(String(id))?.time > Number(row.ts ?? row.time)) continue;
    unique.set(String(id), {
      id: String(id), time: Number(row.ts ?? row.time), symbol: row.symbol || symbol || "—",
      direction: ["buy", "sell"].includes(row.direction) ? row.direction : "neutral",
      venue: ["binance", "bitfinex", "coinbase", "okx", "bybit"].includes(String(row.mainExchange || row.exchange).toLowerCase()) ? String(row.mainExchange || row.exchange).toLowerCase() : "unknown",
      grade: ["S", "A", "B", "C"].includes(row.impactLevel ?? row.impactGrade) ? (row.impactLevel ?? row.impactGrade) : "—",
      score: numeric(row.score), quality: numeric(row.dataQuality),
    });
  }
  return [...unique.values()].sort((a, b) => a.time - b.time).slice(-80);
}

export function groupObservations(events) {
  return ["buy", "neutral", "sell"].map(direction => ({ direction, items: events.filter(event => event.direction === direction) }));
}

export function buildRidgePath(sample, index, count, maximum) {
  const depth = index / Math.max(1, count - 1);
  const offset = depth * 155;
  const baseline = 250 - depth * 102;
  const points = Array.from({ length: 85 }, (_, i) => {
    const x = i / 84;
    // Each ribbon has two lobes: left = sell volume, right = buy volume.
    // Width is a visual kernel, not a probability/confidence estimate.
    const height = sample ? (sample.sell * Math.exp(-(((x - 0.34) / 0.15) ** 2))
      + sample.buy * Math.exp(-(((x - 0.66) / 0.15) ** 2))) / Math.max(1, maximum) * 145 : 0;
    return `${i === 0 ? "M" : "L"}${(45 + x * 530 + offset).toFixed(2)},${(baseline - height).toFixed(2)}`;
  });
  return points.join(" ");
}

export const formatReading = (value, decimals = 1) => value === null || value === undefined ? "—" : new Intl.NumberFormat("en-US", { maximumFractionDigits: decimals }).format(value);
export const clockLabel = (time) => validTime(time) ? new Date(time).toLocaleTimeString("zh-CN", { hour12: false, hour: "2-digit", minute: "2-digit" }) : "—";
