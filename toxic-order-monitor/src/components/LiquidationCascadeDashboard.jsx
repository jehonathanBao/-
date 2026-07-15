import { useCallback, useEffect, useMemo, useState } from "react";
import {
  fetchLiquidationCascade,
  fetchLiquidationLeverageMap,
  fetchLiquidationLiquidityGap,
  fetchBtcStructure,
  fetchMarketRegime,
} from "../api/liquidationCascade.js";

const REFRESH_MS = 10_000;
const SYMBOL_OPTIONS = ["BTCUSDT", "ETHUSDT"];

const statusTone = {
  ACTIVE: "border-red-400/45 bg-red-500/10 text-red-100",
  IMMINENT: "border-orange-400/45 bg-orange-500/10 text-orange-100",
  WARNING: "border-yellow-400/45 bg-yellow-500/10 text-yellow-100",
  CALM: "border-emerald-400/35 bg-emerald-500/10 text-emerald-100",
};

const directionLabel = {
  UP: "向上挤压",
  DOWN: "向下瀑布",
  NEUTRAL: "中性",
  LONG: "偏多",
  SHORT: "偏空",
};

export default function LiquidationCascadeDashboard() {
  const [symbol, setSymbol] = useState("BTCUSDT");
  const [state, setState] = useState({
    cascade: null,
    leverageMap: null,
    liquidityGap: null,
    regime: null,
    domainState: null,
    loading: true,
    error: null,
  });

  const isBtcSymbol = symbol.startsWith("BTC");

  const load = useCallback(async () => {
    const domainRequest = isBtcSymbol
      ? fetchBtcStructure(symbol)
      : Promise.resolve({ data: null, error: null });
    const regimeRequest = isBtcSymbol
      ? Promise.resolve({ data: null, error: null })
      : fetchMarketRegime(symbol);
    const [cascade, leverageMap, liquidityGap, regime, domainState] = await Promise.all([
      fetchLiquidationCascade(symbol),
      fetchLiquidationLeverageMap(symbol),
      fetchLiquidationLiquidityGap(symbol),
      regimeRequest,
      domainRequest,
    ]);

    setState({
      cascade: cascade.data,
      leverageMap: leverageMap.data,
      liquidityGap: liquidityGap.data,
      regime: regime.data,
      domainState: domainState.data,
      loading: false,
      error:
        cascade.error ||
        leverageMap.error ||
        liquidityGap.error ||
        regime.error ||
        domainState.error ||
        null,
    });
  }, [isBtcSymbol, symbol]);

  useEffect(() => {
    let cancelled = false;
    let timer = null;

    const refresh = async () => {
      if (cancelled || document.visibilityState === "hidden") return;
      await load();
    };

    refresh();
    timer = window.setInterval(refresh, REFRESH_MS);

    const onVisibilityChange = () => {
      if (document.visibilityState !== "hidden") refresh();
    };
    document.addEventListener("visibilitychange", onVisibilityChange);

    return () => {
      cancelled = true;
      if (timer) window.clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, [load]);

  const cascade = state.cascade;
  const leverageMap = state.leverageMap;
  const liquidityGap = state.liquidityGap;
  const regime = state.regime;
  const domainState = state.domainState;

  const heatmap = useMemo(
    () => [...(leverageMap?.heatmap || [])].sort((left, right) => right.intensity - left.intensity),
    [leverageMap],
  );

  return (
    <section className="space-y-5">
      <div className="workspace-panel p-5">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div>
            <p className="text-xs uppercase tracking-[0.32em] text-cyan-300">Liquidation Cascade Predictor</p>
            <h3 className="mt-2 text-2xl font-black text-white">强平瀑布预测</h3>
            <p className="mt-2 max-w-4xl text-sm leading-6 text-slate-400">
              独立展示杠杆集中、流动性缺口、触发接近度和市场状态输出；只读观察，不推送、不下单。
            </p>
          </div>
          <label className="block text-sm text-slate-300">
            <span className="mb-2 block text-xs text-slate-500">Symbol</span>
            <select
              className="min-w-44 rounded-xl border border-slate-700 bg-slate-950 px-4 py-3 font-bold text-slate-100 outline-none focus:border-cyan-400"
              onChange={(event) => {
                setState((previous) => ({ ...previous, loading: true }));
                setSymbol(event.target.value);
              }}
              value={symbol}
            >
              {SYMBOL_OPTIONS.map((item) => (
                <option key={item} value={item}>
                  {item}
                </option>
              ))}
            </select>
          </label>
        </div>
        {state.error ? (
          <div className="mt-4 rounded-xl border border-yellow-400/35 bg-yellow-400/10 px-4 py-3 text-sm text-yellow-100">
            数据源短暂不可用，当前展示最近可用结构或安全 fallback：{state.error}
          </div>
        ) : null}
      </div>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.35fr)_minmax(360px,0.65fr)]">
        <section className="workspace-panel p-5">
          <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
            <div>
              <p className="text-xs uppercase tracking-[0.28em] text-cyan-300">Cascade State</p>
              <h4 className="mt-2 text-xl font-black text-white">{symbol} 强平瀑布状态</h4>
            </div>
            <span
              className={[
                "rounded-full border px-3 py-1 text-xs font-black",
                statusTone[cascade?.status] || statusTone.CALM,
              ].join(" ")}
            >
              {cascade?.status || (state.loading ? "LOADING" : "CALM")}
            </span>
          </div>

          <div className="mt-5 grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <MetricCard label="瀑布概率" value={percent(cascade?.cascadeProbability)} tone="cyan" />
            <MetricCard label="方向" value={directionLabel[cascade?.direction] || "中性"} tone="slate" />
            <MetricCard label="预估波动" value={cascade?.estimatedMove || "-"} tone="orange" />
            <MetricCard label="时间窗口" value={cascade?.timeWindow || "-"} tone="slate" />
          </div>

          <div className="mt-5 rounded-xl border border-slate-700/70 bg-slate-950/55 p-4">
            <div className="mb-3 flex items-center justify-between gap-3">
              <p className="text-xs uppercase tracking-[0.25em] text-slate-400">Risk Components</p>
              <span className="text-xs text-slate-500">0–100</span>
            </div>
            <div className="space-y-3">
              <Bar label="杠杆集中" value={cascade?.components?.leverageConcentration} />
              <Bar label="流动性缺口" value={cascade?.components?.liquidityGap} />
              <Bar label="Funding 压力" value={cascade?.components?.fundingStress} />
              <Bar label="触发接近度" value={cascade?.components?.triggerProximity} />
              <Bar label="OI 压力" value={cascade?.components?.oiStress} />
            </div>
          </div>

          <div className="mt-5 grid gap-4 lg:grid-cols-2">
            <InfoCard title="风险区间">
              {cascade?.riskZone ? (
                <p className="text-lg font-black text-white">
                  ${formatNumber(cascade.riskZone[0])} – ${formatNumber(cascade.riskZone[1])}
                </p>
              ) : (
                <p className="text-sm text-slate-400">当前没有明确高风险价格带。</p>
              )}
            </InfoCard>
            <InfoCard title="触发信号">
              <TagList items={cascade?.signals || []} empty="暂无瀑布触发信号" />
            </InfoCard>
          </div>
        </section>

        <section className="space-y-4">
          <Panel title="市场状态">
            <div className="grid gap-3 sm:grid-cols-2">
              <MetricCard
                label="Regime"
                value={(isBtcSymbol ? domainState?.regime : regime?.regime) || "-"}
                tone="cyan"
              />
              <MetricCard
                label="方向偏置"
                value={
                  directionLabel[isBtcSymbol ? domainState?.bias : regime?.directionBias] ||
                  (isBtcSymbol ? domainState?.bias : regime?.directionBias) ||
                  "-"
                }
                tone="slate"
              />
              <MetricCard
                label="置信度"
                value={percent(isBtcSymbol ? domainState?.confidence : regime?.confidence)}
                tone="emerald"
              />
              {isBtcSymbol ? (
                <MetricCard label="结构强度" value={percent(domainState?.structureScore)} tone="orange" />
              ) : null}
            </div>
            <div className="mt-4">
              <TagList
                items={[...(regime?.signals || []), ...(domainState?.signals || [])]}
                empty="暂无市场状态标签"
              />
            </div>
          </Panel>

          <Panel title="流动性缺口">
            <div className="space-y-3">
              <Bar label="下方缺口" value={liquidityGap?.belowPrice} />
              <Bar label="上方缺口" value={liquidityGap?.abovePrice} />
              <Line label="主导方向" value={directionLabel[liquidityGap?.dominantGap] || "中性"} />
            </div>
          </Panel>
        </section>
      </div>

      <section className="workspace-panel">
        <div className="flex items-center justify-between gap-3 border-b border-slate-700/70 px-5 py-4">
          <div>
            <p className="text-xs uppercase tracking-[0.28em] text-cyan-300">Leverage Heatmap</p>
            <h4 className="mt-1 text-lg font-black text-white">杠杆热力与高风险带</h4>
          </div>
          <span className="rounded-full border border-cyan-400/35 px-3 py-1 text-xs font-black text-cyan-100">
            {heatmap.length} levels
          </span>
        </div>
        <div className="grid gap-0 xl:grid-cols-[minmax(0,1fr)_360px]">
          <div className="overflow-x-auto">
            <table className="w-full min-w-[760px] text-left text-sm">
              <thead className="bg-slate-950/80 text-xs uppercase tracking-[0.18em] text-slate-500">
                <tr>
                  <th className="px-5 py-3">价格</th>
                  <th className="px-5 py-3">方向</th>
                  <th className="px-5 py-3">强度</th>
                  <th className="px-5 py-3">名义额</th>
                  <th className="px-5 py-3">距离</th>
                  <th className="px-5 py-3">结构</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800">
                {heatmap.length > 0 ? (
                  heatmap.map((level, index) => (
                    <tr key={`${level.price}-${level.side}-${index}`} className="text-slate-200">
                      <td className="px-5 py-4 font-black text-white">${formatNumber(level.price)}</td>
                      <td className="px-5 py-4">{level.side}</td>
                      <td className="px-5 py-4">
                        <Bar compact value={level.intensity} />
                      </td>
                      <td className="px-5 py-4">${formatUsd(level.notionalUsd)}</td>
                      <td className="px-5 py-4">{formatNumber(level.distanceBps)} bps</td>
                      <td className="px-5 py-4 text-slate-400">
                        {level.side === "long" ? "多头清算簇" : level.side === "short" ? "空头清算簇" : "中性簇"}
                      </td>
                    </tr>
                  ))
                ) : (
                  <tr>
                    <td className="px-5 py-8 text-center text-slate-500" colSpan={6}>
                      暂无杠杆热力数据。
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
          <div className="border-t border-slate-700/70 p-5 xl:border-l xl:border-t-0">
            <p className="mb-3 text-xs uppercase tracking-[0.24em] text-slate-500">High Risk Zones</p>
            <div className="space-y-3">
              {(leverageMap?.highRiskZones || []).length > 0 ? (
                leverageMap.highRiskZones.map((zone, index) => (
                  <div key={`${zone.low}-${zone.high}-${index}`} className="rounded-xl border border-slate-700/70 bg-slate-950/55 p-4">
                    <div className="flex items-center justify-between gap-3">
                      <p className="font-black text-white">
                        ${formatNumber(zone.low)} – ${formatNumber(zone.high)}
                      </p>
                      <span className="rounded-full border border-cyan-400/35 px-2 py-1 text-xs font-bold text-cyan-100">
                        {percent(zone.strength)}
                      </span>
                    </div>
                    <p className="mt-2 text-sm text-slate-400">{zone.side}</p>
                  </div>
                ))
              ) : (
                <p className="rounded-xl border border-slate-700/70 bg-slate-950/55 p-4 text-sm text-slate-500">
                  当前没有聚合出的高风险价格带。
                </p>
              )}
            </div>
          </div>
        </div>
      </section>
    </section>
  );
}

function Panel({ title, children }) {
  return (
    <section className="workspace-panel p-5">
      <h4 className="mb-4 text-sm font-black uppercase tracking-[0.25em] text-cyan-300">{title}</h4>
      {children}
    </section>
  );
}

function InfoCard({ title, children }) {
  return (
    <div className="rounded-xl border border-slate-700/70 bg-slate-950/55 p-4">
      <p className="mb-2 text-xs uppercase tracking-[0.22em] text-slate-500">{title}</p>
      {children}
    </div>
  );
}

function MetricCard({ label, value, tone = "slate" }) {
  const toneClass = {
    cyan: "border-cyan-400/30 bg-cyan-400/10 text-cyan-100",
    emerald: "border-emerald-400/30 bg-emerald-400/10 text-emerald-100",
    orange: "border-orange-400/30 bg-orange-400/10 text-orange-100",
    slate: "border-slate-700/70 bg-slate-950/55 text-white",
  }[tone];
  return (
    <div className={["rounded-xl border p-4", toneClass].join(" ")}>
      <p className="text-xs text-slate-400">{label}</p>
      <p className="mt-2 text-lg font-black">{value}</p>
    </div>
  );
}

function Bar({ label = null, value = 0, compact = false }) {
  const safe = Math.min(1, Math.max(0, Number(value || 0)));
  return (
    <div className={compact ? "min-w-36" : ""}>
      {label ? (
        <div className="mb-1 flex items-center justify-between gap-3 text-xs">
          <span className="text-slate-400">{label}</span>
          <span className="font-bold text-slate-200">{percent(safe)}</span>
        </div>
      ) : null}
      <div className="h-2 overflow-hidden rounded-full bg-slate-800">
        <div className="h-full rounded-full bg-cyan-300" style={{ width: `${Math.round(safe * 100)}%` }} />
      </div>
    </div>
  );
}

function TagList({ items, empty }) {
  if (!items.length) {
    return <p className="text-sm text-slate-500">{empty}</p>;
  }
  return (
    <div className="flex flex-wrap gap-2">
      {items.map((item) => (
        <span key={item} className="rounded-full border border-slate-700 bg-slate-950 px-3 py-1 text-xs font-bold text-slate-300">
          {item}
        </span>
      ))}
    </div>
  );
}

function Line({ label, value }) {
  return (
    <div className="flex items-start justify-between gap-4 rounded-lg border border-slate-800 bg-slate-950/45 px-3 py-2">
      <span className="text-slate-500">{label}</span>
      <span className="text-right font-bold text-slate-200">{value}</span>
    </div>
  );
}

function percent(value) {
  const parsed = Number(value || 0);
  return `${Math.round(Math.min(1, Math.max(0, parsed)) * 100)}%`;
}

function formatNumber(value) {
  const parsed = Number(value || 0);
  return parsed.toLocaleString("en-US", { maximumFractionDigits: 0 });
}

function formatUsd(value) {
  const parsed = Number(value || 0);
  if (Math.abs(parsed) >= 1_000_000_000) return `${(parsed / 1_000_000_000).toFixed(1)}B`;
  if (Math.abs(parsed) >= 1_000_000) return `${(parsed / 1_000_000).toFixed(1)}M`;
  if (Math.abs(parsed) >= 1_000) return `${(parsed / 1_000).toFixed(1)}K`;
  return parsed.toFixed(0);
}
