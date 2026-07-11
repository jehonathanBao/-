import { useCallback, useEffect, useMemo, useState } from "react";
import {
  addNewTokenWatch,
  fetchNewTokenChart,
  fetchNewTokenReconstruction,
  fetchNewTokenWatchList,
  NEW_TOKEN_WATCH_MAX_ACTIVE,
  normalizeNewTokenWatchList,
  removeNewTokenWatch,
} from "../api/newTokenWatch.js";
import { useReconnectingWebSocket } from "../hooks/useReconnectingWebSocket.js";

const timeframeOptions = [
  { value: "5m", label: "5m 行为窗口" },
  { value: "15m", label: "15m 主视图" },
  { value: "1h", label: "1h 结构趋势" },
  { value: "4h", label: "4h 宏观结构" },
];

const phaseLabels = {
  accumulation: "静默吸筹",
  markup: "拉升阶段",
  distribution: "出货阶段",
  breakdown: "退出阶段",
  washout: "洗盘阶段",
  exit: "退出阶段",
  neutral: "中性",
};

const phaseTone = {
  accumulation: "border-emerald-400/40 bg-emerald-400/10 text-emerald-200",
  markup: "border-cyan-400/40 bg-cyan-400/10 text-cyan-200",
  distribution: "border-orange-400/40 bg-orange-400/10 text-orange-200",
  breakdown: "border-red-400/40 bg-red-400/10 text-red-200",
  washout: "border-sky-400/40 bg-sky-400/10 text-sky-200",
  exit: "border-red-400/40 bg-red-400/10 text-red-200",
  neutral: "border-slate-500/40 bg-slate-500/10 text-slate-300",
};

const phaseStrip = [
  { key: "accumulation", label: "Accumulation", sublabel: "建仓", matches: ["accumulation"] },
  { key: "washout", label: "Washout", sublabel: "洗盘", matches: ["washout"] },
  { key: "markup", label: "Markup", sublabel: "拉升", matches: ["markup"] },
  { key: "distribution", label: "Distribution", sublabel: "出货", matches: ["distribution"] },
  { key: "exit", label: "Exit", sublabel: "退出", matches: ["breakdown", "exit"] },
];

export default function NewTokenWatch() {
  const [items, setItems] = useState([]);
  const [maxActiveTokens, setMaxActiveTokens] = useState(NEW_TOKEN_WATCH_MAX_ACTIVE);
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
        const payload = JSON.parse(event.data);
        if (payload?.item) {
          const streamed = normalizeNewTokenWatchList({ items: [payload.item] }).items[0];
          setItems((current) => current.map((item) => (item.symbol === streamed.symbol ? streamed : item)));
          return;
        }
        const snapshot = normalizeNewTokenWatchList(payload);
        syncItems(snapshot.items, snapshot.maxActiveTokens);
      } catch {
        // HTTP remains the fallback for malformed snapshot frames.
      }
    },
    [syncItems],
  );

  const reconstructionWsPath = selectedSymbol
    ? `/ws/new-token-reconstruction?symbol=${encodeURIComponent(selectedSymbol)}`
    : "";
  const { status: wsStatus } = useReconnectingWebSocket(reconstructionWsPath, {
    enabled: Boolean(selectedSymbol),
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
  const selectedItem = useMemo(
    () => items.find((item) => item.symbol === selectedSymbol) || null,
    [items, selectedSymbol],
  );

  const capacityReached = items.length >= maxActiveTokens;

  return (
    <section className="space-y-4">
      <div className="rounded-2xl border border-slate-700/70 bg-slate-900/80 p-5 shadow-glow">
        <div className="flex flex-col gap-4 2xl:flex-row 2xl:items-end 2xl:justify-between">
          <div>
            <div className="flex flex-wrap items-center gap-3">
              <p className="text-xs uppercase tracking-[0.3em] text-cyan-300">BINANCE USD-M FLOW + L2</p>
              <span className="rounded-full border border-cyan-400/35 bg-cyan-400/10 px-3 py-1 text-xs font-black text-cyan-100">
                beta
              </span>
            </div>
            <h3 className="mt-2 text-2xl font-black text-white">新币合约流量与盘口观察</h3>
            <p className="mt-2 max-w-4xl text-sm leading-6 text-slate-400">
              默认仅展示公开成交流量。每个标的完成 Binance USD-M L2 快照与连续序列校验后，才显示盘口证据；系统不识别具体参与者，不输出交易指令。
            </p>
          </div>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <MetricPill label="活动币种" value={`${items.length}/${maxActiveTokens}`} />
            <MetricPill label="数据通道" value={wsStatusLabel(wsStatus)} tone={wsStatus === "open" ? "emerald" : "yellow"} />
            <MetricPill label="主视图周期" value={timeframe} tone="cyan" />
            <MetricPill label="安全边界" value="只读" tone="cyan" />
          </div>
        </div>
        <p className="mt-3 text-xs text-slate-500">
          新增与移除监控需要 operator 权限；公网页面只展示已启用的只读观察会话。
        </p>

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
                  reconstruction={item.symbol === reconstruction?.symbol ? reconstruction : null}
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
            <ReconstructionDashboard
              chart={chart}
              item={selectedItem}
              loading={detailLoading}
              reconstruction={reconstruction}
            />
          ) : (
            <EmptyState />
          )}
        </main>
      </div>
    </section>
  );
}

function ReconstructionDashboard({ reconstruction, chart, item, loading }) {
  return (
    <div className="space-y-4">
      <MarketHeader reconstruction={reconstruction} loading={loading} />
      <L2EvidencePanel item={item} reconstruction={reconstruction} />
      <FlowObservationPanel item={item} reconstruction={reconstruction} />
      <StructureChart chart={chart} reconstruction={reconstruction} />
    </div>
  );
}

function L2EvidencePanel({ item, reconstruction }) {
  const ready = reconstruction.orderbookEvidenceAvailable || item?.orderbookEvidenceAvailable;
  const mode = reconstruction.evidenceMode || item?.evidenceMode || "flow_only";
  const reason = reconstruction.intentReason || item?.intentReason || "l2_session_not_ready";
  const book = reconstruction.l2Orderbook;
  const intent = reconstruction.l2Intent;
  const tradeFlow = reconstruction.l2TradeFlow;
  const openInterest = reconstruction.l2OpenInterest;
  const walls = reconstruction.l2WallEvidence || [];
  return (
    <section className="rounded-xl border border-cyan-400/25 bg-slate-950/70 p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Evidence Boundary</p>
          <h5 className="mt-1 text-lg font-black text-white">{ready ? "L2 盘口证据已就绪" : "流量观察模式"}</h5>
        </div>
        <span className={`rounded-full border px-3 py-1 text-xs font-black ${ready ? "border-emerald-400/40 bg-emerald-400/10 text-emerald-100" : "border-amber-400/40 bg-amber-400/10 text-amber-100"}`}>
          {mode === "l2_ready" ? "L2 READY" : "FLOW ONLY"}
        </span>
      </div>
      <p className="mt-3 text-sm leading-6 text-slate-300">
        {ready
          ? "已获得公开订单簿连续序列，可展示概率性盘口压力；这不是对具体参与者、真实墙体或控盘方的确认。"
          : <><span className="font-semibold text-amber-100">订单簿证据未就绪</span>。当前仅基于公开成交流量展示，不显示盘口意图、真实墙体、spoof 确认或交易建议。</>}
      </p>
      <p className="mt-2 text-xs text-slate-500">状态原因：{reason}</p>
      {ready && book ? (
        <>
          <p className="mt-3 text-xs uppercase tracking-[0.18em] text-slate-500">
            LISTING PHASE · {String(reconstruction.l2ListingPhase || "syncing").toUpperCase()}
          </p>
          <div className="mt-4 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
            <MetricStack label="SPREAD" value={`${Number(book.spreadBps || 0).toFixed(2)} bps`} detail="best bid / ask" />
            <MetricStack label="TOP DEPTH IMBALANCE" value={percent((Number(book.imbalance || 0) + 1) / 2)} detail="public L2 depth proxy" />
            <MetricStack label="VISIBLE PULL / ADD" value={Number(book.visibleCancelToAddRatio || 0).toFixed(2)} detail="visible-level change proxy" />
            <MetricStack label="INTENT STATE" value={String(intent?.state || "unavailable").toUpperCase()} detail={intent?.reason || "awaiting confirmation"} />
          </div>
          <div className="mt-3 grid gap-3 sm:grid-cols-2">
            <MetricStack label="AGGRESSIVE BUY 15S" value={formatUsd(tradeFlow?.buyNotional15s || 0)} detail="Binance aggTrade taker flow" />
            <MetricStack label="AGGRESSIVE SELL 15S" value={formatUsd(tradeFlow?.sellNotional15s || 0)} detail={tradeFlow?.reason || "awaiting aggTrade"} />
          </div>
          <div className="mt-3 grid gap-3 sm:grid-cols-2">
            <MetricStack label="OPEN INTEREST" value={openInterest?.available ? Number(openInterest.currentContracts || 0).toLocaleString(undefined, { maximumFractionDigits: 3 }) : "N/A"} detail={openInterest?.reason || "open interest unavailable"} />
            <MetricStack label="OI DELTA 15S" value={openInterest?.available && openInterest.delta15sPct !== null ? `${openInterest.delta15sPct.toFixed(2)}%` : "N/A"} detail="symbol REST context" />
          </div>
          <div className="mt-4 border-t border-slate-800 pt-3">
            <p className="text-xs font-bold uppercase tracking-[0.18em] text-slate-400">Visible L2 Level Evidence</p>
            {walls.length ? (
              <div className="mt-2 flex flex-wrap gap-2">
                {walls.slice(0, 6).map((wall) => (
                  <span className="rounded-full border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-300" key={`${wall.side}-${wall.price}-${wall.lifecycle}`}>
                    {String(wall.side || "level").toUpperCase()} {formatPrice(wall.price)} · {wall.lifecycle || "visible"}
                  </span>
                ))}
              </div>
            ) : <p className="mt-2 text-xs text-slate-500">暂无持续可见的大额 L2 档位证据。</p>}
          </div>
        </>
      ) : null}
    </section>
  );
}

function FlowObservationPanel({ item, reconstruction }) {
  const signal = item?.lastSignal || {};
  const impact = signal.impactResponse || {};
  return (
    <section className="grid gap-3 rounded-xl border border-slate-700 bg-slate-950/70 p-4 sm:grid-cols-2 xl:grid-cols-4">
      <MetricStack label="FLOW REGIME" value={String(signal.regime || "neutral").toUpperCase()} detail="public trade-flow classification" />
      <MetricStack label="FLOW CONFIDENCE" value={percent(signal.confidence)} detail="flow evidence only" />
      <MetricStack label="FLOW PERSISTENCE" value={percent(signal.flowPersistence)} detail="rolling observation" />
      <MetricStack label="PRICE RESPONSE" value={percent(Number(impact.priceMovePct || 0) / 100)} detail={impact.classification || "unknown"} />
      <p className="sm:col-span-2 xl:col-span-4 text-xs text-slate-500">
        市场价 {formatPrice(reconstruction.marketPrice)} · 数据源 {String(reconstruction.marketPriceSource || "market_perp").toUpperCase()} · 不构成交易建议。
      </p>
    </section>
  );
}

function InstitutionalCompletionLayer({ reconstruction }) {
  return (
    <section className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <div className="flex flex-col gap-1 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Institutional Completion Layer</p>
          <h5 className="mt-1 text-lg font-black text-white">资金时间轴 · 仓位曲线 · 流动性反应</h5>
        </div>
        <span className="text-xs text-slate-500">从结构识别升级为时间维度资金动力学。</span>
      </div>
      <div className="mt-4 grid gap-4 2xl:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)_minmax(320px,0.8fr)]">
        <CapitalNarrativeTimeline timeline={reconstruction.capitalTimeline} />
        <PositionFlowCurvePanel curve={reconstruction.positionFlowCurve} />
        <LiquidityReactionMapPanel map={reconstruction.liquidityReactionMap} />
      </div>
    </section>
  );
}

function MarketDynamicsPanel({ reconstruction }) {
  const dynamics = reconstruction.marketDynamics || {};
  const vector = dynamics.stateVector || {};
  const velocity = dynamics.stateVelocity || {};
  const energy = dynamics.marketEnergy || {};
  const transitions = dynamics.transitionMatrix || [];
  const trajectoryPoints = buildDynamicsTrajectory(vector, velocity);
  const smpPath = buildSvgPath(trajectoryPoints, "smp");
  const positionPath = buildSvgPath(trajectoryPoints, "position");
  const liquidityPath = buildSvgPath(trajectoryPoints, "liquidity");

  return (
    <section className="rounded-xl border border-cyan-400/20 bg-slate-950/75 p-4">
      <div className="flex flex-col gap-1 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Market Dynamics Engine</p>
          <h5 className="mt-1 text-lg font-black text-white">状态转移函数 · 时间演化模型</h5>
        </div>
        <span className="text-xs text-slate-500">{trajectorySummaryLabel(dynamics.trajectorySummary)}</span>
      </div>

      <div className="mt-4 grid gap-4 2xl:grid-cols-[minmax(0,1.1fr)_minmax(300px,0.75fr)_minmax(320px,0.85fr)]">
        <InfoCard title="State Trajectory Curve">
          <div className="grid grid-cols-3 gap-2">
            <MiniMetric label="SMP" value={signed(vector.smp)} />
            <MiniMetric label="Position" value={formatSignedUsd(vector.positionUsd)} />
            <MiniMetric label="Liquidity" value={percent(vector.liquidity)} />
          </div>
          <div className="mt-3 h-32 overflow-hidden rounded-xl border border-slate-800 bg-[#040a16]">
            <svg className="h-full w-full" preserveAspectRatio="none" viewBox="0 0 100 100" role="img" aria-label="Market state trajectory">
              <g opacity="0.18">
                {[25, 50, 75].map((y) => (
                  <line key={y} stroke="#94a3b8" strokeWidth="0.25" x1="0" x2="100" y1={y} y2={y} />
                ))}
              </g>
              <path d={smpPath} fill="none" stroke="#22d3ee" strokeLinecap="round" strokeWidth="1.4" />
              <path d={positionPath} fill="none" stroke="#facc15" strokeLinecap="round" strokeWidth="1.4" />
              <path d={liquidityPath} fill="none" stroke="#34d399" strokeLinecap="round" strokeWidth="1.4" />
            </svg>
          </div>
          <div className="mt-2 flex flex-wrap gap-3 text-[11px] text-slate-400">
            <span className="text-cyan-200">SMP trajectory</span>
            <span className="text-yellow-200">Position trajectory</span>
            <span className="text-emerald-200">Liquidity trajectory</span>
          </div>
        </InfoCard>

        <InfoCard title="Market Energy">
          <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-3">
            <div className="flex items-center justify-between gap-3">
              <span className={`rounded-full border px-2 py-1 text-[11px] font-black ${energyTone(energy.level)}`}>
                {energyLabel(energy.level)}
              </span>
              <span className="text-2xl font-black text-white">{percent(energy.score)}</span>
            </div>
            <p className="mt-2 text-xs text-slate-500">Energy = Flow Strength × Liquidity Availability × Regime Stability</p>
          </div>
          <ProgressRow label="Flow Strength" value={energy.flowStrength} />
          <ProgressRow label="Liquidity Availability" value={energy.liquidityAvailability} />
          <ProgressRow label="Regime Stability" value={energy.regimeStability} />
        </InfoCard>

        <InfoCard title="Velocity Indicator">
          <MetricStack label="d(Position)/dt" value={`${formatSignedUsd(velocity.positionVelocityUsdPerMin)}/m`} detail="主力仓位变化速度" />
          <MetricStack label="d(Liquidity)/dt" value={signed(velocity.liquidityShiftRate)} detail="吸收与真空的相对变化" />
          <MetricStack label="d(Flow)/dt" value={signed(velocity.flowAcceleration)} detail="SMP / MFE 驱动后的资金加速度" />
          <MetricStack label="d(Regime)/dt" value={percent(velocity.regimeTransitionSpeed)} detail="状态转移速度" />
        </InfoCard>
      </div>

      <div className="mt-4 rounded-xl border border-slate-800 bg-slate-950/70 p-4">
        <div className="mb-3 flex items-center justify-between gap-3">
          <h5 className="text-sm font-black text-white">Regime Transition Map</h5>
          <span className="text-xs text-slate-500">S(t) → S(t+1)</span>
        </div>
        <div className="grid gap-3 lg:grid-cols-2">
          {transitions.length ? (
            transitions.map((entry, index) => (
              <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-3" key={`${entry.from}-${entry.to}-${index}`}>
                <div className="mb-2 flex items-center justify-between gap-3 text-xs">
                  <span className="font-black text-slate-100">
                    {phaseLabels[entry.from] || entry.from} → {phaseLabels[entry.to] || entry.to}
                  </span>
                  <span className="font-black text-cyan-100">{percent(entry.probability)}</span>
                </div>
                <div className="h-2 overflow-hidden rounded-full bg-slate-800">
                  <div className="h-full rounded-full bg-cyan-300" style={{ width: `${Math.round(Number(entry.probability || 0) * 100)}%` }} />
                </div>
                <p className="mt-2 text-xs text-slate-500">{dynamicsReasonLabel(entry.reason)}</p>
              </div>
            ))
          ) : (
            <div className="rounded-lg border border-slate-800 bg-slate-900/70 p-3 text-sm text-slate-400">
              等待足够窗口生成状态转移矩阵。
            </div>
          )}
        </div>
      </div>
    </section>
  );
}

function LiquidityForcePanel({ reconstruction }) {
  const force = reconstruction.liquidityForce || {};
  const cascade = force.stopLossCascade || {};
  const attribution = force.forcedFlowAttribution || {};
  const impact = force.priceImpactDecomposition || {};
  const zones = force.liquidationZones || [];

  return (
    <section className="rounded-xl border border-red-400/20 bg-slate-950/75 p-4">
      <div className="flex flex-col gap-1 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-red-300">Liquidity Force Panel</p>
          <h5 className="mt-1 text-lg font-black text-white">清算 / 止损 / 强制流驱动层</h5>
        </div>
        <span className="text-xs text-slate-500">proxy layer · no exchange liquidation feed required</span>
      </div>

      <div className="mt-4 grid gap-4 2xl:grid-cols-[minmax(0,1.15fr)_minmax(300px,0.75fr)_minmax(300px,0.75fr)]">
        <InfoCard title="Liquidation Zones">
          <div className="grid gap-2">
            {zones.length ? (
              zones.map((zone, index) => (
                <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-3" key={`${zone.side}-${index}`}>
                  <div className="flex items-center justify-between gap-3">
                    <span className={`rounded-full border px-2 py-1 text-[11px] font-black ${liquidationSideTone(zone.side)}`}>
                      {liquidationSideLabel(zone.side)}
                    </span>
                    <span className="text-xs font-black text-slate-100">{formatPrice(zone.lower)} - {formatPrice(zone.upper)}</span>
                  </div>
                  <div className="mt-2 grid gap-2 sm:grid-cols-2">
                    <ProgressRow label="Intensity" value={zone.intensity} />
                    <ProgressRow label="Leverage Density" value={zone.leverageDensity} />
                  </div>
                  <p className="mt-2 text-xs text-slate-500">{forceReasonLabel(zone.reason)}</p>
                </div>
              ))
            ) : (
              <div className="rounded-lg border border-slate-800 bg-slate-900/70 p-3 text-sm text-slate-400">
                等待清算区间代理模型。
              </div>
            )}
          </div>
        </InfoCard>

        <InfoCard title="Stop-loss Cascade">
          <MetricStack label="Active Zone" value={activeForceZoneLabel(force.activeZone)} detail={primaryDriverLabel(force.primaryDriver)} />
          <MetricStack label="Sweep Direction" value={directionLabel(cascade.sweepDirection)} detail={liquiditySweepLabel(cascade.liquiditySweep)} />
          <ProgressRow label="Stop Hunt Probability" value={cascade.stopHuntProbability} />
          <ProgressRow label="Cascade Intensity" value={cascade.cascadeIntensity} />
        </InfoCard>

        <InfoCard title="Forced Flow Attribution">
          <MetricStack label="Primary Driver" value={primaryDriverLabel(attribution.dominantDriver)} detail="whale / retail / forced flow split" />
          <ProgressRow label="Whale Initiated" value={attribution.whalePct} />
          <ProgressRow label="Retail Chasing" value={attribution.retailPct} />
          <ProgressRow label="Forced Liquidation" value={attribution.liquidationPct} />
        </InfoCard>
      </div>

      <div className="mt-4 rounded-xl border border-slate-800 bg-slate-950/70 p-4">
        <div className="mb-3 flex items-center justify-between gap-3">
          <h5 className="text-sm font-black text-white">Price Impact Decomposition</h5>
          <span className="text-xs text-slate-500">whale + liquidation + stops + absorption</span>
        </div>
        <div className="grid gap-3 md:grid-cols-4">
          <ProgressRow label="Whale Impact" value={impact.whaleImpact} />
          <ProgressRow label="Liquidation Cascade" value={impact.liquidationCascade} />
          <ProgressRow label="Stop-loss Sweep" value={impact.stopLossSweep} />
          <ProgressRow label="Passive Absorption" value={impact.passiveAbsorption} />
        </div>
      </div>
    </section>
  );
}

function TradingDecisionKernelPanel({ reconstruction }) {
  const decision = reconstruction.tradingDecision || {};
  const entry = decision.entry || {};
  const exit = decision.exit || {};
  const size = decision.positionSize || {};
  const invalidation = decision.invalidation || {};

  return (
    <section className="rounded-xl border border-emerald-400/20 bg-slate-950/75 p-4">
      <div className="flex flex-col gap-1 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-emerald-300">Trading Decision Kernel</p>
          <h5 className="mt-1 text-lg font-black text-white">唯一交易输出层</h5>
        </div>
        <span className="text-xs text-slate-500">advisory-only · no execution path</span>
      </div>

      <div className="mt-4 grid gap-4 xl:grid-cols-4">
        <InfoCard title="ENTRY">
          <div className="flex items-center justify-between gap-3">
            <span className={`rounded-full border px-2 py-1 text-[11px] font-black ${directionTone(decision.direction)}`}>
              {directionLabel(decision.direction)}
            </span>
            <span className="text-xs font-black text-cyan-100">{percent(decision.confidence)}</span>
          </div>
          <MetricStack label="Type / Timing" value={`${orderTypeLabel(entry.orderType)} · ${timingLabel(entry.timing)}`} detail={decisionConditionLabel(entry.condition)} />
          <MetricStack label="Entry Zone" value={entry.zoneHigh > 0 ? `${formatPrice(entry.zoneLow)} - ${formatPrice(entry.zoneHigh)}` : "N/A"} detail="cost basis ± deviation" />
        </InfoCard>

        <InfoCard title="EXIT">
          <MetricStack label="Exit Zone" value={exit.zoneHigh > 0 ? `${formatPrice(exit.zoneLow)} - ${formatPrice(exit.zoneHigh)}` : "N/A"} detail={decisionConditionLabel(exit.condition)} />
          <MetricStack label="Timing" value={timingLabel(exit.timing)} detail="condition-based exit only" />
        </InfoCard>

        <InfoCard title="POSITION_SIZE">
          <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-3">
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs text-slate-500">Suggested</span>
              <span className="text-2xl font-black text-white">{Math.round(Number(size.pct || 0))}%</span>
            </div>
            <div className="mt-2 h-2 overflow-hidden rounded-full bg-slate-800">
              <div className="h-full rounded-full bg-emerald-300" style={{ width: `${Math.max(0, Math.min(100, Number(size.pct || 0)))}%` }} />
            </div>
          </div>
          <MetricStack label="Multiplier" value={`${Number(size.multiplier || 0).toFixed(2)}x`} detail={decisionConditionLabel(size.reason)} />
        </InfoCard>

        <InfoCard title="INVALIDATION">
          <div className="flex items-center justify-between gap-3">
            <span className={`rounded-full border px-2 py-1 text-[11px] font-black ${invalidation.active ? "border-red-400/40 bg-red-400/10 text-red-200" : "border-emerald-400/40 bg-emerald-400/10 text-emerald-200"}`}>
              {invalidation.active ? "ACTIVE" : "ARMED"}
            </span>
            <span className="text-sm font-black text-slate-100">{formatPrice(invalidation.priceLevel)}</span>
          </div>
          <MetricStack label="Regime" value={decisionConditionLabel(invalidation.regimeCondition)} detail="state flip condition" />
          <MetricStack label="Flow / Liquidity" value={decisionConditionLabel(invalidation.flowCondition)} detail={decisionConditionLabel(invalidation.liquidityCondition)} />
        </InfoCard>
      </div>
    </section>
  );
}

function ExecutionStrategyKernelPanel({ reconstruction }) {
  const strategy = reconstruction.executionStrategy || {};
  const entry = strategy.entry || {};
  const exit = strategy.exit || {};
  const size = strategy.positionSize || {};
  const stop = strategy.stop || {};
  const reasoning = Array.isArray(strategy.reasoning) ? strategy.reasoning : [];

  return (
    <section className="rounded-xl border border-cyan-400/20 bg-slate-950/75 p-4">
      <div className="flex flex-col gap-1 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Execution Strategy Kernel</p>
          <h5 className="mt-1 text-lg font-black text-white">Market Force → Action Compiler</h5>
        </div>
        <span className="text-xs text-slate-500">advisory-only · read-only · no exchange execution</span>
      </div>

      <div className="mt-4 grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)_minmax(0,1fr)]">
        <InfoCard title="ENTRY">
          <div className="flex items-center justify-between gap-3">
            <span className={`rounded-full border px-2 py-1 text-[11px] font-black ${directionTone(strategy.direction)}`}>
              {directionLabel(strategy.direction)}
            </span>
            <span className="text-xs font-black text-cyan-100">{percent(strategy.confidence)}</span>
          </div>
          <MetricStack label="Primary Driver" value={marketDriverLabel(strategy.primaryDriver)} detail={`Secondary: ${marketDriverLabel(strategy.secondaryDriver)}`} />
          <MetricStack label="Entry Window" value={`${orderTypeLabel(entry.orderType)} · ${timingLabel(entry.timing)}`} detail={decisionConditionLabel(entry.condition)} />
          <MetricStack label="Zone" value={entry.zoneHigh > 0 ? `${formatPrice(entry.zoneLow)} - ${formatPrice(entry.zoneHigh)}` : "N/A"} detail="cost basis + liquidity band" />
        </InfoCard>

        <InfoCard title="EXIT">
          <MetricStack label="Exit Zone" value={exit.zoneHigh > 0 ? `${formatPrice(exit.zoneLow)} - ${formatPrice(exit.zoneHigh)}` : "N/A"} detail={decisionConditionLabel(exit.condition)} />
          <MetricStack label="Timing" value={timingLabel(exit.timing)} detail="driver state must remain valid" />
          <MetricStack label="Action Boundary" value={strategy.advisoryOnly ? "Advisory" : "Unsafe"} detail={strategy.readOnly ? "No order route exposed" : "Read-only flag missing"} />
        </InfoCard>

        <InfoCard title="POSITION SIZE">
          <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-3">
            <div className="flex items-center justify-between gap-3">
              <span className="text-xs text-slate-500">Compiled Size</span>
              <span className="text-2xl font-black text-white">{Math.round(Number(size.pct || 0))}%</span>
            </div>
            <div className="mt-2 h-2 overflow-hidden rounded-full bg-slate-800">
              <div className="h-full rounded-full bg-cyan-300" style={{ width: `${Math.max(0, Math.min(100, Number(size.pct || 0)))}%` }} />
            </div>
          </div>
          <MetricStack label="Multiplier" value={`${Number(size.multiplier || 0).toFixed(2)}x`} detail={decisionConditionLabel(size.reason)} />
        </InfoCard>

        <InfoCard title="STOP / INVALIDATION">
          <div className="flex items-center justify-between gap-3">
            <span className={`rounded-full border px-2 py-1 text-[11px] font-black ${stop.active ? "border-red-400/40 bg-red-400/10 text-red-200" : "border-emerald-400/40 bg-emerald-400/10 text-emerald-200"}`}>
              {stop.active ? "BLOCKING" : "ARMED"}
            </span>
            <span className="text-sm font-black text-slate-100">{formatPrice(stop.priceLevel)}</span>
          </div>
          <MetricStack label="Regime" value={decisionConditionLabel(stop.regimeCondition)} detail="driver flip condition" />
          <MetricStack label="Flow / Liquidity" value={decisionConditionLabel(stop.flowCondition)} detail={decisionConditionLabel(stop.liquidityCondition)} />
        </InfoCard>
      </div>

      <div className="mt-4 flex flex-wrap gap-2">
        {reasoning.length ? (
          reasoning.map((item) => (
            <span className="rounded-full border border-slate-800 bg-slate-900/70 px-3 py-1 text-xs text-slate-300" key={item}>
              {executionReasonLabel(item)}
            </span>
          ))
        ) : (
          <span className="rounded-full border border-slate-800 bg-slate-900/70 px-3 py-1 text-xs text-slate-500">
            awaiting execution strategy reasoning
          </span>
        )}
      </div>
    </section>
  );
}

function CapitalNarrativeTimeline({ timeline = {} }) {
  const phases = timeline.phases || [];
  return (
    <InfoCard title="Capital Timeline Layer">
      <div className="flex items-center justify-between gap-3 text-xs text-slate-400">
        <span>Dominant: {phaseLabels[timeline.dominantPhase] || "中性"}</span>
        <span>{formatDuration(timeline.totalDurationSec)}</span>
      </div>
      <div className="mt-2 space-y-2">
        {phases.length ? (
          phases.map((phase, index) => (
            <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-3" key={`${phase.label}-${index}`}>
              <div className="flex items-center justify-between gap-3">
                <span className={`rounded-full border px-2 py-1 text-[11px] font-black ${phaseTone[phase.phase] || phaseTone.neutral}`}>
                  {phaseLabels[phase.phase] || phase.phase}
                </span>
                <span className="text-xs font-bold text-slate-300">{formatSignedUsd(phase.netFlowUsd)}</span>
              </div>
              <div className="mt-2 grid gap-2 text-xs text-slate-400 sm:grid-cols-2">
                <span>{formatDuration(phase.durationSec)}</span>
                <span>{transitionReasonLabel(phase.transitionReason)}</span>
              </div>
            </div>
          ))
        ) : (
          <div className="rounded-lg border border-slate-800 bg-slate-900/70 p-3 text-sm text-slate-400">
            等待连续资金时间轴。
          </div>
        )}
      </div>
      <p className="text-xs text-slate-500">{timeline.narrative || "awaiting timeline narrative"}</p>
    </InfoCard>
  );
}

function PositionFlowCurvePanel({ curve = {} }) {
  const points = curve.points || [];
  const path = buildSvgPath(points, "positionUsd");
  return (
    <InfoCard title="Position Flow Curve">
      <div className="grid grid-cols-3 gap-2">
        <MiniMetric label="Latest" value={formatSignedUsd(curve.latestPositionUsd)} />
        <MiniMetric label="Acc Speed" value={`${formatUsd(curve.accumulationSlopeUsdPerMin)}/m`} />
        <MiniMetric label="Dist Speed" value={`${formatUsd(curve.distributionSlopeUsdPerMin)}/m`} />
      </div>
      <div className="mt-2 h-28 overflow-hidden rounded-xl border border-slate-800 bg-[#050b18]">
        {points.length ? (
          <svg className="h-full w-full" preserveAspectRatio="none" viewBox="0 0 100 100" role="img" aria-label="仓位流曲线">
            <g opacity="0.22">
              {[25, 50, 75].map((y) => (
                <line key={y} stroke="#94a3b8" strokeWidth="0.25" x1="0" x2="100" y1={y} y2={y} />
              ))}
            </g>
            <path d={path} fill="none" stroke="#facc15" strokeLinecap="round" strokeWidth="1.5" />
            {points.map((point, index) => (
              <circle
                cx={(index / Math.max(1, points.length - 1)) * 100}
                cy="50"
                fill={point.speedUsdPerMin >= 0 ? "#34d399" : "#fb7185"}
                key={`${point.ts}-${index}`}
                opacity="0.7"
                r="1.1"
              />
            ))}
          </svg>
        ) : (
          <div className="flex h-full items-center justify-center text-xs text-slate-500">awaiting position curve</div>
        )}
      </div>
    </InfoCard>
  );
}

function LiquidityReactionMapPanel({ map = {} }) {
  const zones = map.vacuumZones || [];
  return (
    <InfoCard title="Liquidity Reaction Map">
      <div className="grid grid-cols-2 gap-2">
        <MiniMetric label="Impact" value={percent(map.impactEfficiency)} />
        <MiniMetric label="Absorption" value={percent(map.absorptionRatio)} />
      </div>
      <MetricStack
        detail="price impact vs liquidity imbalance"
        label="Liquidity Response"
        value={liquidityResponseLabel(map.liquidityResponse)}
      />
      <div className="space-y-2">
        {zones.length ? (
          zones.slice(0, 3).map((zone, index) => (
            <div className="rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2 text-xs" key={`${zone.reason}-${index}`}>
              <div className="flex items-center justify-between gap-3">
                <span className="font-bold text-slate-200">{formatPrice(zone.lower)} - {formatPrice(zone.upper)}</span>
                <span className="text-cyan-200">{percent(zone.intensity)}</span>
              </div>
              <p className="mt-1 text-slate-500">{zone.reason}</p>
            </div>
          ))
        ) : (
          <div className="rounded-lg border border-slate-800 bg-slate-900/70 p-3 text-sm text-slate-400">
            暂无流动性真空区确认。
          </div>
        )}
      </div>
    </InfoCard>
  );
}

function MarketHeader({ reconstruction, loading }) {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <div className="flex flex-col gap-4 2xl:flex-row 2xl:items-end 2xl:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Market Header</p>
          <div className="mt-2 flex flex-wrap items-center gap-3">
            <h4 className="text-3xl font-black text-white">{reconstruction.symbol}</h4>
            <span className={`rounded-full border px-3 py-1 text-xs font-black ${phaseTone[reconstruction.currentPhase] || phaseTone.neutral}`}>
              {phaseLabels[reconstruction.currentPhase] || "中性"}
            </span>
            {loading ? <span className="text-xs text-cyan-300">刷新中...</span> : null}
          </div>
          <p className="mt-2 text-sm text-slate-400">资金行为重建终端，不展示 tick 级噪声。</p>
        </div>
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-6">
          <MetricPill label="Market Price" value={formatPrice(reconstruction.marketPrice || reconstruction.currentPrice)} tone="cyan" />
          <MetricPill label="Price Source" value={priceSourceLabel(reconstruction.marketPriceSource)} tone={priceSourceTone(reconstruction.marketPriceSource)} />
          <MetricPill label="24h Change" value={formatPctOrNA(reconstruction.change24hPct)} tone={Number(reconstruction.change24hPct || 0) >= 0 ? "emerald" : "yellow"} />
          <MetricPill label="24h Volume" value={formatOptionalUsd(reconstruction.volume24hUsd)} />
          <MetricPill label="Liquidity" value={liquidityCondition(reconstruction)} tone="cyan" />
          <MetricPill label="Regime" value={marketRegime(reconstruction)} tone="yellow" />
        </div>
      </div>
      <p className="mt-3 text-xs text-slate-500">
        Analysis price: {formatPrice(reconstruction.analysisPrice || reconstruction.vwapAnchor)} · Source {priceSourceLabel(reconstruction.analysisPriceSource)}
        {reconstruction.priceFallbackReason ? ` · Fallback: ${priceFallbackReasonLabel(reconstruction.priceFallbackReason)}` : ""}
      </p>
    </div>
  );
}

function CapitalStructureOverview({ reconstruction }) {
  return (
    <section className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <div className="flex flex-col gap-1 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Capital Structure Overview</p>
          <h5 className="mt-1 text-lg font-black text-white">资金行为优先视图</h5>
        </div>
        <span className="text-xs text-slate-500">Time buckets: 5m / 15m / 1h / 4h</span>
      </div>
      <div className="mt-4 grid gap-4 2xl:grid-cols-[minmax(0,1.4fr)_minmax(260px,0.6fr)_minmax(280px,0.8fr)]">
        <PhaseStrip reconstruction={reconstruction} />
        <CurrentPositionState reconstruction={reconstruction} />
        <MarketRegimeOverlay reconstruction={reconstruction} />
      </div>
      <TimeframeStructurePanel reconstruction={reconstruction} />
    </section>
  );
}

function PhaseStrip({ reconstruction }) {
  const timeline = reconstruction.phaseTimeline || [];
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Capital Phase Bar</p>
      <div className="mt-3 grid gap-2 md:grid-cols-5">
        {phaseStrip.map((phase) => {
          const segment = timeline.find((entry) => phase.matches.includes(entry.phase));
          const active = phase.matches.includes(reconstruction.currentPhase);
          return (
            <div
              className={[
                "rounded-xl border px-3 py-2",
                active ? phaseTone[phase.key] || phaseTone.neutral : "border-slate-800 bg-slate-900/70 text-slate-400",
              ].join(" ")}
              key={phase.key}
            >
              <div className="text-sm font-black">{phase.label}</div>
              <div className="mt-1 text-xs opacity-80">{phase.sublabel}</div>
              <div className="mt-2 text-xs opacity-80">{segment ? formatDuration(segment.durationSec) : "未确认"}</div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function CurrentPositionState({ reconstruction }) {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Current Position State</p>
      <p className="mt-3 text-lg font-black text-white">
        {phaseLabels[reconstruction.currentPhase] || "中性"} {phaseStageSuffix(reconstruction)}
      </p>
      <p className="mt-2 text-sm text-slate-400">Confidence: {confidenceBand(reconstruction.confidence)}</p>
      <div className="mt-4 grid gap-2">
        <MetricStack label="Cost Range" value={`${formatPrice(reconstruction.costBasisLow)} - ${formatPrice(reconstruction.costBasisHigh)}`} detail={`VWAP ${formatPrice(reconstruction.vwapAnchor)}`} />
        <MetricStack label="Position" value={positionRange(reconstruction)} detail="estimated smart money range" />
      </div>
    </div>
  );
}

function MarketRegimeOverlay({ reconstruction }) {
  const window15m = behaviorWindowFor(reconstruction, 900);
  const window4h = behaviorWindowFor(reconstruction, 14_400);
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/70 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Market Regime Overlay</p>
      <div className="mt-3 grid gap-2">
        <MetricStack label="Liquidity" value={liquidityCondition(reconstruction)} detail="depth and volume condition" />
        <MetricStack label="Volatility" value={volatilityRegime(window15m)} detail={window15m ? `15m vol ${signed(window15m.volatilityPct)}%` : "awaiting 15m window"} />
        <MetricStack label="Flow" value={flowRegime(window15m)} detail={window15m ? `15m OFI ${signed(window15m.normalizedOfi)}` : "no window confirmation"} />
        <MetricStack label="4h Macro Bias" value={flowRegime(window4h)} detail={window4h ? `4h OFI ${signed(window4h.normalizedOfi)} · vol ${signed(window4h.volatilityPct)}%` : "awaiting 4h window"} />
      </div>
    </div>
  );
}

function TimeframeStructurePanel({ reconstruction }) {
  const layers = [
    { label: "5m 执行层", role: "execution", windowSec: 300 },
    { label: "15m 主视图", role: "primary", windowSec: 900 },
    { label: "1h 结构层", role: "structure", windowSec: 3600 },
    { label: "4h 宏观层", role: "macro", windowSec: 14_400 },
  ];
  return (
    <div className="mt-4 rounded-xl border border-slate-800 bg-slate-900/70 p-4">
      <div className="flex flex-col gap-1 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Timeframe Structure Stack</p>
          <h5 className="mt-1 text-base font-black text-white">跨周期资金结构层</h5>
        </div>
        <span className="text-xs text-slate-500">4h 作为宏观层，不参与秒级噪声展示</span>
      </div>
      <div className="mt-4 grid gap-3 md:grid-cols-2 2xl:grid-cols-4">
        {layers.map((layer) => {
          const window = behaviorWindowFor(reconstruction, layer.windowSec);
          return (
            <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-3" key={layer.windowSec}>
              <div className="flex items-center justify-between gap-3">
                <div>
                  <p className="text-sm font-black text-white">{layer.label}</p>
                  <p className="mt-1 text-[11px] uppercase tracking-[0.14em] text-slate-500">{layer.role}</p>
                </div>
                <span className={`rounded-full border px-2 py-1 text-[11px] font-black ${flowTone(window)}`}>
                  {flowRegime(window)}
                </span>
              </div>
              <div className="mt-3 grid grid-cols-3 gap-2">
                <MiniMetric label="OFI" value={window ? signed(window.normalizedOfi) : "N/A"} />
                <MiniMetric label="VWAP" value={window ? formatPrice(window.vwap) : "N/A"} />
                <MiniMetric label="Abs" value={window ? percent(window.absorptionScore) : "N/A"} />
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function StabilityLayerPanel({ item, reconstruction }) {
  const compression = item?.lastSignal?.signalCompression || {};
  const stable = compression.stableSignals || {};
  const regime = compression.regimeState || {};
  return (
    <section className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <div className="flex flex-col gap-1 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">System Lock Stability Layer</p>
          <h5 className="mt-1 text-lg font-black text-white">信号稳定 · 状态平滑 · 成本精炼</h5>
        </div>
        <span className="text-xs text-slate-500">只读稳定层，不触发交易执行。</span>
      </div>
      <div className="mt-4 grid gap-3 xl:grid-cols-3">
        <InfoCard title="Signal Compression Stability">
          <div className="grid grid-cols-3 gap-2">
            <MiniMetric label="SMP" value={signed(stable.smpStable)} />
            <MiniMetric label="MFE" value={signed(stable.mfeStable)} />
            <MiniMetric label="LSM" value={signed(stable.lsmStable)} />
          </div>
          <ProgressRow
            detail={`${stable.persistenceWindows || 0} persistent windows · flip ${percent(stable.flipPenalty)}`}
            label="Stability Score"
            value={stable.stabilityScore}
          />
        </InfoCard>
        <InfoCard title="Regime Smoothing">
          <MetricStack
            detail={`transition risk ${regime.transitionRisk || "low"}`}
            label="Current Regime"
            value={regimeLabel(regime.current)}
          />
          <ProgressRow label="Regime Confidence" value={regime.confidence} />
          <ProgressRow label="Regime Stability" value={regime.stability} />
        </InfoCard>
        <InfoCard title="Cost Basis Refinement">
          <MetricStack
            detail="clustered accumulation density"
            label="Density Peak"
            value={formatPrice(reconstruction.densityPeak)}
          />
          <MetricStack
            detail={`anchor ${formatPrice(reconstruction.vwapAnchor)}`}
            label="Refined Cost Band"
            value={`${formatPrice(reconstruction.costBasisLow)} - ${formatPrice(reconstruction.costBasisHigh)}`}
          />
        </InfoCard>
      </div>
    </section>
  );
}

function StructureChart({ chart, reconstruction }) {
  const points = chart?.points?.length ? chart.points : fallbackPoints(reconstruction);
  const path = buildSvgPath(points, "price");
  const netPath = buildSvgPath(points, "netPosition");
  const priceY = priceYMapper(points, reconstruction);
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <div className="mb-3 flex items-center justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Market Price + Capital Reconstruction Chart</p>
          <h5 className="mt-1 text-lg font-black text-white">价格路径、成本带与净仓位曲线</h5>
        </div>
        <span className="text-xs text-slate-500">价格路径使用 {priceSourceLabel(chart?.marketPriceSource || reconstruction.marketPriceSource)}</span>
      </div>
      <div className="relative h-80 overflow-hidden rounded-xl border border-slate-800 bg-[#050b18]">
        <svg className="h-full w-full" preserveAspectRatio="none" viewBox="0 0 100 100" role="img" aria-label="主力成本结构重建图">
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
          <rect
            fill="#10b981"
            height={Math.max(1.5, Math.abs(priceY(reconstruction.costBasisLow) - priceY(reconstruction.costBasisHigh)))}
            opacity="0.12"
            width="100"
            x="0"
            y={Math.min(priceY(reconstruction.costBasisLow), priceY(reconstruction.costBasisHigh))}
          />
          <line stroke="#34d399" strokeDasharray="3 2" strokeWidth="0.6" x1="0" x2="100" y1={priceY(reconstruction.costBasisLow)} y2={priceY(reconstruction.costBasisLow)} />
          <line stroke="#fbbf24" strokeWidth="0.8" x1="0" x2="100" y1={priceY(reconstruction.vwapAnchor)} y2={priceY(reconstruction.vwapAnchor)} />
          <line stroke="#34d399" strokeDasharray="3 2" strokeWidth="0.6" x1="0" x2="100" y1={priceY(reconstruction.costBasisHigh)} y2={priceY(reconstruction.costBasisHigh)} />
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
          <span className="text-cyan-200">市场价路径</span>
          <span className="text-yellow-200">净仓位</span>
          <span className="text-emerald-200">成本带</span>
          <span className="text-emerald-200">成交量柱</span>
        </div>
      </div>
    </div>
  );
}

function SmartMoneyStructurePanel({ reconstruction }) {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Smart Money Structure Panel</p>
      <h5 className="mt-1 text-lg font-black text-white">成本结构与仓位状态</h5>
      <div className="mt-4 grid gap-3">
        <MetricStack label="Smart Money Cost Range" value={`${formatPrice(reconstruction.costBasisLow)} - ${formatPrice(reconstruction.costBasisHigh)}`} detail={`VWAP Anchor ${formatPrice(reconstruction.vwapAnchor)} · ${confidenceBand(reconstruction.confidence)}`} />
        <MetricStack label="Estimated Position Size" value={positionRange(reconstruction)} detail="position reconstruction range" />
        <MetricStack label="Unrealized PnL" value={pnlRange(reconstruction)} detail="relative to reconstructed cost range" />
        <MetricStack label="Distribution Completion" value={`${Math.round(reconstruction.distributionCompletionPct)}%`} detail={`intensity ${Math.round(reconstruction.distributionIntensityScore)} / 100`} />
        <MetricStack label="Last Accumulation Node" value={lastNodeRange(reconstruction.lastAccumulationNode)} detail={lastNodeDetail(reconstruction.lastAccumulationNode)} />
      </div>
    </div>
  );
}

function LiquidityFlowPanel({ reconstruction }) {
  const window15m = behaviorWindowFor(reconstruction, 900);
  const node = reconstruction.lastAccumulationNode;
  return (
    <section className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Liquidity + Flow Intelligence</p>
      <h5 className="mt-1 text-lg font-black text-white">15m 流动性与资金流</h5>
      <div className="mt-4 grid gap-3 md:grid-cols-3">
        <MetricStack
          label="Flow Imbalance"
          value={window15m ? signed(window15m.normalizedOfi) : "N/A"}
          detail={window15m ? flowRegime(window15m) : "awaiting 15m aggregation"}
        />
        <MetricStack
          label="Absorption Efficiency"
          value={node ? percent(node.absorptionEfficiency) : window15m ? percent(window15m.absorptionScore) : "N/A"}
          detail="bid-side absorption proxy"
        />
        <MetricStack
          label="Liquidity Map"
          value={liquidityCondition(reconstruction)}
          detail={liquidityMapDetail(reconstruction)}
        />
      </div>
    </section>
  );
}

function ShortTermOutlookPanel({ probabilities = {} }) {
  return (
    <section className="rounded-xl border border-slate-800 bg-slate-950/70 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Short Term Outlook</p>
      <h5 className="mt-1 text-lg font-black text-white">三状态低噪声展望</h5>
      <div className="mt-4 space-y-3">
        <ProgressRow label="Continue Distribution" value={probabilities.continueDistribution} />
        <ProgressRow label="Range Consolidation" value={probabilities.rangeConsolidation} />
        <ProgressRow label="Re-accumulation" value={probabilities.secondaryAccumulation} />
      </div>
    </section>
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

function WatchListItem({ item, active, busy, onSelect, onRemove, reconstruction }) {
  const capital = item.lastSignal?.capitalStructure || {};
  const phase = reconstruction?.currentPhase || capital.phase || "neutral";
  const costBasis = reconstruction
    ? {
        lower: reconstruction.costBasisLow,
        upper: reconstruction.costBasisHigh,
        vwapAnchor: reconstruction.vwapAnchor,
      }
    : capital.costBasis;
  const confidence = reconstruction?.confidence ?? capital.phaseConfidence;
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
          成本 {costBasisRange(costBasis)} · Conf {percent(confidence)}
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

function MiniMetric({ label, value }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/70 px-2 py-2">
      <p className="text-[10px] uppercase tracking-[0.12em] text-slate-500">{label}</p>
      <p className="mt-1 truncate text-xs font-black text-slate-100">{value}</p>
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

function buildDynamicsTrajectory(vector = {}, velocity = {}) {
  const baseSmp = Number(vector.smp || 0);
  const basePosition = Number(vector.positionUsd || 0);
  const baseLiquidity = Number(vector.liquidity || 0);
  const flowAcceleration = Number(velocity.flowAcceleration || 0);
  const positionVelocity = Number(velocity.positionVelocityUsdPerMin || 0);
  const liquidityShift = Number(velocity.liquidityShiftRate || 0);

  return Array.from({ length: 8 }, (_, index) => {
    const step = index / 7;
    return {
      smp: baseSmp + flowAcceleration * step * 0.6,
      position: basePosition + positionVelocity * step * 5,
      liquidity: Math.max(0, Math.min(1, baseLiquidity + liquidityShift * step * 0.45)),
    };
  });
}

function priceYMapper(points, reconstruction) {
  const values = [
    ...(points || []).map((point) => Number(point.price || 0)),
    Number(reconstruction.costBasisLow || 0),
    Number(reconstruction.costBasisHigh || 0),
    Number(reconstruction.vwapAnchor || 0),
  ].filter((value) => Number.isFinite(value) && value > 0);
  const min = Math.min(...values);
  const max = Math.max(...values);
  const range = Math.max(0.000001, max - min);
  return (price) => {
    const numeric = Number(price || min);
    return 92 - ((numeric - min) / range) * 78;
  };
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

function behaviorWindowFor(reconstruction = {}, windowSec) {
  return (reconstruction.behaviorWindows || []).find((window) => Number(window.windowSec) === windowSec) || null;
}

function confidenceBand(value) {
  const numeric = Number(value || 0);
  if (numeric >= 0.75) return "High";
  if (numeric >= 0.5) return "Medium";
  return "Low";
}

function phaseStageSuffix(reconstruction = {}) {
  const completion = Number(reconstruction.distributionCompletionPct || 0);
  if (reconstruction.currentPhase === "distribution" && completion >= 60) return "(Late Stage)";
  if (reconstruction.currentPhase === "distribution") return "(Mid Stage)";
  if (reconstruction.currentPhase === "accumulation" && Number(reconstruction.confidence || 0) >= 0.72) return "(Confirmed)";
  if (reconstruction.currentPhase === "markup") return "(Expansion)";
  if (reconstruction.currentPhase === "breakdown") return "(Exit)";
  return "";
}

function energyLabel(value) {
  if (value === "overheating") return "过热";
  if (value === "high") return "高能量";
  if (value === "medium") return "中能量";
  return "低能量";
}

function energyTone(value) {
  if (value === "overheating") return "border-red-400/40 bg-red-400/10 text-red-200";
  if (value === "high") return "border-orange-400/40 bg-orange-400/10 text-orange-200";
  if (value === "medium") return "border-yellow-400/40 bg-yellow-400/10 text-yellow-200";
  return "border-slate-500/40 bg-slate-500/10 text-slate-300";
}

function trajectorySummaryLabel(value) {
  if (!value) return "等待市场轨迹";
  return String(value)
    .replaceAll("_", " ")
    .replace("energy expanding", "能量扩张")
    .replace("liquidity deteriorating", "流动性走弱")
    .replace("transition risk rising", "转移风险上升")
    .replace("trajectory stable", "轨迹稳定");
}

function dynamicsReasonLabel(value) {
  if (value === "flow acceleration plus stable liquidity") return "资金加速 + 流动性稳定";
  if (value === "accumulation inertia") return "吸筹惯性";
  if (value === "cost pressure and liquidity shift") return "成本压力 + 流动性转向";
  if (value === "trend persistence") return "趋势惯性";
  if (value === "liquidity depletion stress") return "流动性消耗压力";
  if (value === "absorption rebuild") return "吸收重建";
  if (value === "post stress re-accumulation") return "压力后再吸筹";
  if (value === "breakdown inertia") return "退出惯性";
  if (value === "liquidity rebuild") return "流动性重建";
  if (value === "flow expansion") return "资金流扩张";
  if (value === "cost pressure") return "成本压力";
  return value || "transition model";
}

function liquidationSideLabel(value) {
  if (value === "long_liquidation") return "Long Liquidation";
  if (value === "short_liquidation") return "Short Liquidation";
  return "Liquidation";
}

function liquidationSideTone(value) {
  if (value === "long_liquidation") return "border-red-400/40 bg-red-400/10 text-red-200";
  if (value === "short_liquidation") return "border-emerald-400/40 bg-emerald-400/10 text-emerald-200";
  return "border-slate-500/40 bg-slate-500/10 text-slate-300";
}

function forceReasonLabel(value) {
  if (value === "downside stop-loss and long liquidation proxy") return "下方止损与多头清算代理区";
  if (value === "upside stop-loss and short liquidation proxy") return "上方止损与空头清算代理区";
  return value || "liquidation proxy";
}

function activeForceZoneLabel(value) {
  if (value === "short_squeeze_zone") return "Short Squeeze Zone";
  if (value === "long_liquidation_zone") return "Long Liquidation Zone";
  if (value === "two_sided_stop_hunt_zone") return "Two-sided Stop Hunt";
  return "Neutral Zone";
}

function primaryDriverLabel(value) {
  if (value === "liquidation_cascade") return "Liquidation Cascade";
  if (value === "whale_initiated_flow") return "Whale Initiated Flow";
  if (value === "retail_chasing_flow") return "Retail Chasing Flow";
  return "Unknown Driver";
}

function marketDriverLabel(value) {
  if (value === "whale_intent") return "Whale Intent";
  if (value === "liquidity_forcing") return "Liquidity Forcing";
  if (value === "derivatives_pressure") return "Derivatives Pressure";
  if (value === "reflexivity_feedback") return "Reflexivity Feedback";
  if (value === "liquidation_cascade") return "Liquidation Cascade";
  if (value === "none") return "None";
  return value || "Unknown Driver";
}

function liquiditySweepLabel(value) {
  if (value === "upside_short_sweep") return "上方空头止损扫单";
  if (value === "downside_long_sweep") return "下方多头止损扫单";
  if (value === "balanced_sweep_risk") return "双向止损风险";
  return "无明显扫单";
}

function directionLabel(value) {
  if (value === "long") return "LONG";
  if (value === "short") return "SHORT";
  return "NO TRADE";
}

function directionTone(value) {
  if (value === "long") return "border-emerald-400/40 bg-emerald-400/10 text-emerald-200";
  if (value === "short") return "border-red-400/40 bg-red-400/10 text-red-200";
  return "border-slate-500/40 bg-slate-500/10 text-slate-300";
}

function orderTypeLabel(value) {
  if (value === "market") return "Market";
  if (value === "limit") return "Limit";
  return "None";
}

function timingLabel(value) {
  if (value === "immediate") return "Immediate";
  if (value === "wait") return "Wait";
  return "Invalid";
}

function decisionConditionLabel(value) {
  if (value === "wait_for_alignment_or_invalidation_clear") return "等待信号对齐或失效解除";
  if (value === "enter_near_cost_basis_when_smp_regime_liquidity_align") return "成本区附近，SMP/Regime/Liquidity 对齐";
  if (value === "enter_near_upper_cost_band_when_distribution_pressure_persists") return "成本上沿附近，出货压力持续";
  if (value === "exit_on_distribution_transition_or_mfe_exhaustion") return "出货转换或动能耗尽退出";
  if (value === "exit_on_reaccumulation_transition_or_smp_reversal") return "再吸筹转换或 SMP 反转退出";
  if (value === "confidence_x_regime_stability_x_liquidity_x_market_energy_x_pvg") return "置信度 × 状态稳定 × 流动性 × 能量 × PVG";
  if (value === "regime_not_aligned_or_stress_unstable") return "状态未对齐或压力状态不稳";
  if (value === "regime_flip_against_direction") return "状态反向切换";
  if (value === "smp_lacks_directional_pressure") return "SMP 方向压力不足";
  if (value === "smp_reversal_against_direction") return "SMP 反向";
  if (value === "liquidity_stress_or_manipulation") return "流动性压力或操控环境";
  if (value === "liquidity_collapse_or_vacuum_expansion") return "流动性坍塌或真空扩张";
  if (value === "execution_kernel_waits_for_driver_alignment") return "等待驱动力与风险窗口对齐";
  if (value === "driver_dominance_x_regime_stability_x_liquidity_health") return "主导驱动力 × 状态稳定 × 流动性健康";
  if (value === "no_trade") return "无交易";
  if (value === "no_entry") return "无入场";
  if (value === "no_exit") return "无退出";
  return value || "N/A";
}

function executionReasonLabel(value) {
  if (value === "advisory_only_no_exchange_execution") return "advisory only · no exchange execution";
  if (value === "liquidity_supportive=true") return "liquidity supportive";
  if (value === "liquidity_supportive=false") return "liquidity not supportive";
  if (value === "trap_active=true") return "trap risk active";
  if (value === "trap_active=false") return "trap risk inactive";
  if (String(value).startsWith("primary_driver=")) {
    const [, payload] = String(value).split("=");
    const [driver, score] = payload.split(":");
    return `primary ${marketDriverLabel(driver)} ${score || ""}`.trim();
  }
  if (String(value).startsWith("secondary_driver=")) {
    const [, payload] = String(value).split("=");
    const [driver, score] = payload.split(":");
    return `secondary ${marketDriverLabel(driver)} ${score || ""}`.trim();
  }
  return value || "reason unavailable";
}

function liquidityCondition(reconstruction = {}) {
  const volume = Number(reconstruction.volume24hUsd || 0);
  const distribution = Number(reconstruction.distributionIntensityScore || 0);
  const absorption = Number(reconstruction.lastAccumulationNode?.absorptionEfficiency || 0);
  if (volume >= 100_000_000 || absorption >= 0.75) return "High";
  if (volume >= 10_000_000 || distribution >= 45 || absorption >= 0.45) return "Normal";
  return "Low";
}

function marketRegime(reconstruction = {}) {
  if (Number(reconstruction.distributionIntensityScore || 0) >= 65) return "Stress";
  if (["markup", "breakdown"].includes(reconstruction.currentPhase)) return "Trend";
  if (["accumulation", "distribution"].includes(reconstruction.currentPhase)) return "Chop";
  return "Neutral";
}

function volatilityRegime(window) {
  if (!window) return "Unknown";
  const value = Math.abs(Number(window.volatilityPct || 0));
  if (value >= 1.2) return "Expanding";
  if (value >= 0.35) return "Normal";
  return "Compressed";
}

function flowRegime(window) {
  if (!window) return "Unknown";
  const ofi = Number(window.normalizedOfi || 0);
  if (ofi >= 0.2) return "Accumulation bias";
  if (ofi <= -0.2) return "Distribution bias";
  return "Balanced";
}

function flowTone(window) {
  const regime = flowRegime(window);
  if (regime === "Accumulation bias") return "border-emerald-400/40 bg-emerald-400/10 text-emerald-200";
  if (regime === "Distribution bias") return "border-orange-400/40 bg-orange-400/10 text-orange-200";
  if (regime === "Balanced") return "border-slate-500/40 bg-slate-500/10 text-slate-300";
  return "border-slate-700 bg-slate-900 text-slate-500";
}

function liquidityMapDetail(reconstruction = {}) {
  const levels = reconstruction.smartLevels || [];
  const anchor = levels.find((level) => level.role === "anchor") || levels[0];
  if (!anchor) return "awaiting structural levels";
  return `${anchor.label} ${formatPrice(anchor.price)}`;
}

function regimeLabel(value) {
  const labels = {
    trend: "Trend",
    chop: "Chop",
    liquidity_expansion: "Liquidity Expansion",
    liquidity_stress: "Liquidity Stress",
    manipulation: "Manipulation",
    neutral: "Neutral",
  };
  return labels[value] || "Neutral";
}

function transitionReasonLabel(value) {
  const labels = {
    "low volatility absorption": "低波动吸收",
    "liquidity exhaustion": "流动性耗尽",
    "negative delta persistence": "持续负 Delta",
    "positive delta persistence": "持续正 Delta",
    "volatility expansion": "波动展开",
    "delta divergence": "Delta 背离",
    "mixed flow": "混合资金流",
  };
  return labels[value] || value || "混合资金流";
}

function liquidityResponseLabel(value) {
  const labels = {
    absorption_dominant: "吸收主导",
    liquidity_vacuum: "流动性真空",
    distribution_pressure: "出货压力",
    balanced_liquidity: "流动性平衡",
    unknown: "未知",
  };
  return labels[value] || value || "未知";
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

function formatPctOrNA(value) {
  if (value === null || value === undefined) return "N/A";
  return `${signed(value)}%`;
}

function priceSourceLabel(value) {
  if (value === "market_perp") return "PERP";
  if (value === "market_spot") return "SPOT";
  if (value === "mark_price") return "MARK";
  if (value === "vwap") return "VWAP";
  if (value === "reconstructed") return "MODEL";
  return "UNKNOWN";
}

function priceSourceTone(value) {
  if (value === "market_perp" || value === "market_spot") return "emerald";
  if (value === "mark_price") return "yellow";
  if (value === "vwap") return "cyan";
  return "yellow";
}

function priceFallbackReasonLabel(value) {
  if (value === "market_price_unavailable_using_analysis_vwap") return "market unavailable, using VWAP/model";
  if (value === "perp_last_price_unavailable_using_mark_price") return "perp unavailable, using mark price";
  return value;
}

function formatUsd(value) {
  const numeric = Number(value || 0);
  if (!Number.isFinite(numeric) || numeric <= 0) return "$0";
  if (numeric >= 1_000_000) return `$${(numeric / 1_000_000).toFixed(1)}M`;
  if (numeric >= 1_000) return `$${(numeric / 1_000).toFixed(0)}K`;
  return `$${numeric.toFixed(0)}`;
}

function formatSignedUsd(value) {
  const numeric = Number(value || 0);
  if (!Number.isFinite(numeric) || numeric === 0) return "$0";
  return `${numeric > 0 ? "+" : "-"}${formatUsd(Math.abs(numeric))}`;
}

function errorMessage(error) {
  if (error === "max_active_tokens_reached") return "最多只能同时监控 50 个币。";
  if (error === "invalid_symbol") return "Symbol 格式无效。";
  if (error === "token_not_found") return "该 symbol 当前未在监控列表。";
  if (error === "operator_token_required") return "该操作需要 operator 权限。";
  if (error === "binance_usdm_contract_not_trading") return "该标的不是可交易的 Binance USD-M USDT 永续合约。";
  return error || "操作失败";
}
