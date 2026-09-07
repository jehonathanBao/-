import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fetchBinanceOrderflow } from "../api/binanceOrderflow.js";

const echartsLoader = import("echarts");

const INTERVALS = [
  ["1m", "1 分钟"],
  ["5m", "5 分钟"],
  ["15m", "15 分钟"],
  ["30m", "30 分钟"],
  ["1h", "1 小时"],
  ["4h", "4 小时"],
  ["1d", "1 天"],
];

const SYMBOLS = ["BTCUSDT", "ETHUSDT"];
const FUTURE_SLOTS = 60;
const DEFAULT_ZOOM = { start: 30, end: 90 };
const HISTORY_REFRESH_MS = 15_000;
const LIVE_REFRESH_MS = 5_000;
const DELTA_MARK_RULES = {
  BTCUSDT: { mode: "base", threshold: 1000, description: "|Delta| ≥ 1,000 BTC" },
  ETHUSDT: { mode: "quote", threshold: 40_000_000, description: "|Delta| ≥ 4,000 万 USDT" },
};

export default function BinanceOrderflowChart() {
  const [symbol, setSymbol] = useState("BTCUSDT");
  const [interval, setInterval] = useState("1h");
  const [payload, setPayload] = useState(null);
  const [requestState, setRequestState] = useState({ phase: "loading", error: null });
  const [livePhase, setLivePhase] = useState("idle");
  const [nowMs, setNowMs] = useState(() => Date.now());
  const chartRef = useRef(null);
  const chartInstanceRef = useRef(null);
  const chartOptionRef = useRef(null);
  const zoomRef = useRef(DEFAULT_ZOOM);
  const userZoomedRef = useRef(false);
  const requestGenerationRef = useRef(0);

  const load = useCallback(async () => {
    const generation = ++requestGenerationRef.current;
    try {
      setRequestState((current) => ({ ...current, phase: current.phase === "ready" ? "refreshing" : "loading", error: null }));
      const next = await fetchBinanceOrderflow({ symbol, interval, limit: 150 });
      if (generation !== requestGenerationRef.current) return;
      setPayload((current) => mergeBinanceOrderflowPayload(current, next));
      setRequestState({ phase: "ready", error: null });
    } catch (error) {
      if (generation !== requestGenerationRef.current) return;
      setRequestState({ phase: "error", error: error?.response?.data?.error || error?.message || "Binance 数据读取失败" });
    }
  }, [interval, symbol]);

  useEffect(() => {
    load();
    const timer = window.setInterval(load, HISTORY_REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [load]);

  const refreshLatest = useCallback(async () => {
    const generation = ++requestGenerationRef.current;
    try {
      setLivePhase("refreshing");
      const next = await fetchBinanceOrderflow({ symbol, interval, limit: 3 });
      if (generation !== requestGenerationRef.current) return;
      setPayload((current) => mergeBinanceOrderflowPayload(current, next));
      setLivePhase("ready");
    } catch {
      if (generation !== requestGenerationRef.current) return;
      // Keep the chart visible while a single live tick is retried.
      setLivePhase("degraded");
    }
  }, [interval, symbol]);

  useEffect(() => {
    const timer = window.setInterval(refreshLatest, LIVE_REFRESH_MS);
    return () => window.clearInterval(timer);
  }, [refreshLatest]);

  useEffect(() => {
    const timer = window.setInterval(() => setNowMs(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!chartRef.current) return undefined;
    let disposed = false;
    let chart = null;
    let observer = null;
    let pan = null;
    let panCleanup = null;
    const resize = () => {
      if (chart && !chart.isDisposed()) chart.resize();
    };
    echartsLoader.then((module) => {
      if (disposed || !chartRef.current) return;
      const echarts = module.default || module;
      chart = echarts.init(chartRef.current);
      chartInstanceRef.current = chart;
      chart.setOption(chartOptionRef.current || buildChartOption([]), true);
      chart.on("datazoom", (event = {}) => {
        // The event payload is the authoritative range for both inside and
        // slider zooms. Reading the slider from getOption() here can lag one
        // frame behind an inside drag and would save the old right-anchored
        // range back over the user's viewport on the next poll.
        let start = Number(event.start);
        let end = Number(event.end);
        if (!Number.isFinite(start) || !Number.isFinite(end)) {
          const option = chart.getOption();
          const zoom = option?.dataZoom?.find((item) => item.type === "inside") || option?.dataZoom?.[0];
          start = Number(zoom?.start);
          end = Number(zoom?.end);
        }
        if (Number.isFinite(start) && Number.isFinite(end)) {
          zoomRef.current = { start, end };
          // A partial range is an explicit user viewport. Polling updates must
          // never snap it back to the newest candle; returning to the far right
          // restores the normal follow-latest behaviour.
          userZoomedRef.current = end < 99.5 || start > 0.5;
        }
      });
      const element = chartRef.current;
      const onPointerDown = (event) => {
        if (event.pointerType === "mouse" && event.button !== 0) return;
        const rect = element.getBoundingClientRect();
        if (event.clientY - rect.top > rect.height - 44) return;
        const { start, end } = zoomRef.current;
        pan = { pointerId: event.pointerId, x: event.clientX, start, end };
        element.setPointerCapture?.(event.pointerId);
        event.preventDefault();
      };
      const onPointerMove = (event) => {
        if (!pan || event.pointerId !== pan.pointerId) return;
        const span = Math.max(1, pan.end - pan.start);
        const shift = -((event.clientX - pan.x) / Math.max(1, element.clientWidth)) * 100;
        const start = Math.max(0, Math.min(100 - span, pan.start + shift));
        const end = start + span;
        zoomRef.current = { start, end };
        userZoomedRef.current = start > 0.5 || end < 99.5;
        chart.dispatchAction({ type: "dataZoom", start, end });
        event.preventDefault();
      };
      const onPointerUp = (event) => {
        if (pan && event.pointerId === pan.pointerId) pan = null;
      };
      // Native pointer events are used on the plot container so dragging the
      // candle area pans the linked axes reliably, independent of ECharts'
      // internal gesture handling.
      element.addEventListener("pointerdown", onPointerDown, true);
      element.addEventListener("pointermove", onPointerMove, true);
      element.addEventListener("pointerup", onPointerUp, true);
      element.addEventListener("pointercancel", onPointerUp, true);
      panCleanup = () => {
        element.removeEventListener("pointerdown", onPointerDown, true);
        element.removeEventListener("pointermove", onPointerMove, true);
        element.removeEventListener("pointerup", onPointerUp, true);
        element.removeEventListener("pointercancel", onPointerUp, true);
      };
      if (typeof ResizeObserver === "function") {
        observer = new ResizeObserver(resize);
        observer.observe(chartRef.current);
      }
      window.addEventListener("resize", resize);
    });
    return () => {
      disposed = true;
      observer?.disconnect();
      window.removeEventListener("resize", resize);
      panCleanup?.();
      panCleanup = null;
      chart?.dispose();
      chartInstanceRef.current = null;
    };
  }, []);

  useEffect(() => {
    zoomRef.current = DEFAULT_ZOOM;
    userZoomedRef.current = false;
    const chart = chartInstanceRef.current;
    if (chart && !chart.isDisposed()) {
      chart.dispatchAction({ type: "dataZoom", start: DEFAULT_ZOOM.start, end: DEFAULT_ZOOM.end });
    }
  }, [interval, symbol]);

  const activeSymbol = payload?.symbol || symbol;
  const hasMonitorFlow = (payload?.monitorFlowCandles || 0) > 0;
  // Binance futures K-lines include taker-buy volume, which gives a reliable
  // historical Delta even before the local monitor stream has warmed up.
  // Public fallback K-lines intentionally omit those fields.
  const hasKlineDelta = payload?.candles?.some((candle) => (
    Number(candle.buyBase || 0) > 0 || Number(candle.sellBase || 0) > 0 || Math.abs(Number(candle.deltaBase || 0)) > 0
  )) || false;
  const hasDelta = hasMonitorFlow || hasKlineDelta;
  const deltaSource = hasMonitorFlow ? "monitor" : hasKlineDelta ? "kline" : "none";
  const chartOption = useMemo(
    () => buildChartOption(payload?.candles || [], hasDelta, activeSymbol, interval, zoomRef.current, deltaSource),
    [activeSymbol, deltaSource, hasDelta, interval, payload],
  );
  chartOptionRef.current = chartOption;

  useEffect(() => {
    const chart = chartInstanceRef.current;
    if (chart && !chart.isDisposed()) {
      // Keep dataZoom out of polling updates. Including it in every setOption
      // call makes ECharts treat the newest candle as the active anchor and
      // causes a manually dragged viewport to jump back to the right edge.
      const chartOptionWithoutZoom = { ...chartOption };
      delete chartOptionWithoutZoom.dataZoom;
      chart.setOption(chartOptionWithoutZoom, {
        lazyUpdate: true,
        notMerge: false,
        replaceMerge: ["series", "xAxis", "yAxis"],
      });
      if (!userZoomedRef.current) {
        chart.dispatchAction({ type: "dataZoom", start: zoomRef.current.start, end: zoomRef.current.end });
      }
    }
  }, [chartOption]);

  const latest = payload?.candles?.[payload.candles.length - 1] || null;

  return (
    <section className="rounded-2xl border border-cyan-400/20 bg-slate-950/80 p-4 shadow-glow" data-testid="binance-orderflow-chart">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-slate-800 pb-4">
        <div>
          <p className="text-[10px] font-semibold uppercase tracking-[0.2em] text-cyan-300">Binance orderflow</p>
          <h2 className="mt-1 text-xl font-semibold text-slate-100">主动买卖差 K 线</h2>
        <p className="mt-1 text-xs text-slate-500">Binance USDⓈ-M 永续 · Delta = 主动买 - 主动卖 · VPIN/TOF 异常自动标记 · {getDeltaMarkRule(activeSymbol, interval).description} · 只读数据</p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex rounded-lg border border-slate-700/70 bg-slate-900/80 p-1" role="group" aria-label="选择交易对">
            {SYMBOLS.map((item) => (
              <button
                className={`rounded-md px-3 py-1.5 text-xs font-semibold transition ${symbol === item ? "bg-cyan-400/15 text-cyan-200" : "text-slate-500 hover:text-slate-200"}`}
                key={item}
                onClick={() => setSymbol(item)}
                type="button"
              >
                {item}
              </button>
            ))}
          </div>
          <div className="flex flex-wrap rounded-lg border border-slate-700/70 bg-slate-900/80 p-1" role="group" aria-label="选择周期">
            {INTERVALS.map(([value, label]) => (
              <button
                aria-label={label}
                className={`rounded-md px-2.5 py-1.5 text-xs transition ${interval === value ? "bg-cyan-400/15 font-semibold text-cyan-200" : "text-slate-500 hover:text-slate-200"}`}
                key={value}
                onClick={() => setInterval(value)}
                type="button"
              >
                {value}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="mt-4 grid gap-2 sm:grid-cols-5">
        <Metric label="最新价" value={latest ? formatPrice(latest.close) : "—"} />
        <Metric label="主动买" value={latest && hasDelta ? formatQuantity(latest.buyBase, activeSymbol) : "—"} tone="buy" />
        <Metric label="主动卖" value={latest && hasDelta ? formatQuantity(latest.sellBase, activeSymbol) : "—"} tone="sell" />
        <Metric label="当前 Delta" value={latest && hasDelta ? formatSignedQuantity(latest.deltaBase, activeSymbol) : "—"} tone={latest?.deltaBase >= 0 ? "buy" : "sell"} />
        <Metric label="VPIN" value={latest?.vpin == null ? "—" : latest.vpin.toFixed(3)} tone={latest?.vpinExtreme || latest?.tofAlert ? "alert" : latest?.vpinSpike ? "warn" : undefined} />
      </div>

      {requestState.phase === "error" ? (
        <div className="mt-4 rounded-lg border border-red-400/30 bg-red-500/10 px-3 py-2 text-sm text-red-200" role="alert">
          {requestState.error}
        </div>
      ) : null}
      <div aria-label="Binance K线与买卖差图表" className="mt-3 h-[620px] w-full" ref={chartRef} role="img" style={{ touchAction: "none" }} />
      <div className="mt-2 flex flex-wrap items-center justify-between gap-2 text-[11px] text-slate-500">
        <span>{payload?.candles?.length || 0} 根 K 线 · 历史 {HISTORY_REFRESH_MS / 1000} 秒刷新</span>
        <span className="flex flex-wrap items-center gap-2" data-testid="orderflow-live-status" role="status" aria-live="polite">
          <span className={`inline-flex items-center gap-1.5 ${livePhase === "degraded" ? "text-amber-300" : "text-emerald-300"}`}>
            <span aria-hidden="true" className={`h-1.5 w-1.5 rounded-full ${livePhase === "degraded" ? "bg-amber-300" : "bg-emerald-300"}`} />
            {livePhase === "refreshing" ? "实时更新中" : livePhase === "degraded" ? "实时链路降级，自动重试" : `实时 ${LIVE_REFRESH_MS / 1000} 秒`}
          </span>
          <span>最后更新 {formatAgeLabel(payload?.asOfMs, nowMs)}</span>
          <span>{latest?.closed === false ? "当前 K 线未收盘，持续更新" : requestState.phase === "refreshing" ? "历史数据刷新中…" : payload && !hasMonitorFlow && hasKlineDelta ? "Binance K线主动买卖差 · 监控流预热中" : payload && !hasDelta ? "监控买卖差暂无，K线仍保留" : payload ? `数据时间 ${formatTime(payload.asOfMs)}` : "等待 Binance 数据"}</span>
        </span>
      </div>
    </section>
  );
}

function Metric({ label, value, tone }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2">
      <p className="text-[10px] uppercase tracking-[0.12em] text-slate-500">{label}</p>
    <p className={`mt-1 text-sm font-semibold ${tone === "buy" ? "text-cyan-200" : tone === "sell" ? "text-rose-200" : tone === "alert" ? "text-red-300" : tone === "warn" ? "text-amber-300" : "text-slate-100"}`}>{value}</p>
    </div>
  );
}

function buildChartOption(candles, hasDelta, symbol = "BTCUSDT", interval = "1h", zoom = DEFAULT_ZOOM, deltaSource = "monitor") {
  // Extra category slots create a right-side gutter without adding synthetic
  // series values (ECharts leaves those future categories empty).
  const categories = [
    ...candles.map((candle) => formatTime(candle.time)),
    ...Array.from({ length: FUTURE_SLOTS }, () => ""),
  ];
  const markRule = getDeltaMarkRule(symbol, interval);
  const largeDeltaMarkers = hasDelta ? candles.flatMap((candle, index) => {
    const deltaBase = Number(candle.deltaBase || 0);
    const thresholdDelta = markRule.mode === "quote" ? Number(candle.deltaQuote || 0) : deltaBase;
    if (Math.abs(thresholdDelta) < markRule.threshold) return [];
    const positive = deltaBase >= 0;
    return [{
      coord: [index, candle.close],
      value: deltaBase,
      itemStyle: { color: positive ? "#22d3ee" : "#fb7185", borderColor: "#0f172a", borderWidth: 1 },
      label: {
        show: true,
        position: positive ? "top" : "bottom",
        color: positive ? "#a5f3fc" : "#fecdd3",
        fontSize: 10,
        fontWeight: 700,
        backgroundColor: "rgba(15,23,42,0.92)",
        borderColor: positive ? "#155e75" : "#9f1239",
        borderWidth: 1,
        borderRadius: 3,
        padding: [2, 4],
        formatter: `Δ ${formatSignedQuantity(deltaBase, symbol)}`,
      },
    }];
  }) : [];
  const tofMarkers = candles.flatMap((candle, index) => {
    if (!candle.vpinSpike && !candle.vpinHigh && !candle.vpinExtreme && !candle.tofAlert) return [];
    const extreme = candle.vpinExtreme || candle.tofAlert;
    return [{
      coord: [index, candle.close],
      value: candle.vpin,
      itemStyle: { color: extreme ? "#ef4444" : "#f59e0b", borderColor: "#0f172a", borderWidth: 1 },
      label: {
        show: true,
        position: "top",
        color: extreme ? "#fecaca" : "#fde68a",
        fontSize: 9,
        fontWeight: 700,
        backgroundColor: "rgba(15,23,42,0.92)",
        padding: [2, 3],
        formatter: extreme ? "TOF" : "VPIN",
      },
    }];
  });
  return {
    animation: false,
    backgroundColor: "transparent",
    axisPointer: { link: [{ xAxisIndex: "all" }] },
    grid: [
      { left: 64, right: 20, top: 36, height: "48%" },
      { left: 64, right: 20, top: "60%", height: "16%" },
      { left: 64, right: 20, top: "81%", height: "12%" },
    ],
    legend: { data: ["K线", "Delta", "VPIN"], textStyle: { color: "#94a3b8" }, top: 4, right: 18 },
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "cross", lineStyle: { color: "#64748b" } },
      backgroundColor: "rgba(15, 23, 42, 0.96)",
      borderColor: "#334155",
      textStyle: { color: "#e2e8f0", fontSize: 12 },
      formatter: (params) => formatTooltip(params, hasDelta, symbol, deltaSource),
    },
      xAxis: [
      xAxis(categories, 0, false),
      xAxis(categories, 1, false),
      xAxis(categories, 2, true),
    ],
    yAxis: [
      { type: "value", gridIndex: 0, scale: true, axisLabel: { color: "#94a3b8" }, splitLine: { lineStyle: { color: "#1e293b" } } },
      { type: "value", gridIndex: 1, scale: true, axisLabel: { color: "#94a3b8", formatter: compactNumber }, splitLine: { lineStyle: { color: "#1e293b" } } },
      { type: "value", gridIndex: 2, min: 0, max: 1, axisLabel: { color: "#94a3b8", formatter: (value) => Number(value).toFixed(1) }, splitLine: { lineStyle: { color: "#1e293b" } } },
    ],
    dataZoom: [
      {
        type: "inside",
        xAxisIndex: [0, 1, 2],
        filterMode: "none",
        realtime: true,
        throttle: 16,
        start: zoom.start,
        end: zoom.end,
        // Keep panning available even when the pointer is over a candle.
        moveOnMouseMove: false,
        moveOnMouseWheel: true,
        zoomOnMouseWheel: true,
      },
      {
        type: "slider",
        xAxisIndex: [0, 1, 2],
        bottom: 8,
        height: 18,
        borderColor: "#334155",
        backgroundColor: "#0f172a",
        fillerColor: "rgba(34,211,238,0.16)",
        handleStyle: { color: "#22d3ee" },
        realtime: true,
        throttle: 16,
        start: zoom.start,
        end: zoom.end,
        moveOnMouseMove: true,
      },
    ],
    series: [
      {
        name: "K线",
        type: "candlestick",
        xAxisIndex: 0,
        yAxisIndex: 0,
        data: candles.map((candle) => ({ value: [candle.open, candle.close, candle.low, candle.high], candle })),
        itemStyle: { color: "#22c55e", color0: "#f43f5e", borderColor: "#4ade80", borderColor0: "#fb7185" },
        markPoint: {
          silent: true,
          symbol: "circle",
          symbolSize: 8,
          data: [...largeDeltaMarkers, ...tofMarkers],
        },
      },
      {
        name: "Delta",
        type: "bar",
        xAxisIndex: 1,
        yAxisIndex: 1,
        barMaxWidth: 18,
        data: hasDelta ? candles.map((candle) => ({ value: candle.deltaBase, candle, itemStyle: { color: candle.deltaBase >= 0 ? "#22d3ee" : "#fb7185", opacity: Math.min(1, 0.45 + Math.abs(candle.deltaPct) / 100) } })) : [],
        markLine: { silent: true, symbol: "none", lineStyle: { color: "#475569" }, data: [{ yAxis: 0 }] },
      },
      {
        name: "VPIN",
        type: "line",
        xAxisIndex: 2,
        yAxisIndex: 2,
        showSymbol: false,
        connectNulls: false,
        lineStyle: { color: "#f59e0b", width: 1.5 },
        data: candles.map((candle) => ({ value: candle.vpin, candle })),
        markLine: { silent: true, symbol: "none", lineStyle: { color: "#92400e", type: "dashed" }, data: [{ yAxis: 0.7 }, { yAxis: 0.85 }] },
      },
    ],
  };
}

function xAxis(data, gridIndex, showLabels) {
  return {
    type: "category",
    gridIndex,
    data,
    boundaryGap: true,
    axisLine: { lineStyle: { color: "#334155" } },
    axisLabel: { color: "#64748b", show: showLabels, hideOverlap: true },
    axisTick: { show: false },
  };
}

function formatTooltip(params, hasDelta, symbol = "BTCUSDT", deltaSource = "monitor") {
  const candle = params.find((item) => item.data?.candle)?.data?.candle;
  if (!candle) return "";
  return [
    `<div style="margin-bottom:6px;color:#cbd5e1">${formatTime(candle.time)}</div>`,
    `开 ${formatPrice(candle.open)}　高 ${formatPrice(candle.high)}`,
    `低 ${formatPrice(candle.low)}　收 ${formatPrice(candle.close)}`,
    ...(hasDelta ? [
      `<span style="color:#67e8f9">主动买 ${formatQuantity(candle.buyBase, symbol)}</span>`,
      `<span style="color:#fda4af">主动卖 ${formatQuantity(candle.sellBase, symbol)}</span>`,
      `<b>Delta ${formatSignedQuantity(candle.deltaBase, symbol)} (${candle.deltaPct.toFixed(2)}%)</b>`,
      ...(deltaSource === "kline" ? ["<span style=\"color:#94a3b8\">来源：Binance K线主动买量（实时监控流预热中）</span>"] : []),
    ] : ["<span style=\"color:#94a3b8\">监控买卖差暂无数据</span>"]),
    ...(candle.vpin == null ? [] : [
      `<span style="color:#fbbf24">VPIN ${candle.vpin.toFixed(3)}${candle.vpinZscore == null ? "" : ` · Z ${candle.vpinZscore.toFixed(2)}`}</span>`,
      candle.tofAlert || candle.vpinExtreme ? `<b style="color:#fca5a5">TOF ${candle.tofSeverity || "EXTREME"}</b>` : candle.vpinSpike ? "<span style=\"color:#fde68a\">VPIN SPIKE</span>" : "",
    ].filter(Boolean)),
    `成交量 ${formatQuantity(candle.volumeBase, symbol)} · ${candle.tradeCount.toLocaleString()} 笔`,
  ].join("<br/>");
}

function formatTime(value) {
  if (!value) return "—";
  return new Date(value).toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

function formatAgeLabel(value, now = Date.now()) {
  const timestamp = Number(value);
  if (!Number.isFinite(timestamp) || timestamp <= 0) return "等待数据";
  const ageMs = Math.max(0, Number(now) - timestamp);
  if (ageMs < 1_000) return "刚刚";
  if (ageMs < 60_000) return `${Math.floor(ageMs / 1_000)} 秒前`;
  return `${Math.floor(ageMs / 60_000)} 分钟前`;
}

export function mergeBinanceOrderflowPayload(current, next, maxCandles = 150) {
  if (!next) return current;
  if (!current || current.symbol !== next.symbol || current.interval !== next.interval) return next;
  const currentAsOf = Number(current.asOfMs);
  const nextAsOf = Number(next.asOfMs);
  // History and live requests run concurrently; ignore an older response so
  // it cannot roll the current, still-open candle back to stale values.
  if (Number.isFinite(currentAsOf) && Number.isFinite(nextAsOf) && nextAsOf < currentAsOf) return current;
  if (!Array.isArray(next.candles) || next.candles.length === 0) return { ...current, ...next };
  const byTime = new Map((current.candles || []).map((candle) => [Number(candle.time), candle]));
  next.candles.forEach((candle) => byTime.set(Number(candle.time), candle));
  const candles = [...byTime.values()]
    .filter((candle) => Number.isFinite(Number(candle.time)))
    .sort((left, right) => Number(left.time) - Number(right.time))
    .slice(-maxCandles);
  const monitorFlowCandles = Math.max(Number(current.monitorFlowCandles || 0), Number(next.monitorFlowCandles || 0));
  return {
    ...current,
    ...next,
    monitorFlowCandles,
    source: monitorFlowCandles > 0 ? (current.source?.includes("monitor_flow") ? current.source : "binance_monitor_flow_with_futures_kline") : next.source,
    candles,
  };
}

function formatPrice(value) {
  return Number(value || 0).toLocaleString("en-US", { maximumFractionDigits: 2 });
}


function getDeltaMarkRule(symbol, interval = "1h") {
  const baseRule = DELTA_MARK_RULES[String(symbol || "BTCUSDT").toUpperCase()] || DELTA_MARK_RULES.BTCUSDT;
  const intervalMinutes = {
    "1m": 1,
    "5m": 5,
    "15m": 15,
    "30m": 30,
    "1h": 60,
    "4h": 240,
    "1d": 1440,
  }[interval] || 60;
  // Keep the user's 1h floors (BTC 1,000 coins / ETH 40M USDT), while
  // scaling the visual gate with sqrt(time) so a 1m candle is not silent and
  // a 1d candle is not marked on nearly every bar.
  const scale = Math.sqrt(intervalMinutes / 60);
  const threshold = baseRule.threshold * scale;
  return {
    ...baseRule,
    threshold,
    description: `${baseRule.mode === "quote" ? "|Delta|" : "|Delta|"} ≥ ${formatThreshold(threshold, baseRule.mode)}`,
  };
}

function formatThreshold(value, mode) {
  if (mode === "quote") {
    return `${(value / 1_000_000).toFixed(value >= 10_000_000 ? 1 : 2)}M USDT`;
  }
  return `${Math.round(value).toLocaleString("en-US")} BTC`;
}

function formatQuantity(value, symbol = "BTCUSDT") {
  const number = Number(value || 0);
  const absolute = Math.abs(number);
  const digits = absolute >= 100 ? 1 : absolute >= 1 ? 2 : absolute >= 0.01 ? 3 : 4;
  const asset = String(symbol || "BTCUSDT").toUpperCase().replace(/USDT$/, "");
  return `${number.toFixed(digits)} ${asset}`;
}

function formatSignedQuantity(value, symbol = "BTCUSDT") {
  const number = Number(value || 0);
  return `${number >= 0 ? "+" : "-"}${formatQuantity(Math.abs(number), symbol)}`;
}

function compactNumber(value) {
  const number = Number(value || 0);
  const absolute = Math.abs(number);
  if (absolute >= 1e9) return `${(number / 1e9).toFixed(2)}B`;
  if (absolute >= 1e6) return `${(number / 1e6).toFixed(2)}M`;
  if (absolute >= 1e3) return `${(number / 1e3).toFixed(1)}K`;
  return number.toFixed(0);
}
