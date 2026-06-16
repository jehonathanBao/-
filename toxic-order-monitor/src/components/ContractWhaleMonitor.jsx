import { useEffect, useState } from "react";
import {
  CWM_MAX_PRICE_DEVIATION_PCT,
  fetchContractWhaleEvents,
  fetchContractWhaleHistory,
  fetchContractWhaleLatest,
  fetchContractWhaleSummary,
} from "../api/contractWhale.js";

const SUMMARY_REFRESH_MS = 5_000;
const LATEST_REFRESH_MS = 10_000;
const DEFAULT_FILTERS = {
  symbol: "BTC",
  severity: "all",
  signal_type: "all",
  direction: "all",
  discord_sent: "all",
  window_sec: "all",
  exchange: "all",
};

export default function ContractWhaleMonitor() {
  const [state, setState] = useState({
    loading: true,
    summary: null,
    items: [],
    events: [],
    meta: null,
    error: null,
  });
  const [selectedSignalId, setSelectedSignalId] = useState(null);
  const [selectedWhaleId, setSelectedWhaleId] = useState(null);
  const [filters, setFilters] = useState(DEFAULT_FILTERS);

  useEffect(() => {
    let cancelled = false;
    let summaryTimer = null;
    let latestTimer = null;

    const refreshLatest = () => {
      const request = shouldUseHistory(filters)
        ? fetchContractWhaleHistory({ ...filters, limit: 50 })
        : fetchContractWhaleLatest(50, filters.symbol);
      Promise.all([request, fetchContractWhaleEvents({ symbol: filters.symbol, limit: 12 })]).then(([payload, eventsPayload]) => {
        if (cancelled) return;
        setState((previous) => ({
          loading: false,
          summary: payload.error ? previous.summary : payload.summary,
          items: payload.error ? previous.items : payload.items,
          events: eventsPayload.error ? previous.events : eventsPayload.items,
          meta: payload.error ? previous.meta : (payload.meta || null),
          error: payload.error || eventsPayload.error || null,
        }));
      });
    };

    const refreshSummary = () => {
      fetchContractWhaleSummary(filters.symbol).then((payload) => {
        if (cancelled) return;
        setState((previous) => ({
          ...previous,
          loading: false,
          summary: payload.error ? previous.summary : payload.summary,
          meta: payload.error ? previous.meta : (payload.meta || previous.meta),
          error: payload.error || null,
        }));
      });
    };

    const clearTimers = () => {
      if (summaryTimer) window.clearInterval(summaryTimer);
      if (latestTimer) window.clearInterval(latestTimer);
      summaryTimer = null;
      latestTimer = null;
    };

    const configurePolling = () => {
      clearTimers();
      if (document.visibilityState === "hidden") return;
      summaryTimer = window.setInterval(refreshSummary, SUMMARY_REFRESH_MS);
      latestTimer = window.setInterval(refreshLatest, LATEST_REFRESH_MS);
    };

    const handleVisibilityChange = () => {
      configurePolling();
      if (document.visibilityState !== "hidden") {
        refreshSummary();
        refreshLatest();
      }
    };

    refreshLatest();
    configurePolling();
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      cancelled = true;
      clearTimers();
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [filters]);

  useEffect(() => {
    if (selectedSignalId && !state.items.some((item) => item.id === selectedSignalId)) {
      setSelectedSignalId(null);
    }
  }, [selectedSignalId, state.items]);

  useEffect(() => {
    if (state.items.length === 0) {
      if (selectedWhaleId) setSelectedWhaleId(null);
      return;
    }
    const entities = buildWhaleEntities(state.items);
    if (!selectedWhaleId || !entities.some((entity) => entity.id === selectedWhaleId)) {
      setSelectedWhaleId(entities[0]?.id || null);
    }
  }, [selectedWhaleId, state.items]);

  const summary = state.summary || {
    status: "calm",
    healthStatus: "disabled",
    healthReason: "contract_whale_monitor_disabled",
    thresholdProfile: "binance_bitfinex",
    thresholdProfileReason: "active_contract_sources=binance,bitfinex",
    configuredContractSources: ["binance", "bitfinex"],
    eligibleContractSources: ["binance", "bitfinex"],
    activeExchangeCount: 0,
    enabledExchanges: [],
    disabledExchanges: ["binance", "okx", "bitfinex"],
        activeContractExchanges: [],
        direction: "neutral",
    latestDirection: "neutral",
    latestSeverity: "calm",
    latestPushedAtMs: null,
    lastDiscordSentAt: null,
    signalCount: 0,
        enabled: false,
        dryRun: true,
        contractDataQuality: 0,
        spotDataQuality: 0,
        overallDataQuality: 0,
        discordDryRunStats: {
          signals1h: 0,
          high1h: 0,
          critical1h: 0,
          s1h: 0,
          wouldSend1h: 0,
          skippedLowScore1h: 0,
          skippedCooldown1h: 0,
          skippedDataQuality1h: 0,
          skippedWarmup1h: 0,
          skippedDisplayOnly1h: 0,
        },
        marketStructureLite: {
          status: "calm",
          regimeType: "unclear",
          mainForceScore: 0,
          extremeImpactScore: 0,
          structureBias: 0,
          confidence: 0,
          dataQuality: 0,
          spotScore: 0,
          contractScore: 0,
          crossConfirmScore: 0,
          mainForceConfirmed: false,
          extremeImpactConfirmed: false,
          reason: "",
        },
        trend60s: {
      buyVolumeBtc: 0,
      sellVolumeBtc: 0,
      totalVolumeBtc: 0,
      netVolumeBtc: 0,
      dominance: 0,
      buyRatio: 0,
      sellRatio: 0,
      updatedAtMs: null,
    },
    exchanges: {
      binance: { connected: false, status: "disconnected", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
      okx: { connected: false, status: "disabled", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
      bitfinex: { connected: false, status: "disconnected", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
      coinbase: { connected: false, status: "spot_only", lastTradeAt: null, latencyMs: null, reconnectCount: 0 },
    },
    platforms: {
      binance: { platformEnabled: true, status: "active", markets: {} },
      bitfinex: { platformEnabled: true, status: "active", markets: {} },
      coinbase: { platformEnabled: true, status: "spot_only", markets: {} },
      okx: { platformEnabled: false, status: "disabled", markets: {} },
    },
  };
  const platformCapabilities = summary.platforms || {};
  const selectedSignal = state.items.find((item) => item.id === selectedSignalId) || null;
  const whaleEntities = buildWhaleEntities(state.items);

  return (
    <section className="console-panel mb-5 p-4 md:p-5">
      <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
        <div className="max-w-3xl">
          <p className="console-label text-cyan-300">Contract Whale Flow</p>
          <h3 className="mt-2 text-lg font-bold text-white">主力合约监控</h3>
          <p className="mt-1 text-sm leading-6 text-slate-400">
            BTC / ETH 永续合约主动成交流异常，Critical / S 才进入外部告警判断。
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-4 xl:grid-cols-7">
          <StatusPill label="当前状态" value={statusLabel(summary.status)} tone={statusTone(summary.status)} />
          <StatusPill label="健康状态" value={healthStatusLabel(summary.healthStatus)} tone={healthStatusTone(summary.healthStatus)} />
          <StatusPill label="当前方向" value={directionLabel(summary.latestDirection || summary.direction)} tone="cyan" />
          <StatusPill label="最新等级" value={severityLabel(summary.latestSeverity)} tone={severityTone(summary.latestSeverity)} />
          <StatusPill label="阈值模式" value={thresholdProfileLabel(summary.thresholdProfile)} tone="cyan" />
          <StatusPill label="运行模式" value={modeLabel(summary)} tone={summary.enabled ? (summary.dryRun ? "yellow" : "cyan") : "slate"} />
          <StatusPill label="最近推送" value={summary.lastDiscordSentAt ? relativeAge(summary.lastDiscordSentAt) : "暂无"} tone="slate" />
        </div>
      </div>

      <ContractWhaleTrendBar trend={summary.trend60s} symbol={filters.symbol} />

      <p className="mt-3 rounded-lg border border-slate-800/80 bg-slate-950/35 px-3 py-2 text-xs leading-5 text-slate-400">
        合约数据质量 {formatScore(summary.contractDataQuality)} · 现货数据质量 {formatScore(summary.spotDataQuality)} · 总体 {formatScore(summary.overallDataQuality)} · {summary.thresholdProfileReason}
      </p>

      <MarketStructureLitePanel summary={summary} />

      <PlatformCapabilitySection
        exchanges={summary.exchanges || {}}
        platforms={platformCapabilities}
        summary={summary}
      />

      {state.error ? (
        <p className="mt-3 rounded-lg border border-yellow-500/30 bg-yellow-500/10 px-3 py-2 text-xs text-yellow-100">
          主力合约监控数据暂时不可用，已保留上一次结果。
        </p>
      ) : null}
      {state.meta?.reason === "coinbase_perp_disabled" ? (
        <p className="mt-3 rounded-lg border border-cyan-500/30 bg-cyan-500/10 px-3 py-2 text-xs text-cyan-100">
          Coinbase 当前仅启用现货，未启用合约；本页只统计 perp 合约成交，因此不会返回 Coinbase 合约信号。
        </p>
      ) : null}

      <ContractWhaleFilters
        filters={filters}
        onChange={(nextFilters) => {
          setSelectedSignalId(null);
          setSelectedWhaleId(null);
          setFilters(nextFilters);
        }}
      />
      <p className="mt-3 rounded-xl border border-cyan-500/20 bg-cyan-500/5 px-3 py-2 text-xs text-cyan-100">
        已隐藏价格偏离超过 {CWM_MAX_PRICE_DEVIATION_PCT}% 的合约信号；详情里可查看当前价格、信号价格和偏离比例。
      </p>

      <RawSignalDebugSection
        enabled={summary.enabled}
        items={state.items}
        loading={state.loading}
        onOpenSignal={setSelectedSignalId}
      />

      <WhaleTrajectoryDashboard
        enabled={summary.enabled}
        loading={state.loading}
        onOpenSignal={setSelectedSignalId}
        onSelectWhale={setSelectedWhaleId}
        selectedWhaleId={selectedWhaleId}
        symbol={filters.symbol}
        whales={whaleEntities}
      />

      <MainForceEventsSection events={state.events} symbol={filters.symbol} />

      {selectedSignal ? (
        <ContractWhaleDetailModal
          summary={summary}
          signal={selectedSignal}
          relatedSignals={state.items}
          onClose={() => setSelectedSignalId(null)}
        />
      ) : null}
    </section>
  );
}

function WhaleTrajectoryDashboard({
  enabled,
  loading,
  onOpenSignal,
  onSelectWhale,
  selectedWhaleId,
  symbol,
  whales,
}) {
  const selectedWhale = whales.find((whale) => whale.id === selectedWhaleId) || whales[0] || null;
  return (
    <section className="mt-4 rounded-2xl border border-cyan-500/20 bg-slate-950/35 p-4">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <p className="console-label text-cyan-300">Whale Behavior Timeline</p>
          <h4 className="mt-1 text-base font-bold text-white">主力行为轨迹（辅助）</h4>
          <p className="mt-1 max-w-3xl text-xs leading-5 text-slate-400">
            按同一 symbol、方向、价格区间和时间连续性把碎片信号合并成 whale entity，用于复盘连续主力意图；上方逐条合约信号表保留每一次检测结果。
          </p>
        </div>
        <div className="grid grid-cols-3 gap-2 text-xs text-slate-300">
          <MiniInfoCard label="Whale Entities" value={`${whales.length}`} detail={`当前筛选 ${symbol}`} />
          <MiniInfoCard label="Merged Signals" value={`${whales.reduce((sum, whale) => sum + whale.signalCount, 0)}`} detail="去重后的主力投影" />
          <MiniInfoCard label="Focus" value={selectedWhale ? shortWhaleId(selectedWhale.id) : "N/A"} detail={selectedWhale ? trajectoryIntentLabel(selectedWhale.intent) : "等待数据"} />
        </div>
      </div>

      {loading ? (
        <p className="mt-4 rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
          主力轨迹载入中...
        </p>
      ) : whales.length === 0 ? (
        <p className="mt-4 rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
          {enabled ? `暂无 ${symbol} 主力轨迹` : "主力合约监控未启用"}
        </p>
      ) : (
        <div className="mt-4 grid gap-4 xl:grid-cols-[minmax(240px,0.38fr)_minmax(0,1fr)]">
          <aside className="rounded-xl border border-slate-800 bg-slate-950/45 p-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="console-label">Whale Entity List</p>
                <h5 className="mt-1 text-sm font-bold text-white">主力实体</h5>
              </div>
              <span className="rounded-full border border-cyan-500/30 px-2 py-1 text-[11px] font-semibold text-cyan-100">
                {whales.length} active
              </span>
            </div>
            <div className="mt-3 space-y-2">
              {whales.map((whale) => (
                <WhaleEntityCard
                  key={whale.id}
                  onSelect={() => onSelectWhale(whale.id)}
                  selected={whale.id === selectedWhale?.id}
                  whale={whale}
                />
              ))}
            </div>
          </aside>

          <TrajectoryFocusPanel onOpenSignal={onOpenSignal} whale={selectedWhale} />
        </div>
      )}
    </section>
  );
}

function WhaleEntityCard({ onSelect, selected, whale }) {
  return (
    <button
      className={`w-full rounded-xl border px-3 py-3 text-left outline-none transition focus-visible:ring-2 focus-visible:ring-cyan-500/35 ${
        selected
          ? "border-cyan-400/70 bg-cyan-500/10 shadow-glow"
          : "border-slate-800 bg-slate-900/55 hover:border-cyan-500/40 hover:bg-slate-900"
      }`}
      data-testid={`whale-entity-card-${whale.id}`}
      onClick={onSelect}
      type="button"
    >
      <div className="flex items-start justify-between gap-3">
        <div>
          <p className="text-sm font-bold text-slate-100">{shortWhaleId(whale.id)}</p>
          <p className="mt-1 text-xs text-cyan-100">{trajectoryIntentLabel(whale.intent)}</p>
        </div>
        <span className={`rounded-full px-2 py-1 text-[11px] font-bold ${severityBadgeClass(whale.severity)}`}>
          {severityLabel(whale.severity)}
        </span>
      </div>
      <div className="mt-3 grid grid-cols-2 gap-2 text-[11px] text-slate-400">
        <p>signals {whale.signalCount}</p>
        <p>duration {formatMsDuration(whale.durationMs)}</p>
        <p>stealth {formatPct(whale.stealthGamma * 100)}</p>
        <p>λ proxy {formatPct(whale.hazardPeak * 100)}</p>
      </div>
      <p className="mt-2 truncate text-[11px] text-slate-500" title={regimePathLabel(whale.regimePath)}>
        {regimePathLabel(whale.regimePath)}
      </p>
    </button>
  );
}

function TrajectoryFocusPanel({ onOpenSignal, whale }) {
  if (!whale) return null;
  const primarySignal = whale.signals[0];
  return (
    <article className="rounded-xl border border-slate-800 bg-slate-950/45 p-4">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
        <div>
          <p className="console-label">Trajectory Overview</p>
          <h5 className="mt-1 text-base font-bold text-white">
            {primarySignal.symbol} · {trajectoryIntentLabel(whale.intent)}
          </h5>
          <p className="mt-1 max-w-3xl text-xs leading-5 text-slate-400">
            {whale.conclusion || "轨迹证据不足，保持观察。"}
          </p>
        </div>
        <button
          className="rounded-lg border border-cyan-500/40 px-3 py-2 text-xs font-semibold text-cyan-100 outline-none transition hover:border-cyan-300 hover:bg-cyan-500/10 focus-visible:ring-2 focus-visible:ring-cyan-500/35"
          onClick={() => onOpenSignal(primarySignal.id)}
          type="button"
        >
          查看代表信号
        </button>
      </div>

      <div className="mt-4 grid gap-2 text-xs md:grid-cols-2 xl:grid-cols-4">
        <MiniInfoCard label="Dominant Intent" value={trajectoryIntentLabel(whale.intent)} detail={clusterIntentLabel(whale.clusterIntent)} />
        <MiniInfoCard label="Regime Path" value={regimePathLabel(whale.regimePath)} detail="phase path" />
        <MiniInfoCard label="Persistence" value={formatPct(whale.persistenceScore * 100)} detail={`stability ${formatPct(whale.regimeStability * 100)}`} />
        <MiniInfoCard label="Duration" value={formatMsDuration(whale.durationMs)} detail={`${whale.signalCount} signals merged`} />
      </div>

      <TrajectoryTimeline phases={whale.phases} />

      <div className="mt-4 grid gap-3 lg:grid-cols-2">
        <PhaseBreakdown phases={whale.phases} />
        <div className="grid gap-3">
          <CurvePanel label="Stealth Curve (gamma)" points={whale.stealthCurve} tone="cyan" />
          <CurvePanel label="Hazard Curve (lambda proxy)" points={whale.hazardCurve} tone="amber" />
        </div>
      </div>

      <details className="mt-4 rounded-xl border border-slate-800 bg-slate-950/45 px-3 py-2 text-xs text-slate-400">
        <summary className="cursor-pointer select-none text-slate-300 outline-none transition hover:text-cyan-200 focus-visible:ring-2 focus-visible:ring-cyan-500/35">
          Signals collapsed debug · {whale.signalCount} 条
        </summary>
        <div className="mt-3 grid gap-2 md:grid-cols-2 xl:grid-cols-3">
          {whale.signals.map((signal) => (
            <button
              className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2 text-left outline-none transition hover:border-cyan-500/40 hover:bg-cyan-500/5 focus-visible:ring-2 focus-visible:ring-cyan-500/35"
              key={signal.id}
              onClick={() => onOpenSignal(signal.id)}
              type="button"
            >
              <p className="font-semibold text-slate-100">{formatTime(signal.ts)} · {signalTypeLabel(signal.signalType)}</p>
              <p className="mt-1 text-slate-400">
                {formatBaseVolume(signal.totalVolumeBtc, signal.symbol)} · {netDirection(signal.netVolumeBtc, signal.symbol)}
              </p>
            </button>
          ))}
        </div>
      </details>
    </article>
  );
}

function TrajectoryTimeline({ phases }) {
  return (
    <section className="mt-4">
      <p className="console-label">Trajectory Timeline</p>
      <div className="mt-3 grid gap-2 md:grid-cols-3">
        {phases.map((phase, index) => (
          <div className={`rounded-xl border px-3 py-3 ${phaseToneClass(phase.type)}`} key={`${phase.type}-${index}`}>
            <div className="flex items-center justify-between gap-2">
              <p className="text-xs font-bold text-slate-100">{index + 1}. {phaseLabel(phase.type)}</p>
              <span className="text-[11px] text-slate-400">{formatTime(phase.ts)}</span>
            </div>
            <div className="mt-3 h-2 overflow-hidden rounded-full bg-slate-800">
              <div
                className={phaseBarClass(phase.type)}
                style={{ width: `${Math.max(8, Math.min(100, phase.intensity * 100))}%` }}
              />
            </div>
            <p className="mt-2 text-[11px] leading-5 text-slate-400">{phase.detail}</p>
          </div>
        ))}
      </div>
    </section>
  );
}

function PhaseBreakdown({ phases }) {
  return (
    <section className="rounded-xl border border-slate-800 bg-slate-950/45 p-3">
      <p className="console-label">Phase Breakdown</p>
      <div className="mt-3 space-y-2">
        {phases.map((phase, index) => (
          <div className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2" key={`${phase.type}-breakdown-${index}`}>
            <div className="flex items-center justify-between gap-3">
              <p className="text-xs font-semibold text-slate-100">{phaseLabel(phase.type)}</p>
              <span className="text-[11px] text-cyan-100">{formatPct(phase.intensity * 100)}</span>
            </div>
            <p className="mt-1 text-[11px] leading-5 text-slate-400">{phase.detail}</p>
          </div>
        ))}
      </div>
    </section>
  );
}

function CurvePanel({ label, points, tone }) {
  return (
    <section className="rounded-xl border border-slate-800 bg-slate-950/45 p-3">
      <div className="flex items-center justify-between gap-3">
        <p className="console-label">{label}</p>
        <span className="text-xs font-semibold text-slate-100">{formatPct(Math.max(...points, 0) * 100)}</span>
      </div>
      <div className="mt-3 flex h-16 items-end gap-1">
        {points.map((point, index) => (
          <span
            className={`flex-1 rounded-t ${curveBarClass(tone)}`}
            key={`${label}-${index}`}
            style={{ height: `${Math.max(8, Math.min(100, point * 100))}%` }}
            title={formatPct(point * 100)}
          />
        ))}
      </div>
    </section>
  );
}

function RawSignalDebugSection({ enabled, items, loading, onOpenSignal }) {
  return (
    <section className="mt-4 rounded-2xl border border-cyan-500/20 bg-slate-950/35">
      <div className="flex flex-col gap-2 px-4 py-3 md:flex-row md:items-end md:justify-between">
        <div>
          <p className="console-label text-cyan-300">Contract Signal Feed</p>
          <h4 className="mt-1 text-base font-bold text-white">逐条合约信号</h4>
          <p className="mt-1 text-xs leading-5 text-slate-400">
            每一次 CWM 检测到的合约信号都会在这里展示；下方主力行为轨迹只是辅助聚合，不会替代原始信号列表。
          </p>
        </div>
        <span className="rounded-full border border-cyan-500/30 px-3 py-1 text-xs font-semibold text-cyan-100">
          {items.length} rows
        </span>
      </div>
      <div className="overflow-x-auto border-t border-slate-800">
        {loading ? (
          <p className="px-4 py-5 text-sm text-slate-400">主力合约监控载入中...</p>
        ) : items.length === 0 ? (
          <p className="px-4 py-5 text-sm text-slate-400">{enabled ? "暂无主力合约异动" : "主力合约监控未启用"}</p>
        ) : (
          <RawSignalDebugTable items={items} onOpenSignal={onOpenSignal} />
        )}
      </div>
    </section>
  );
}

function RawSignalDebugTable({ items, onOpenSignal }) {
  return (
    <table className="min-w-full table-fixed text-left text-xs" data-testid="raw-contract-whale-signals">
      <thead className="bg-slate-950/80 text-slate-400">
        <tr>
          <HeaderCell>时间</HeaderCell>
          <HeaderCell>币种 / 价格</HeaderCell>
          <HeaderCell>类型</HeaderCell>
          <HeaderCell>等级</HeaderCell>
          <HeaderCell>窗口</HeaderCell>
          <HeaderCell>成交量</HeaderCell>
          <HeaderCell>名义金额</HeaderCell>
          <HeaderCell>价格</HeaderCell>
          <HeaderCell>价格偏离</HeaderCell>
          <HeaderCell>主力评分</HeaderCell>
          <HeaderCell>轨迹</HeaderCell>
          <HeaderCell>现货 / 合约</HeaderCell>
          <HeaderCell>净方向</HeaderCell>
          <HeaderCell>方向占比</HeaderCell>
          <HeaderCell>异常倍数</HeaderCell>
          <HeaderCell>历史分位</HeaderCell>
          <HeaderCell>主导平台</HeaderCell>
          <HeaderCell>价格变化</HeaderCell>
          <HeaderCell>清算</HeaderCell>
          <HeaderCell>OI</HeaderCell>
          <HeaderCell>资金费率</HeaderCell>
          <HeaderCell>Discord</HeaderCell>
          <HeaderCell>详情</HeaderCell>
        </tr>
      </thead>
      <tbody className="divide-y divide-slate-800 text-slate-300">
        {items.map((item) => (
          <tr
            className="console-row"
            data-testid={`contract-whale-row-${item.id}`}
            key={item.id}
            onClick={() => onOpenSignal(item.id)}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onOpenSignal(item.id);
              }
            }}
            tabIndex={0}
          >
            <Cell>{formatTime(item.ts)}</Cell>
            <Cell>
              <SymbolWithPrice item={item} />
            </Cell>
            <Cell>
              <span className="inline-flex items-center gap-1">
                <span className={signalTypeIconClass(item.signalType)} aria-hidden="true">
                  {signalTypeIcon(item.signalType)}
                </span>
                {signalTypeLabel(item.signalType)}
              </span>
            </Cell>
            <Cell>
              <span className={`rounded-full px-2 py-1 font-bold ${severityBadgeClass(item.severity)}`}>
                {severityLabel(item.severity)}
              </span>
            </Cell>
            <Cell>{item.windowSec}s</Cell>
            <Cell>{formatBaseVolume(item.totalVolumeBtc, item.symbol)}</Cell>
            <Cell>{formatUsd(item.totalNotionalUsd)}</Cell>
            <Cell>{formatPrice(signalTriggerPrice(item))}</Cell>
            <Cell>{formatDeviation(item.priceDeviationPct)}</Cell>
            <Cell>{formatScore(item.mainForceScore ?? item.score)}</Cell>
            <Cell>{clusterTableLabel(item)}</Cell>
            <Cell>{formatScorePair(item.spotScore, item.contractScore)}</Cell>
            <Cell>{netDirection(item.netVolumeBtc, item.symbol)}</Cell>
            <Cell>{formatPct(item.dominance * 100)}</Cell>
            <Cell>{formatMultiple(item.dynamicMultiple)}</Cell>
            <Cell>{formatPercentile(item.percentileLevel)}</Cell>
            <Cell>{item.mainExchange}</Cell>
            <Cell>{formatSignedPct(item.priceMovePct)}</Cell>
            <Cell>{liquidationStatus(item)}</Cell>
            <Cell>{oiStatus(item)}</Cell>
            <Cell>{fundingStatus(item)}</Cell>
            <Cell>{discordStatus(item)}</Cell>
            <Cell>
              <button
                aria-label={`查看主力合约信号详情 ${item.id}`}
                className="rounded-lg border border-cyan-500/40 px-2 py-1 text-cyan-100 outline-none transition hover:border-cyan-300 hover:bg-cyan-500/10 focus-visible:ring-2 focus-visible:ring-cyan-500/35"
                onClick={(event) => {
                  event.stopPropagation();
                  onOpenSignal(item.id);
                }}
                type="button"
              >
                详情
              </button>
            </Cell>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

function MainForceEventsSection({ events, symbol }) {
  return (
    <section className="mt-5">
      <div className="mb-3 flex items-end justify-between gap-3">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-slate-500">Main Force Events</p>
          <h4 className="mt-1 text-sm font-bold text-white">主力结构事件历史</h4>
        </div>
        <p className="text-xs text-slate-500">让你知道这里发生过什么主力行为</p>
      </div>
      {events.length === 0 ? (
        <p className="rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-4 text-sm text-slate-400">
          暂无 {symbol} 主力结构事件
        </p>
      ) : (
        <div className="grid gap-3 xl:grid-cols-2">
          {events.map((event) => (
            <article
              className="rounded-xl border border-slate-800 bg-slate-950/50 p-4"
              data-testid={`main-force-event-${event.id}`}
              key={event.id}
            >
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div>
                  <p className="text-sm font-bold text-slate-100">
                    {regimeTypeLabel(event.regimeType)}
                  </p>
                  <p className="mt-1 text-xs text-slate-500">
                    {formatEventRange(event.startedAt, event.endedAt)}
                  </p>
                </div>
                <span className={`rounded-full px-2 py-1 text-xs font-bold ${marketSeverityBadgeClass(event.severity)}`}>
                  {event.severity}
                </span>
              </div>
              <div className="mt-3 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
                <EventMetric label="峰值主力评分" value={`${Math.round(event.peakMainForceScore)}`} />
                <EventMetric label="峰值极端冲击" value={`${Math.round(event.peakExtremeImpactScore)}`} />
                <EventMetric label="峰值结构方向" value={`${biasText(event.peakStructureBias)}`} />
                <EventMetric label="置信度" value={`${Math.round(event.confidence)}`} />
              </div>
              <div className="mt-3 flex flex-wrap gap-2">
                {event.mainForceConfirmed ? <EventTag label="主力确认" tone="emerald" /> : null}
                {event.extremeImpactConfirmed ? <EventTag label="极端冲击" tone="amber" /> : null}
                <EventTag label={event.liquidationDriven ? "清算驱动" : "非清算驱动"} tone={event.liquidationDriven ? "red" : "cyan"} />
                {event.endedAt ? <EventTag label="已结束" tone="slate" /> : <EventTag label="进行中" tone="emerald" />}
              </div>
              <p className="mt-3 text-sm text-slate-300">
                {event.reasons.finalResult || event.reasons.coreReason || "主力结构事件已记录，可用于后续复盘。"}
              </p>
            </article>
          ))}
        </div>
      )}
    </section>
  );
}

function ContractWhaleDetailModal({ signal, relatedSignals, summary, onClose }) {
  const windowRows = [5, 15, 60].map((windowSec) => {
    const match = relatedSignals.find(
      (item) =>
        item.symbol === signal.symbol &&
        item.signalType === signal.signalType &&
        item.direction === signal.direction &&
        item.windowSec === windowSec,
    );
    return match || (signal.windowSec === windowSec ? signal : null);
  });
  const scoringRows = scoringBreakdown(signal);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 px-4 py-6">
      <div
        aria-label="主力合约信号详情"
        aria-modal="true"
        className="max-h-[90vh] w-full max-w-5xl overflow-y-auto rounded-2xl border border-cyan-500/30 bg-slate-950 p-5 shadow-glow"
        role="dialog"
      >
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="text-xs uppercase tracking-[0.26em] text-cyan-300">Contract Whale Detail</p>
            <h3 className="mt-2 text-lg font-bold text-white">{signal.symbol} 主力合约信号详情</h3>
            <p className="mt-1 text-sm text-slate-400">{signal.finalResult}</p>
          </div>
          <button
            aria-label="关闭主力合约信号详情"
            className="rounded-lg border border-slate-700 px-3 py-2 text-sm text-slate-200 transition hover:border-cyan-400 hover:text-cyan-100"
            onClick={onClose}
            type="button"
          >
            关闭
          </button>
        </div>

        <div className="mt-5 grid gap-4 lg:grid-cols-2">
          <DetailSection title="基础信息">
            <DetailGrid
              rows={[
                ["Symbol", signal.symbol],
                ["类型", signalTypeLabel(signal.signalType)],
                ["方向", directionLabel(signal.direction)],
                ["价格响应", priceResponseLabel(signal.priceResponseType)],
                ["等级", severityLabel(signal.severity)],
                ["窗口", `${signal.windowSec}s`],
                ["触发时间", formatTime(signal.ts)],
                ["触发价格", formatPrice(signalTriggerPrice(signal))],
                ["信号价格", formatPrice(signal.orderPriceUsd ?? signalTriggerPrice(signal))],
                ["当前价格", formatPrice(signal.currentMarketPriceUsd)],
                ["价格偏离", formatDeviation(signal.priceDeviationPct)],
                ["偏离过滤", signal.priceDeviationFiltered ? "已过滤" : `未过滤（阈值 ${CWM_MAX_PRICE_DEVIATION_PCT}%）`],
                ["Market Type", marketLabel(signal.marketType)],
                ["Source Role", sourceRoleLabel(signal.sourceRole)],
                ["Risk Score", `${signal.score}/100`],
                ["Main Force Score", formatScore(signal.mainForceScore)],
                ["Spot Score", formatScore(signal.spotScore)],
                ["Contract Score", formatScore(signal.contractScore)],
                ["Data Quality", `${signal.dataQuality}/100`],
                ["Threshold Profile", thresholdProfileLabel(signal.thresholdProfile || summary?.thresholdProfile)],
                ["Profile Reason", signal.thresholdProfileReason || signal.activeSources?.thresholdProfileReason || summary?.thresholdProfileReason || "N/A"],
                ["Configured Sources", sourceListLabel(signal.configuredContractSources || signal.activeSources?.configuredContractSources || summary?.configuredContractSources)],
                ["Eligible Sources", sourceListLabel(signal.eligibleContractSources || signal.activeSources?.eligibleContractSources || summary?.eligibleContractSources)],
                ["Active Sources", sourceListLabel(signal.activeContractSources || signal.activeSources?.activeContractSources || summary?.activeContractExchanges)],
              ]}
            />
          </DetailSection>

          <DetailSection title="Discord Gate">
            <DetailGrid
              rows={[
                ["Gate Result", signal.discordEligible ? "可进入推送判断" : "仅展示"],
                ["Would Send", signal.discordWouldSend ? "dry-run 会推送" : "不会推送"],
                ["Discord Sent", signal.discordSent ? "已推送" : "未推送"],
                ["Skip Reason", signal.discordSent ? "sent" : signal.discordReason],
                ["多平台确认", signal.multiExchangeConfirmed ? "是" : "否"],
                ["疑似强平", signal.liquidationSuspected ? "是" : "否"],
                ["合并来源", signal.mergedFrom?.length ? signal.mergedFrom.join(", ") : "无"],
              ]}
            />
          </DetailSection>
        </div>

        <DetailSection title="核心判断" className="mt-4">
          <div className="rounded-xl border border-cyan-500/20 bg-cyan-500/5 p-3 text-sm leading-6 text-cyan-50">
            <p className="font-semibold text-slate-100">{signal.finalResult}</p>
            <p className="mt-1 text-xs text-cyan-100">{priceResponseNarrative(signal)}</p>
            {signal.cluster?.signalCount > 1 ? (
              <p className="mt-1 text-xs text-cyan-100">
                {clusterTrajectoryNarrative(signal)}
              </p>
            ) : null}
          </div>
        </DetailSection>

        <DetailSection title="Signal Cluster / Persistence" className="mt-4">
          <DetailGrid
            rows={[
              ["Cluster ID", signal.cluster?.clusterId || "N/A"],
              ["Dominant Intent", clusterIntentLabel(signal.cluster?.dominantIntent)],
              ["Cluster Signals", `${signal.cluster?.signalCount || 1}`],
              ["Cluster Duration", formatMsDuration(signal.cluster?.durationMs)],
              ["Cluster Intensity", formatPct(Number(signal.cluster?.intensity || 0) * 100)],
              ["Price Range", formatOptionalPct(signal.cluster?.priceRangePct)],
              ["Persistence Score", formatPct(Number(signal.persistence?.persistenceScore || 0) * 100)],
              ["Half Life", formatMsDuration(signal.persistence?.signalHalfLifeMs)],
              ["Regime Stability", formatPct(Number(signal.persistence?.regimeStability || 0) * 100)],
              ["Redundant Projection", signal.persistence?.redundantWithPrevious ? repetitionReasonLabel(signal.persistence?.redundantReason) : "否"],
            ]}
          />
          <p className="mt-2 rounded-xl border border-cyan-500/20 bg-cyan-500/5 px-3 py-2 text-xs leading-6 text-cyan-100">
            Cluster 表示同 symbol、同方向、120 秒内且价格区间小于 0.3% 的连续信号；它更像同一主力意图轨迹，不等同于多个独立机会。
          </p>
        </DetailSection>

        <DetailSection title="Whale Trajectory" className="mt-4">
          <DetailGrid
            rows={[
              ["Trajectory ID", signal.trajectory?.trajectoryId || "N/A"],
              ["Intent", trajectoryIntentLabel(signal.trajectory?.intent)],
              ["Duration", formatMsDuration(signal.trajectory?.durationMs)],
              ["Regime Path", regimePathLabel(signal.trajectory?.regimePath)],
              ["Stealth Gamma", formatPct(Number(signal.trajectory?.stealthProfile?.gamma || 0) * 100)],
              ["Fragmentation", formatPct(Number(signal.trajectory?.stealthProfile?.fragmentation || 0) * 100)],
              ["Entropy", formatPct(Number(signal.trajectory?.stealthProfile?.entropy || 0) * 100)],
              ["Cross Exchange", formatPct(Number(signal.trajectory?.stealthProfile?.crossExchangeDispersion || 0) * 100)],
            ]}
          />
          <div className="mt-2 rounded-xl border border-slate-800 bg-slate-900/60 p-3">
            <p className="text-xs leading-6 text-cyan-100">
              {signal.trajectory?.conclusion || "轨迹证据不足，保持观察。"}
            </p>
            <div className="mt-3 grid gap-2 md:grid-cols-2">
              {(signal.trajectory?.actions || []).map((action, index) => (
                <div className="rounded-lg border border-slate-800 bg-slate-950/50 p-2 text-xs text-slate-300" key={`${action.ts}-${index}`}>
                  <p className="font-semibold text-slate-100">
                    {index + 1}. {actionTypeLabel(action.actionType)}
                  </p>
                  <p className="mt-1">
                    {formatTime(action.ts)} · {exchangeLabel(action.exchange)} · {formatBaseVolume(action.volume, action.symbol || signal.symbol)}
                  </p>
                  <p className="mt-1 text-slate-500">价格冲击 {formatSignedPct(action.priceImpact)}</p>
                </div>
              ))}
            </div>
          </div>
        </DetailSection>

        <DetailSection title="现货确认" className="mt-4">
          <DetailGrid
            rows={[
              ["状态", spotConfirmationStatusLabel(signal.spotConfirmation?.status)],
              ["确认类型", spotConfirmationTypeLabel(signal.spotConfirmation?.confirmationType)],
              ["现货方向", directionLabel(signal.spotConfirmation?.direction)],
              ["现货评分", `${Number(signal.spotConfirmation?.score || 0)}/100`],
              ["现货类型", signal.spotConfirmation?.signalType ? spotSignalTypeLabel(signal.spotConfirmation.signalType) : "N/A"],
              ["现货等级", signal.spotConfirmation?.severity ? severityLabel(signal.spotConfirmation.severity) : "N/A"],
              ["现货成交量", signal.spotConfirmation?.totalVolumeBtc === null || signal.spotConfirmation?.totalVolumeBtc === undefined ? "N/A" : formatBaseVolume(signal.spotConfirmation.totalVolumeBtc, signal.symbol)],
              ["现货净方向", signal.spotConfirmation?.netVolumeBtc === null || signal.spotConfirmation?.netVolumeBtc === undefined ? "N/A" : netDirection(signal.spotConfirmation.netVolumeBtc, signal.symbol)],
              ["Coinbase 溢价", signal.spotConfirmation?.coinbasePremiumPct === null || signal.spotConfirmation?.coinbasePremiumPct === undefined ? "N/A" : formatSignedPct(signal.spotConfirmation.coinbasePremiumPct)],
              ["现货结论", signal.spotConfirmation?.finalResult || "N/A"],
            ]}
          />
        </DetailSection>

        <DetailSection title="Active Source Snapshot" className="mt-4">
          <div className="grid gap-4 lg:grid-cols-2">
            <SourceSnapshotCard entries={signal.activeSources?.contract} title="合约源" />
            <SourceSnapshotCard entries={signal.activeSources?.spot} title="现货源" />
          </div>
        </DetailSection>

        <DetailSection title="5s / 15s / 60s 窗口数据" className="mt-4">
          <div className="grid gap-2 md:grid-cols-3">
            {[5, 15, 60].map((windowSec, index) => {
              const item = windowRows[index];
              return (
                <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-3" key={windowSec}>
                  <p className="font-bold text-slate-100">{windowSec}s</p>
                  {item ? (
                    <div className="mt-2 space-y-1 text-xs text-slate-300">
                      <p>成交量：{formatBaseVolume(item.totalVolumeBtc, signal.symbol)}</p>
                      <p>名义金额：{formatUsd(item.totalNotionalUsd)}</p>
                      <p>价格：{formatPrice(signalTriggerPrice(item))}</p>
                      <p>净方向：{netDirection(item.netVolumeBtc, signal.symbol)}</p>
                      <p>价格变化：{formatSignedPct(item.priceMovePct)}</p>
                      <p>异常倍数：{formatMultiple(item.dynamicMultiple)}</p>
                    </div>
                  ) : (
                    <p className="mt-2 text-xs text-slate-500">未触发或已被代表信号合并</p>
                  )}
                </div>
              );
            })}
          </div>
        </DetailSection>

        <DetailSection title="平台拆分" className="mt-4">
          <div className="grid gap-2 md:grid-cols-3">
            {signal.exchanges.length ? (
              signal.exchanges.map((exchange) => (
                <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-3" key={exchange.exchange}>
                  <p className="font-bold text-slate-100">{exchangeLabel(exchange.exchange)}</p>
                  <div className="mt-2 space-y-1 text-xs text-slate-300">
                    <p>主动买入：{formatBaseVolume(exchange.buyVolumeBtc, signal.symbol)}</p>
                    <p>主动卖出：{formatBaseVolume(exchange.sellVolumeBtc, signal.symbol)}</p>
                    <p>总量：{formatBaseVolume(exchange.totalVolumeBtc, signal.symbol)}</p>
                    <p>买/卖占比：{formatPct(Number(exchange.buyShare || 0) * 100)} / {formatPct(Number(exchange.sellShare || 0) * 100)}</p>
                    <p>净方向：{netDirection(exchange.netVolumeBtc, signal.symbol)}</p>
                    <p>方向强度：{formatPct(exchange.dominance * 100)}</p>
                    <p>净流贡献：{formatPct(Number(exchange.netContributionShare || 0) * 100)}</p>
                  </div>
                </div>
              ))
            ) : (
              <p className="text-sm text-slate-500">暂无平台拆分</p>
            )}
          </div>
        </DetailSection>

        <div className="mt-4 grid gap-4 lg:grid-cols-2">
          <DetailSection title="上下文指标">
            <DetailGrid
              rows={[
                ["Dynamic Multiple", formatMultiple(signal.dynamicMultiple)],
                ["Dynamic Baseline", signal.dynamicBaselineBtc === null || signal.dynamicBaselineBtc === undefined ? "N/A" : formatBaseVolume(signal.dynamicBaselineBtc, signal.symbol)],
                ["Dynamic Level", dynamicThresholdLevelLabel(signal.dynamicThresholdLevel)],
                ["Percentile", formatPercentile(signal.percentileLevel)],
                ["Price Move", formatSignedPct(signal.priceMovePct)],
                ["5s Price Move", formatSignedPct(signal.priceMove5sPct)],
                ["15s Price Move", formatSignedPct(signal.priceMove15sPct)],
                ["30s Price Move", formatSignedPct(signal.priceMove30sPct)],
                ["Price Response", priceResponseLabel(signal.priceResponseType)],
                ["Price Reversal", signal.priceReversalRatio === null || signal.priceReversalRatio === undefined ? "N/A" : formatPct(signal.priceReversalRatio * 100)],
                ["Dominant Net Flow", formatPct(dominantNetFlowShare(signal) * 100)],
                ["Liquidation", liquidationStatus(signal)],
                ["OI", oiStatus(signal)],
                ["Funding", fundingStatus(signal)],
              ]}
            />
          </DetailSection>

          <DetailSection title="Score Breakdown">
            <DetailGrid rows={scoringRows} />
          </DetailSection>
        </div>

        <DetailSection title="口径说明" className="mt-4">
          <div className="rounded-xl border border-cyan-500/20 bg-cyan-500/5 p-3 text-xs leading-6 text-cyan-50">
            <p>方向强度 = abs(主动买入 - 主动卖出) / 总成交量，表示本轮信号整体方向是否集中。</p>
            <p>买入/卖出占比 = 单个平台内部的主动买卖比例，只说明该平台自己的流向结构。</p>
            <p>净流贡献 = 该平台对本轮信号同方向净流的贡献比例，用来判断主导平台。</p>
          </div>
        </DetailSection>
      </div>
    </div>
  );
}

function DetailSection({ title, children, className = "" }) {
  return (
    <section className={className}>
      <p className="mb-2 text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-500">{title}</p>
      {children}
    </section>
  );
}

function DetailGrid({ rows }) {
  return (
    <div className="grid grid-cols-1 gap-2 rounded-xl border border-slate-800 bg-slate-900/60 p-3 text-sm md:grid-cols-2">
      {rows.map(([label, value]) => (
        <div key={label}>
          <p className="text-[11px] uppercase tracking-[0.12em] text-slate-500">{label}</p>
          <p className="mt-1 break-words font-semibold text-slate-100">{value ?? "N/A"}</p>
        </div>
      ))}
    </div>
  );
}

function ContractWhaleTrendBar({ trend, symbol }) {
  const item = trend || {};
  const baseSymbol = baseAssetSymbol(item.symbol || symbol);
  const total = Number(item.totalVolumeBtc || 0);
  const buyRatio = total > 0 ? clampRatio(item.buyRatio) : 0;
  const sellRatio = total > 0 ? clampRatio(item.sellRatio || (1 - buyRatio)) : 0;
  const netDirectionLabel = netDirection(Number(item.netVolumeBtc || 0), baseSymbol);
  return (
    <div className="console-panel-muted mt-4 px-4 py-3">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <p className="console-label">60s Contract Flow</p>
          <p className="mt-1 text-sm font-semibold text-slate-100">
            Buy {formatPct(buyRatio * 100)} / Sell {formatPct(sellRatio * 100)}
          </p>
        </div>
        <div className="text-xs text-slate-400 md:text-right">
          <p>{netDirectionLabel}</p>
          <p>总量 {formatBaseVolume(total, baseSymbol)} · dominance {formatPct(Number(item.dominance || 0) * 100)}</p>
        </div>
      </div>
      <div className="mt-3 h-2 overflow-hidden rounded-full bg-red-500/20">
        <div
          aria-label="最近 60 秒主动买入占比"
          className="h-full rounded-full bg-emerald-400"
          style={{ width: total > 0 ? `${Math.max(3, buyRatio * 100)}%` : "0%" }}
        />
      </div>
      <div className="mt-2 flex justify-between text-[11px] text-slate-400">
        <span>主动买入 {formatBaseVolume(item.buyVolumeBtc, baseSymbol)}</span>
        <span>主动卖出 {formatBaseVolume(item.sellVolumeBtc, baseSymbol)}</span>
      </div>
      <p className="mt-2 text-[11px] text-slate-500">
        最近 60 秒主动成交流只表示 flow，不用于判断平台在线 / 离线状态。
      </p>
    </div>
  );
}

function MarketStructureLitePanel({ summary }) {
  const lite = summary.marketStructureLite || {};
  const stats = summary.discordDryRunStats || {};
  return (
    <div className="mt-3 grid gap-2 text-xs md:grid-cols-2 xl:grid-cols-4">
      <MiniInfoCard
        label="结构判断"
        value={`${regimeTypeLabel(lite.regimeType || "unclear")} · ${marketStructureStatusLabel(lite.status)}`}
        detail={lite.reason || "等待现货与合约确认"}
      />
      <MiniInfoCard
        label="主力评分"
        value={`${Math.round(Number(lite.mainForceScore || 0))}/100`}
        detail={`方向 ${biasText(lite.structureBias || 0)} · 置信 ${Math.round(Number(lite.confidence || 0))}`}
      />
      <MiniInfoCard
        label="现货确认"
        value={`Spot ${Math.round(Number(lite.spotScore || 0))} / Contract ${Math.round(Number(lite.contractScore || 0))}`}
        detail={`Cross ${Math.round(Number(lite.crossConfirmScore || 0))} · ${lite.mainForceConfirmed ? "已确认" : "待确认"}`}
      />
      <MiniInfoCard
        label="Dry-run 1h"
        value={`would-send ${Number(stats.wouldSend1h || 0)}`}
        detail={`signals ${Number(stats.signals1h || 0)} · C/S ${Number(stats.critical1h || 0)}/${Number(stats.s1h || 0)}`}
      />
    </div>
  );
}

function MiniInfoCard({ label, value, detail }) {
  return (
    <div className="console-panel-muted px-3 py-2">
      <p className="console-label">{label}</p>
      <p className="mt-1 font-bold text-slate-100">{value}</p>
      <p className="mt-1 truncate text-slate-400" title={detail}>{detail}</p>
    </div>
  );
}

function ContractWhaleFilters({ filters, onChange }) {
  const update = (key, value) => onChange({ ...filters, [key]: value });
  return (
    <div className="mt-4 grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-4 xl:grid-cols-7">
      <FilterSelect label="币种" value={filters.symbol} onChange={(value) => update("symbol", value)}>
        <option value="BTC">BTC</option>
        <option value="ETH">ETH</option>
        <option value="SOL">SOL</option>
      </FilterSelect>
      <FilterSelect label="等级" value={filters.severity} onChange={(value) => update("severity", value)}>
        <option value="all">全部</option>
        <option value="s">S</option>
        <option value="critical">Critical</option>
        <option value="high">High</option>
        <option value="medium">Medium</option>
      </FilterSelect>
      <FilterSelect label="类型" value={filters.signal_type} onChange={(value) => update("signal_type", value)}>
        <option value="all">全部</option>
        <option value="aggressive_buy">主力拉盘</option>
        <option value="aggressive_sell">主力砸盘</option>
        <option value="downside_absorption">下方吸收</option>
        <option value="upside_suppression">上方压制</option>
      </FilterSelect>
      <FilterSelect label="方向" value={filters.direction} onChange={(value) => update("direction", value)}>
        <option value="all">全部</option>
        <option value="buy">主动买入</option>
        <option value="sell">主动卖出</option>
        <option value="absorption">吸收</option>
        <option value="suppression">压制</option>
      </FilterSelect>
      <FilterSelect label="Discord" value={filters.discord_sent} onChange={(value) => update("discord_sent", value)}>
        <option value="all">全部</option>
        <option value="true">已推送</option>
        <option value="false">未推送</option>
      </FilterSelect>
      <FilterSelect label="窗口" value={filters.window_sec} onChange={(value) => update("window_sec", value)}>
        <option value="all">全部</option>
        <option value="5">5s</option>
        <option value="15">15s</option>
        <option value="60">60s</option>
      </FilterSelect>
      <FilterSelect label="交易所" value={filters.exchange} onChange={(value) => update("exchange", value)}>
        <option value="all">全部</option>
        <option value="binance">Binance</option>
        <option value="bitfinex">Bitfinex</option>
        <option value="coinbase">Coinbase</option>
      </FilterSelect>
    </div>
  );
}

function FilterSelect({ label, value, onChange, children }) {
  return (
    <label className="block">
      <span className="mb-1 block text-[11px] font-medium text-slate-400">{label}</span>
      <select
        className="console-field font-semibold"
        onChange={(event) => onChange(event.target.value)}
        value={value}
      >
        {children}
      </select>
    </label>
  );
}

function StatusPill({ label, value, tone }) {
  return (
    <div className="rounded-xl border border-slate-700/70 bg-slate-950/45 px-3 py-2">
      <p className="text-[11px] text-slate-400">{label}</p>
      <p className={`mt-1 text-base font-bold ${toneClass(tone)}`}>{value}</p>
    </div>
  );
}

function EventMetric({ label, value }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2">
      <p className="text-[11px] uppercase tracking-[0.12em] text-slate-500">{label}</p>
      <p className="mt-1 font-semibold text-slate-100">{value}</p>
    </div>
  );
}

function EventTag({ label, tone }) {
  return <span className={`rounded-full px-2 py-1 text-[11px] font-semibold ${eventTagClass(tone)}`}>{label}</span>;
}

function PlatformCapabilitySection({ exchanges, platforms, summary }) {
  const contractSources = contractSourceLabels(summary);
  const spotSources = spotSourceLabels(platforms);
  const platformStatuses = compactPlatformStatuses(exchanges, platforms);
  return (
    <section className="mt-4 rounded-xl border border-slate-800 bg-slate-950/35 px-3 py-3" data-testid="platform-status-strip">
      <div className="flex flex-col gap-3 xl:flex-row xl:items-center xl:justify-between">
        <div className="min-w-0">
          <p className="console-label">Platform Status</p>
          <h4 className="mt-1 text-sm font-bold text-white">平台状态</h4>
          <p className="mt-1 truncate text-xs text-slate-400" title={`合约源 ${contractSources.length ? contractSources.join(", ") : "无"} · 现货确认 ${spotSources.length ? spotSources.join(", ") : "无"} · 阈值 ${thresholdProfileLabel(summary?.thresholdProfile)}`}>
            合约源 {contractSources.length ? contractSources.join(", ") : "无"} · 现货确认 {spotSources.length ? spotSources.join(", ") : "无"} · 阈值 {thresholdProfileLabel(summary?.thresholdProfile)}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          {platformStatuses.map((entry) => (
            <PlatformStatusChip entry={entry} key={entry.exchange} />
          ))}
        </div>
      </div>
      <details className="mt-2 text-[11px] text-slate-400">
        <summary className="cursor-pointer select-none text-slate-300 outline-none transition hover:text-cyan-200 focus-visible:ring-2 focus-visible:ring-cyan-500/35">
          平台口径
        </summary>
        <div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-slate-500">
          <span>profile: {summary?.thresholdProfileReason || "N/A"}</span>
          <span>Coinbase 仅现货确认，不参与 CWM 合约成交量、阈值和 Discord gate。</span>
        </div>
      </details>
    </section>
  );
}

function PlatformStatusChip({ entry }) {
  return (
    <span
      className={`inline-flex items-center gap-2 rounded-full border px-3 py-1 text-xs font-semibold ${compactPlatformStatusClass(entry.tone)}`}
      data-testid={`platform-status-chip-${entry.exchange}`}
    >
      <span className={`h-2 w-2 rounded-full ${compactPlatformDotClass(entry.tone)}`} aria-hidden="true" />
      <span>{exchangeLabel(entry.exchange)}</span>
      <span className="text-slate-400">·</span>
      <span>{entry.label}</span>
    </span>
  );
}

function SourceSnapshotCard({ entries, title }) {
  const items = Array.isArray(entries) ? entries : [];
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-950/50 p-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-sm font-semibold text-slate-100">{title}</p>
        <span className="text-xs text-slate-500">{items.length} 个来源</span>
      </div>
      {items.length === 0 ? (
        <p className="mt-3 text-sm text-slate-500">暂无快照</p>
      ) : (
        <div className="mt-3 space-y-2">
          {items.map((entry) => (
            <div className="rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-2" key={`${title}-${entry.exchange}-${entry.marketType}`}>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <p className="text-sm font-medium text-slate-100">
                  {exchangeLabel(entry.exchange)} · {marketLabel(entry.marketType)}
                </p>
                <span className={`text-[11px] font-semibold ${snapshotStatusClass(entry.status)}`}>
                  {snapshotStatusLabel(entry.status)}
                </span>
              </div>
              <p className="mt-1 text-xs text-slate-400">
                {sourceRoleLabel(entry.sourceRole)}{entry.productId ? ` · ${entry.productId}` : ""}
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function HeaderCell({ children }) {
  return <th className="whitespace-nowrap px-3 py-3 text-[11px] font-semibold uppercase tracking-[0.08em]">{children}</th>;
}

function Cell({ children }) {
  return <td className="whitespace-nowrap px-3 py-3">{children}</td>;
}

function SymbolWithPrice({ item }) {
  return (
    <span className="flex min-w-[96px] flex-col leading-tight">
      <span className="font-semibold text-slate-100">{item.symbol}</span>
      <span className="mt-1 text-[11px] font-semibold text-cyan-200">{formatPrice(signalTriggerPrice(item))}</span>
    </span>
  );
}

function signalTypeLabel(type) {
  const labels = {
    aggressive_buy: "主力拉盘",
    aggressive_sell: "主力砸盘",
    downside_absorption: "下方吸收",
    upside_suppression: "上方压制",
  };
  return labels[type] || type || "未知";
}

function priceResponseLabel(type) {
  const labels = {
    trend_follow_up: "买盘推动上涨",
    trend_follow_down: "卖盘推动下跌",
    downside_absorption: "卖出被承接",
    upside_resistance: "买入被压制",
    no_clear_response: "价格响应不明确",
  };
  return labels[String(type || "no_clear_response").toLowerCase()] || "价格响应不明确";
}

function priceResponseNarrative(signal) {
  const move = formatSignedPct(signal.priceMovePct);
  const response = priceResponseLabel(signal.priceResponseType);
  const base = `价格响应：${response}，当前窗口价格变化 ${move}。`;
  const value = String(signal.priceResponseType || "").toLowerCase();
  if (value === "downside_absorption") {
    return `${base} 主动卖出放大但没有有效打穿价格，优先按下方承接观察。`;
  }
  if (value === "upside_resistance") {
    return `${base} 主动买入放大但没有有效推升价格，优先按上方压制观察。`;
  }
  if (value === "trend_follow_up" || value === "trend_follow_down") {
    return `${base} 成交流和价格方向一致，说明短线冲击更直接。`;
  }
  return `${base} 缺少明确价格配合时，只作为成交流异常观察，不单独确认趋势。`;
}

function regimeTypeLabel(value) {
  const labels = {
    main_force_long_build: "主力建多",
    main_force_short_build: "主力建空",
    contract_flow_shock: "合约冲击",
    spot_accumulation: "现货吸筹",
    spot_distribution: "现货派发",
    contract_short_squeeze: "空头挤压",
    long_liquidation_cascade: "多头清算瀑布",
    downside_absorption: "下方吸收",
    upside_resistance: "上方压制",
    range_rotation: "高换手震荡",
  };
  return labels[value] || value || "结构未明";
}

function marketStructureStatusLabel(value) {
  const status = String(value || "calm").toLowerCase();
  if (status === "confirmed") return "已确认";
  if (status === "watch") return "观察";
  return "平静";
}

function dynamicThresholdLevelLabel(value) {
  const level = String(value || "normal").toLowerCase();
  if (level === "s") return "S 级动态异常";
  if (level === "critical") return "Critical 动态异常";
  if (level === "high") return "High 动态异常";
  if (level === "watch") return "Watch 动态异常";
  return "正常";
}

function spotConfirmationStatusLabel(value) {
  const status = String(value || "unavailable").toLowerCase();
  if (status === "confirmed") return "现货确认";
  if (status === "divergent") return "现货分歧";
  if (status === "context") return "仅作上下文";
  if (status === "disabled") return "现货监控未启用";
  if (status === "no_spot_sample") return "暂无现货样本";
  return "不可用";
}

function spotConfirmationTypeLabel(value) {
  const type = String(value || "unavailable").toLowerCase();
  const labels = {
    confirms_contract_direction: "现货与合约同向",
    spot_absorption_against_contract_sell: "合约卖压被现货承接",
    spot_resistance_against_contract_buy: "合约买盘遇现货压制",
    spot_divergence: "现货与合约分歧",
    spot_context_only: "现货上下文",
    spot_monitor_disabled: "现货监控未启用",
    unavailable: "不可用",
  };
  return labels[type] || labels.unavailable;
}

function spotSignalTypeLabel(type) {
  const labels = {
    spot_aggressive_buy: "现货主动买入",
    spot_aggressive_sell: "现货主动卖出",
    spot_downside_absorption: "现货下方吸收",
    spot_upside_suppression: "现货上方压制",
    spot_exchange_dislocation: "现货跨所错位",
  };
  return labels[String(type || "").toLowerCase()] || type || "N/A";
}

function signalTypeIcon(type) {
  const icons = {
    aggressive_buy: "▲",
    aggressive_sell: "▼",
    downside_absorption: "▣",
    upside_suppression: "⊣",
  };
  return icons[type] || "•";
}

function signalTypeIconClass(type) {
  const value = String(type || "").toLowerCase();
  if (value === "aggressive_buy") return "text-emerald-300";
  if (value === "aggressive_sell") return "text-red-300";
  if (value === "downside_absorption") return "text-cyan-300";
  if (value === "upside_suppression") return "text-yellow-300";
  return "text-slate-400";
}

function severityLabel(severity) {
  const value = String(severity || "calm").toLowerCase();
  if (value === "s") return "S";
  if (value === "critical") return "Critical";
  if (value === "high") return "High";
  if (value === "medium") return "Medium";
  return "平静";
}

function healthStatusLabel(status) {
  const value = String(status || "disabled").toLowerCase();
  if (value === "healthy") return "健康";
  if (value === "degraded") return "降级";
  if (value === "unhealthy") return "异常";
  if (value === "warming_up") return "预热";
  return "未启用";
}

function healthStatusTone(status) {
  const value = String(status || "disabled").toLowerCase();
  if (value === "healthy") return "cyan";
  if (value === "degraded" || value === "warming_up") return "yellow";
  if (value === "unhealthy") return "red";
  return "slate";
}

function marketSeverityBadgeClass(severity) {
  const value = String(severity || "").toLowerCase();
  if (value === "extreme") return "border border-fuchsia-500/40 bg-fuchsia-500/15 text-fuchsia-200";
  if (value === "major") return "border border-red-500/40 bg-red-500/15 text-red-200";
  if (value === "confirmed") return "border border-amber-500/40 bg-amber-500/15 text-amber-200";
  return "border border-slate-700/80 bg-slate-900/70 text-slate-300";
}

function eventTagClass(tone) {
  if (tone === "emerald") return "border border-emerald-500/40 bg-emerald-500/15 text-emerald-200";
  if (tone === "amber") return "border border-amber-500/40 bg-amber-500/15 text-amber-200";
  if (tone === "red") return "border border-red-500/40 bg-red-500/15 text-red-200";
  if (tone === "cyan") return "border border-cyan-500/40 bg-cyan-500/15 text-cyan-200";
  return "border border-slate-700/80 bg-slate-900/70 text-slate-300";
}

function directionLabel(direction) {
  const value = String(direction || "neutral").toLowerCase();
  if (value.includes("disabled")) return "未启用";
  if (value.includes("buy")) return "多";
  if (value.includes("sell")) return "空";
  if (value.includes("absorption")) return "吸收";
  if (value.includes("suppression")) return "压制";
  return "平静";
}

function shouldUseHistory(filters) {
  return ["severity", "signal_type", "direction", "discord_sent", "window_sec", "exchange"].some(
    (key) => filters[key] && filters[key] !== "all",
  );
}

function modeLabel(summary) {
  if (!summary.enabled) return "未启用";
  return summary.dryRun ? "Dry-run" : "实时提醒";
}

function thresholdProfileLabel(profile) {
  const value = String(profile || "").toLowerCase();
  if (value === "no_contract_sources") return "无合约源";
  if (value === "binance_bitfinex") return "Binance+Bitfinex";
  if (value === "binance_bitfinex_coinbase") return "Binance+Bitfinex+Coinbase";
  if (value === "three_exchange") return "三平台";
  return "默认";
}

function exchangeLabel(exchange) {
  const labels = {
    binance: "Binance",
    okx: "OKX",
    bitfinex: "Bitfinex",
    coinbase: "Coinbase",
  };
  return labels[exchange] || exchange;
}

function contractSourceLabels(summary) {
  const sources = Array.isArray(summary?.activeContractSources) && summary.activeContractSources.length
    ? summary.activeContractSources
    : Array.isArray(summary?.activeContractExchanges) && summary.activeContractExchanges.length
      ? summary.activeContractExchanges
      : Array.isArray(summary?.eligibleContractSources)
        ? summary.eligibleContractSources
        : [];
  return sources.map((exchange) => `${exchangeLabel(exchange)} Perp`);
}

function spotSourceLabels(platforms) {
  const source = platforms && typeof platforms === "object" ? platforms : {};
  return ["coinbase", "binance", "bitfinex", "okx"]
    .filter((exchange) => {
      const spot = source[exchange]?.markets?.spot;
      return Boolean(spot?.enabled);
    })
    .map((exchange) => `${exchangeLabel(exchange)} Spot`);
}

function compactPlatformStatuses(exchanges, platforms) {
  const platformSource = platforms && typeof platforms === "object" ? platforms : {};
  return ["binance", "bitfinex", "coinbase", "okx"].map((exchange) => {
    const platform = platformSource[exchange] || { platformEnabled: false, status: "disabled", markets: {} };
    const runtime = exchanges?.[exchange] || {};
    return {
      exchange,
      ...compactPlatformStatus(platform, runtime),
    };
  });
}

function compactPlatformStatus(platform, runtime) {
  const platformStatus = String(platform?.status || runtime?.status || "disabled").toLowerCase();
  const runtimeStatus = String(runtime?.status || "").toLowerCase();
  const platformEnabled = Boolean(platform?.platformEnabled ?? platform?.enabled);
  const connected = Boolean(runtime?.connected) || runtimeStatus === "connected";
  if (!platformEnabled || platformStatus === "disabled") {
    return { label: "未启用", tone: "slate" };
  }
  if (platformStatus === "spot_only") {
    return { label: "仅现货", tone: "cyan" };
  }
  if (runtimeStatus === "reconnecting" || platformStatus === "reconnecting") {
    return { label: "重连中", tone: "yellow" };
  }
  if (connected) {
    return { label: "运行中", tone: "emerald" };
  }
  if (runtimeStatus === "disconnected" || platformStatus === "disconnected") {
    return { label: "离线", tone: "red" };
  }
  if (platformStatus === "active" || platformStatus === "enabled") {
    return { label: "已启用", tone: "cyan" };
  }
  return { label: "等待数据", tone: "cyan" };
}

function compactPlatformStatusClass(tone) {
  if (tone === "emerald") return "border-emerald-500/40 bg-emerald-500/10 text-emerald-200";
  if (tone === "cyan") return "border-cyan-500/40 bg-cyan-500/10 text-cyan-200";
  if (tone === "yellow") return "border-yellow-500/40 bg-yellow-500/10 text-yellow-100";
  if (tone === "red") return "border-red-500/40 bg-red-500/10 text-red-100";
  return "border-slate-700/80 bg-slate-900/70 text-slate-300";
}

function compactPlatformDotClass(tone) {
  if (tone === "emerald") return "bg-emerald-300";
  if (tone === "cyan") return "bg-cyan-300";
  if (tone === "yellow") return "bg-yellow-300";
  if (tone === "red") return "bg-red-300";
  return "bg-slate-500";
}

function snapshotStatusLabel(status) {
  const value = String(status || "configured").toLowerCase();
  if (value === "active") return "已参与";
  if (value === "spot_only") return "仅现货";
  if (value === "configured") return "已配置";
  if (value === "disabled") return "未启用";
  return value;
}

function snapshotStatusClass(status) {
  const value = String(status || "configured").toLowerCase();
  if (value === "active") return "text-emerald-300";
  if (value === "spot_only") return "text-cyan-300";
  if (value === "disabled") return "text-slate-500";
  return "text-yellow-300";
}

function sourceRoleLabel(role) {
  const value = String(role || "disabled").toLowerCase();
  if (value === "primary" || value === "primary_liquidity") return "主流动性源";
  if (value === "confirmation") return "确认源";
  if (value === "spot_confirmation") return "现货确认源";
  if (value === "optional") return "可选源";
  return "未参与";
}

function marketLabel(value) {
  const labels = {
    spot: "Spot",
    perp: "Perp",
    funding: "Funding",
    oi: "OI",
    liquidation: "Liquidation",
    level2: "Level2",
  };
  return labels[value] || value;
}

function sourceListLabel(value) {
  if (!Array.isArray(value) || value.length === 0) return "无";
  return value.map(exchangeLabel).join(", ");
}

function buildWhaleEntities(items) {
  const groups = new Map();
  for (const item of items || []) {
    const id = trajectoryKey(item);
    const existing = groups.get(id) || {
      id,
      signals: [],
      severity: item.severity,
      score: 0,
      startTs: item.ts,
      endTs: item.ts,
    };
    existing.signals.push(item);
    existing.severity = strongestSeverity(existing.severity, item.severity);
    existing.score = Math.max(existing.score, Number(item.mainForceScore ?? item.score ?? 0));
    existing.startTs = Math.min(existing.startTs, Number(item.ts || existing.startTs));
    existing.endTs = Math.max(existing.endTs, Number(item.ts || existing.endTs));
    groups.set(id, existing);
  }

  return Array.from(groups.values())
    .map((group) => {
      const signals = group.signals.sort((a, b) => Number(a.ts || 0) - Number(b.ts || 0));
      const lead = signals[signals.length - 1] || signals[0];
      const trajectory = lead?.trajectory || {};
      const cluster = lead?.cluster || {};
      const stealthProfile = trajectory.stealthProfile || {};
      const actions = Array.isArray(trajectory.actions) ? trajectory.actions : [];
      const signalCount = Math.max(
        signals.length,
        Number(cluster.signalCount || trajectory.signalCount || 0),
        1,
      );
      const durationMs = Math.max(
        Number(trajectory.durationMs || 0),
        Number(cluster.durationMs || 0),
        Math.max(0, group.endTs - group.startTs),
      );
      const regimePath = Array.isArray(trajectory.regimePath) && trajectory.regimePath.length
        ? trajectory.regimePath
        : inferRegimePath(lead);
      const stealthGamma = clampRatio(stealthProfile.gamma || inferStealthGamma(signals));
      const hazardCurve = buildHazardCurve(signals, actions);
      return {
        ...group,
        actions,
        clusterIntent: cluster.dominantIntent,
        conclusion: trajectory.conclusion || clusterTrajectoryNarrativeSafe(lead),
        durationMs,
        hazardCurve,
        hazardPeak: Math.max(...hazardCurve, 0),
        intent: trajectory.intent || inferTrajectoryIntent(lead),
        persistenceScore: clampRatio(lead?.persistence?.persistenceScore || signalCount / 6),
        phases: deriveTrajectoryPhases(signals, actions, regimePath),
        regimePath,
        regimeStability: clampRatio(lead?.persistence?.regimeStability || lead?.cluster?.intensity || 0),
        signalCount,
        stealthCurve: buildStealthCurve(signals, stealthProfile),
        stealthGamma,
      };
    })
    .sort((a, b) => {
      const severityDelta = severityRank(b.severity) - severityRank(a.severity);
      if (severityDelta !== 0) return severityDelta;
      return Number(b.endTs || 0) - Number(a.endTs || 0);
    });
}

function trajectoryKey(item) {
  if (item?.trajectory?.trajectoryId) return item.trajectory.trajectoryId;
  if (item?.cluster?.clusterId) return `trajectory:${item.cluster.clusterId}`;
  return `trajectory:${item?.symbol || "unknown"}:${item?.direction || "neutral"}:${Math.floor(Number(item?.ts || 0) / 120_000)}`;
}

function shortWhaleId(id) {
  const text = String(id || "whale");
  const suffix = text.split(":").filter(Boolean).pop() || text;
  return `Whale #${suffix.slice(-6).toUpperCase()}`;
}

function strongestSeverity(a, b) {
  return severityRank(b) > severityRank(a) ? b : a;
}

function severityRank(value) {
  const ranks = { calm: 0, low: 1, medium: 2, high: 3, critical: 4, s: 5 };
  return ranks[String(value || "calm").toLowerCase()] || 0;
}

function inferRegimePath(item) {
  const type = String(item?.signalType || "");
  if (type.includes("absorption")) return ["manipulation", "accumulation"];
  if (type.includes("suppression")) return ["manipulation", "distribution"];
  if (String(item?.direction || "") === "buy") return ["accumulation"];
  if (String(item?.direction || "") === "sell") return ["distribution"];
  return ["unclear"];
}

function inferTrajectoryIntent(item) {
  const type = String(item?.signalType || "");
  if (type.includes("absorption")) return "accumulation";
  if (type.includes("suppression")) return "distribution";
  if (String(item?.direction || "") === "buy") return "accumulation";
  if (String(item?.direction || "") === "sell") return "distribution";
  return "unknown";
}

function inferStealthGamma(signals) {
  if (!Array.isArray(signals) || signals.length === 0) return 0;
  const averagePersistence = signals.reduce((sum, item) => sum + Number(item?.persistence?.persistenceScore || 0), 0) / signals.length;
  const averageIntensity = signals.reduce((sum, item) => sum + Number(item?.cluster?.intensity || 0), 0) / signals.length;
  return Math.max(averagePersistence, averageIntensity);
}

function buildStealthCurve(signals, stealthProfile) {
  const base = [
    Number(stealthProfile.fragmentation || 0),
    Number(stealthProfile.entropy || 0),
    Number(stealthProfile.crossExchangeDispersion || 0),
    Number(stealthProfile.gamma || 0),
  ].map(clampRatio);
  const signalValues = (signals || []).map((signal) => clampRatio(signal?.persistence?.persistenceScore || signal?.cluster?.intensity || 0));
  const points = [...signalValues, ...base].filter((value) => value > 0);
  return points.length ? points.slice(-8) : [0.12, 0.18, 0.16, 0.2];
}

function buildHazardCurve(signals, actions) {
  const actionValues = (actions || []).map((action) => clampRatio(Math.abs(Number(action?.priceImpact || 0)) / 0.5));
  const signalValues = (signals || []).map((signal) => {
    const volume = clampRatio(Number(signal.totalVolumeBtc || 0) / 4_500);
    const dominance = clampRatio(Number(signal.dominance || 0));
    const priceMove = clampRatio(Math.abs(Number(signal.priceMovePct || 0)) / 0.35);
    return clampRatio(volume * 0.45 + dominance * 0.35 + priceMove * 0.2);
  });
  const points = [...signalValues, ...actionValues].filter((value) => value > 0);
  return points.length ? points.slice(-8) : [0.08, 0.1, 0.09, 0.12];
}

function deriveTrajectoryPhases(signals, actions, regimePath) {
  if (Array.isArray(actions) && actions.length > 0) {
    return actions.slice(0, 4).map((action) => ({
      detail: `${exchangeLabel(action.exchange)} · ${formatBaseVolume(action.volume, action.symbol || signals?.[0]?.symbol)} · price impact ${formatSignedPct(action.priceImpact)}`,
      intensity: clampRatio(Math.abs(Number(action.priceImpact || 0)) / 0.5 || Number(action.volume || 0) / 4_500),
      ts: action.ts || signals?.[0]?.ts,
      type: action.actionType || "unknown",
    }));
  }
  const source = (signals || []).slice(-4);
  if (source.length > 0) {
    return source.map((signal) => ({
      detail: `${signalTypeLabel(signal.signalType)} · ${netDirection(signal.netVolumeBtc, signal.symbol)} · ${formatUsd(signal.totalNotionalUsd)}`,
      intensity: clampRatio(Math.max(Number(signal.dominance || 0), Number(signal?.cluster?.intensity || 0))),
      ts: signal.ts,
      type: signal.signalType || "unknown",
    }));
  }
  return (regimePath || ["unclear"]).map((type) => ({
    detail: "等待更多连续信号确认。",
    intensity: 0.2,
    ts: null,
    type,
  }));
}

function clusterTrajectoryNarrativeSafe(signal) {
  if (signal?.cluster?.signalCount > 1) return clusterTrajectoryNarrative(signal);
  return signal?.finalResult || "该信号暂未形成连续主力轨迹。";
}

function phaseLabel(value) {
  const labels = {
    accumulation: "吸筹阶段",
    aggressive_buy: "主动拉盘",
    aggressive_sell: "主动砸盘",
    distribution: "派发阶段",
    downside_absorption: "下方吸收",
    liquidity_probe: "流动性测试",
    manipulation: "操控试探",
    passive_absorb: "被动吸收",
    stop_hunt: "扫损/清算",
    unknown: "证据不足",
    upside_suppression: "上方压制",
  };
  return labels[value] || actionTypeLabel(value);
}

function phaseToneClass(value) {
  const text = String(value || "");
  if (text.includes("buy") || text.includes("accumulation") || text.includes("absorption")) {
    return "border-emerald-500/20 bg-emerald-500/5";
  }
  if (text.includes("sell") || text.includes("distribution") || text.includes("suppression")) {
    return "border-red-500/20 bg-red-500/5";
  }
  if (text.includes("hunt") || text.includes("manipulation")) {
    return "border-amber-500/20 bg-amber-500/5";
  }
  return "border-slate-800 bg-slate-900/60";
}

function phaseBarClass(value) {
  const text = String(value || "");
  if (text.includes("buy") || text.includes("accumulation") || text.includes("absorption")) return "h-full rounded-full bg-emerald-400";
  if (text.includes("sell") || text.includes("distribution") || text.includes("suppression")) return "h-full rounded-full bg-red-400";
  if (text.includes("hunt") || text.includes("manipulation")) return "h-full rounded-full bg-amber-300";
  return "h-full rounded-full bg-cyan-300";
}

function curveBarClass(tone) {
  if (tone === "amber") return "bg-amber-300/80";
  if (tone === "cyan") return "bg-cyan-300/80";
  return "bg-slate-400";
}

function netDirection(value, symbol = "BTC") {
  if (value > 0) return `净买入 ${formatBaseVolume(Math.abs(value), symbol)}`;
  if (value < 0) return `净卖出 ${formatBaseVolume(Math.abs(value), symbol)}`;
  return "中性";
}

function formatBtc(value) {
  return formatBaseVolume(value, "BTC");
}

function formatBaseVolume(value, symbol = "BTC") {
  return `${Math.round(Number(value || 0)).toLocaleString("en-US")} ${baseAssetSymbol(symbol)}`;
}

function baseAssetSymbol(symbol = "BTC") {
  return String(symbol || "BTC")
    .toUpperCase()
    .replace(/[-_/]?(USDT|USD|PERP|SWAP)$/i, "") || "BTC";
}

function formatUsd(value) {
  const number = Number(value || 0);
  if (number >= 1_000_000_000) return `$${(number / 1_000_000_000).toFixed(2)}B`;
  if (number >= 1_000_000) return `$${Math.round(number / 1_000_000).toLocaleString("en-US")}M`;
  return `$${Math.round(number).toLocaleString("en-US")}`;
}

function signalTriggerPrice(item) {
  const explicit = Number(
    item?.triggerPriceUsd ??
      item?.triggerPrice ??
      item?.avgPriceUsd ??
      item?.price,
  );
  if (Number.isFinite(explicit) && explicit > 0) {
    return explicit;
  }
  const totalVolumeBtc = Number(item?.totalVolumeBtc || 0);
  const totalNotionalUsd = Number(item?.totalNotionalUsd || 0);
  if (totalVolumeBtc > 0 && totalNotionalUsd > 0) {
    return totalNotionalUsd / totalVolumeBtc;
  }
  return null;
}

function formatPrice(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  if (number >= 1000) return `$${Math.round(number).toLocaleString("en-US")}`;
  if (number >= 1) return `$${number.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  return `$${number.toLocaleString("en-US", { minimumFractionDigits: 4, maximumFractionDigits: 4 })}`;
}

function formatPct(value) {
  return `${Number(value || 0).toFixed(1)}%`;
}

function formatScore(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) return "0/100";
  return `${Math.round(number)}/100`;
}

function formatScorePair(spotScore, contractScore) {
  return `S ${Math.round(Number(spotScore || 0))} / C ${Math.round(Number(contractScore || 0))}`;
}

function clusterTableLabel(item) {
  const count = Number(item?.cluster?.signalCount || 1);
  const persistence = Number(item?.persistence?.persistenceScore || 0);
  if (count <= 1 && persistence <= 0) return "单点";
  return `${count}条 · ${formatPct(persistence * 100)}`;
}

function clusterIntentLabel(value) {
  const labels = {
    liquidity_probe_buy: "买方流动性测试",
    liquidity_probe_sell: "卖方流动性测试",
    downside_absorption: "下方吸收",
    upside_suppression: "上方压制",
    single_signal: "单点信号",
  };
  return labels[value] || value || "N/A";
}

function clusterTrajectoryNarrative(signal) {
  return `该 cluster 共 ${signal.cluster.signalCount} 条同向信号，持续 ${formatMsDuration(signal.cluster.durationMs)}，价格区间 ${formatOptionalPct(signal.cluster.priceRangePct)}，更接近同一主力意图的连续投影。`;
}

function repetitionReasonLabel(value) {
  const labels = {
    same_intent_within_60s: "是：60 秒内同意图重复投影",
  };
  return labels[value] || "是";
}

function trajectoryIntentLabel(value) {
  const labels = {
    accumulation: "隐蔽吸筹",
    distribution: "分段派发",
    liquidity_manipulation: "流动性操控",
    stop_hunting: "扫损 / 清算猎取",
    unknown: "证据不足",
  };
  return labels[value] || value || "N/A";
}

function actionTypeLabel(value) {
  const labels = {
    aggressive_buy: "主动买入",
    aggressive_sell: "主动卖出",
    passive_absorb: "被动吸收",
    liquidity_probe: "流动性测试",
    stop_hunt: "扫损/清算",
    unknown: "未知动作",
  };
  return labels[value] || value || "N/A";
}

function regimePathLabel(path) {
  if (!Array.isArray(path) || path.length === 0) return "N/A";
  const labels = {
    accumulation: "吸筹",
    distribution: "派发",
    manipulation: "操控",
    unclear: "不明确",
  };
  return path.map((item) => labels[item] || item).join(" -> ");
}

function formatMsDuration(value) {
  const number = Number(value || 0);
  if (!Number.isFinite(number) || number <= 0) return "0s";
  if (number < 60_000) return `${Math.round(number / 1000)}s`;
  return `${Math.floor(number / 60_000)}m ${Math.round((number % 60_000) / 1000)}s`;
}

function formatOptionalPct(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) return "N/A";
  return formatPct(number);
}

function formatDeviation(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) return "N/A";
  return `${number.toFixed(2)}%`;
}

function clampRatio(value) {
  const number = Number(value || 0);
  if (!Number.isFinite(number)) return 0;
  return Math.max(0, Math.min(1, number));
}

function formatMultiple(value) {
  if (value === null || value === undefined) return "N/A";
  return `${Number(value).toFixed(1)}x`;
}

function formatPercentile(value) {
  if (value === null || value === undefined) return "N/A";
  return `P${Number(value).toFixed(1)}`;
}

function formatSignedPct(value) {
  if (value === null || value === undefined) return "N/A";
  const number = Number(value);
  return `${number >= 0 ? "+" : ""}${number.toFixed(2)}%`;
}

function formatTime(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  return new Date(number).toLocaleTimeString("zh-CN", {
    hour: "2-digit",
    hour12: false,
    minute: "2-digit",
    second: "2-digit",
  });
}

function formatEventRange(startedAt, endedAt) {
  const start = formatTime(startedAt);
  if (!endedAt) {
    return `${start} - 进行中`;
  }
  return `${start} - ${formatTime(endedAt)}`;
}

function biasText(value) {
  const number = Number(value || 0);
  if (number >= 15) return `偏多 +${Math.round(number)}`;
  if (number <= -15) return `偏空 ${Math.round(number)}`;
  return `中性 ${Math.round(number)}`;
}

function discordStatus(item) {
  if (item.discordSent) return "已推";
  if (item.discordEligible) return "待推";
  return "未推";
}

function liquidationStatus(item) {
  if (!item.liquidationSuspected) return "正常";
  const total = Number(item.liquidationLongBtc || 0) + Number(item.liquidationShortBtc || 0);
  const ratio = item.liquidationRatio === null || item.liquidationRatio === undefined
    ? "N/A"
    : formatPct(Number(item.liquidationRatio) * 100);
  return `疑似强平 ${formatBaseVolume(total, item.symbol)} / ${ratio}`;
}

function oiStatus(item) {
  const bias = oiBiasLabel(item.oiBias);
  if (item.oiChange5mBtc === null || item.oiChange5mBtc === undefined) return bias;
  const pct = item.oiChangePct === null || item.oiChangePct === undefined
    ? ""
    : ` / ${formatSignedPct(item.oiChangePct)}`;
  return `${formatSignedBaseVolume(item.oiChange5mBtc, item.symbol)}${pct} ${bias}`;
}

function fundingStatus(item) {
  const bias = fundingBiasLabel(item.fundingBias);
  if (item.fundingRate === null || item.fundingRate === undefined) return bias;
  return `${formatSignedPct(Number(item.fundingRate) * 100)} ${bias}`;
}

function scoringBreakdown(item) {
  const breakdown = item?.scoreBreakdown || {};
  const hasBackendBreakdown = Number(breakdown.finalScore || 0) > 0
    || ["volumeScore", "notionalScore", "dynamicAnomalyScore", "directionalStrengthScore", "priceResponseScore"].some((key) => Number(breakdown[key] || 0) !== 0);
  if (hasBackendBreakdown) {
    return [
      ["Volume Strength", scorePart(breakdown.volumeScore)],
      ["Notional Size", scorePart(breakdown.notionalScore)],
      ["Dynamic Anomaly", scorePart(breakdown.dynamicAnomalyScore)],
      ["Directional Strength", scorePart(breakdown.directionalStrengthScore)],
      ["Price Response", scorePart(breakdown.priceResponseScore)],
      ["Multi Source", scorePart(breakdown.multiSourceScore)],
      ["Data Quality", scorePart(breakdown.dataQualityScore)],
      ["Dominant Venue", scorePart(breakdown.dominantVenueScore)],
      ["OI Context", scorePart(breakdown.oiContextScore)],
      ["Penalty", scorePart(breakdown.penaltyScore)],
      ["Final Score", `${Number(breakdown.finalScore || item.score || 0).toFixed(1)} / 100`],
    ];
  }
  const volumeScore = Math.min(35, (Number(item.totalVolumeBtc || 0) / 4_500) * 35);
  const dynamicScore = item.dynamicMultiple === null || item.dynamicMultiple === undefined
    ? 0
    : Math.min(20, (Number(item.dynamicMultiple) / 10) * 20);
  const dominanceScore = Math.max(0, Math.min(15, ((Number(item.dominance || 0) - 0.5) / 0.25) * 15));
  const priceScore = item.priceMovePct === null || item.priceMovePct === undefined
    ? 0
    : Math.min(15, (Math.abs(Number(item.priceMovePct)) / 0.25) * 15);
  const exchangeScore = item.exchanges.length >= 3 ? 10 : item.exchanges.length === 2 ? 8 : item.exchanges.length === 1 ? 4 : 0;
  const dataQualityScore = Math.min(5, (Number(item.dataQuality || 0) / 100) * 5);
  const dominantNetFlowScore = Math.max(0, Math.min(5, ((dominantNetFlowShare(item) - 0.7) / 0.3) * 5));
  return [
    ["Volume Strength", `${volumeScore.toFixed(1)} / 35`],
    ["Dynamic Multiple", `${dynamicScore.toFixed(1)} / 20`],
    ["Dominance", `${dominanceScore.toFixed(1)} / 15`],
    ["Price Impact", `${priceScore.toFixed(1)} / 15`],
    ["Multi Exchange", `${exchangeScore.toFixed(1)} / 10`],
    ["Data Quality", `${dataQualityScore.toFixed(1)} / 5`],
    ["Dominant Venue Net Flow", `${dominantNetFlowScore.toFixed(1)} / 5`],
    ["Penalty Notes", item.liquidationSuspected ? "liquidation_suspected" : "none"],
  ];
}

function scorePart(value) {
  const number = Number(value || 0);
  if (!Number.isFinite(number)) return "0.0";
  return number.toFixed(1);
}

function dominantNetFlowShare(item) {
  const explicit = Number(item?.dominantVenueNetContributionShare);
  if (Number.isFinite(explicit) && explicit > 0) return explicit;
  return Math.max(
    0,
    ...((item?.exchanges || []).map((exchange) => Number(exchange.netContributionShare || 0))),
  );
}

function oiBiasLabel(value) {
  const bias = String(value || "unknown").toLowerCase();
  if (bias === "rising") return "OI上升";
  if (bias === "falling") return "OI下降";
  if (bias === "flat") return "OI横盘";
  return "OI N/A";
}

function fundingBiasLabel(value) {
  const bias = String(value || "unknown").toLowerCase();
  if (bias === "long") return "偏多";
  if (bias === "short") return "偏空";
  if (bias === "neutral") return "中性";
  return "Funding N/A";
}

function formatSignedBtc(value) {
  return formatSignedBaseVolume(value, "BTC");
}

function formatSignedBaseVolume(value, symbol = "BTC") {
  const number = Number(value || 0);
  const sign = number >= 0 ? "+" : "-";
  return `${sign}${formatBaseVolume(Math.abs(number), symbol)}`;
}

function relativeAge(value) {
  const seconds = Math.max(0, Math.round((Date.now() - Number(value)) / 1000));
  if (seconds < 60) return `${seconds} 秒前`;
  if (seconds < 3600) return `${Math.round(seconds / 60)} 分钟前`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)} 小时前`;
  return `${Math.round(seconds / 86400)} 天前`;
}

function formatLatency(value) {
  const ms = Number(value);
  if (!Number.isFinite(ms) || ms < 0) return "N/A";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${Math.round(ms / 1000)}s`;
}

function statusTone(status) {
  const value = String(status || "calm").toLowerCase();
  if (value === "disabled" || status === "未启用") return "slate";
  if (value === "warmup" || status === "预热") return "yellow";
  if (value === "strong" || status === "强异动") return "red";
  if (value === "active" || status === "异动") return "orange";
  return "slate";
}

function statusLabel(status) {
  const value = String(status || "calm").toLowerCase();
  if (value === "disabled" || status === "未启用") return "未启用";
  if (value === "warmup" || status === "预热") return "预热";
  if (value === "strong" || status === "强异动") return "强异动";
  if (value === "active" || status === "异动") return "异动";
  return "平静";
}

function severityTone(severity) {
  const value = String(severity || "calm").toLowerCase();
  if (value === "s") return "fuchsia";
  if (value === "critical") return "red";
  if (value === "high") return "orange";
  if (value === "medium") return "yellow";
  return "slate";
}

function toneClass(tone) {
  const classes = {
    cyan: "text-cyan-200",
    fuchsia: "text-fuchsia-300",
    red: "text-red-300",
    orange: "text-orange-300",
    yellow: "text-yellow-300",
    slate: "text-slate-300",
  };
  return classes[tone] || classes.slate;
}

function severityBadgeClass(severity) {
  const value = String(severity || "calm").toLowerCase();
  if (value === "s") return "bg-fuchsia-500/15 text-fuchsia-200 ring-1 ring-fuchsia-400/40";
  if (value === "critical") return "bg-red-500/15 text-red-200 ring-1 ring-red-400/40";
  if (value === "high") return "bg-orange-500/15 text-orange-200 ring-1 ring-orange-400/40";
  if (value === "medium") return "bg-yellow-500/15 text-yellow-200 ring-1 ring-yellow-400/30";
  return "bg-slate-500/15 text-slate-200 ring-1 ring-slate-400/30";
}
