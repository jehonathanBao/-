import { useCallback, useEffect, useMemo, useState } from "react";
import {
  addNewTokenWatch,
  fetchNewTokenChart,
  fetchNewTokenReconstruction,
  fetchNewTokenWatchList,
  normalizeNewTokenWatchList,
  removeNewTokenWatch,
} from "../api/newTokenWatch.js";
import { useReconnectingWebSocket } from "../hooks/useReconnectingWebSocket.js";

const timeframeOptions = [
  { value: "1m", label: "1m 行为窗口" },
  { value: "5m", label: "5m 行为窗口" },
  { value: "15m", label: "15m 结构窗口" },
  { value: "1h", label: "1h 周期窗口" },
  { value: "4h", label: "4h 周期窗口" },
];

const phaseLabels = {
  accumulation: "静默吸筹",
  markup: "拉升阶段",
  distribution: "出货阶段",
  breakdown: "回调阶段",
  neutral: "中性",
};

const phaseTone = {
  accumulation: "border-emerald-400/40 bg-emerald-400/10 text-emerald-200",
  markup: "border-cyan-400/40 bg-cyan-400/10 text-cyan-200",
  distribution: "border-orange-400/40 bg-orange-400/10 text-orange-200",
  breakdown: "border-red-400/40 bg-red-400/10 text-red-200",
  neutral: "border-slate-500/40 bg-slate-500/10 text-slate-300",
};

const phaseStrip = [
  { key: "accumulation", label: "静默吸筹" },
  { key: "washout", label: "洗盘阶段" },
  { key: "markup", label: "拉升阶段" },
  { key: "distribution", label: "出货阶段" },
  { key: "breakdown", label: "回调阶段" },
  { key: "neutral", label: "中性" },
];

export default function NewTokenWatch() {
  const [items, setItems] = useState([]);
  const [maxActiveTokens, setMaxActiveTokens] = useState(10);
  const [symbolInput, setSymbolInput] = useState("");
  const [selectedSymbol, setSelectedSymbol] = useState("");
  const [timeframe, setTimeframe] = useState("15m");
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [reconstruction, setReconstruction] = useState(null);
  const [chart, setChart] = useState(null);
  const [notice, setNotice] = useState(null);
  const [loading, setLoading] = useState(true);
  const [detailLoading, setDetailLoading] = useState(false);
  const [busySymbol, setBusySymbol] = useState(null);

  const syncItems = useCallback((nextItems, nextMax = maxActiveTokens) => {
    setItems(nextItems);
    setMaxActiveTokens(nextMax);
    setSelectedSymbol((current) => {
      if (current && nextItems.some((item) => item.symbol === current)) return current;
      return nextItems[0]?.symbol || "";
    });
  }, [maxActiveTokens]);

  const load = useCallback(async () => {
    const result = await fetchNewTokenWatchList();
    syncItems(result.items, result.maxActiveTokens);
    setLoading(false);
  }, [syncItems]);

  const loadDetail = useCallback(async (symbol, tf) => {
    if (!symbol) {
      setReconstruction(null);
      setChart(null);
      return;
    }
    setDetailLoading(true);
    try {
      const [nextReconstruction, nextChart] = await Promise.all([
        fetchNewTokenReconstruction(symbol, tf),
        fetchNewTokenChart(symbol, tf),
      ]);
      setReconstruction(nextReconstruction);
      setChart(nextChart);
    } catch (error) {
      setNotice({ type: "error", message: `结构数据加载失败：${error?.message || "NETWORK_ERROR"}` });
    } finally {
      setDetailLoading(false);
    }
  }, []);

  useEffect(() => {
    load().catch((error) => {
      setLoading(false);
      setNotice({ type: "error", message: `加载失败：${error?.message || "NETWORK_ERROR"}` });
    });
  }, [load]);

  useEffect(() => {
    loadDetail(selectedSymbol, timeframe);
  }, [loadDetail, selectedSymbol, timeframe]);

  useEffect(() => {
    if (!autoRefresh || !selectedSymbol) return undefined;
    const timer = window.setInterval(() => loadDetail(selectedSymbol, timeframe), 30_000);
    return () => window.clearInterval(timer);
  }, [autoRefresh, loadDetail, selectedSymbol, timeframe]);

  const handleWsMessage = useCallback(
    (event) => {
      try {
        const snapshot = normalizeNewTokenWatchList(JSON.parse(event.data));
        syncItems(snapshot.items, snapshot.maxActiveTokens);
      } catch {
        // HTTP remains the fallback for malformed snapshot frames.
      }
    },
    [syncItems],
  );

  const { status: wsStatus } = useReconnectingWebSocket("/ws/new-token-reconstruction", {
    enabled: true,
    retryMs: 1500,
    maxRetryMs: 10000,
    onMessage: handleWsMessage,
  });

  async function handleAdd(event) {
    event.preventDefault();
    const raw = symbolInput.trim();
    if (!raw || busySymbol) return;
    setBusySymbol(raw);
    setNotice(null);
    try {
      const result = await addNewTokenWatch(raw);
      if (!result.ok) {
        setNotice({ type: "error", message: errorMessage(result.error) });
        return;
      }
      syncItems(result.items, result.maxActiveTokens);
      setSelectedSymbol(result.item?.symbol || raw.toUpperCase());
      setSymbolInput("");
      setNotice({ type: "success", message: `${result.item?.symbol || raw.toUpperCase()} 已加入结构观察` });
    } catch (error) {
      setNotice({ type: "error", message: errorMessage(error?.response?.data?.error || error?.message) });
    } finally {
      setBusySymbol(null);
    }
  }

  async function handleRemove(item) {
    if (!item?.symbol || busySymbol) return;
    setBusySymbol(item.symbol);
    setNotice(null);
    try {
      const result = await removeNewTokenWatch(item.symbol);
      if (!result.ok) {
        setNotice({ type: "error", message: errorMessage(result.error) });
        return;
      }
      syncItems(result.items, result.maxActiveTokens);
      setNotice({ type: "success", message: `${item.symbol} 已停止观察` });
    } catch (error) {
      setNotice({ type: "error", message: errorMessage(error?.response?.data?.error || error?.message) });
    } finally {
      setBusySymbol(null);
    }
  }

  function handleExportReport() {
    if (!reconstruction) return;
    const blob = new Blob([JSON.stringify({ reconstruction, chart }, null, 2)], {
      type: "application/json",
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `${reconstruction.symbol || "new-token"}-${reconstruction.timeframe}-reconstruction.json`;
    link.click();
    URL.revokeObjectURL(url);
    setNotice({ type: "success", message: "只读结构报告已导出。" });
  }

  const sortedItems = useMemo(
    () =>
      [...items].sort((left, right) => {
        const confidenceDelta =
          Number(right.lastSignal?.capitalStructure?.phaseConfidence || 0) -
          Number(left.lastSignal?.capitalStructure?.phaseConfidence || 0);
        if (Math.abs(confidenceDelta) > 0.001) return confidenceDelta;
        return String(left.symbol).localeCompare(String(right.symbol));
      }),
    [items],
  );

  const capacityReached = items.length >= maxActiveTokens;

  return (
    <section className="space-y-4">
      <div className="rounded-2xl border border-slate-700/70 bg-slate-900/80 p-5 shadow-glow">
        <div className="flex flex-col gap-4 2xl:flex-row 2xl:items-end 2xl:justify-between">
          <div>
            <div className="flex flex-wrap items-center gap-3">
              <p className="text-xs uppercase tracking-[0.3em] text-cyan-300">Smart Money Reconstruction</p>
              <span className="rounded-full border border-cyan-400/35 bg-cyan-400/10 px-3 py-1 text-xs font-black text-cyan-100">
                beta
              </span>
            </div>
            <h3 className="mt-2 text-2xl font-black text-white">智能资金仓位重建引擎</h3>
            <p className="mt-2 max-w-4xl text-sm leading-6 text-slate-400">
              不展示秒级跳动信号；按行为窗口重建主力仓位、成本区间、阶段轨迹和出货风险。模块只读，不下单、不撤单、不操作资金。
            </p>
          </div>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <MetricPill label="活动币种" value={`${items.length}/${maxActiveTokens}`} />
            <MetricPill label="数据通道" value={wsStatusLabel(wsStatus)} tone={wsStatus === "open" ? "emerald" : "yellow"} />
            <MetricPill label="当前周期" value={timeframe} tone="cyan" />
            <MetricPill label="安全边界" value="只读" tone="cyan" />
          </div>
        </div>

        <div className="mt-5 grid gap-3 xl:grid-cols-[minmax(280px,1fr)_auto_auto_auto] xl:items-end">
          <form className="grid gap-3 sm:grid-cols-[1fr_auto]" onSubmit={handleAdd}>
            <label>
              <span className="mb-2 block text-xs text-slate-400">Symbol</span>
              <input
                aria-label="新币合约 symbol"
                className="w-full rounded-xl border border-slate-700 bg-slate-950 px-4 py-3 text-sm font-semibold text-white outline-none transition placeholder:text-slate-600 focus:border-cyan-400/70 focus:ring-2 focus:ring-cyan-500/20"
                disabled={capacityReached}
                onChange={(event) => setSymbolInput(event.target.value)}
                placeholder="例如 JTOUSDT"
                value={symbolInput}
              />
            </label>
            <button
              className="rounded-xl border border-cyan-400/40 bg-cyan-400/10 px-5 py-3 text-sm font-bold text-cyan-100 transition hover:bg-cyan-400/20 disabled:cursor-not-allowed disabled:border-slate-700 disabled:bg-slate-800/60 disabled:text-slate-500 sm:self-end"
              disabled={!symbolInput.trim() || Boolean(busySymbol) || capacityReached}
              type="submit"
            >
              加入监控
            </button>
          </form>
          <label>
            <span className="mb-2 block text-xs text-slate-400">行为窗口</span>
            <select
              className="w-full rounded-xl border border-slate-700 bg-slate-950 px-4 py-3 text-sm font-bold text-white outline-none focus:border-cyan-400/70"
              onChange={(event) => setTimeframe(event.target.value)}
              value={timeframe}
            >
              {timeframeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label className="flex items-center gap-3 rounded-xl border border-slate-700 bg-slate-950 px-4 py-3 text-sm font-bold text-slate-200">
            <input
              checked={autoRefresh}
              className="h-4 w-4 accent-cyan-300"
              onChange={(event) => setAutoRefresh(event.target.checked)}
              type="checkbox"
            />
            自动刷新
          </label>
          <button
            className="rounded-xl border border-emerald-400/40 bg-emerald-400/10 px-5 py-3 text-sm font-bold text-emerald-100 transition hover:bg-emerald-400/20 disabled:cursor-not-allowed disabled:opacity-50"
            disabled={!reconstruction}
            onClick={handleExportReport}
            type="button"
          >
            导出报告
          </button>
        </div>

        {notice ? <Notice notice={notice} /> : null}
      </div>

      <div className="grid gap-4 xl:grid-cols-[320px_minmax(0,1fr)]">
        <aside className="rounded-2xl border border-slate-700/70 bg-slate-900/80 p-4 shadow-glow">
          <div className="mb-3 flex items-center justify-between">
            <h4 className="text-sm font-black text-white">监控列表</h4>
            <span className="rounded-full border border-slate-700 bg-slate-950 px-3 py-1 text-xs text-slate-400">
              最多 {maxActiveTokens}
            </span>
          </div>
          {loading ? (
            <div className="rounded-xl border border-slate-800 bg-slate-950 p-4 text-sm text-slate-400">加载中...</div>
          ) : sortedItems.length === 0 ? (
            <div className="rounded-xl border border-slate-800 bg-slate-950 p-4 text-sm text-slate-400">暂无活动 symbol。</div>
          ) : (
            <div className="space-y-2">
              {sortedItems.map((item) => (
                <WatchListItem
                  active={item.symbol === selectedSymbol}
                  busy={busySymbol === item.symbol}
                  item={item}
                  key={item.symbol}
                  onRemove={() => handleRemove(item)}
                  onSelect={() => setSelectedSymbol(item.symbol)}
                />
              ))}
            </div>
          )}
        </aside>

        <main className="min-w-0 rounded-2xl border border-slate-700/70 bg-slate-900/80 p-4 shadow-glow">
          {!selectedSymbol ? (
            <EmptyState />
          ) : detailLoading && !reconstruction ? (
            <div className="rounded-xl border border-slate-800 bg-slate-950 p-5 text-sm text-slate-400">结构数据加载中...</div>
          ) : reconstruction ? (
            <ReconstructionDashboard reconstruction={reconstruction} chart={chart} loading={detailLoading} />
          ) : (
            <EmptyState />
          )}
        </main>
      </div>
    </section>
  );
}

function ReconstructionDashboard({ reconstruction, chart, loading }) {
  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-4 border-b border-slate-800 pb-4 2xl:flex-row 2xl:items-end 2xl:justify-between">
        <div>
          <div className="flex flex-wrap items-center gap-3">
            <h4 className="text-2xl font-black text-white">{reconstruction.symbol}</h4>
            <span className={`rounded-full border px-3 py-1 text-xs font-black ${phaseTone[reconstruction.currentPhase] || phaseTone.neutral}`}>
              {phaseLabels[reconstruction.currentPhase] || "中性"}
            </span>
            {loading ? <span className="text-xs text-cyan-300">刷新中...</span> : null}
          </div>
          <p className="mt-2 text-sm text-slate-400">
            当前价格 {formatPrice(reconstruction.currentPrice)} · 24h成交 {formatOptionalUsd(reconstruction.volume24hUsd)} ·
            市值 {formatOptionalUsd(reconstruction.marketCapUsd)}
          </p>
        </div>
        <div className="grid gap-3 sm:grid-cols-4">
          <MetricPill label="成本锚点" value={formatPrice(reconstruction.vwapAnchor)} tone="cyan" />
          <MetricPill label="仓位估计" value={positionRange(reconstruction)} />
          <MetricPill label="浮盈浮亏" value={pnlRange(reconstruction)} tone={reconstruction.floatingPnlLowPct >= 0 ? "emerald" : "yellow"} />
          <MetricPill label="置信度" value={percent(reconstruction.confidence)} tone="emerald" />
        </div>
      </div>

      <PhaseStrip reconstruction={reconstruction} />

      <div className="grid gap-4 2xl:grid-cols-[minmax(0,1.6fr)_380px]">
        <StructureChart chart={chart} reconstruction={reconstruction} />
        <SummaryPanel reconstruction={reconstruction} />
      </div>

      <div className="grid gap-4 xl:grid-cols-3">
        <PathPanel title="分批建仓路径" rows={reconstruction.accumulationPath} empty="暂无建仓路径确认" />
        <LastNodePanel node={reconstruction.lastAccumulationNode} />
        <PathPanel title="出货分布轨迹" rows={reconstruction.distributionPath} empty="暂无出货轨迹确认" />
      </div>

      <div className="grid gap-4 xl:grid-cols-4">
        <CostDistributionCard reconstruction={reconstruction} />
        <SmartLevelsCard levels={reconstruction.smartLevels} />
        <BehaviorProbabilityCard probabilities={reconstruction.shortTermBehaviorProbabilities} />
        <PositionChangeCard chart={chart} />
      </div>
    </div>
  );
}

function PhaseStrip({ reconstruction }) {
  const timeline = reconstruction.phaseTimeline || [];
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Capital Phase Timeline</p>
      <div className="mt-3 grid gap-2 md:grid-cols-6">
        {phaseStrip.map((phase) => {
          const segment = timeline.find((entry) => entry.phase === phase.key);
          const active = reconstruction.currentPhase === phase.key;
          return (
            <div
              className={[
                "rounded-xl border px-3 py-2",
                active ? phaseTone[phase.key] || phaseTone.neutral : "border-slate-800 bg-slate-900/70 text-slate-400",
              ].join(" ")}
              key={phase.key}
            >
              <div className="text-sm font-black">{phase.label}</div>
              <div className="mt-1 text-xs opacity-80">{segment ? formatDuration(segment.durationSec) : "等待确认"}</div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function StructureChart({ chart, reconstruction }) {
  const points = chart?.points?.length ? chart.points : fallbackPoints(reconstruction);
  const path = buildSvgPath(points, "price");
  const netPath = buildSvgPath(points, "netPosition");
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <div className="mb-3 flex items-center justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Position Reconstruction Chart</p>
          <h5 className="mt-1 text-lg font-black text-white">价格路径 + 净仓位曲线</h5>
        </div>
        <span className="text-xs text-slate-500">背景色代表资金阶段</span>
      </div>
      <div className="relative h-80 overflow-hidden rounded-xl border border-slate-800 bg-[#050b18]">
        <svg className="h-full w-full" preserveAspectRatio="none" viewBox="0 0 100 100" role="img" aria-label="主力仓位重建图">
          {reconstruction.phaseTimeline?.map((segment, index) => (
            <rect
              fill={phaseFill(segment.phase)}
              height="100"
              key={`${segment.label}-${index}`}
              opacity="0.18"
              width={100 / Math.max(1, reconstruction.phaseTimeline.length)}
              x={(index * 100) / Math.max(1, reconstruction.phaseTimeline.length)}
              y="0"
            />
          ))}
          <g opacity="0.24">
            {[20, 40, 60, 80].map((y) => (
              <line key={y} stroke="#94a3b8" strokeWidth="0.2" x1="0" x2="100" y1={y} y2={y} />
            ))}
          </g>
          {points.map((point, index) => (
            <rect
              fill={point.netPosition >= 0 ? "#22c55e" : "#fb7185"}
              height={Math.min(16, 2 + Math.abs(point.volume || 0) * 3)}
              key={`${point.ts}-${index}`}
              opacity="0.4"
              width={Math.max(1, 80 / Math.max(1, points.length))}
              x={(index * 100) / Math.max(1, points.length)}
              y={96 - Math.min(16, 2 + Math.abs(point.volume || 0) * 3)}
            />
          ))}
          <path d={path} fill="none" stroke="#67e8f9" strokeLinecap="round" strokeWidth="1.2" />
          <path d={netPath} fill="none" stroke="#facc15" strokeDasharray="2 2" strokeLinecap="round" strokeWidth="1" />
          {chart?.markers?.map((marker, index) => (
            <circle
              cx={(index + 1) * (100 / (chart.markers.length + 1))}
              cy="42"
              fill={marker.kind === "distribution" ? "#fb923c" : "#34d399"}
              key={`${marker.kind}-${index}`}
              r="1.5"
            />
          ))}
        </svg>
        <div className="absolute bottom-3 left-4 flex gap-3 text-xs text-slate-400">
          <span className="text-cyan-200">价格</span>
          <span className="text-yellow-200">净仓位</span>
          <span className="text-emerald-200">成交量柱</span>
        </div>
      </div>
    </div>
  );
}

function SummaryPanel({ reconstruction }) {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Structure Summary</p>
      <h5 className="mt-1 text-lg font-black text-white">主力仓位摘要</h5>
      <div className="mt-4 grid gap-3">
        <MetricStack label="当前阶段" value={phaseLabels[reconstruction.currentPhase] || "中性"} detail={`confidence ${percent(reconstruction.confidence)}`} />
        <MetricStack label="估算净仓位" value={formatUsd(reconstruction.estimatedNetPositionUsdt)} detail="impact-adjusted latent position" />
        <MetricStack label="成本区间" value={`${formatPrice(reconstruction.costBasisLow)} - ${formatPrice(reconstruction.costBasisHigh)}`} detail={`VWAP ${formatPrice(reconstruction.vwapAnchor)}`} />
        <MetricStack label="浮盈浮亏区间" value={pnlRange(reconstruction)} detail="按当前价格相对成本区估算" />
        <MetricStack label="最后吸筹点" value={lastNodeRange(reconstruction.lastAccumulationNode)} detail={lastNodeDetail(reconstruction.lastAccumulationNode)} />
        <MetricStack label="出货强度" value={`${Math.round(reconstruction.distributionIntensityScore)} / 100`} detail={`completion ${Math.round(reconstruction.distributionCompletionPct)}%`} />
      </div>
    </div>
  );
}

function PathPanel({ title, rows, empty }) {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <h5 className="text-sm font-black text-white">{title}</h5>
      <div className="mt-3 overflow-x-auto">
        {rows?.length ? (
          <table className="w-full min-w-[420px] text-left text-xs">
            <thead className="text-slate-500">
              <tr>
                <th className="pb-2">阶段</th>
                <th className="pb-2">价格区间</th>
                <th className="pb-2">Delta</th>
                <th className="pb-2">置信</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800">
              {rows.map((row, index) => (
                <tr key={`${row.label}-${index}`}>
                  <td className="py-2 font-bold text-slate-200">{segmentLabel(row.label)}</td>
                  <td className="py-2 text-slate-400">{formatPrice(row.startPrice)} → {formatPrice(row.endPrice)}</td>
                  <td className="py-2 text-slate-300">{signed(row.cumulativeDelta)}</td>
                  <td className="py-2 text-cyan-200">{percent(row.confidence)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <div className="rounded-lg border border-slate-800 bg-slate-900/70 p-3 text-sm text-slate-400">{empty}</div>
        )}
      </div>
    </div>
  );
}

function LastNodePanel({ node }) {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <h5 className="text-sm font-black text-white">拉升前最后吸筹点</h5>
      <div className="mt-3 rounded-xl border border-emerald-400/25 bg-emerald-400/10 p-4">
        <p className="text-lg font-black text-emerald-100">{lastNodeRange(node)}</p>
        <p className="mt-2 text-sm text-emerald-200/80">{lastNodeDetail(node)}</p>
      </div>
      <div className="mt-3 space-y-2">
        {(node?.characteristics?.length ? node.characteristics : ["等待低波动吸收窗口"]).map((entry) => (
          <div className="rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2 text-xs text-slate-300" key={entry}>
            {entry}
          </div>
        ))}
      </div>
    </div>
  );
}

function CostDistributionCard({ reconstruction }) {
  return (
    <InfoCard title="成本分布">
      {(reconstruction.costDistribution || []).map((band) => (
        <ProgressRow key={band.label} label={band.label} value={band.pct} detail={`${formatPrice(band.lower)} - ${formatPrice(band.upper)}`} />
      ))}
    </InfoCard>
  );
}

function SmartLevelsCard({ levels }) {
  return (
    <InfoCard title="关键结构位">
      {(levels || []).map((level) => (
        <div className="flex items-center justify-between gap-3 text-sm" key={`${level.label}-${level.price}`}>
          <span className="text-slate-400">{level.label}</span>
          <span className="font-black text-cyan-100">{formatPrice(level.price)}</span>
        </div>
      ))}
    </InfoCard>
  );
}

function BehaviorProbabilityCard({ probabilities = {} }) {
  return (
    <InfoCard title="短期行为概率">
      <ProgressRow label="继续出货" value={probabilities.continueDistribution} />
      <ProgressRow label="区间震荡" value={probabilities.rangeConsolidation} />
      <ProgressRow label="反弹拉升" value={probabilities.reboundMarkup} />
      <ProgressRow label="二次吸筹" value={probabilities.secondaryAccumulation} />
    </InfoCard>
  );
}

function PositionChangeCard({ chart }) {
  const points = chart?.points || [];
  const first = points[0]?.netPosition || 0;
  const last = points[points.length - 1]?.netPosition || 0;
  return (
    <InfoCard title="仓位变化">
      <MetricStack label="起始净仓" value={signed(first)} detail="window start" />
      <MetricStack label="当前净仓" value={signed(last)} detail="window latest" />
    </InfoCard>
  );
}

function InfoCard({ title, children }) {
  return (
    <div className="space-y-3 rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <h5 className="text-sm font-black text-white">{title}</h5>
      {children}
    </div>
  );
}

function ProgressRow({ label, value, detail }) {
  const pct = Math.max(0, Math.min(100, Math.round(Number(value || 0) * 100)));
  return (
    <div>
      <div className="mb-1 flex justify-between gap-3 text-xs">
        <span className="text-slate-400">{label}</span>
        <span className="font-bold text-slate-200">{pct}%</span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-slate-800">
        <div className="h-full rounded-full bg-cyan-300" style={{ width: `${pct}%` }} />
      </div>
      {detail ? <p className="mt-1 text-[11px] text-slate-500">{detail}</p> : null}
    </div>
  );
}

function WatchListItem({ item, active, busy, onSelect, onRemove }) {
  const capital = item.lastSignal?.capitalStructure || {};
  const phase = capital.phase || "neutral";
  return (
    <div className={`rounded-xl border p-3 ${active ? "border-cyan-400/70 bg-cyan-400/10" : "border-slate-800 bg-slate-950/70"}`}>
      <button className="block w-full text-left" onClick={onSelect} type="button">
        <div className="flex items-center justify-between gap-3">
          <span className="font-black text-white">{item.symbol}</span>
          <span className={`rounded-full border px-2 py-1 text-[11px] font-black ${phaseTone[phase] || phaseTone.neutral}`}>
            {phaseLabels[phase] || "中性"}
          </span>
        </div>
        <p className="mt-2 text-xs text-slate-400">
          成本 {costBasisRange(capital.costBasis)} · Conf {percent(capital.phaseConfidence)}
        </p>
      </button>
      <button
        className="mt-3 w-full rounded-lg border border-red-400/35 bg-red-400/10 px-3 py-2 text-xs font-bold text-red-100 disabled:opacity-50"
        disabled={busy}
        onClick={onRemove}
        type="button"
      >
        Stop
      </button>
    </div>
  );
}

function Notice({ notice }) {
  return (
    <div
      className={[
        "mt-4 rounded-xl border px-4 py-3 text-sm",
        notice.type === "success"
          ? "border-emerald-400/35 bg-emerald-400/10 text-emerald-200"
          : "border-red-400/35 bg-red-400/10 text-red-200",
      ].join(" ")}
      role="status"
    >
      {notice.message}
    </div>
  );
}

function EmptyState() {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-8 text-center text-sm text-slate-400">
      先从左侧加入或选择一个 symbol，系统会展示该币的主力仓位重建、阶段行为回放和成本结构。
    </div>
  );
}

function MetricStack({ label, value, detail }) {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/70 px-3 py-2">
      <p className="text-[11px] uppercase tracking-[0.16em] text-slate-500">{label}</p>
      <p className="mt-1 text-sm font-bold text-slate-100">{value}</p>
      <p className="mt-1 text-xs text-slate-400">{detail}</p>
    </div>
  );
}

function MetricPill({ label, value, tone = "slate" }) {
  const toneClass =
    tone === "emerald"
      ? "text-emerald-200"
      : tone === "yellow"
        ? "text-yellow-200"
        : tone === "cyan"
          ? "text-cyan-200"
          : "text-white";
  return (
    <div className="rounded-xl border border-slate-700/70 bg-slate-950/70 px-4 py-3">
      <p className="text-xs text-slate-500">{label}</p>
      <p className={`mt-1 text-sm font-black ${toneClass}`}>{value}</p>
    </div>
  );
}

function fallbackPoints(reconstruction) {
  const price = reconstruction.currentPrice || reconstruction.vwapAnchor || 1;
  return Array.from({ length: 8 }, (_, index) => ({
    ts: index,
    price: price * (1 + (index - 3) * 0.001),
    volume: index + 1,
    netPosition: reconstruction.estimatedNetPositionUsdt / Math.max(price, 1) / 8 * (index + 1),
  }));
}

function buildSvgPath(points, key) {
  if (!points?.length) return "";
  const values = points.map((point) => Number(point[key] || 0));
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = Math.max(0.000001, max - min);
  return points
    .map((point, index) => {
      const x = (index / Math.max(1, points.length - 1)) * 100;
      const y = 92 - ((Number(point[key] || 0) - min) / range) * 78;
      return `${index === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`;
    })
    .join(" ");
}

function phaseFill(phase) {
  if (phase === "accumulation") return "#10b981";
  if (phase === "markup") return "#22d3ee";
  if (phase === "distribution") return "#fb923c";
  if (phase === "breakdown") return "#fb7185";
  return "#64748b";
}

function wsStatusLabel(status) {
  if (status === "open") return "结构流";
  if (status === "connecting") return "连接中";
  if (status === "reconnecting") return "重连";
  return "待连接";
}

function costBasisRange(cost = {}) {
  return `${formatPrice(cost.lower)} - ${formatPrice(cost.upper)}`;
}

function positionRange(reconstruction = {}) {
  return `${formatUsd(reconstruction.estimatedTotalPositionUsdtLow)} - ${formatUsd(reconstruction.estimatedTotalPositionUsdtHigh)}`;
}

function pnlRange(reconstruction = {}) {
  return `${signed(reconstruction.floatingPnlLowPct)}% / ${signed(reconstruction.floatingPnlHighPct)}%`;
}

function lastNodeRange(node) {
  if (!node) return "未确认";
  return `${formatPrice(node.lower)} - ${formatPrice(node.upper)}`;
}

function lastNodeDetail(node) {
  if (!node) return "等待低波动吸收窗口";
  return `${formatDuration(node.durationSec)} · Abs ${percent(node.absorptionEfficiency)} · Conf ${percent(node.confidence)}`;
}

function segmentLabel(label) {
  const labels = {
    silent_accumulation: "静默吸筹",
    absorption_zone: "吸收区",
    final_accumulation: "最后吸筹",
    markup_expansion: "拉升扩张",
    hidden_distribution: "隐蔽派发",
    retail_absorption: "散户承接",
    exit_preparation: "退出准备",
    exit_acceleration: "退出加速",
    neutral_segment: "中性片段",
  };
  return labels[label] || label || "未知片段";
}

function percent(value) {
  return `${Math.round(Number(value || 0) * 100)}%`;
}

function signed(value) {
  const numeric = Number(value || 0);
  return `${numeric >= 0 ? "+" : ""}${numeric.toFixed(2)}`;
}

function formatDuration(seconds) {
  const numeric = Number(seconds || 0);
  if (numeric >= 60) return `${Math.floor(numeric / 60)}m ${Math.round(numeric % 60)}s`;
  return `${Math.round(numeric)}s`;
}

function formatPrice(value) {
  const numeric = Number(value || 0);
  if (!Number.isFinite(numeric) || numeric <= 0) return "N/A";
  if (numeric >= 100) return `$${numeric.toLocaleString(undefined, { maximumFractionDigits: 0 })}`;
  if (numeric >= 1) return `$${numeric.toLocaleString(undefined, { maximumFractionDigits: 3 })}`;
  return `$${numeric.toLocaleString(undefined, { maximumFractionDigits: 6 })}`;
}

function formatOptionalUsd(value) {
  return value === null || value === undefined ? "N/A" : formatUsd(value);
}

function formatUsd(value) {
  const numeric = Number(value || 0);
  if (!Number.isFinite(numeric) || numeric <= 0) return "$0";
  if (numeric >= 1_000_000) return `$${(numeric / 1_000_000).toFixed(1)}M`;
  if (numeric >= 1_000) return `$${(numeric / 1_000).toFixed(0)}K`;
  return `$${numeric.toFixed(0)}`;
}

function errorMessage(error) {
  if (error === "max_active_tokens_reached") return "最多只能同时监控 10 个币。";
  if (error === "invalid_symbol") return "Symbol 格式无效。";
  if (error === "token_not_found") return "该 symbol 当前未在监控列表。";
  return error || "操作失败";
}
