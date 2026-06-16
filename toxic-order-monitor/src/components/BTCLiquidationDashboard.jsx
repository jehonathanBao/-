import { useEffect, useMemo, useRef, useState } from "react";
import { fetchBtcLiquidationDashboard } from "../api/btcLiquidation.js";
import { createForceFieldRenderer } from "../renderers/webgl/ForceFieldRenderer.js";

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
                WebGL Market Physics 视图，展示清算簇、Gamma 墙、级联路径、流动性真空区和下一步力场偏向。只读分析，不下单。
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
          <ForceFieldView
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

function ForceFieldView({ data, model, overlayMode, heatIntensity }) {
  const showHeat = overlayMode === "all" || overlayMode === "heat";
  const showGamma = overlayMode === "all" || overlayMode === "gamma";
  const showCascade = overlayMode === "all" || overlayMode === "cascade";
  const forceField = data.forceField || {};
  const squeeze = data.squeeze || {};
  const instability = clampNumber(forceField.instabilityIndex);

  return (
    <div className="relative min-h-[620px] overflow-hidden bg-[#07111f]">
      <div className="absolute inset-0 bg-[linear-gradient(rgba(148,163,184,0.08)_1px,transparent_1px),linear-gradient(90deg,rgba(148,163,184,0.06)_1px,transparent_1px)] bg-[size:100%_54px,86px_100%]" />
      <div className="absolute inset-x-0 top-0 z-20 flex flex-wrap items-center justify-between gap-3 border-b border-slate-800/80 bg-slate-950/55 px-4 py-3 backdrop-blur">
        <div className="flex flex-wrap items-center gap-2 text-xs">
          <ChartBadge label={`Stress ${formatPct(forceField.totalStress)}`} tone={Number(forceField.totalStress || 0) > 0.7 ? "red" : "cyan"} />
          <ChartBadge label={`Cascade ${formatPct(forceField.cascadeProbability)}`} tone={Number(forceField.cascadeProbability || 0) > 0.65 ? "red" : "yellow"} />
          <ChartBadge label={`Gamma ${formatPct(forceField.gammaField)}`} tone="purple" />
          <ChartBadge label={`Bias ${forceBiasLabel(forceField.nextMoveBias)}`} tone="cyan" />
        </div>
        <div className="font-mono text-xs text-slate-400">
          {data.live ? "LIVE" : "WAITING"} · {data.dataStatus || "unknown"}
        </div>
      </div>

      <div className="absolute inset-x-0 bottom-0 top-[58px]">
        <HeatFieldCanvas
          cascadePoints={showCascade ? model.cascadePoints : []}
          gammaBands={showGamma ? model.gammaBands : []}
          heatCells={showHeat ? model.heatCells : []}
          fieldState={forceField}
          intensity={heatIntensity}
        />

        <InstabilityGlow value={instability} />
        <GammaWallOverlay bands={showGamma ? model.gammaBands : []} />
        <LiquidationDensityOverlay bands={showHeat ? model.heatBands : []} />
        <CascadeVectorOverlay points={showCascade ? model.cascadePoints : []} />
        <ForceVectorOverlay model={model} forceField={forceField} />

        <div className="absolute bottom-4 left-4 z-20 grid gap-2 text-xs text-slate-400 sm:grid-cols-3">
          <Info label="力场偏向" value={forceBiasLabel(forceField.nextMoveBias)} />
          <Info label="挤压概率" value={formatPct(forceField.squeezeProbability || Math.max(squeeze.upProbability || 0, squeeze.downProbability || 0))} />
          <Info label="预测状态" value={regimeLabel(forceField.predictedRegime)} />
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
  const forceField = data.forceField || {};
  const squeeze = data.squeeze || {};
  return (
    <aside className="border-t border-slate-800/80 bg-slate-950/80 p-4 xl:border-l xl:border-t-0">
      <p className="text-xs uppercase tracking-[0.28em] text-cyan-300">Liquidation Status</p>
      <h4 className="mt-2 text-lg font-bold text-white">BTC 清算状态</h4>
      <div className="mt-4 grid gap-3">
        <Metric label="Total Stress" value={formatPct(forceField.totalStress)} tone="cyan" />
        <Metric label="Instability" value={formatPct(forceField.instabilityIndex)} tone="orange" />
        <Metric label="Squeeze Probability" value={formatPct(forceField.squeezeProbability)} tone="emerald" />
        <Metric label="Cascade Probability" value={formatPct(forceField.cascadeProbability)} tone="yellow" />
      </div>
      <div className="mt-4 grid gap-3">
        <Info label="Next Bias" value={forceBiasLabel(forceField.nextMoveBias)} />
        <Info label="Regime" value={regimeLabel(forceField.predictedRegime)} />
        <Info label="Net Liq Bias" value={formatSigned(squeeze.netLiquidationBias)} />
      </div>
      <ForceFieldRadar forceField={forceField} squeeze={squeeze} />
      <div className="mt-4 rounded-xl border border-cyan-400/20 bg-cyan-400/10 p-3 text-xs leading-5 text-cyan-100">
        Force Field 是研究/预测视图，只表达结构压力与清算物理层，不代表交易建议。
      </div>
    </aside>
  );
}

function HeatFieldCanvas({ heatCells, gammaBands, cascadePoints, fieldState, intensity }) {
  const canvasRef = useRef(null);
  const rendererRef = useRef(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;

    rendererRef.current = createForceFieldRenderer(canvas);
    let animationFrame = 0;
    const startedAt = performance.now();

    const draw = (now) => {
      const renderer = rendererRef.current;
      if (renderer) {
        renderer.render({
          cascadePoints,
          fieldState,
          gammaBands,
          heatCells,
          intensity: Number(intensity || 78) / 100,
          time: (now - startedAt) / 1000,
        });
      } else {
        drawCanvasFallback(canvas, { cascadePoints, fieldState, gammaBands, heatCells, intensity });
      }
      animationFrame = window.requestAnimationFrame(draw);
    };

    animationFrame = window.requestAnimationFrame(draw);
    return () => {
      window.cancelAnimationFrame(animationFrame);
      rendererRef.current?.dispose?.();
      rendererRef.current = null;
    };
  }, [cascadePoints, fieldState, gammaBands, heatCells, intensity]);

  return (
    <canvas
      aria-hidden="true"
      className="absolute inset-0 z-[1] h-full w-full"
      data-renderer="webgl-market-force-field"
      ref={canvasRef}
    />
  );
}

function InstabilityGlow({ value }) {
  const intensity = clampNumber(value);
  return (
    <div
      aria-hidden="true"
      className="pointer-events-none absolute inset-0 z-[2]"
      style={{
        background: `linear-gradient(180deg, rgba(251, 113, 133, ${0.04 + intensity * 0.11}), transparent 24%, rgba(34, 211, 238, ${0.03 + intensity * 0.08}) 50%, transparent 74%, rgba(251, 191, 36, ${0.03 + intensity * 0.08})), linear-gradient(90deg, transparent, rgba(34, 211, 238, ${0.02 + intensity * 0.05}) 50%, transparent)`,
        boxShadow: `inset 0 0 0 1px rgba(148, 163, 184, ${0.04 + intensity * 0.08})`,
      }}
    />
  );
}

function ForceVectorOverlay({ model, forceField }) {
  const bias = String(forceField?.nextMoveBias || "neutral");
  const tone = bias.includes("down") ? "#fb7185" : bias.includes("up") ? "#34d399" : "#22d3ee";
  const opacity = 0.18 + clampNumber(forceField?.totalStress) * 0.52;

  return (
    <svg className="pointer-events-none absolute inset-0 z-10 h-full w-full" preserveAspectRatio="none" viewBox="0 0 1000 520">
      <defs>
        <marker id="btcForceArrow" markerHeight="8" markerWidth="8" orient="auto" refX="7" refY="4">
          <path d="M0,0 L8,4 L0,8 Z" fill={tone} opacity={opacity} />
        </marker>
      </defs>
      {(model.forceVectors || []).map((vector) => (
        <g key={vector.key} opacity={opacity * vector.intensity}>
          <line
            markerEnd="url(#btcForceArrow)"
            stroke={tone}
            strokeLinecap="round"
            strokeWidth={1.2 + vector.intensity * 3.2}
            x1={vector.x1}
            x2={vector.x2}
            y1={vector.y1}
            y2={vector.y2}
          />
          <circle cx={vector.x1} cy={vector.y1} fill={tone} opacity="0.45" r={2 + vector.intensity * 3} />
        </g>
      ))}
    </svg>
  );
}

function GammaWallOverlay({ bands }) {
  return (
    <div className="pointer-events-none absolute inset-0 z-[8]">
      {bands.map((band) => (
        <div
          className={[
            "absolute left-0 right-[74px] border-y",
            band.role === "support"
              ? "border-cyan-300/35 bg-cyan-300/10"
              : "border-fuchsia-300/35 bg-fuchsia-300/10",
          ].join(" ")}
          key={band.key}
          style={{
            height: `${Math.max(6, band.height)}px`,
            top: `${band.y}%`,
            transform: "translateY(-50%)",
          }}
        >
          <span
            className={[
              "absolute right-3 top-1/2 -translate-y-1/2 rounded border bg-slate-950/75 px-2 py-0.5 font-mono text-[10px] uppercase tracking-[0.14em]",
              band.role === "support" ? "border-cyan-300/30 text-cyan-100" : "border-fuchsia-300/30 text-fuchsia-100",
            ].join(" ")}
          >
            {band.role === "support" ? "GEX SUPPORT" : "GEX RESIST"}
          </span>
        </div>
      ))}
    </div>
  );
}

function LiquidationDensityOverlay({ bands }) {
  return (
    <div className="pointer-events-none absolute inset-y-0 left-0 z-[7] w-8 border-r border-slate-700/60 bg-slate-950/30">
      {bands.map((band) => (
        <div
          className={["absolute left-1 right-1 rounded-full", band.side === "above" ? "bg-orange-300" : band.side === "below" ? "bg-red-300" : "bg-cyan-300"].join(" ")}
          key={band.key}
          style={{
            height: `${Math.max(3, band.height * 0.5)}px`,
            opacity: 0.22 + clampNumber(band.intensity) * 0.68,
            top: `${band.y}%`,
            transform: "translateY(-50%)",
          }}
        />
      ))}
    </div>
  );
}

function CascadeVectorOverlay({ points }) {
  if (!points.length) return null;
  const path = points
    .map((point, index) => `${index === 0 ? "M" : "L"} ${point.x * 10} ${point.y * 5.2}`)
    .join(" ");

  return (
    <svg className="pointer-events-none absolute inset-0 z-10 h-full w-full" preserveAspectRatio="none" viewBox="0 0 1000 520">
      <defs>
        <marker id="btcCascadeArrow" markerHeight="8" markerWidth="8" orient="auto" refX="7" refY="4">
          <path d="M0,0 L8,4 L0,8 Z" fill="#fbbf24" opacity="0.78" />
        </marker>
      </defs>
      <path
        d={path}
        fill="none"
        markerEnd="url(#btcCascadeArrow)"
        stroke="#fbbf24"
        strokeDasharray="8 10"
        strokeLinecap="round"
        strokeLinejoin="round"
        strokeOpacity="0.55"
        strokeWidth="2.5"
      />
      {points.map((point) => (
        <line
          key={point.key}
          opacity={0.26 + clampNumber(point.intensity) * 0.54}
          stroke="#fbbf24"
          strokeLinecap="round"
          strokeWidth={1.8 + clampNumber(point.intensity) * 3.2}
          x1={point.x * 10 - 14}
          x2={point.x * 10 + 14}
          y1={point.y * 5.2}
          y2={point.y * 5.2}
        />
      ))}
    </svg>
  );
}

function drawCanvasFallback(canvas, { heatCells, gammaBands, cascadePoints, fieldState, intensity }) {
  const ctx = canvas?.getContext?.("2d");
  if (!canvas || !ctx) return;

  const rect = canvas.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const width = Math.max(1, Math.floor(rect.width * dpr));
  const height = Math.max(1, Math.floor(rect.height * dpr));
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, rect.width, rect.height);

  const heatScale = Math.max(0.2, Number(intensity || 78) / 100);
  const stress = clampNumber(fieldState?.totalStress);
  const instability = clampNumber(fieldState?.instabilityIndex);
  const background = ctx.createLinearGradient(0, 0, rect.width, rect.height);
  background.addColorStop(0, `rgba(8, 47, 73, ${0.20 + stress * 0.18})`);
  background.addColorStop(0.52, "rgba(15, 23, 42, 0.08)");
  background.addColorStop(1, `rgba(76, 29, 149, ${0.15 + instability * 0.22})`);
  ctx.fillStyle = background;
  ctx.fillRect(0, 0, rect.width, rect.height);

  heatCells.forEach((cell) => {
    const y = rect.height * (cell.y / 100);
    const hot = cell.side === "above" ? "251, 146, 60" : "248, 113, 113";
    const height = (8 + cell.intensity * 28) * heatScale;
    const gradient = ctx.createLinearGradient(0, y, rect.width, y);
    gradient.addColorStop(0, `rgba(${hot}, ${0.10 * cell.intensity * heatScale})`);
    gradient.addColorStop(0.45, `rgba(${hot}, ${0.42 * cell.intensity * heatScale})`);
    gradient.addColorStop(1, `rgba(${hot}, ${0.16 * cell.intensity * heatScale})`);
    ctx.fillStyle = gradient;
    ctx.fillRect(0, y - height / 2, rect.width, height);
  });

  gammaBands.forEach((band) => {
    const y = rect.height * (band.y / 100);
    const gradient = ctx.createLinearGradient(0, y, rect.width, y);
    const color = band.role === "support" ? "34, 211, 238" : "217, 70, 239";
    gradient.addColorStop(0, "rgba(15, 23, 42, 0)");
    gradient.addColorStop(0.5, `rgba(${color}, ${0.28 + band.intensity * 0.42})`);
    gradient.addColorStop(1, "rgba(15, 23, 42, 0)");
    ctx.fillStyle = gradient;
    ctx.fillRect(0, y - 10 - band.intensity * 11, rect.width, 20 + band.intensity * 22);
  });

  cascadePoints.forEach((point) => {
    const x = rect.width * (point.x / 100);
    const y = rect.height * (point.y / 100);
    const size = point.size * 0.72;
    ctx.strokeStyle = `rgba(251, 191, 36, ${0.34 + point.intensity * 0.46})`;
    ctx.lineWidth = 1.4 + point.intensity * 2.2;
    ctx.beginPath();
    ctx.moveTo(x - size * 1.2, y);
    ctx.lineTo(x + size * 1.2, y - size * 0.22);
    ctx.lineTo(x + size * 0.74, y - size * 0.54);
    ctx.moveTo(x + size * 1.2, y - size * 0.22);
    ctx.lineTo(x + size * 0.70, y + size * 0.16);
    ctx.stroke();
  });
}

function ForceFieldRadar({ forceField, squeeze }) {
  const axes = [
    { label: "LIQ", value: forceField.liquidationField || squeeze.longLiquidationPressure || 0, angle: -90 },
    { label: "GEX", value: forceField.gammaField || 0, angle: 0 },
    { label: "SQZ", value: forceField.squeezeProbability || Math.max(squeeze.upProbability || 0, squeeze.downProbability || 0), angle: 90 },
    { label: "CAS", value: forceField.cascadeProbability || forceField.cascadeField || 0, angle: 180 },
  ];
  const points = axes
    .map((axis) => {
      const radius = 16 + clampNumber(axis.value) * 52;
      const rad = (axis.angle * Math.PI) / 180;
      return `${70 + Math.cos(rad) * radius},${70 + Math.sin(rad) * radius}`;
    })
    .join(" ");

  return (
    <div className="mt-4 rounded-xl border border-slate-700/70 bg-slate-950/65 p-3">
      <div className="flex items-center justify-between">
        <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Force Field Radar</p>
        <span className="font-mono text-xs text-slate-500">read-only</span>
      </div>
      <svg className="mt-3 h-40 w-full" viewBox="0 0 140 140">
        <circle cx="70" cy="70" fill="none" r="24" stroke="rgba(148,163,184,0.18)" />
        <circle cx="70" cy="70" fill="none" r="46" stroke="rgba(148,163,184,0.14)" />
        <circle cx="70" cy="70" fill="none" r="68" stroke="rgba(148,163,184,0.10)" />
        {axes.map((axis) => {
          const rad = (axis.angle * Math.PI) / 180;
          const x = 70 + Math.cos(rad) * 66;
          const y = 70 + Math.sin(rad) * 66;
          return (
            <g key={axis.label}>
              <line stroke="rgba(148,163,184,0.18)" x1="70" x2={x} y1="70" y2={y} />
              <text fill="#94a3b8" fontSize="9" fontWeight="700" textAnchor="middle" x={x} y={y + 3}>
                {axis.label}
              </text>
            </g>
          );
        })}
        <polygon fill="rgba(34,211,238,0.20)" points={points} stroke="rgba(34,211,238,0.85)" strokeWidth="2" />
        <circle cx="70" cy="70" fill="#22d3ee" r="3" />
      </svg>
    </div>
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

  return {
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
    heatCells: buildHeatCells(heatmap, gammaWalls, data, priceToY, current),
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
    forceVectors: buildForceVectors(data.forceField || {}, timeframe),
  };
}

function buildHeatCells(heatmap, gammaWalls, data, priceToY, current) {
  const heat = (heatmap || []).flatMap((item, index) => {
    const baseY = priceToY(Number(item.priceUsd) || current);
    const intensity = clampNumber(item.riskScore);
    return [0.08, 0.32, 0.56, 0.80].map((phase, cellIndex) => ({
      key: `liq-strip-${index}-${cellIndex}`,
      x: phase * 100,
      y: baseY,
      intensity,
      side: item.side,
    }));
  });
  const gamma = (gammaWalls || []).map((wall, index) => ({
    key: `gamma-cell-${index}`,
    x: (0.18 + index * 0.11) * 100,
    y: priceToY(Number(wall.strikeUsd) || current),
    intensity: Math.min(1, Math.abs(Number(wall.gammaExposure || 0)) / 6),
    side: wall.role === "support" ? "below" : "above",
  }));
  const squeeze = data?.squeeze || {};
  const dominant = squeeze.dominantDirection === "down" ? "below" : "above";
  const squeezeIntensity = Math.max(clampNumber(squeeze.upProbability), clampNumber(squeeze.downProbability));
  const squeezeCells = squeezeIntensity > 0
    ? Array.from({ length: 5 }, (_, index) => ({
        key: `squeeze-${index}`,
        x: 24 + index * 13,
        y: dominant === "above" ? 28 + index * 1.6 : 76 - index * 1.6,
        intensity: squeezeIntensity,
        side: dominant,
      }))
    : [];

  return [...heat, ...gamma, ...squeezeCells].filter((cell) => cell.intensity > 0.01);
}

function buildForceVectors(forceField, timeframe) {
  const bias = String(forceField.nextMoveBias || "neutral");
  const stress = clampNumber(forceField.totalStress);
  const instability = clampNumber(forceField.instabilityIndex);
  const count = timeframe === "1m" ? 8 : timeframe === "15m" ? 12 : 10;
  const upward = bias.includes("up");
  const downward = bias.includes("down");
  const neutralSwing = !upward && !downward;
  const length = 44 + stress * 86;

  return Array.from({ length: count }, (_, index) => {
    const x = 92 + index * (820 / Math.max(1, count - 1));
    const yBase = 268 + Math.sin(index * 0.9) * (neutralSwing ? 34 : 20);
    const direction = neutralSwing ? (index % 2 === 0 ? -1 : 1) : upward ? -1 : 1;
    const jitter = Math.cos(index * 1.33) * 16 * instability;
    return {
      key: `force-vector-${index}`,
      x1: x,
      y1: yBase + jitter,
      x2: x + 28 + stress * 28,
      y2: yBase + direction * length + jitter,
      intensity: Math.max(0.22, stress * 0.72 + instability * 0.28),
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

function clampNumber(value, min = 0, max = 1) {
  const number = Number(value || 0);
  if (!Number.isFinite(number)) return min;
  return Math.min(max, Math.max(min, number));
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

function forceBiasLabel(value) {
  if (value === "upward_squeeze" || value === "up") return "向上挤压";
  if (value === "downward_squeeze" || value === "down") return "向下挤压";
  if (value === "buy") return "买方压力";
  if (value === "sell") return "卖方压力";
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
