// Fixed, disclosed design fixtures. Imported only by the development preview route.
const start = Date.UTC(2026, 8, 6, 0);
const buy = [344, 401, 483, 620, 767, 861, 735, 612, 561, 683, 792, 947, 1142, 1376, 1203, 1078, 890, 731, 864, 998, 1280, 1535, 1714, 1465, 1189, 1053, 988, 1104, 1317, 1613, 1857, 2094];
const sell = [574, 618, 724, 856, 938, 1034, 1127, 975, 848, 711, 652, 590, 574, 691, 806, 942, 1126, 1267, 1308, 1186, 1004, 842, 713, 643, 559, 613, 678, 792, 971, 884, 1024, 1072];
const closes = [63271, 63309, 63204, 63167, 63311, 63283, 63447, 63381, 63453, 63390, 63551, 63715, 63650, 63874, 63810, 63706, 63647, 63772, 63953, 63901, 64026, 64148, 64095, 64304, 64211, 64367, 64492, 64436, 64615, 64710, 64687, 64832.17];
export const previewCandles = buy.map((buyBase, i) => ({ time: start + i * 3600000, buyBase, sellBase: sell[i], close: closes[i] }));
export const previewEvents = Array.from({ length: 66 }, (_, i) => ({
  id: `design-observation-${i}`, ts: start + i * 1700000, symbol: "BTC",
  mainExchange: ["binance", "coinbase", "bitfinex", "okx"][i % 4],
  direction: i % 9 < 5 ? "buy" : i % 9 < 8 ? "sell" : "neutral",
  impactLevel: ["C", "B", "B", "A", "C", "S"][i % 6], score: 65 + i % 29, dataQuality: 76 + i % 22,
}));
