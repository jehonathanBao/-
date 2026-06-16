import { useEffect, useMemo, useState } from "react";
import { fetchBtcLiquidationDashboard } from "../api/btcLiquidation.js";

const TIMEFRAMES = ["1m", "5m", "15m"];
const OVERLAY_MODES = [
  { key: "all", label: "全部" },
  { key: "heat", label: "清算" },
  { key: "gamma", label: "Gamma" },
  { key: "cascade", label: "级联" },
];

export default function BTCLiquidationDashboard() {
  const [dashboard, setDashboard] = useState(null);
  const [error, setError] = useState(null);
  const [timeframe, setTimeframe] = useState("5m");
  const [overlayMode, setOverlayMode] = useState("all");
  const [heatIntensity, setHeatIntensity] = useState(78);

  useEffect(() => {
    let cancelled = false;
    let timer = null;

    async function load() {
      const result = await fetchBtcLiquidationDashboard();
      if (cancelled) return;
      setDashboard(result.dashboard);
      setError(result.error);
    }

    load();
    timer = window.setInterval(() => {
      if (document.visibilityState !== "hidden") {
        load();
      }
    }, 5000);

    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, []);

  const data = dashboard || emptyDashboard();
  const heatmap = useMemo(() => topByRisk(data.liquidationHeatmap, 13), [data.liquidationHeatmap]);
  const gammaWalls = useMemo(() => topByAbs(data.gammaWalls, "gammaExposure", 6), [data.gammaWalls]);
  const chartModel = useMemo(
    () => buildChartModel(data, heatmap, gammaWalls, timeframe),
    [data, gammaWalls, heatmap, timeframe],
  );

  return (
    <section className="space-y-4">
      <div className="console-panel overflow-hidden p-0">
        <div className="border-b border-slate-800/90 bg-slate-950/70 px-4 py-3">
          <div className="flex flex-col gap-4 xl:flex-row xl:items-center xl:justify-between">
            <div>
              <p className="text-xs uppercase tracking-[0.32em] text-cyan-300">BTC Liquidation Force Field</p>
              <div className="mt-2 flex flex-wrap items-end gap-3">
                <h3 className="text-2xl font-black text-white">BTC 清算力场</h3>
                <span className="font-mono text-lg font-bold text-cyan-100">{formatUsd(data.currentPriceUsd)}</span>
                <StatusDot live={data.live} />
              </div>
              <p className="mt-2 max-w-4xl text-sm leading-6 text-slate-400">
                TradingView 风格热力主屏，展示清算簇、Gamma 墙、级联路径和流动性真空区。只读分析，不下单。
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-3">
              <SegmentedControl items={TIMEFRAMES} value={timeframe} onChange={setTimeframe} />
              <SegmentedControl
                items={OVERLAY_MODES}
                value={overlayMode}
                onChange={setOverlayMode}
                valueKey="key"
                labelKey="label"
              />
              <label className="flex min-w-[190px] items-center gap-3 rounded-xl border border-slate-700/70 bg-slate-950/70 px-3 py-2 text-xs text-slate-400">
                热度
                <input
                  aria-label="heat intensity"
                  className="accent-cyan-300"
                  max="100"
                  min="20"
                  onChange={(event) => setHeatIntensity(Number(event.target.value))}
                  type="range"
                  value={heatIntensity}
                />
                <span className="font-mono text-cyan-100">{heatIntensity}</span>
              </label>
            </div>
          </div>
          {error ? (
            <div className="mt-3 rounded-xl border border-yellow-400/30 bg-yellow-400/10 px-4 py-3 text-sm text-yellow-100">
              BTC 清算 API 暂不可用，前端已显示安全空态：{error}
            </div>
          ) : null}
        </div>

        <div className="grid min-h-[620px] grid-cols-1 xl:grid-cols-[minmax(0,1fr)_330px]">
          <TradingChart
            data={data}
            heatIntensity={heatIntensity}
            model={chartModel}
            overlayMode={overlayMode}
          />
          <RightMetricsPanel data={data} />
        </div>
      </div>

      <div className="grid gap-4 xl:grid-cols-[1.2fr_0.8fr]">
        <LiquidationHeatTable levels={heatmap} />
        <GammaWallPanel walls={gammaWalls} />
      </div>

      <div className="grid gap-4 xl:grid-cols-[0.95fr_1.05fr]">
        <CascadeTimeline items={data.cascadeTimeline} />
        <LiquidityMap items={data.liquidityMap} sources={data.sources} notes={data.notes} />
      </div>
    </section>
  );
}

function TradingChart({ data, model, overlayMode, heatIntensity }) {
  const showHeat = overlayMode === "all" || overlayMode === "heat";
  const showGamma = overlayMode === "all" || overlayMode === "gamma";
  const showCascade = overlayMode === "all" || overlayMode === "cascade";
  const stress = data.marketStress || {};
  const squeeze = data.squeeze || {};

  return (
    <div className="relative min-h-[620px] overflow-hidden bg-[#07111f]">
      <div className="absolute inset-0 bg-[linear-gradient(rgba(148,163,184,0.08)_1px,transparent_1px),linear-gradient(90deg,rgba(148,163,184,0.06)_1px,transparent_1px)] bg-[size:100%_54px,86px_100%]" />
      <div className="absolute inset-x-0 top-0 z-20 flex flex-wrap items-center justify-between gap-3 border-b border-slate-800/80 bg-slate-950/55 px-4 py-3 backdrop-blur">
        <div className="flex flex-wrap items-center gap-2 text-xs">
          <ChartBadge label={`Stress ${formatPct(stress.stressScore)}`} tone={Number(stress.stressScore || 0) > 0.7 ? "red" : "cyan"} />
          <ChartBadge label={`Cascade ${formatPct(stress.cascadeRisk)}`} tone={Number(stress.cascadeRisk || 0) > 0.65 ? "red" : "yellow"} />
          <ChartBadge label={`Gamma ${formatPct(stress.gammaPressure)}`} tone="purple" />
        </div>
        <div className="font-mono text-xs text-slate-400">
          {data.live ? "LIVE" : "WAITING"} · {data.dataStatus || "unknown"}
        </div>
      </div>

      <div className="absolute inset-x-0 bottom-0 top-[58px]">
        {showHeat
          ? model.heatBands.map((band) => (
              <div
                className="btc-heat-wave absolute left-0 right-[74px] rounded-full"
                key={`heat-${band.key}`}
                style={{
                  top: `${band.y}%`,
                  height: `${Math.max(18, band.height)}px`,
                  opacity: Math.min(0.88, band.intensity * (heatIntensity / 100)),
                  background:
                    band.side === "above"
                      ? "linear-gradient(90deg, rgba(248,113,113,0.04), rgba(251,146,60,0.55), rgba(248,113,113,0.08))"
                      : "linear-gradient(90deg, rgba(14,165,233,0.04), rgba(248,113,113,0.62), rgba(14,165,233,0.04))",
                  boxShadow: `0 0 ${24 + band.intensity * 42}px rgba(248,113,113,${0.18 + band.intensity * 0.35})`,
                  animationDelay: `${band.delay}s`,
                }}
              />
            ))
          : null}

        {showGamma
          ? model.gammaBands.map((band) => (
              <div
                className="btc-gamma-wall absolute left-8 right-[82px]"
                key={`gamma-${band.key}`}
                style={{
                  top: `${band.y}%`,
                  height: `${Math.max(6, band.height)}px`,
                  opacity: 0.35 + band.intensity * 0.55,
                  background:
                    band.role === "support"
                      ? "linear-gradient(90deg, transparent, rgba(168,85,247,0.65), rgba(34,211,238,0.35), transparent)"
                      : "linear-gradient(90deg, transparent, rgba(217,70,239,0.70), rgba(248,113,113,0.35), transparent)",
                  animationDelay: `${band.delay}s`,
                }}
              />
            ))
          : null}

        {showCascade
          ? model.cascadePoints.map((point) => (
              <div
                className="btc-cascade-ripple absolute"
                key={`cascade-${point.key}`}
                style={{
                  left: `${point.x}%`,
                  top: `${point.y}%`,
                  width: `${point.size}px`,
                  height: `${point.size}px`,
                  opacity: 0.35 + point.intensity * 0.55,
                  animationDelay: `${point.delay}s`,
                }}
              />
            ))
          : null}

        <svg className="absolute inset-0 z-10 h-full w-full" preserveAspectRatio="none" viewBox="0 0 1000 520">
          <defs>
            <linearGradient id="btcForceLine" x1="0" x2="1" y1="0" y2="0">
              <stop offset="0%" stopColor="#22d3ee" stopOpacity="0.55" />
              <stop offset="50%" stopColor="#e2e8f0" stopOpacity="0.85" />
              <stop offset="100%" stopColor="#fb7185" stopOpacity="0.55" />
            </linearGradient>
          </defs>
          <path d={model.areaPath} fill="url(#btcForceLine)" opacity="0.05" />
          <path d={model.linePath} fill="none" stroke="url(#btcForceLine)" strokeLinecap="round" strokeWidth="2.4" />
          {model.candles.map((candle) => (
            <g key={candle.key}>
              <line
                stroke={candle.up ? "#34d399" : "#fb7185"}
                strokeOpacity="0.75"
                strokeWidth="1.2"
                x1={candle.x}
                x2={candle.x}
                y1={candle.high}
                y2={candle.low}
              />
              <rect
                fill={candle.up ? "#34d399" : "#fb7185"}
                fillOpacity="0.9"
                height={Math.max(3, Math.abs(candle.close - candle.open))}
                rx="1.2"
                width="9"
                x={candle.x - 4.5}
                y={Math.min(candle.open, candle.close)}
              />
            </g>
          ))}
        </svg>

        <div className="absolute bottom-4 left-4 z-20 grid gap-2 text-xs text-slate-400 sm:grid-cols-3">
          <Info label="向上挤压" value={formatPct(squeeze.upProbability)} />
          <Info label="向下挤压" value={formatPct(squeeze.downProbability)} />
          <Info label="主方向" value={squeezeDirectionLabel(squeeze.dominantDirection)} />
        </div>

        <div className="absolute bottom-0 right-0 top-0 z-20 w-[74px] border-l border-slate-800/80 bg-slate-950/45">
          {model.axisLabels.map((label) => (
            <span
              className="absolute right-2 font-mono text-[11px] text-slate-500"
              key={label.value}
              style={{ top: `${label.y}%` }}
            >
              {formatUsd(label.value)}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

function RightMetricsPanel({ data }) {
  const stress = data.marketStress || {};
  const squeeze = data.squeeze || {};
  return (
    <aside className="border-t border-slate-800/80 bg-slate-950/80 p-4 xl:border-l xl:border-t-0">
      <p className="text-xs uppercase tracking-[0.28em] text-cyan-300">Liquidation Status</p>
      <h4 className="mt-2 text-lg font-bold text-white">BTC 清算状态</h4>
      <div className="mt-4 grid gap-3">
        <Metric label="Squeeze Up" value={formatPct(squeeze.upProbability)} tone="emerald" />
        <Metric label="Squeeze Down" value={formatPct(squeeze.downProbability)} tone="red" />
        <Metric label="Cascade Risk" value={formatPct(stress.cascadeRisk)} tone="yellow" />
        <Metric label="Gamma Pressure" value={formatPct(stress.gammaPressure)} tone="purple" />
      </div>
      <div className="mt-4 grid gap-3">
        <Info label="Bias" value={squeezeDirectionLabel(squeeze.dominantDirection)} />
        <Info label="Regime" value={regimeLabel(stress.regime)} />
        <Info label="Net Liq Bias" value={formatSigned(squeeze.netLiquidationBias)} />
      </div>
      <div className="mt-4 rounded-xl border border-cyan-400/20 bg-cyan-400/10 p-3 text-xs leading-5 text-cyan-100">
        Heat layer 是当前 flow proxy，可用于看结构压力，不代表交易建议。
      </div>
    </aside>
  );
}

function LiquidationHeatTable({ levels }) {
  return (
    <Panel eyebrow="Liquidation Heatmap" title="清算热力表">
      <div className="overflow-x-auto">
        <div className="min-w-[720px] space-y-2">
          {levels.length ? (
            levels.map((level) => (
              <div className="grid grid-cols-[110px_minmax(0,1fr)_92px_92px] items-center gap-3 text-sm" key={`${level.normalizedPrice}-${level.side}`}>
                <div>
                  <p className="font-semibold text-slate-100">{formatUsd(level.priceUsd)}</p>
                  <p className="text-xs text-slate-500">{sideLabel(level.side)}</p>
                </div>
                <div className="h-3 overflow-hidden rounded-full bg-slate-800">
                  <div
                    className={["h-full rounded-full", level.side === "above" ? "bg-orange-400" : level.side === "below" ? "bg-red-400" : "bg-cyan-400"].join(" ")}
                    style={{ width: `${Math.max(3, Math.round(level.riskScore * 100))}%` }}
                  />
                </div>
                <span className="text-right font-mono text-cyan-100">{formatPct(level.riskScore)}</span>
                <span className="text-right text-slate-400">{formatBtc(level.liquidationVolume)}</span>
              </div>
            ))
          ) : (
            <EmptyLine text="等待 BTC flow 生成清算热力数据" />
          )}
        </div>
      </div>
    </Panel>
  );
}

function GammaWallPanel({ walls }) {
  return (
    <Panel eyebrow="Gamma Wall Zones" title="Gamma 墙">
      <div className="space-y-3">
        {walls.length ? (
          walls.map((wall) => (
            <div className="rounded-xl border border-slate-700/70 bg-slate-950/60 p-3" key={`${wall.normalizedStrike}-${wall.gammaExposure}`}>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="font-semibold text-white">{formatUsd(wall.strikeUsd)}</p>
                  <p className="text-xs uppercase tracking-[0.18em] text-slate-500">{wall.role}</p>
                </div>
                <span className={["rounded-full px-2 py-1 text-xs font-semibold", wall.gammaExposure >= 0 ? "bg-emerald-400/10 text-emerald-200" : "bg-red-400/10 text-red-200"].join(" ")}>
                  {formatSigned(wall.gammaExposure)}
                </span>
              </div>
              <p className="mt-2 text-xs text-slate-400">Call/Put imbalance {formatSigned(wall.callPutImbalance)}</p>
            </div>
          ))
        ) : (
          <EmptyLine text="暂无 gamma wall proxy" />
        )}
      </div>
    </Panel>
  );
}

function CascadeTimeline({ items }) {
  return (
    <Panel eyebrow="Cascade Risk Timeline" title="级联路径">
      <div className="space-y-3">
        {items.length ? (
          items.map((item) => (
            <div className="grid grid-cols-[36px_1fr_auto] items-center gap-3 rounded-xl border border-slate-700/70 bg-slate-950/60 p-3" key={`${item.step}-${item.normalizedPrice}`}>
              <span className="flex h-8 w-8 items-center justify-center rounded-full bg-cyan-400/10 text-xs font-black text-cyan-200">{item.step}</span>
              <div>
                <p className="font-semibold text-white">{formatUsd(item.priceUsd)}</p>
                <p className="text-xs text-slate-500">expected liq {formatBtc(item.expectedLiquidation)}</p>
              </div>
              <span className="font-mono text-sm text-yellow-200">x{Number(item.impactAmplification || 0).toFixed(2)}</span>
            </div>
          ))
        ) : (
          <EmptyLine text="当前没有形成连续级联路径" />
        )}
      </div>
    </Panel>
  );
}

function LiquidityMap({ items, sources, notes }) {
  return (
    <Panel eyebrow="Real-time Liquidity Map" title="流动性地图">
      <div className="space-y-3">
        {items.length ? (
          items.map((item) => (
            <div className="grid grid-cols-[110px_minmax(0,1fr)_70px] items-center gap-3 text-sm" key={`${item.normalizedPrice}-${item.label}`}>
              <div>
                <p className="font-semibold text-white">{formatUsd(item.priceUsd)}</p>
                <p className="text-xs text-slate-500">{sideLabel(item.side)}</p>
              </div>
              <div className="h-2 overflow-hidden rounded-full bg-slate-800">
                <div className="h-full rounded-full bg-blue-300" style={{ width: `${Math.max(3, Math.round(item.pressure * 100))}%` }} />
              </div>
              <span className="text-right font-mono text-blue-100">{formatPct(item.pressure)}</span>
            </div>
          ))
        ) : (
          <EmptyLine text="暂无显著流动性真空区" />
        )}
      </div>
      <div className="mt-4 grid gap-2 text-xs text-slate-400 sm:grid-cols-2">
        <span>Flow: {sources?.flow || "unavailable"}</span>
        <span>Liquidation: {sources?.liquidation || "unavailable"}</span>
        <span>Gamma: {sources?.optionsGamma || "unavailable"}</span>
        <span>Orderbook: {sources?.orderbook || "unavailable"}</span>
      </div>
      <div className="mt-4 space-y-1 text-xs text-slate-500">
        {(notes || []).map((note) => (
          <p key={note}>{note}</p>
        ))}
      </div>
    </Panel>
  );
}

function SegmentedControl({ items, value, onChange, valueKey, labelKey }) {
  return (
    <div className="flex rounded-xl border border-slate-700/70 bg-slate-950/70 p-1">
      {items.map((item) => {
        const itemValue = valueKey ? item[valueKey] : item;
        const label = labelKey ? item[labelKey] : item;
        return (
          <button
            className={[
              "rounded-lg px-3 py-1.5 text-xs font-semibold transition",
              value === itemValue ? "bg-cyan-400/15 text-cyan-100" : "text-slate-400 hover:text-slate-100",
            ].join(" ")}
            key={itemValue}
            onClick={() => onChange(itemValue)}
            type="button"
          >
            {label}
          </button>
        );
      })}
    </div>
  );
}

function Panel({ eyebrow, title, children }) {
  return (
    <section className="rounded-2xl border border-slate-700/70 bg-slate-900/70 p-4 shadow-glow">
      <p className="text-xs uppercase tracking-[0.28em] text-cyan-300">{eyebrow}</p>
      <h4 className="mt-2 text-lg font-bold text-white">{title}</h4>
      <div className="mt-4">{children}</div>
    </section>
  );
}

function Metric({ label, value, tone = "cyan" }) {
  const tones = {
    cyan: "text-cyan-200 border-cyan-400/25 bg-cyan-400/10",
    emerald: "text-emerald-200 border-emerald-400/25 bg-emerald-400/10",
    red: "text-red-200 border-red-400/25 bg-red-400/10",
    yellow: "text-yellow-200 border-yellow-400/25 bg-yellow-400/10",
    orange: "text-orange-200 border-orange-400/25 bg-orange-400/10",
    purple: "text-fuchsia-200 border-fuchsia-400/25 bg-fuchsia-400/10",
  };
  return (
    <div className={["rounded-xl border p-3", tones[tone] || tones.cyan].join(" ")}>
      <p className="text-xs text-slate-400">{label}</p>
      <p className="mt-1 text-xl font-black">{value}</p>
    </div>
  );
}

function StatusDot({ live }) {
  return (
    <span className={["rounded-full border px-2 py-1 text-xs font-semibold", live ? "border-emerald-400/35 bg-emerald-400/10 text-emerald-200" : "border-yellow-400/35 bg-yellow-400/10 text-yellow-200"].join(" ")}>
      {live ? "实时" : "等待数据"}
    </span>
  );
}

function ChartBadge({ label, tone }) {
  const colors = {
    cyan: "border-cyan-400/25 bg-cyan-400/10 text-cyan-200",
    red: "border-red-400/25 bg-red-400/10 text-red-200",
    yellow: "border-yellow-400/25 bg-yellow-400/10 text-yellow-200",
    purple: "border-fuchsia-400/25 bg-fuchsia-400/10 text-fuchsia-200",
  };
  return <span className={["rounded-full border px-2.5 py-1 font-semibold", colors[tone] || colors.cyan].join(" ")}>{label}</span>;
}

function Info({ label, value }) {
  return (
    <div className="rounded-xl border border-slate-700/70 bg-slate-950/60 p-3">
      <p className="text-xs text-slate-500">{label}</p>
      <p className="mt-1 font-semibold text-white">{value}</p>
    </div>
  );
}

function EmptyLine({ text }) {
  return <div className="rounded-xl border border-slate-700/70 bg-slate-950/50 px-4 py-6 text-sm text-slate-500">{text}</div>;
}

function buildChartModel(data, heatmap, gammaWalls, timeframe) {
  const current = Number(data.currentPriceUsd) || 62000;
  const heatPrices = heatmap.map((item) => Number(item.priceUsd)).filter(Number.isFinite);
  const gammaPrices = gammaWalls.map((item) => Number(item.strikeUsd)).filter(Number.isFinite);
  const cascadePrices = (data.cascadeTimeline || []).map((item) => Number(item.priceUsd)).filter(Number.isFinite);
  const allPrices = [current, ...heatPrices, ...gammaPrices, ...cascadePrices];
  const minObserved = Math.min(...allPrices);
  const maxObserved = Math.max(...allPrices);
  const pad = Math.max(current * 0.006, (maxObserved - minObserved) * 0.25, 350);
  const min = minObserved - pad;
  const max = maxObserved + pad;
  const priceToY = (price) => 92 - ((price - min) / (max - min || 1)) * 78;
  const candles = buildCandles(current, min, max, timeframe);
  const linePath = candles.map((candle, index) => `${index === 0 ? "M" : "L"} ${candle.x} ${candle.close}`).join(" ");
  const areaPath = `${linePath} L ${candles.at(-1)?.x || 960} 520 L ${candles[0]?.x || 40} 520 Z`;

  return {
    candles,
    linePath,
    areaPath,
    axisLabels: [max, (max + current) / 2, current, (min + current) / 2, min].map((value) => ({
      value,
      y: priceToY(value),
    })),
    heatBands: heatmap.map((item, index) => ({
      key: `${item.normalizedPrice}-${index}`,
      y: priceToY(Number(item.priceUsd) || current),
      height: 16 + Number(item.riskScore || 0) * 30,
      intensity: Number(item.riskScore || 0),
      side: item.side,
      delay: index * 0.16,
    })),
    gammaBands: gammaWalls.map((item, index) => ({
      key: `${item.normalizedStrike}-${index}`,
      y: priceToY(Number(item.strikeUsd) || current),
      height: 4 + Math.min(14, Math.abs(Number(item.gammaExposure || 0)) * 4),
      intensity: Math.min(1, Math.abs(Number(item.gammaExposure || 0)) / 8),
      role: item.role,
      delay: index * 0.22,
    })),
    cascadePoints: (data.cascadeTimeline || []).map((item, index) => ({
      key: `${item.step}-${index}`,
      x: 18 + index * 14,
      y: priceToY(Number(item.priceUsd) || current),
      size: 34 + Math.min(46, Number(item.impactAmplification || 1) * 12),
      intensity: Math.min(1, Number(item.impactAmplification || 1) / 3.5),
      delay: index * 0.28,
    })),
  };
}

function buildCandles(current, min, max, timeframe) {
  const count = timeframe === "1m" ? 30 : timeframe === "15m" ? 40 : 34;
  const volatility = timeframe === "15m" ? 0.006 : timeframe === "1m" ? 0.0025 : 0.004;
  const priceToY = (price) => 500 - ((price - min) / (max - min || 1)) * 460;
  let previous = current * (1 - volatility * 0.7);
  return Array.from({ length: count }, (_, index) => {
    const drift = Math.sin(index / 3.2) * current * volatility;
    const openPrice = previous;
    const closePrice = current + drift + Math.cos(index / 4.4) * current * volatility * 0.35;
    const highPrice = Math.max(openPrice, closePrice) + current * volatility * (0.35 + (index % 3) * 0.08);
    const lowPrice = Math.min(openPrice, closePrice) - current * volatility * (0.32 + (index % 4) * 0.06);
    previous = closePrice;
    return {
      key: `candle-${index}`,
      x: 42 + index * (900 / Math.max(1, count - 1)),
      open: priceToY(openPrice),
      close: priceToY(closePrice),
      high: priceToY(highPrice),
      low: priceToY(lowPrice),
      up: closePrice >= openPrice,
    };
  });
}

function topByRisk(items, limit) {
  return [...(items || [])].sort((a, b) => Number(b.riskScore || 0) - Number(a.riskScore || 0)).slice(0, limit);
}

function topByAbs(items, key, limit) {
  return [...(items || [])].sort((a, b) => Math.abs(Number(b[key] || 0)) - Math.abs(Number(a[key] || 0))).slice(0, limit);
}

function formatPct(value) {
  const number = Number(value || 0);
  return `${Math.round(number * 100)}%`;
}

function formatUsd(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  return `$${Math.round(number).toLocaleString()}`;
}

function formatBtc(value) {
  const number = Number(value || 0);
  if (number >= 100) return `${Math.round(number).toLocaleString()} BTC`;
  return `${number.toFixed(2)} BTC`;
}

function formatSigned(value) {
  const number = Number(value || 0);
  const sign = number > 0 ? "+" : "";
  return `${sign}${number.toFixed(2)}`;
}

function sideLabel(side) {
  if (side === "above") return "上方";
  if (side === "below") return "下方";
  return "现价";
}

function regimeLabel(value) {
  const labels = {
    stable: "稳定",
    compression: "压缩",
    fragileaccumulation: "脆弱吸筹",
    fragiledistribution: "脆弱派发",
    criticalinstability: "临界不稳定",
    unknown: "未知",
  };
  return labels[String(value || "").replaceAll("_", "").toLowerCase()] || value || "未知";
}

function squeezeDirectionLabel(value) {
  if (value === "up") return "向上";
  if (value === "down") return "向下";
  return "中性";
}

function emptyDashboard() {
  return {
    symbol: "BTC",
    currentPriceUsd: null,
    dataStatus: "loading",
    readOnly: true,
    live: false,
    marketStress: {},
    liquidationHeatmap: [],
    gammaWalls: [],
    squeeze: {},
    cascadeTimeline: [],
    liquidityMap: [],
    sources: {},
    notes: [],
  };
}
