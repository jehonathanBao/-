import { useEffect, useMemo, useState } from "react";
import {
  displayThresholdForSignal,
  fetchBinanceAltContractHistory,
  fetchBinanceAltContractLatest,
  fetchBinanceAltContractSummary,
  shouldDisplayByAltImpact,
} from "../api/binanceAltContract.js";

const SUMMARY_REFRESH_MS = 5_000;
const LATEST_REFRESH_MS = 10_000;

const DEFAULT_FILTERS = {
  symbol: "all",
  severity: "all",
  signal_type: "all",
  direction: "all",
  would_send: "all",
  liquidationDriven: "all",
  tier: "all",
  min_build_score: "all",
};

export default function BinanceAltContractMonitor() {
  const [state, setState] = useState({ loading: true, summary: null, items: [], error: null });
  const [filters, setFilters] = useState(DEFAULT_FILTERS);
  const [selectedSignalId, setSelectedSignalId] = useState(null);

  useEffect(() => {
    let cancelled = false;
    let summaryTimer = null;
    let latestTimer = null;

    const refreshSummary = () => {
      fetchBinanceAltContractSummary(filters.symbol).then((payload) => {
        if (cancelled) return;
        setState((previous) => ({
          ...previous,
          loading: false,
          summary: payload.error ? previous.summary : payload.summary,
          error: payload.error || null,
        }));
      });
    };

    const refreshLatest = () => {
      const request = shouldUseHistory(filters)
        ? fetchBinanceAltContractHistory({ ...filters, limit: 50 })
        : fetchBinanceAltContractLatest(50, filters.symbol);
      request.then((payload) => {
        if (cancelled) return;
        setState((previous) => ({
          loading: false,
          summary: payload.error ? previous.summary : payload.summary,
          items: payload.error ? previous.items : payload.items,
          error: payload.error || null,
        }));
      });
    };

    const configurePolling = () => {
      if (summaryTimer) window.clearInterval(summaryTimer);
      if (latestTimer) window.clearInterval(latestTimer);
      summaryTimer = null;
      latestTimer = null;
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

    refreshSummary();
    refreshLatest();
    configurePolling();
    document.addEventListener("visibilitychange", handleVisibilityChange);
    return () => {
      cancelled = true;
      if (summaryTimer) window.clearInterval(summaryTimer);
      if (latestTimer) window.clearInterval(latestTimer);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [filters]);

  const summary = state.summary || fallbackSummary();
  const visibleItems = useMemo(
    () =>
      state.items.filter(
        (item) =>
          shouldDisplayByAltImpact(item) ||
          (!hasAltImpactSnapshot(item) &&
            Number(item.totalNotionalUsd || 0) >= displayThresholdForSignal(item, summary)),
      ),
    [summary, state.items],
  );
  const symbolOptions = useMemo(
    () => ["all", ...(summary.monitoredSymbols || []).map((symbol) => symbol.replace(/USDT$/, ""))],
    [summary.monitoredSymbols],
  );
  const selectedSignal = visibleItems.find((item) => item.id === selectedSignalId) || null;

  return (
    <section className="console-panel mb-5 p-4 md:p-5">
      <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
        <div className="max-w-3xl">
          <p className="console-label text-cyan-300">Binance Alt Contract Anomaly</p>
          <h3 className="mt-2 text-lg font-bold text-white">山寨合约异常监控</h3>
          <p className="mt-1 max-w-3xl text-sm leading-6 text-slate-400">
            全量监控 Binance USDT 永续山寨合约，默认排除 BTC / ETH；用于识别主力建仓、异常拉砸、吸收、压制和清算瀑布。
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-4 xl:grid-cols-6">
          <StatusPill label="运行状态" value={statusLabel(summary.status)} tone={statusTone(summary.status)} />
          <StatusPill label="健康" value={healthStatusLabel(summary.healthStatus)} tone={healthStatusTone(summary.healthStatus)} />
          <StatusPill label="监控合约" value={summary.symbolUniverse?.monitoredCount || summary.monitoredSymbols.length || 0} tone="cyan" />
          <StatusPill label="活跃异常" value={summary.activeAnomalyCount} tone="yellow" />
          <StatusPill label="Critical/S" value={summary.recentCriticalOrSCount} tone="red" />
          <StatusPill label="dry-run" value={summary.dryRunWouldSendCount} tone="slate" />
        </div>
      </div>

      <AltTrendBar trend={summary.trend60s} />
      <RuntimeSummary summary={summary} />
      <SmafAuditCard report={summary.smafReport} />
      <SmllLearningCard report={summary.smllReport} />
      <AtcaAgentCard report={summary.atcaReport} />
      <AmiosOsCard report={summary.amiosReport} />
      <p className="mt-3 rounded-xl border border-cyan-400/20 bg-cyan-400/10 px-3 py-2 text-xs font-semibold text-cyan-100">
        相对冲击展示：AIS ≥ 70 才进入列表，AIS ≥ 85 才进入 Discord gate，AIS ≥ 90 才允许 S 级；旧名义额门槛仅兼容历史信号。
      </p>

      <div className="mt-4 grid gap-3 lg:grid-cols-[minmax(0,1fr)_260px]">
        <CollapsedUniverseSummary summary={summary} />
        <ExchangeStatus status={summary.exchanges?.binance} />
      </div>
      <DryRunAndUniverse summary={summary} />

      {state.error ? (
        <p className="mt-3 rounded-lg border border-yellow-500/30 bg-yellow-500/10 px-3 py-2 text-xs text-yellow-100">
          山寨合约监控数据暂时不可用，已保留上一次结果。
        </p>
      ) : null}

      <AltFilters
        filters={filters}
        onChange={(next) => {
          setSelectedSignalId(null);
          setFilters(next);
        }}
        symbolOptions={symbolOptions}
      />

      <div className="mt-4 overflow-x-auto rounded-xl border border-slate-800 bg-slate-950/30">
        {state.loading ? (
          <p className="rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
            山寨合约监控载入中...
          </p>
        ) : visibleItems.length === 0 ? (
          <p className="rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
            {summary.enabled ? "暂无山寨合约异常" : "山寨合约异常监控未启用"}
          </p>
        ) : (
          <table className="min-w-full table-fixed text-left text-xs">
            <thead className="bg-slate-950/80 text-slate-400">
              <tr>
                <HeaderCell>时间</HeaderCell>
                <HeaderCell>币种 / 价格</HeaderCell>
                <HeaderCell>市场层级</HeaderCell>
                <HeaderCell>MCSS</HeaderCell>
                <HeaderCell>Regime</HeaderCell>
                <HeaderCell>Lifecycle</HeaderCell>
                <HeaderCell>Prediction</HeaderCell>
                <HeaderCell>类型</HeaderCell>
                <HeaderCell>等级</HeaderCell>
                <HeaderCell>窗口</HeaderCell>
                <HeaderCell>异常分</HeaderCell>
                <HeaderCell>建仓分</HeaderCell>
                <HeaderCell>方向</HeaderCell>
                <HeaderCell>1m 名义额</HeaderCell>
                <HeaderCell>AIS</HeaderCell>
                <HeaderCell>市场冲击</HeaderCell>
                <HeaderCell>异常倍数</HeaderCell>
                <HeaderCell>OI</HeaderCell>
                <HeaderCell>价格变化</HeaderCell>
                <HeaderCell>清算</HeaderCell>
                <HeaderCell>Gate 类型</HeaderCell>
                <HeaderCell>Discord</HeaderCell>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800 text-slate-300">
              {visibleItems.map((item) => (
                <tr
                  className="console-row"
                  data-testid={`alt-contract-row-${item.id}`}
                  key={item.id}
                  onClick={() => setSelectedSignalId(item.id)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" || event.key === " ") {
                      event.preventDefault();
                      setSelectedSignalId(item.id);
                    }
                  }}
                  tabIndex={0}
                >
                  <Cell>{formatTime(item.ts)}</Cell>
                  <Cell>
                    <span className="flex flex-col leading-tight">
                      <span className="font-semibold text-slate-100">{item.symbol}</span>
                      <span className="mt-1 text-[11px] font-semibold text-cyan-200">{formatPrice(item.triggerPriceUsd)}</span>
                    </span>
                  </Cell>
                  <Cell>{marketTierLabel(item.marketTier)}</Cell>
                  <Cell>
                    <span className="flex flex-col leading-tight">
                      <span className="font-semibold text-cyan-100">{formatMcss(item.masterCapitalStrength?.mcss)}</span>
                      <span className="mt-1 text-[11px] text-slate-500">{item.masterCapitalStrength?.interpretation || "N/A"}</span>
                    </span>
                  </Cell>
                  <Cell>
                    <span className="flex flex-col leading-tight">
                      <span className="font-semibold text-slate-100">{marketRegimeLabel(item.marketRegime?.regime)}</span>
                      <span className="mt-1 text-[11px] text-slate-500">{formatConfidence(item.marketRegime?.confidence)}</span>
                    </span>
                  </Cell>
                  <Cell>
                    <span className="flex flex-col leading-tight">
                      <span className="font-semibold text-emerald-100">{lifecycleStateLabel(item.smartMoneyLifecycle?.lifecycleState)}</span>
                      <span className="mt-1 text-[11px] text-slate-500">{formatConfidence(item.smartMoneyLifecycle?.stateConfidence)}</span>
                    </span>
                  </Cell>
                  <Cell>
                    <span className="flex flex-col leading-tight">
                      <span className="font-semibold text-sky-100">{lifecycleStateLabel(item.smartMoneyPrediction?.nextState)}</span>
                      <span className="mt-1 text-[11px] text-slate-500">{formatConfidence(item.smartMoneyPrediction?.probability)}</span>
                    </span>
                  </Cell>
                  <Cell>{signalTypeLabel(item.signalType)}</Cell>
                  <Cell><span className={`rounded-full px-2 py-1 font-bold ${severityBadgeClass(item.severity)}`}>{severityLabel(item.severity)}</span></Cell>
                  <Cell>{item.windowSec}s</Cell>
                  <Cell>{item.abnormalScore}/100</Cell>
                  <Cell>{item.buildScore}/100</Cell>
                  <Cell>{directionLabel(item.direction)} {signedNumber(item.directionBias)}</Cell>
                  <Cell>{formatUsd(item.totalNotionalUsd)}</Cell>
                  <Cell>{formatImpactScore(item.altImpactScore)}</Cell>
                  <Cell>{formatImpactRatio(item.altImpactScore?.marketImpactRatio)}</Cell>
                  <Cell>{item.dynamicMultiple ? `${item.dynamicMultiple.toFixed(1)}x` : "N/A"}</Cell>
                  <Cell>{formatSignedBase(item.oiChange1mBase ?? item.oiChange5mBase, item.symbol)}</Cell>
                  <Cell>{formatSignedPct(item.priceMovePct)}</Cell>
                  <Cell>{item.liquidationSuspected ? "疑似" : "否"}</Cell>
                  <Cell>{discordAlertKindLabel(item.discordAlertKind)}</Cell>
                  <Cell>{discordStatus(item)}</Cell>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {selectedSignal ? (
        <AltSignalDetail onClose={() => setSelectedSignalId(null)} signal={selectedSignal} summary={summary} />
      ) : null}
    </section>
  );
}

function AltFilters({ filters, onChange, symbolOptions }) {
  return (
    <div className="mt-5 grid gap-3 text-xs md:grid-cols-3 xl:grid-cols-8">
      <label className="space-y-1">
        <span className="text-slate-400">币种</span>
        <select className="console-field" onChange={(event) => onChange({ ...filters, symbol: event.target.value })} value={filters.symbol}>
          {symbolOptions.map((symbol) => (
            <option key={symbol} value={symbol}>{symbol === "all" ? "全部" : symbol}</option>
          ))}
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-400">等级</span>
        <select className="console-field" onChange={(event) => onChange({ ...filters, severity: event.target.value })} value={filters.severity}>
          <option value="all">全部</option>
          <option value="s">S</option>
          <option value="critical">Critical</option>
          <option value="high">High</option>
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-400">类型</span>
        <select className="console-field" onChange={(event) => onChange({ ...filters, signal_type: event.target.value })} value={filters.signal_type}>
          <option value="all">全部</option>
          <option value="main_force_long_build">主力建多</option>
          <option value="main_force_short_build">主力建空</option>
          <option value="abnormal_pump">异常拉升</option>
          <option value="abnormal_dump">异常下跌</option>
          <option value="downside_absorption">下方吸收</option>
          <option value="upside_resistance">上方压制</option>
          <option value="liquidation_cascade">清算瀑布</option>
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-400">方向</span>
        <select className="console-field" onChange={(event) => onChange({ ...filters, direction: event.target.value })} value={filters.direction}>
          <option value="all">全部</option>
          <option value="buy">主动买入</option>
          <option value="sell">主动卖出</option>
          <option value="absorption">下方吸收</option>
          <option value="suppression">上方压制</option>
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-400">Discord</span>
        <select className="console-field" onChange={(event) => onChange({ ...filters, would_send: event.target.value })} value={filters.would_send}>
          <option value="all">全部</option>
          <option value="true">would_send</option>
          <option value="false">未进入 gate</option>
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-400">清算</span>
        <select className="console-field" onChange={(event) => onChange({ ...filters, liquidationDriven: event.target.value })} value={filters.liquidationDriven}>
          <option value="all">全部</option>
          <option value="true">疑似清算</option>
          <option value="false">非清算</option>
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-400">流动性 Tier</span>
        <select className="console-field" onChange={(event) => onChange({ ...filters, tier: event.target.value })} value={filters.tier}>
          <option value="all">全部</option>
          <option value="a">A</option>
          <option value="b">B</option>
          <option value="c">C</option>
          <option value="d">D</option>
          <option value="e">E</option>
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-400">建仓分</span>
        <select className="console-field" onChange={(event) => onChange({ ...filters, min_build_score: event.target.value })} value={filters.min_build_score}>
          <option value="all">全部</option>
          <option value="80">≥ 80</option>
          <option value="85">≥ 85</option>
          <option value="90">≥ 90</option>
        </select>
      </label>
    </div>
  );
}

function AltTrendBar({ trend }) {
  const buyPct = Math.round((trend?.buyRatio || 0) * 1000) / 10;
  const sellPct = Math.round((trend?.sellRatio || 0) * 1000) / 10;
  return (
    <div className="mt-4 rounded-xl border border-slate-700/60 bg-slate-950/60 p-4">
        <p className="console-label">60s Alt Contract Flow</p>
      <div className="mt-2 flex flex-wrap items-center justify-between gap-2 text-sm font-semibold text-white">
        <span className="text-emerald-300">Buy {buyPct}%</span>
        <span className="text-red-300">Sell {sellPct}%</span>
      </div>
      <div className="mt-3 flex h-2 overflow-hidden rounded-full bg-slate-800">
        <div className="bg-emerald-400" style={{ width: `${Math.max(0, Math.min(100, buyPct))}%` }} />
        <div className="bg-red-400" style={{ width: `${Math.max(0, Math.min(100, sellPct))}%` }} />
      </div>
      <p className="mt-2 text-xs text-slate-500">
        60s 名义金额 {formatUsd(trend?.totalNotionalUsd || 0)} · 净方向 {formatSignedNumber(trend?.netVolumeBase || 0)}
      </p>
    </div>
  );
}

function RuntimeSummary({ summary }) {
  const context = summary.allMarketContext || {};
  const chips = [
    ["collector", collectorStatusLabel(summary.collectorStatus)],
    ["last trade", summary.lastTradeAt ? relativeAge(summary.lastTradeAt) : "暂无"],
    ["OI", summary.lastOiPollAt ? relativeAge(summary.lastOiPollAt) : "暂无"],
    ["markPrice", contextStatus(context.markPriceConnected, context.lastMarkPriceAt)],
    ["ticker", contextStatus(context.tickerConnected, context.lastTickerAt)],
    ["ForceOrder", contextStatus(context.forceOrderConnected, context.lastForceOrderAt || summary.lastForceOrderAt)],
    ["Candidate", (context.candidateSymbols || []).length],
    ["Hot OI", (context.hotOiSymbols || []).length],
    ["1m trades", summary.flowBuckets1m],
    ["errors 1h", summary.errors1h],
  ];
  return (
    <div className="mt-3 flex flex-wrap gap-2 rounded-xl border border-slate-700/60 bg-slate-950/50 p-3 text-xs text-slate-300">
      {chips.map(([label, value]) => (
        <span className="rounded-lg border border-slate-700 bg-slate-900/80 px-3 py-2" key={label}>
          <span className="text-slate-500">{label}</span>
          <span className="ml-2 font-semibold text-slate-100">{value}</span>
        </span>
      ))}
      {summary.topActiveSymbols?.length ? (
        <span className="rounded-lg border border-cyan-400/20 bg-cyan-400/10 px-3 py-2 text-cyan-100">
          活跃 {summary.topActiveSymbols.join(", ")}
        </span>
      ) : null}
    </div>
  );
}

function AtcaAgentCard({ report }) {
  const item = report || {};
  const agents = Array.isArray(item.agents) ? item.agents : [];
  const topAgents = agents.slice(0, 3);
  const counters = [
    ["感知", item.perceptionCount || 0],
    ["解释", item.interpretationCount || 0],
    ["意图", item.intentionCount || 0],
    ["预测", item.predictionCount || 0],
    ["决策", item.decisionCount || 0],
  ];
  return (
    <div className="mt-3 rounded-xl border border-violet-400/20 bg-violet-400/10 p-3 text-xs text-slate-300">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="console-label text-violet-300">ATCA Cognition Agent</p>
          <p className="mt-1 text-sm font-bold text-white">
            认知状态 {atcaStatusLabel(item.cognitionStatus)} · {item.memorySummary || "short_memory=0 symbols"}
          </p>
        </div>
        <span className="rounded-full border border-violet-400/30 bg-violet-400/10 px-3 py-1 font-semibold text-violet-100">
          {item.protectedRealtime === false ? "可执行" : "只读认知"}
        </span>
      </div>
      <div className="mt-3 grid gap-2 md:grid-cols-5">
        {counters.map(([label, value]) => (
          <div className="rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2" key={label}>
            <p className="text-slate-500">{label}</p>
            <p className="mt-1 font-semibold text-slate-100">{value}</p>
          </div>
        ))}
      </div>
      {topAgents.length ? (
        <div className="mt-3 grid gap-2 xl:grid-cols-3">
          {topAgents.map((agent) => (
            <div className="rounded-lg border border-violet-400/20 bg-slate-950/50 p-3" key={agent.symbol}>
              <div className="flex items-center justify-between gap-2">
                <p className="font-bold text-white">{agent.symbol}</p>
                <span className="rounded-full border border-slate-700 px-2 py-1 text-[11px] text-slate-300">
                  {agent.decision?.severity || "Ignore"}
                </span>
              </div>
              <p className="mt-2 text-slate-300">
                {lifecycleStateLabel(agent.state)} · {atcaIntentLabel(agent.intent)} → {lifecycleStateLabel(agent.prediction)}
              </p>
              <p className="mt-1 text-slate-500">
                confidence {formatPercentNumber(agent.confidence)} · risk {atcaRiskLabel(agent.risk)} · {agent.marketState?.priceStructure || "unknown"}
              </p>
              <p className="mt-2 text-[11px] text-violet-100">{agent.decision?.reason || "agent_filtered"}</p>
            </div>
          ))}
        </div>
      ) : (
        <p className="mt-3 text-slate-500">
          ATCA 等待最新 signal；Agent 层只统一感知、解释、意图、预测和通知决策，不会执行交易。
        </p>
      )}
    </div>
  );
}

function AmiosOsCard({ report }) {
  const item = report || {};
  const processes = Array.isArray(item.activeProcesses) ? item.activeProcesses : [];
  const states = Array.isArray(item.currentStates) ? item.currentStates : [];
  return (
    <div className="mt-3 rounded-xl border border-emerald-400/20 bg-emerald-400/10 p-3 text-xs text-slate-300">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="console-label text-emerald-300">AMIOS Market OS</p>
          <p className="mt-1 text-sm font-bold text-white">
            {amiosMarketStateLabel(item.marketState)} · Kernel Load {formatAuditScore(item.kernelLoad)}
          </p>
        </div>
        <div className="flex flex-wrap gap-2">
          <span className="rounded-full border border-emerald-400/30 bg-emerald-400/10 px-3 py-1 font-semibold text-emerald-100">
            {item.readOnly === false ? "可执行 OS" : "只读 OS"}
          </span>
          <span className="rounded-full border border-slate-700 px-3 py-1 font-semibold text-slate-200">
            {amiosStatusLabel(item.osStatus)}
          </span>
        </div>
      </div>

      <div className="mt-3 grid gap-2 md:grid-cols-4 xl:grid-cols-6">
        <MetricTile label="吞吐" value={amiosThroughputLabel(item.signalThroughput)} />
        <MetricTile label="可信度" value={formatAuditScore(item.confidence)} />
        <MetricTile label="调度" value={amiosDecisionLabel(item.schedulerDecision)} />
        <MetricTile label="风险" value={amiosRiskLabel(item.risk)} />
        <MetricTile label="Discord Gate" value={item.directDiscordGate ? "接管" : "不接管"} />
        <MetricTile label="实时保护" value={item.protectedRealtime === false ? "关闭" : "开启"} />
      </div>

      {processes.length ? (
        <div className="mt-3 grid gap-2 md:grid-cols-3 xl:grid-cols-5">
          {processes.slice(0, 10).map((process) => (
            <div className="rounded-lg border border-slate-800 bg-slate-950/50 p-3" key={`${process.layer}-${process.name}`}>
              <div className="flex items-center justify-between gap-2">
                <p className="font-bold text-white">{process.name}</p>
                <span className="rounded-full border border-slate-700 px-2 py-1 text-[11px] text-slate-300">
                  {amiosLayerLabel(process.layer)}
                </span>
              </div>
              <p className="mt-2 text-slate-300">{amiosProcessStatusLabel(process.status)}</p>
              <p className="mt-1 text-slate-500">load {formatAuditScore(process.load)} · {process.role}</p>
            </div>
          ))}
        </div>
      ) : null}

      {states.length ? (
        <div className="mt-3 grid gap-2 xl:grid-cols-2">
          {states.slice(0, 4).map((state) => (
            <div className="rounded-lg border border-emerald-400/20 bg-slate-950/50 p-3" key={state.symbol}>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <p className="font-bold text-white">{state.symbol}</p>
                <span className="text-emerald-100">{amiosMarketStateLabel(state.marketState)}</span>
              </div>
              <p className="mt-2 text-slate-300">
                {marketRegimeLabel(state.regime)} · {lifecycleStateLabel(state.lifecycleState)} → {lifecycleStateLabel(state.prediction)}
              </p>
              <p className="mt-1 text-slate-500">
                confidence {formatAuditScore(state.confidence)} · risk {amiosRiskLabel(state.risk)} · {state.control}
              </p>
              <p className="mt-2 text-[11px] text-emerald-100">{state.explanation}</p>
            </div>
          ))}
        </div>
      ) : (
        <p className="mt-3 text-slate-500">
          AMIOS 等待最新 signal；它只把各层输出合成为 Market OS 视角，不改变 signal、Discord gate 或任何执行路径。
        </p>
      )}

      <p className="mt-3 text-[11px] text-slate-500">{item.auditSummary}</p>
    </div>
  );
}

function SmllLearningCard({ report }) {
  const item = report || {};
  const weights = item.suggestedWeights || {};
  const drift = item.driftReport || {};
  const updates = Array.isArray(item.calibrationUpdates) ? item.calibrationUpdates : [];
  const errors = Array.isArray(item.errorReports) ? item.errorReports : [];
  const rows = [
    ["样本", `${item.sampleSize || 0}/${item.minSamplesForUpdate || 3}`],
    ["准确率", formatPercentNumber(item.accuracyRate)],
    ["错误", item.wrongCount || 0],
    ["中性", item.neutralCount || 0],
    ["OI 权重", formatWeight(weights.oiWeight)],
    ["价格权重", formatWeight(weights.priceWeight)],
    ["清算权重", formatWeight(weights.liquidationWeight)],
    ["漂移", drift.driftDetected ? "已检测" : "未检测"],
  ];
  return (
    <div className="mt-3 rounded-xl border border-sky-400/20 bg-sky-400/10 p-3 text-xs text-slate-300">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="console-label text-sky-300">SMLL Self-Learning Loop</p>
          <p className="mt-1 text-sm font-bold text-white">
            自学习 {formatAuditScore(item.learningScore)} · {smllStatusLabel(item.status)}
          </p>
        </div>
        <span className="rounded-full border border-sky-400/30 bg-sky-400/10 px-3 py-1 font-semibold text-sky-100">
          {item.protectedRealtime === false ? "实时可变更" : "只读建议"}
        </span>
      </div>
      <div className="mt-3 grid gap-2 md:grid-cols-4 xl:grid-cols-8">
        {rows.map(([label, value]) => (
          <div className="rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2" key={label}>
            <p className="text-slate-500">{label}</p>
            <p className="mt-1 font-semibold text-slate-100">{value}</p>
          </div>
        ))}
      </div>
      {errors.length || updates.length ? (
        <div className="mt-3 grid gap-2 md:grid-cols-2">
          {errors.length ? (
            <div className="rounded-lg border border-yellow-400/20 bg-yellow-400/10 p-3 text-yellow-100">
              <p className="font-semibold">误差归因</p>
              <p className="mt-1 text-slate-300">
                {errors.slice(0, 3).map((error) => `${smllErrorLabel(error.rootCause)} · ${error.affectedModule}`).join(" / ")}
              </p>
            </div>
          ) : null}
          {updates.length ? (
            <div className="rounded-lg border border-cyan-400/20 bg-cyan-400/10 p-3 text-cyan-100">
              <p className="font-semibold">校准建议</p>
              <p className="mt-1 text-slate-300">
                {updates.slice(0, 3).map((update) => `${update.parameter}: ${Number(update.oldValue).toFixed(2)} → ${Number(update.newValue).toFixed(2)}`).join(" / ")}
              </p>
            </div>
          ) : null}
        </div>
      ) : (
        <p className="mt-3 text-slate-500">
          自学习层只记录结果、归因错误并给出延迟校准建议；不会自动修改当前阈值、权重、信号或 Discord gate。
        </p>
      )}
    </div>
  );
}

function SmafAuditCard({ report }) {
  const item = report || {};
  const rows = [
    ["数据完整性", item.dataAudit?.integrityScore, item.dataAudit?.dataRiskLevel || "unknown"],
    ["信号稳定性", item.signalAudit?.integrityScore, `single ${formatAuditScore(item.signalAudit?.singleSourceDependency)}`],
    ["行为结构", item.behaviorAudit?.structuralIntegrity, `entropy ${formatAuditScore(item.behaviorAudit?.transitionEntropy)}`],
    ["预测可靠性", item.predictionAudit?.integrityScore, `flip ${formatAuditScore(item.predictionAudit?.flipRate)}`],
  ];
  const issues = Array.isArray(item.criticalIssues) ? item.criticalIssues : [];
  return (
    <div className="mt-3 rounded-xl border border-slate-700/60 bg-slate-950/50 p-3 text-xs text-slate-300">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p className="console-label">SMAF System Audit</p>
          <p className="mt-1 text-sm font-bold text-white">
            系统健康 {formatAuditScore(item.smafScore)} · {smafRiskLabel(item.riskLevel)}
          </p>
        </div>
        <span className={`rounded-full border px-3 py-1 font-semibold ${smafToneClass(item.smafScore)}`}>
          {smafRiskLabel(item.riskLevel)}
        </span>
      </div>
      <div className="mt-3 grid gap-2 md:grid-cols-4">
        {rows.map(([label, value, hint]) => (
          <div className="rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2" key={label}>
            <p className="text-slate-500">{label}</p>
            <p className="mt-1 font-semibold text-slate-100">{formatAuditScore(value)}</p>
            <p className="mt-1 text-[11px] text-slate-500">{hint}</p>
          </div>
        ))}
      </div>
      {issues.length ? (
        <div className="mt-3 flex flex-wrap gap-2">
          {issues.map((issue) => (
            <span className="rounded-full border border-yellow-400/20 bg-yellow-400/10 px-3 py-1 text-yellow-100" key={issue}>
              {smafIssueLabel(issue)}
            </span>
          ))}
        </div>
      ) : (
        <p className="mt-3 text-slate-500">未发现关键审计问题；SMAF 只做旁路审计，不影响信号和 Discord gate。</p>
      )}
    </div>
  );
}

function CollapsedUniverseSummary({ summary }) {
  const universe = summary.symbolUniverse || {};
  const monitoredSymbols = summary.monitoredSymbols || [];
  const excludedSymbols = universe.excludedSymbols || [];
  const monitoredCount = universe.monitoredCount || monitoredSymbols.length || 0;

  return (
    <div className="console-panel-muted p-3 text-xs text-slate-400">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <span>
          <span className="font-semibold text-white">监控范围</span>
          <span className="ml-2 text-slate-500">
            已隐藏列表 · {universeModeLabel(universe.mode)} · {monitoredCount} 个合约 · Tier {formatTierCounts(universe.tierCounts)}
          </span>
        </span>
        <span className="rounded-lg border border-cyan-400/20 bg-cyan-400/10 px-2 py-1 text-[11px] font-semibold text-cyan-100">
          范围摘要
        </span>
      </div>
      <p className="mt-2 leading-5 text-slate-500">
        仅 USDT / PERPETUAL / TRADING · 排除 {excludedSymbols.join(", ") || "无"} · Candidate only · 只读提醒 · 不下单 ·
        dry-run 默认开启
      </p>
    </div>
  );
}

function DryRunAndUniverse({ summary }) {
  const stats = summary.dryRunStats || {};
  const universe = summary.symbolUniverse || {};
  const context = summary.allMarketContext || {};
  return (
    <div className="mt-3 grid gap-3 text-xs lg:grid-cols-2">
      <div className="console-panel-muted p-3">
        <p className="font-semibold text-white">Dry-run 1h</p>
        <p className="mt-2 text-slate-400">
          signals {stats.signals1h || 0} · High {stats.high1h || 0} · Critical {stats.critical1h || 0} · S {stats.s1h || 0} · would_send {stats.wouldSend1h || 0}
        </p>
        <p className="mt-1 text-slate-500">
          skipped: low_score {stats.skippedLowScore1h || 0}, cooldown {stats.skippedCooldown1h || 0}, data_quality {stats.skippedDataQuality1h || 0}, liquidation {stats.liquidationDriven1h || 0}
        </p>
      </div>
      <div className="console-panel-muted p-3">
        <p className="font-semibold text-white">Dry-run 24h</p>
        <p className="mt-2 text-slate-400">
          signals {stats.signals24h || 0} · High {stats.high24h || 0} · Critical {stats.critical24h || 0} · S {stats.s24h || 0} · would_send {stats.wouldSend24h || 0}
        </p>
        <p className="mt-1 text-slate-500">
          skipped: low_score {stats.skippedLowScore24h || 0}, cooldown {stats.skippedCooldown24h || 0}, data_quality {stats.skippedDataQuality24h || 0}, liquidation {stats.liquidationDriven24h || 0}
        </p>
      </div>
      <details className="console-panel-muted p-3 lg:col-span-2">
        <summary className="cursor-pointer list-none text-xs font-semibold text-white">
          Symbol Universe · 已折叠 · 监控 {universe.monitoredCount || 0} · Tier {formatTierCounts(universe.tierCounts)}
        </summary>
        <div className="mt-3 max-h-28 overflow-y-auto rounded-lg border border-slate-800 bg-slate-950/50 p-3 text-xs leading-5 text-slate-500">
          <p>
            {universeModeLabel(universe.mode)} · limit {universe.limit || 0} · min 24h {formatUsd(universe.min24hQuoteVolumeUsd || 0)}
          </p>
          <p className="mt-1">
            whitelist {(universe.whitelist || []).slice(0, 8).join(", ") || "none"} · blacklist{" "}
            {(universe.blacklist || []).join(", ") || "none"} · excluded {(universe.excludedSymbols || []).join(", ") || "none"}
          </p>
          <p className="mt-1">
            Candidate {(context.candidateSymbols || []).slice(0, 8).join(", ") || "none"} · Hot OI{" "}
            {(context.hotOiSymbols || []).slice(0, 8).join(", ") || "none"}
          </p>
        </div>
      </details>
    </div>
  );
}

function ExchangeStatus({ status }) {
  const item = status || {};
  return (
    <div className="rounded-xl border border-slate-700/60 bg-slate-950/60 p-3 text-xs">
      <div className="flex items-center justify-between gap-2">
        <span className="font-semibold text-white">Binance Perp</span>
        <span className={exchangeStatusClass(item)}>{exchangeStatusLabel(item.status)}</span>
      </div>
      <p className="mt-1 text-slate-500">最近成交 {item.lastTradeAt ? relativeAge(item.lastTradeAt) : "暂无"}</p>
      <p className="mt-1 text-slate-500">重连 {item.reconnectCount || 0}</p>
    </div>
  );
}

function AltSignalDetail({ signal, summary, onClose }) {
  const explainTags = Array.isArray(signal.explainTags) ? signal.explainTags : [];
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 p-4">
      <div className="max-h-[90vh] w-full max-w-4xl overflow-y-auto rounded-2xl border border-cyan-400/30 bg-slate-950 p-5 shadow-glow">
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="text-xs uppercase tracking-[0.25em] text-cyan-300">Alt Contract Review</p>
            <h3 className="mt-2 text-xl font-bold text-white">{signal.symbol} · {signalTypeLabel(signal.signalType)}</h3>
          </div>
          <button className="rounded-lg border border-slate-600 px-3 py-1 text-sm text-slate-200 outline-none transition hover:border-cyan-400 hover:text-cyan-100 focus-visible:ring-2 focus-visible:ring-cyan-500/35" onClick={onClose} type="button">关闭</button>
        </div>
        <div className="mt-5 grid gap-3 text-sm md:grid-cols-3">
          <Detail label="异常评分" value={`${signal.abnormalScore}/100`} />
          <Detail label="建仓评分" value={`${signal.buildScore}/100`} />
          <Detail label="主力置信度" value={`${Math.round(Number(signal.mainForceConfidence || 0))}/100`} />
          <Detail label="证据数量" value={`${signal.evidenceCount || 0} 项`} />
          <Detail label="后续验证" value={postSignalStatusLabel(signal.postSignalStatus)} />
          <Detail label="Signal VWAP" value={formatPrice(signal.signalVwap)} />
          <Detail label="Market Tier" value={marketTierLabel(signal.marketTier)} />
          <Detail label="AIS 展示门槛" value={`${formatImpactScore(signal.altImpactScore)} / ${signal.altImpactScore?.displayThreshold || 70}`} />
          <Detail label="旧名义额门槛" value={formatUsd(displayThresholdForSignal(signal, summary))} />
          <Detail label="市场冲击率" value={formatImpactRatio(signal.altImpactScore?.marketImpactRatio)} />
          <Detail label="24h 参考量" value={`${formatUsd(signal.altImpactScore?.referenceVolume24hUsd || 0)} · ${impactReferenceLabel(signal.altImpactScore?.referenceSource)}`} />
          <Detail
            label="LME 盘口结构"
            value={`${formatLms(signal.liquidityMicrostructure?.lmsScore)} · ${lmeBehaviorLabel(signal.liquidityMicrostructure?.behavior)}`}
          />
          <Detail
            label="MCG 控制图谱"
            value={`${formatCss(signal.marketControlGraph?.controlStrength)} · ${mcgControlTypeLabel(signal.marketControlGraph?.controlType)}`}
          />
          <Detail
            label="MCSS"
            value={`${formatMcss(signal.masterCapitalStrength?.mcss)} · ${signal.masterCapitalStrength?.interpretation || "N/A"}`}
          />
          <Detail
            label="Market Regime"
            value={`${marketRegimeLabel(signal.marketRegime?.regime)} ${formatConfidence(signal.marketRegime?.confidence)}${
              signal.marketRegime?.subType ? ` · ${marketRegimeLabel(signal.marketRegime.subType)}` : ""
            }`}
          />
          <Detail
            label="Smart Money Lifecycle"
            value={`${lifecycleStateLabel(signal.smartMoneyLifecycle?.lifecycleState)} ${formatConfidence(signal.smartMoneyLifecycle?.stateConfidence)}${
              signal.smartMoneyLifecycle?.transitionSignal ? ` · ${signal.smartMoneyLifecycle.transitionSignal}` : ""
            }`}
          />
          <Detail
            label="Next Stage Prediction"
            value={`${lifecycleStateLabel(signal.smartMoneyPrediction?.nextState)} ${formatConfidence(signal.smartMoneyPrediction?.probability)} · ${predictionBiasLabel(signal.smartMoneyPrediction?.directionBias)} ${formatDirectionProbability(signal.smartMoneyPrediction?.directionProbability)}`}
          />
          <Detail
            label="SCC 可信度"
            value={`${formatSccScore(signal.signalConfidence?.confidenceScore)} · ${sccLevelLabel(signal.signalConfidence?.confidenceLevel)}`}
          />
          <Detail label="方向 Bias" value={signedNumber(signal.directionBias)} />
          <Detail label="Data Quality" value={`${signal.dataQuality}/100`} />
          <Detail label="触发价格" value={formatPrice(signal.triggerPriceUsd)} />
          <Detail label="动态倍数" value={signal.dynamicMultiple ? `${signal.dynamicMultiple.toFixed(2)}x` : "N/A"} />
          <Detail label="S 成交额门槛" value={formatUsd(signal.sGradeNotionalThresholdUsd)} />
          <Detail label="S 成交量门槛" value={formatBase(signal.sGradeVolumeThresholdBase, signal.symbol)} />
          <Detail label="S 条件" value={signal.sGradeEligible ? "全部满足" : "未全部满足"} />
          <Detail label="OI 变化" value={formatSignedBase(signal.oiChange1mBase ?? signal.oiChange5mBase, signal.symbol)} />
          <Detail label="OI 质量" value={`${oiQualityLabel(signal.oiQuality)}${signal.oiFreshnessSec == null ? "" : ` · ${signal.oiFreshnessSec}s`}`} />
          <Detail label="Funding" value={signal.fundingRate == null ? "N/A" : `${(signal.fundingRate * 100).toFixed(4)}%`} />
          <Detail label="Funding 拥挤" value={fundingCrowdingLabel(signal.fundingCrowding)} />
          <Detail label="市场共振" value={marketWideText(signal)} />
          <Detail label="清算上下文" value={liquidationContextText(signal)} />
          <Detail label="Discord Gate 类型" value={discordAlertKindLabel(signal.discordAlertKind)} />
          <Detail label="Discord 名义额门槛" value={formatUsd(signal.discordMinNotionalUsd)} />
          <Detail label="Discord 跳过原因" value={discordReasonLabel(signal.discordReason)} />
          <Detail label="Discord dry-run" value={discordStatus(signal)} />
          <Detail label="最终判断" value={signal.finalResult} wide />
        </div>
        <div className="mt-5 grid gap-3 md:grid-cols-2">
          <AltImpactCard altImpactScore={signal.altImpactScore} />
          <LiquidityMicrostructureCard microstructure={signal.liquidityMicrostructure} />
          <MarketControlGraphCard graph={signal.marketControlGraph} />
          <McssCard masterCapitalStrength={signal.masterCapitalStrength} />
          <MarketRegimeCard marketRegime={signal.marketRegime} />
          <SmartMoneyLifecycleCard lifecycle={signal.smartMoneyLifecycle} />
          <SmartMoneyPredictionCard prediction={signal.smartMoneyPrediction} />
          <SignalConfidenceCard confidence={signal.signalConfidence} />
          <BreakdownCard breakdown={signal.scoreBreakdown} />
          <div className="rounded-xl border border-slate-700 bg-slate-900/70 p-4">
            <p className="text-xs uppercase tracking-[0.22em] text-slate-500">Active Source Snapshot</p>
            <div className="mt-3 space-y-2 text-xs text-slate-300">
              {signal.activeSources.length === 0 ? (
                <p>暂无 source snapshot</p>
              ) : signal.activeSources.map((source) => (
                <p key={`${source.exchange}-${source.marketType}`}>
                  {source.exchange} · {source.marketType} · {source.role} · {source.status}
                </p>
              ))}
            </div>
          </div>
          <div className="rounded-xl border border-slate-700 bg-slate-900/70 p-4 md:col-span-2">
            <p className="text-xs uppercase tracking-[0.22em] text-slate-500">Window Confirmations</p>
            <div className="mt-3 grid gap-2 text-xs text-slate-300 sm:grid-cols-3">
              {(signal.windowConfirmations || []).length === 0 ? (
                <p>暂无窗口确认</p>
              ) : signal.windowConfirmations.map((window) => (
                <div className="rounded-lg bg-slate-950/60 px-3 py-2" key={window.windowSec}>
                  <p className="font-semibold text-slate-100">{window.windowSec}s · {window.confirmed ? "已确认" : "未确认"}</p>
                  <p className="mt-1 text-slate-500">{formatUsd(window.notionalUsd)} · {window.dynamicMultiple ? `${window.dynamicMultiple.toFixed(1)}x` : "N/A"} · {Math.round((window.directionalStrength || 0) * 100)}%</p>
                </div>
              ))}
            </div>
          </div>
          <SGradeConditions conditions={signal.sGradeConditions || []} />
        </div>
        <div className="mt-3 grid gap-3 md:grid-cols-3">
          <ExplanationCard title="异常判断" text={signal.abnormalExplanation} />
          <ExplanationCard title="建仓判断" text={signal.buildExplanation} />
          <ExplanationCard title="清算判断" text={signal.liquidationExplanation} />
        </div>
        <div className="mt-3 grid gap-3 text-xs md:grid-cols-4">
          <ExplanationMetric label="Abnormal Score" value={`${signal.abnormalScore}/100`} />
          <ExplanationMetric label="Build Score" value={`${signal.buildScore}/100`} />
          <ExplanationMetric label="OI" value={formatSignedBase(signal.oiChange1mBase ?? signal.oiChange5mBase, signal.symbol)} />
          <ExplanationMetric label="Price Move" value={formatSignedPct(signal.priceMovePct)} />
        </div>
        {explainTags.length ? (
          <div className="mt-3 flex flex-wrap gap-2 text-xs">
            {explainTags.map((tag) => (
              <span className="rounded-full border border-cyan-400/20 bg-cyan-400/10 px-3 py-1 text-cyan-100" key={tag}>{tag}</span>
            ))}
          </div>
        ) : null}
        {(signal.evidenceTags || []).length ? (
          <div className="mt-3 flex flex-wrap gap-2 text-xs">
            {(signal.evidenceTags || []).map((tag) => (
              <span className="rounded-full border border-emerald-400/20 bg-emerald-400/10 px-3 py-1 text-emerald-100" key={tag}>{evidenceTagLabel(tag)}</span>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}

function SGradeConditions({ conditions }) {
  return (
    <div className="rounded-xl border border-slate-700 bg-slate-900/70 p-4 md:col-span-2">
      <p className="text-xs uppercase tracking-[0.22em] text-slate-500">S Grade Conditions</p>
      <div className="mt-3 grid gap-2 text-xs sm:grid-cols-2">
        {conditions.length === 0 ? (
          <p className="text-slate-400">暂无 S 级条件快照</p>
        ) : conditions.map((condition) => (
          <div
            className={`rounded-lg border px-3 py-2 ${
              condition.passed
                ? "border-emerald-400/20 bg-emerald-400/10 text-emerald-100"
                : "border-yellow-400/20 bg-yellow-400/10 text-yellow-100"
            }`}
            key={condition.key}
          >
            <p className="font-semibold">{condition.passed ? "通过" : "未通过"} · {condition.label}</p>
            <p className="mt-1 text-slate-400">当前 {condition.actual} · 门槛 {condition.threshold}</p>
          </div>
        ))}
      </div>
    </div>
  );
}

function ExplanationCard({ title, text }) {
  return (
    <div className="rounded-xl border border-slate-700/60 bg-slate-900/70 p-3">
      <p className="text-xs text-slate-500">{title}</p>
      <p className="mt-2 text-xs leading-5 text-slate-200">{text || "暂无解释"}</p>
    </div>
  );
}

function ExplanationMetric({ label, value }) {
  return (
    <div className="rounded-xl border border-slate-700/60 bg-slate-900/70 p-3">
      <p className="text-[11px] uppercase tracking-[0.14em] text-slate-500">{label}</p>
      <p className="mt-1 text-sm font-semibold text-slate-100">{value || "N/A"}</p>
    </div>
  );
}

function BreakdownCard({ breakdown }) {
  const rows = [
    ["成交量", breakdown.volumeScore],
    ["动态倍数", breakdown.dynamicScore],
    ["方向强度", breakdown.directionalScore],
    ["OI", breakdown.oiScore],
    ["价格响应", breakdown.priceScore],
    ["清算", breakdown.liquidationScore],
    ["持续性", breakdown.persistenceScore],
    ["Funding", breakdown.fundingScore],
    ["数据质量", breakdown.dataQualityScore],
    ["扣分", breakdown.penaltyScore],
  ];
  return (
    <div className="rounded-xl border border-slate-700 bg-slate-900/70 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-slate-500">Score Breakdown</p>
      <div className="mt-3 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        {rows.map(([label, value]) => (
          <div className="flex justify-between rounded-lg bg-slate-950/60 px-3 py-2" key={label}>
            <span>{label}</span>
            <span className="font-semibold text-slate-100">{Number(value || 0).toFixed(1)}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function AltImpactCard({ altImpactScore }) {
  const score = altImpactScore || {};
  const rows = [
    ["Market Impact", `${formatImpactRatio(score.marketImpactRatio)} · ${signedScore(score.marketImpactScore)}`],
    ["Liquidity Pressure", signedScore(score.liquidityImpact)],
    ["Cap Impact", score.capImpact ? signedScore(score.capImpact) : "N/A"],
    ["Direction", `${formatImpactRatio(score.directionalStrength)} · ${signedScore(score.directionalScore)}`],
    ["OI Confirmation", signedScore(score.oiConfirmation)],
    ["24h Reference", `${formatUsd(score.referenceVolume24hUsd || 0)} · ${impactReferenceLabel(score.referenceSource)}`],
  ];
  return (
    <div className="rounded-xl border border-amber-400/20 bg-amber-400/10 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-amber-300">Alt Impact Score</p>
      <div className="mt-3 flex items-end justify-between gap-3">
        <div>
          <p className="text-2xl font-bold text-white">{formatImpactScore(score)}</p>
          <p className="mt-1 text-xs text-amber-100">{score.interpretation || "暂无相对成交冲击解释"}</p>
        </div>
        <p className="rounded-full border border-amber-400/20 px-3 py-1 text-xs font-semibold text-amber-100">
          AIS
        </p>
      </div>
      <div className="mt-4 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        {rows.map(([label, value]) => (
          <div className="flex justify-between rounded-lg bg-slate-950/60 px-3 py-2" key={label}>
            <span>{label}</span>
            <span className="font-semibold text-slate-100">{value}</span>
          </div>
        ))}
      </div>
      <p className="mt-3 text-xs leading-5 text-slate-400">
        AIS 用相对市场冲击代替固定 USD 门槛：展示 ≥70，Discord gate ≥85，S 级 ≥90。当前没有真实市值/盘口深度时，Cap Impact 会显示 N/A。
      </p>
    </div>
  );
}

function LiquidityMicrostructureCard({ microstructure }) {
  const lme = microstructure || {};
  const tags = Array.isArray(lme.explanationTags) ? lme.explanationTags : [];
  const rows = [
    ["Order Flow Pressure", signedScore(lme.orderFlowPressure)],
    ["Absorption", signedScore(lme.absorptionStrength)],
    ["Imbalance", `${formatImpactRatio(Math.abs(Number(lme.imbalance || 0)))} · ${signedScore(lme.imbalanceScore)}`],
    ["Spread Behavior", signedScore(lme.spreadBehavior)],
    ["Spoofing", `${lmeSpoofingLabel(lme.spoofingState)} · ${signedScore(lme.spoofingPenalty)}`],
    ["Discord", lme.directDiscordGate ? "direct gate enabled" : "enhancement only"],
  ];
  return (
    <div className="rounded-xl border border-emerald-400/20 bg-emerald-400/10 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-emerald-300">Liquidity Microstructure</p>
      <div className="mt-3 flex items-end justify-between gap-3">
        <div>
          <p className="text-2xl font-bold text-white">{formatLms(lme.lmsScore)}</p>
          <p className="mt-1 text-xs text-emerald-100">{lme.interpretation || "暂无盘口结构解释"}</p>
        </div>
        <p className="rounded-full border border-emerald-400/20 px-3 py-1 text-xs font-semibold text-emerald-100">
          {lmeBehaviorLabel(lme.behavior)}
        </p>
      </div>
      <div className="mt-4 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        {rows.map(([label, value]) => (
          <div className="flex justify-between rounded-lg bg-slate-950/60 px-3 py-2" key={label}>
            <span>{label}</span>
            <span className="font-semibold text-slate-100">{value}</span>
          </div>
        ))}
      </div>
      <div className="mt-3 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        <div className="rounded-lg bg-slate-950/60 px-3 py-2">
          <span className="text-slate-500">Market Control</span>
          <p className="mt-1 font-semibold text-slate-100">{lmeControlLabel(lme.marketControl)}</p>
        </div>
        <div className="rounded-lg bg-slate-950/60 px-3 py-2">
          <span className="text-slate-500">Spread</span>
          <p className="mt-1 font-semibold text-slate-100">{lmeSpreadLabel(lme.spreadState)}</p>
        </div>
      </div>
      {tags.length ? (
        <div className="mt-3 flex flex-wrap gap-2 text-xs">
          {tags.map((tag) => (
            <span className="rounded-full border border-emerald-400/20 bg-emerald-400/10 px-3 py-1 text-emerald-100" key={tag}>
              {lmeTagLabel(tag)}
            </span>
          ))}
        </div>
      ) : null}
      <p className="mt-3 text-xs leading-5 text-slate-400">
        LME 只解释盘口微观结构，例如扫单、吸收、撤单和虚假流动性；它不会直接触发 Discord，也不代表交易执行建议。
      </p>
    </div>
  );
}

function MarketControlGraphCard({ graph }) {
  const mcg = graph || {};
  const nodes = Array.isArray(mcg.controlNodes) ? mcg.controlNodes : [];
  const edges = Array.isArray(mcg.controlEdges) ? mcg.controlEdges : [];
  const path = Array.isArray(mcg.controlPath) ? mcg.controlPath : [];
  const rows = [
    ["Dominant Side", mcgSideLabel(mcg.dominantSide)],
    ["Control Type", mcgControlTypeLabel(mcg.controlType)],
    ["Nodes", `${nodes.length}`],
    ["Edges", `${edges.length}`],
    ["Discord", mcg.directDiscordGate ? "direct gate enabled" : "enhancement only"],
  ];
  return (
    <div className="rounded-xl border border-violet-400/20 bg-violet-400/10 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-violet-300">Market Control Graph</p>
      <div className="mt-3 flex items-end justify-between gap-3">
        <div>
          <p className="text-2xl font-bold text-white">{formatCss(mcg.controlStrength)}</p>
          <p className="mt-1 text-xs text-violet-100">{mcg.interpretation || "暂无控制关系解释"}</p>
        </div>
        <p className="rounded-full border border-violet-400/20 px-3 py-1 text-xs font-semibold text-violet-100">
          {mcgControlTypeLabel(mcg.controlType)}
        </p>
      </div>
      <div className="mt-4 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        {rows.map(([label, value]) => (
          <div className="flex justify-between rounded-lg bg-slate-950/60 px-3 py-2" key={label}>
            <span>{label}</span>
            <span className="font-semibold text-slate-100">{value}</span>
          </div>
        ))}
      </div>
      {path.length ? (
        <div className="mt-3 flex flex-wrap gap-2 text-xs">
          {path.map((step) => (
            <span className="rounded-full border border-violet-400/20 bg-violet-400/10 px-3 py-1 text-violet-100" key={step}>
              {mcgPathLabel(step)}
            </span>
          ))}
        </div>
      ) : null}
      <div className="mt-3 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        <div className="rounded-lg bg-slate-950/60 px-3 py-2">
          <span className="text-slate-500">Control Nodes</span>
          <div className="mt-2 space-y-1">
            {nodes.length === 0 ? (
              <p className="text-slate-400">暂无节点</p>
            ) : nodes.slice(0, 4).map((node) => (
              <p className="font-semibold text-slate-100" key={node.id || node.label}>
                {node.label || node.id} · {mcgSideLabel(node.side)} · {formatCss(node.strength)}
              </p>
            ))}
          </div>
        </div>
        <div className="rounded-lg bg-slate-950/60 px-3 py-2">
          <span className="text-slate-500">Control Edges</span>
          <div className="mt-2 space-y-1">
            {edges.length === 0 ? (
              <p className="text-slate-400">暂无关系</p>
            ) : edges.slice(0, 4).map((edge) => (
              <p className="font-semibold text-slate-100" key={`${edge.from}-${edge.to}-${edge.relation}`}>
                {mcgRelationLabel(edge.relation)} · {formatCss(edge.strength)}
              </p>
            ))}
          </div>
        </div>
      </div>
      <p className="mt-3 text-xs leading-5 text-slate-400">
        MCG 是控制关系图谱，只解释谁在推价格、吸收、压制或诱导；它不直接触发 Discord，也不是交易建议。
      </p>
    </div>
  );
}

function McssCard({ masterCapitalStrength }) {
  const mcss = masterCapitalStrength || {};
  const rows = [
    ["市场层级", `${mcss.tier || "Unknown"} · x${Number(mcss.liquidityWeight || 0).toFixed(2)}`],
    ["成交强度", signedScore(mcss.notionalScore)],
    ["方向强度", signedScore(mcss.directionScore)],
    ["OI确认", signedScore(mcss.oiScore)],
    ["价格响应", signedScore(mcss.priceScore)],
    ["异常倍率", signedScore(mcss.anomalyScore)],
    ["清算惩罚", `-${Number(mcss.liquidationPenalty || 0).toFixed(1)}`],
  ];
  return (
    <div className="rounded-xl border border-cyan-400/20 bg-cyan-400/10 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-cyan-300">Master Capital Strength</p>
      <div className="mt-3 flex items-end justify-between gap-3">
        <div>
          <p className="text-2xl font-bold text-white">{formatMcss(mcss.mcss)}</p>
          <p className="mt-1 text-xs text-cyan-100">{mcss.interpretation || "暂无主力资金强度解释"}</p>
        </div>
        <p className="rounded-full border border-cyan-400/20 px-3 py-1 text-xs font-semibold text-cyan-100">
          {mcss.tier || "Unknown"}
        </p>
      </div>
      <div className="mt-4 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        {rows.map(([label, value]) => (
          <div className="flex justify-between rounded-lg bg-slate-950/60 px-3 py-2" key={label}>
            <span>{label}</span>
            <span className="font-semibold text-slate-100">{value}</span>
          </div>
        ))}
      </div>
      <p className="mt-3 text-xs leading-5 text-slate-400">
        MCSS 只用于跨市场资金强度解释，不替代 abnormalScore / buildScore，也不直接控制 Discord gate。
      </p>
    </div>
  );
}

function MarketRegimeCard({ marketRegime }) {
  const regime = marketRegime || {};
  const tags = Array.isArray(regime.explanationTags) ? regime.explanationTags : [];
  const rows = [
    ["OI", trendLabel(regime.oiTrend)],
    ["价格", trendLabel(regime.priceTrend)],
    ["5m", trendLabel(regime.trend5m)],
    ["15m", trendLabel(regime.trend15m)],
    ["效率", Number(regime.efficiencyRatio || 0).toFixed(4)],
    ["OI lag", Number(regime.oiLagIndex || 0).toFixed(2)],
  ];
  return (
    <div className="rounded-xl border border-fuchsia-400/20 bg-fuchsia-400/10 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-fuchsia-300">Market Regime</p>
      <div className="mt-3 flex items-end justify-between gap-3">
        <div>
          <p className="text-xl font-bold text-white">
            {marketRegimeLabel(regime.regime)} {formatConfidence(regime.confidence)}
          </p>
          <p className="mt-1 text-xs text-fuchsia-100">
            {regime.subType ? `${marketRegimeLabel(regime.subType)} · ` : ""}
            {regimeConclusion(regime)}
          </p>
        </div>
        <p className="rounded-full border border-fuchsia-400/20 px-3 py-1 text-xs font-semibold text-fuchsia-100">
          行为结构
        </p>
      </div>
      <div className="mt-4 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        {rows.map(([label, value]) => (
          <div className="flex justify-between rounded-lg bg-slate-950/60 px-3 py-2" key={label}>
            <span>{label}</span>
            <span className="font-semibold text-slate-100">{value}</span>
          </div>
        ))}
      </div>
      {tags.length ? (
        <div className="mt-3 flex flex-wrap gap-2 text-xs">
          {tags.map((tag) => (
            <span className="rounded-full border border-fuchsia-400/20 bg-fuchsia-400/10 px-3 py-1 text-fuchsia-100" key={tag}>
              {regimeTagLabel(tag)}
            </span>
          ))}
        </div>
      ) : null}
      <p className="mt-3 text-xs leading-5 text-slate-400">
        Regime 是滞后行为结构判断，用于区分吸筹、派发和操控；不替代 BACM 信号触发，也不直接控制 Discord gate。
      </p>
    </div>
  );
}

function SmartMoneyLifecycleCard({ lifecycle }) {
  const smle = lifecycle || {};
  const tags = Array.isArray(smle.explanationTags) ? smle.explanationTags : [];
  const path = Array.isArray(smle.statePath) && smle.statePath.length ? smle.statePath : [smle.lifecycleState || "Accumulation"];
  const rows = [
    ["持续时间", `${Number(smle.stateDurationMin || 0).toFixed(1)} min`],
    ["状态置信度", formatConfidence(smle.stateConfidence)],
    ["流动一致性", `${Math.round(Number(smle.flowConsistencyScore || 0))}/100`],
    ["生命周期分", `${Math.round(Number(smle.lifecycleScore || 0))}/100`],
    ["转移信号", smle.transitionSignal || "无"],
  ];
  return (
    <div className="rounded-xl border border-emerald-400/20 bg-emerald-400/10 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-emerald-300">Smart Money Lifecycle</p>
      <div className="mt-3 flex flex-col gap-3">
        <div className="flex items-end justify-between gap-3">
          <div>
            <p className="text-xl font-bold text-white">
              {lifecycleStateLabel(smle.lifecycleState)} {formatConfidence(smle.stateConfidence)}
            </p>
            <p className="mt-1 text-xs leading-5 text-emerald-100">
              {smle.currentExplanation || "生命周期结构仍未确认。"}
            </p>
          </div>
          <p className="rounded-full border border-emerald-400/20 px-3 py-1 text-xs font-semibold text-emerald-100">
            行为周期
          </p>
        </div>
        <div className="rounded-lg border border-emerald-400/10 bg-slate-950/50 px-3 py-2 text-xs font-semibold text-emerald-100">
          {path.map(lifecycleStateLabel).join(" → ")}
        </div>
      </div>
      <div className="mt-4 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        {rows.map(([label, value]) => (
          <div className="flex justify-between rounded-lg bg-slate-950/60 px-3 py-2" key={label}>
            <span>{label}</span>
            <span className="font-semibold text-slate-100">{value}</span>
          </div>
        ))}
      </div>
      {tags.length ? (
        <div className="mt-3 flex flex-wrap gap-2 text-xs">
          {tags.map((tag) => (
            <span className="rounded-full border border-emerald-400/20 bg-emerald-400/10 px-3 py-1 text-emerald-100" key={tag}>
              {lifecycleTagLabel(tag)}
            </span>
          ))}
        </div>
      ) : null}
      <p className="mt-3 text-xs leading-5 text-slate-400">
        SMLE 是时间序列状态机视角；BACM 是事件层，MCSS 是强度层，Regime 是行为类型，Lifecycle 才描述完整主力周期。
      </p>
    </div>
  );
}

function SmartMoneyPredictionCard({ prediction }) {
  const smp = prediction || {};
  const factors = Array.isArray(smp.triggerFactors) ? smp.triggerFactors : [];
  const rows = [
    ["当前阶段", lifecycleStateLabel(smp.currentState)],
    ["预测阶段", lifecycleStateLabel(smp.nextState)],
    ["时间窗口", `${Number(smp.timeHorizonMin || 0)} min`],
    ["方向偏好", `${predictionBiasLabel(smp.directionBias)} ${formatDirectionProbability(smp.directionProbability)}`],
    ["预测分", `${Math.round(Number(smp.predictionScore || 0))}/100`],
    ["信心", formatConfidence(smp.confidence)],
  ];
  return (
    <div className="rounded-xl border border-sky-400/20 bg-sky-400/10 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-sky-300">Smart Money Prediction</p>
      <div className="mt-3 flex items-end justify-between gap-3">
        <div>
          <p className="text-xl font-bold text-white">
            {lifecycleStateLabel(smp.nextState)} {formatConfidence(smp.probability)}
          </p>
          <p className="mt-1 text-xs leading-5 text-sky-100">
            {smp.explanation || "预测层等待生命周期确认。"}
          </p>
        </div>
        <p className="rounded-full border border-sky-400/20 px-3 py-1 text-xs font-semibold text-sky-100">
          下一阶段
        </p>
      </div>
      <div className="mt-4 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        {rows.map(([label, value]) => (
          <div className="flex justify-between rounded-lg bg-slate-950/60 px-3 py-2" key={label}>
            <span>{label}</span>
            <span className="font-semibold text-slate-100">{value}</span>
          </div>
        ))}
      </div>
      {factors.length ? (
        <div className="mt-3 flex flex-wrap gap-2 text-xs">
          {factors.map((factor) => (
            <span className="rounded-full border border-sky-400/20 bg-sky-400/10 px-3 py-1 text-sky-100" key={factor}>
              {predictionFactorLabel(factor)}
            </span>
          ))}
        </div>
      ) : null}
      <p className="mt-3 text-xs leading-5 text-slate-400">
        SMP 只预测主力行为阶段转移，不预测具体价格；Manipulation 仅作为噪音过滤，不直接改变主生命周期预测。
      </p>
    </div>
  );
}

function SignalConfidenceCard({ confidence }) {
  const scc = confidence || {};
  const breakdown = scc.breakdown || {};
  const reliabilityFactors = Array.isArray(scc.reliabilityFactors) ? scc.reliabilityFactors : [];
  const riskFactors = Array.isArray(scc.riskFactors) ? scc.riskFactors : [];
  const rows = [
    ["BACM Signal Strength", `${Math.round(Number(breakdown.bacmSignalStrength || 0))}/100`],
    ["MCSS Strength", `${Math.round(Number(breakdown.mcssStrength || 0))}/100`],
    ["SMLE Stability", `${Math.round(Number(breakdown.smleStability || 0))}/100`],
    ["SMP Alignment", `${Math.round(Number(breakdown.smpPredictionAlignment || 0))}/100`],
    ["LME Support", `${Math.round(Number(breakdown.lmeMicrostructureSupport || 0))}/100`],
    ["MCG Coherence", `${Math.round(Number(breakdown.mcgControlCoherence || 0))}/100`],
    ["SMAF Risk Penalty", `-${Math.round(Number(breakdown.smafRiskPenalty || 0))}/100`],
  ];
  return (
    <div className="rounded-xl border border-teal-400/20 bg-teal-400/10 p-4">
      <p className="text-xs uppercase tracking-[0.22em] text-teal-300">Signal Confidence Calibration</p>
      <div className="mt-3 flex items-end justify-between gap-3">
        <div>
          <p className="text-2xl font-bold text-white">{formatSccScore(scc.confidenceScore)}</p>
          <p className="mt-1 text-xs leading-5 text-teal-100">
            {scc.interpretation || "信号可信度不足或缺少多层确认"}
          </p>
        </div>
        <p className="rounded-full border border-teal-400/20 px-3 py-1 text-xs font-semibold text-teal-100">
          {sccLevelLabel(scc.confidenceLevel)}
        </p>
      </div>
      <div className="mt-4 grid gap-2 text-xs text-slate-300 sm:grid-cols-2">
        {rows.map(([label, value]) => (
          <div className="flex justify-between rounded-lg bg-slate-950/60 px-3 py-2" key={label}>
            <span>{label}</span>
            <span className="font-semibold text-slate-100">{value}</span>
          </div>
        ))}
      </div>
      <div className="mt-3 grid gap-3 text-xs md:grid-cols-2">
        <div>
          <p className="text-slate-500">Reliability Factors</p>
          <div className="mt-2 flex flex-wrap gap-2">
            {reliabilityFactors.length === 0 ? (
              <span className="text-slate-400">暂无可靠性加分因素</span>
            ) : reliabilityFactors.map((factor) => (
              <span className="rounded-full border border-teal-400/20 bg-teal-400/10 px-3 py-1 text-teal-100" key={factor}>
                {sccFactorLabel(factor)}
              </span>
            ))}
          </div>
        </div>
        <div>
          <p className="text-slate-500">Risk Factors</p>
          <div className="mt-2 flex flex-wrap gap-2">
            {riskFactors.length === 0 ? (
              <span className="text-slate-400">暂无明显风险扣分</span>
            ) : riskFactors.map((factor) => (
              <span className="rounded-full border border-rose-400/20 bg-rose-400/10 px-3 py-1 text-rose-100" key={factor}>
                {sccFactorLabel(factor)}
              </span>
            ))}
          </div>
        </div>
      </div>
      <p className="mt-3 text-xs leading-5 text-slate-400">
        SCC 是最终可信度校准层，只回答“我有多信”；它不会改变 BACM 信号生成、Discord gate 或任何交易执行路径。
      </p>
    </div>
  );
}

function Detail({ label, value, wide = false }) {
  return (
    <div className={`rounded-xl border border-slate-700/60 bg-slate-900/70 p-3 ${wide ? "md:col-span-3" : ""}`}>
      <p className="text-xs text-slate-500">{label}</p>
      <p className="mt-1 text-sm font-semibold text-slate-100">{value || "N/A"}</p>
    </div>
  );
}

function MetricTile({ label, value }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/70 px-3 py-2">
      <p className="text-slate-500">{label}</p>
      <p className="mt-1 font-semibold text-slate-100">{value}</p>
    </div>
  );
}

function StatusPill({ label, value, tone }) {
  const toneClass = {
    cyan: "border-cyan-400/30 text-cyan-100",
    red: "border-red-400/30 text-red-100",
    yellow: "border-yellow-400/30 text-yellow-100",
    slate: "border-slate-700 text-slate-300",
    emerald: "border-emerald-400/30 text-emerald-100",
  }[tone || "slate"];
  return (
    <div className={`rounded-xl border bg-slate-950/60 px-3 py-2 ${toneClass}`}>
      <p className="text-[11px] text-slate-500">{label}</p>
      <p className="mt-1 font-bold">{value}</p>
    </div>
  );
}

function HeaderCell({ children }) {
  return <th className="min-w-[110px] px-3 py-2 font-medium">{children}</th>;
}

function Cell({ children }) {
  return <td className="px-3 py-3">{children}</td>;
}

function shouldUseHistory(filters) {
  return Object.entries(filters).some(([key, value]) => key !== "symbol" && value !== "all");
}

function hasAltImpactSnapshot(item) {
  const score = item?.altImpactScore || {};
  return Boolean(
    Number(score.finalScore || 0) > 0 ||
      Number(score.marketImpactRatio || 0) > 0 ||
      Number(score.liquidityImpact || 0) > 0 ||
      Number(score.directionalScore || 0) > 0,
  );
}

function fallbackSummary() {
  return {
    ...{
      status: "calm",
      healthStatus: "disabled",
      latestDirection: "neutral",
      latestSeverity: "calm",
      monitoredSymbols: [],
      displayMinNotionalUsd: 500_000,
      displayThresholdsUsd: {
        ultraCore: 750_000,
        mainstream: 500_000,
        alt: 150_000,
      },
      collectorStatus: "disabled",
      lastTradeAt: null,
      lastOiPollAt: null,
      lastForceOrderAt: null,
      flowBuckets1m: 0,
      topActiveSymbols: [],
      errors1h: 0,
      activeAnomalyCount: 0,
      recentCriticalOrSCount: 0,
      dryRunWouldSendCount: 0,
      enabled: false,
      dryRun: true,
      trend60s: {},
      exchanges: {},
      dryRunStats: {},
      symbolUniverse: {},
      allMarketContext: {},
      smafReport: {
        dataAudit: { freshnessScore: 0, completenessScore: 0, consistencyScore: 0, integrityScore: 0, dataRiskLevel: "disabled" },
        signalAudit: { noiseRatio: 0, duplicationRate: 0, singleSourceDependency: 0, falseSignalEstimate: 0, integrityScore: 100 },
        behaviorAudit: { stateStability: 100, transitionEntropy: 0, manipulationNoise: 0, structuralIntegrity: 100 },
        predictionAudit: { accuracy: 100, flipRate: 0, overfittingScore: 0, followThroughRate: 100, integrityScore: 100 },
        smafScore: 0,
        riskLevel: "disabled",
        criticalIssues: [],
      },
      smllReport: {
        enabled: true,
        protectedRealtime: true,
        status: "collecting_outcomes",
        learningScore: 0,
        sampleSize: 0,
        minSamplesForUpdate: 3,
        accuracyRate: 100,
        wrongCount: 0,
        neutralCount: 0,
        outcomeRecords: [],
        errorReports: [],
        suggestedWeights: { volumeWeight: 1, oiWeight: 1, priceWeight: 1, liquidationWeight: 1, fundingWeight: 1 },
        driftReport: { driftDetected: false, affectedComponents: [], suggestedRetrain: false, reason: "no_material_drift" },
        calibrationUpdates: [],
      },
      atcaReport: {
        enabled: true,
        protectedRealtime: true,
        cognitionStatus: "waiting_for_signals",
        memorySummary: "short_memory=0 symbols",
        perceptionCount: 0,
        interpretationCount: 0,
        intentionCount: 0,
        predictionCount: 0,
        decisionCount: 0,
        agents: [],
      },
      amiosReport: {
        enabled: true,
        protectedRealtime: true,
        osStatus: "idle",
        marketState: "CALM",
        kernelLoad: 0,
        signalThroughput: "quiet",
        confidence: 0,
        risk: "normal",
        activeProcesses: [],
        currentStates: [],
        schedulerDecision: "standby",
        auditSummary: "smaf=0 smll_samples=0 atca=waiting_for_signals read_only=true direct_discord_gate=false",
        readOnly: true,
        directDiscordGate: false,
      },
    },
  };
}

function formatAuditScore(value) {
  const number = Number(value || 0);
  return `${Math.round(number)}/100`;
}

function smafRiskLabel(value) {
  return {
    "production ready": "Production Ready",
    "stable but tuning needed": "Stable but tuning",
    risky: "Risky",
    "not reliable": "Not reliable",
    disabled: "Disabled",
  }[String(value || "").toLowerCase()] || value || "Unknown";
}

function smafToneClass(score) {
  const value = Number(score || 0);
  if (value >= 90) return "border-emerald-400/30 bg-emerald-400/10 text-emerald-100";
  if (value >= 75) return "border-cyan-400/30 bg-cyan-400/10 text-cyan-100";
  if (value >= 60) return "border-yellow-400/30 bg-yellow-400/10 text-yellow-100";
  return "border-red-400/30 bg-red-400/10 text-red-100";
}

function smafIssueLabel(value) {
  return {
    data_integrity_low: "数据完整性偏低",
    single_source_dependency_high: "单源依赖过高",
    duplicate_signal_rate_high: "重复信号偏多",
    lifecycle_transition_entropy_high: "生命周期切换过乱",
    manipulation_noise_high: "操控噪音偏高",
    prediction_flip_rate_high: "预测翻转偏高",
    prediction_overfitting_risk: "预测过拟合风险",
  }[String(value || "").toLowerCase()] || value;
}

function smllStatusLabel(value) {
  return {
    collecting_outcomes: "收集结果中",
    stable_learning: "稳定学习",
    calibration_suggested: "建议校准",
    drift_watch: "结构漂移观察",
  }[String(value || "").toLowerCase()] || value || "未知";
}

function smllErrorLabel(value) {
  return {
    data_quality_or_latency: "数据质量/延迟",
    oi_confirmation_misled_direction: "OI 误导方向",
    liquidation_context_misread_as_build: "清算误判建仓",
    lifecycle_or_regime_confidence_overstated: "行为置信过高",
    smp_direction_or_stage_followthrough_failed: "预测未跟随",
  }[String(value || "").toLowerCase()] || value || "未知误差";
}

function formatWeight(value) {
  return `x${Number(value ?? 1).toFixed(2)}`;
}

function formatPercentNumber(value) {
  return `${Math.round(Number(value || 0))}%`;
}

function atcaStatusLabel(value) {
  return {
    waiting_for_signals: "等待信号",
    active_cognition: "主动认知",
    degraded_cognition: "降级认知",
  }[String(value || "").toLowerCase()] || value || "未知";
}

function atcaIntentLabel(value) {
  return {
    accumulate: "吸筹意图",
    distribute: "派发意图",
    trap: "诱多/陷阱",
    stop_hunt: "扫止损",
    trend_drive: "趋势推动",
    exit_liquidity: "退出流动性",
    monitor: "观察",
  }[String(value || "").toLowerCase()] || value || "观察";
}

function atcaRiskLabel(value) {
  return {
    high: "高",
    medium: "中",
    low: "低",
    liquidation_risk: "清算风险",
  }[String(value || "").toLowerCase()] || value || "低";
}

function amiosStatusLabel(value) {
  return {
    idle: "待机",
    running: "运行中",
    degraded: "降级",
  }[String(value || "").toLowerCase()] || value || "未知";
}

function amiosMarketStateLabel(value) {
  return {
    calm: "CALM",
    observation_mode: "观察模式",
    behavior_process_mode: "行为进程模式",
    active_control_mode: "主动控盘模式",
    interrupt_liquidation_mode: "清算中断模式",
  }[String(value || "").toLowerCase()] || value || "观察模式";
}

function amiosThroughputLabel(value) {
  return {
    quiet: "安静",
    low: "低",
    normal: "正常",
    high: "高",
  }[String(value || "").toLowerCase()] || value || "未知";
}

function amiosDecisionLabel(value) {
  return {
    standby: "待机",
    observe_candidate: "观察候选",
    monitor_high_confidence: "高置信观察",
    interrupt_priority: "中断优先",
  }[String(value || "").toLowerCase()] || value || "观察";
}

function amiosRiskLabel(value) {
  return {
    normal: "正常",
    low: "低",
    medium: "中",
    high: "高",
    market_risk: "市场风险",
    high_market_risk: "高市场风险",
    liquidation_interrupt: "清算中断",
    liquidation_risk: "清算风险",
    model_drift_watch: "模型漂移观察",
    system_risk: "系统风险",
  }[String(value || "").toLowerCase()] || value || "正常";
}

function amiosLayerLabel(value) {
  return {
    kernel: "Kernel",
    process: "Process",
    scheduler: "Scheduler",
    audit: "Audit",
    graph: "Graph",
  }[String(value || "").toLowerCase()] || value || "Process";
}

function amiosProcessStatusLabel(value) {
  return {
    standby: "待机",
    active: "运行",
    interrupt: "中断",
    stable: "稳定",
    watch: "观察",
    collecting: "采集",
    observing: "观察中",
    drift_watch: "漂移观察",
    calibrated: "已校准",
    active_cognition: "主动认知",
    degraded_cognition: "降级认知",
    waiting_for_signals: "等待信号",
  }[String(value || "").toLowerCase()] || value || "运行";
}

function universeModeLabel(value) {
  return {
    all_binance_usdt_perp: "全 Binance USDT 永续",
    top_n: "Top-N 调试模式",
    whitelist_only: "白名单调试模式",
    auto: "自动模式",
  }[String(value || "").toLowerCase()] || "全 Binance USDT 永续";
}

function formatTierCounts(counts) {
  if (!counts || typeof counts !== "object") return "A0 / B0 / C0 / D0 / E0";
  return ["A", "B", "C", "D", "E"].map((tier) => `${tier}${counts[tier] || counts[tier.toLowerCase()] || 0}`).join(" / ");
}

function collectorStatusLabel(value) {
  return {
    running: "运行中",
    connecting: "连接中",
    waiting_data: "等待数据",
    disabled: "未启用",
  }[String(value || "").toLowerCase()] || value || "未知";
}

function contextStatus(connected, lastAt) {
  if (connected && lastAt) return `在线 ${relativeAge(lastAt)}`;
  if (connected) return "在线";
  if (lastAt) return `最近 ${relativeAge(lastAt)}`;
  return "等待";
}

function statusLabel(value) {
  return { calm: "平静", watch: "观察", active: "异动", strong: "强异动" }[value] || "平静";
}

function statusTone(value) {
  return { calm: "slate", watch: "yellow", active: "cyan", strong: "red" }[value] || "slate";
}

function healthStatusLabel(value) {
  return { healthy: "健康", degraded: "降级", unhealthy: "异常", disabled: "未启用" }[value] || "未知";
}

function healthStatusTone(value) {
  return { healthy: "emerald", degraded: "yellow", unhealthy: "red", disabled: "slate" }[value] || "slate";
}

function severityLabel(value) {
  return { s: "S", critical: "Critical", high: "High", medium: "Medium", calm: "Calm" }[String(value || "").toLowerCase()] || "High";
}

function severityBadgeClass(value) {
  return {
    s: "bg-fuchsia-400/15 text-fuchsia-200",
    critical: "bg-red-400/15 text-red-200",
    high: "bg-orange-400/15 text-orange-200",
    medium: "bg-yellow-400/15 text-yellow-200",
  }[String(value || "").toLowerCase()] || "bg-slate-700 text-slate-200";
}

function marketTierLabel(value) {
  return {
    ultra_core: "Ultra Core",
    ultracore: "Ultra Core",
    mainstream: "Mainstream",
    alt: "Alt",
  }[String(value || "alt").toLowerCase()] || "Alt";
}

function marketRegimeLabel(value) {
  return {
    accumulation: "Accumulation",
    distribution: "Distribution",
    manipulation: "Manipulation",
    manipulation_up: "Manipulation UP",
    manipulation_down: "Manipulation DOWN",
    liquidity_trap: "Liquidity Trap",
    stop_hunt: "Stop Hunt",
    unclear: "Unclear",
  }[String(value || "unclear").toLowerCase()] || value || "Unclear";
}

function lifecycleStateLabel(value) {
  return {
    accumulation: "Accumulation",
    markup: "Markup",
    distribution: "Distribution",
    markdown: "Markdown",
    reaccumulation: "Re-Accumulation",
    re_accumulation: "Re-Accumulation",
  }[String(value || "accumulation").toLowerCase()] || value || "Accumulation";
}

function signalTypeLabel(value) {
  return {
    main_force_long_build: "主力建多",
    mainforcelongbuild: "主力建多",
    main_force_short_build: "主力建空",
    mainforceshortbuild: "主力建空",
    abnormal_pump: "异常拉升",
    abnormalpump: "异常拉升",
    abnormal_dump: "异常下跌",
    abnormaldump: "异常下跌",
    downside_absorption: "下方吸收",
    downsideabsorption: "下方吸收",
    upside_resistance: "上方压制",
    upsideresistance: "上方压制",
    liquidation_cascade: "清算瀑布",
    liquidationcascade: "清算瀑布",
    unclear_contract_anomaly: "合约异动待确认",
    unclearcontractanomaly: "合约异动待确认",
  }[String(value || "").toLowerCase()] || "山寨合约异动";
}

function directionLabel(value) {
  return {
    buy: "主动买入",
    sell: "主动卖出",
    absorption: "下方吸收",
    suppression: "上方压制",
    neutral: "中性",
  }[String(value || "").toLowerCase()] || "中性";
}

function exchangeStatusLabel(value) {
  return {
    connected: "在线",
    connecting: "连接中",
    reconnecting: "重连中",
    disabled: "未启用",
    stale: "无近期成交",
    degraded: "降级",
  }[String(value || "").toLowerCase()] || "离线";
}

function exchangeStatusClass(item) {
  const status = String(item?.status || "").toLowerCase();
  if (status === "connected" && item?.connected) return "font-bold text-emerald-300";
  if (status === "reconnecting" || status === "connecting") return "font-bold text-yellow-300";
  if (status === "stale" || status === "degraded") return "font-bold text-orange-300";
  return "font-bold text-slate-400";
}

function discordStatus(item) {
  if (item.discordSent) return "已推送";
  if (item.discordWouldSend) return "dry-run would_send";
  if (item.discordEligible) return "符合 gate";
  return discordReasonLabel(item.discordReason || "display");
}

function discordAlertKindLabel(value) {
  return {
    main_force_build: "主力建仓",
    extreme_impulse: "极端异常",
    liquidation_shock: "清算冲击",
    market_wide_summary: "集体异动",
    display_only: "仅展示",
    none: "未进入",
  }[String(value || "none").toLowerCase()] || value || "未进入";
}

function discordReasonLabel(value) {
  return {
    dry_run_would_send: "dry-run would_send",
    main_force_build: "主力建仓 gate 通过",
    extreme_impulse: "极端异常 gate 通过",
    liquidation_shock: "清算冲击 gate 通过",
    low_score: "评分或证据不足",
    impact_score_low: "相对冲击 AIS 不足",
    medium_or_low: "Medium/Low 仅展示",
    display: "仅展示",
    low_display_notional: "低于展示名义额门槛",
    tier_notional_low: "低于 Tier 推送门槛",
    tier_critical_notional_low: "低于 Tier Critical 门槛",
    tier_guard: "Tier 默认不推",
    tier_d_guard: "Tier D 保护",
    low_liquidity_tier_guard: "低流动性 Tier 保护",
    tier_requires_non_liquidation: "该 Tier 要求非清算",
    tier_s_disabled: "该 Tier 不允许 S 推送",
    main_force_evidence_low: "主力证据不足",
    liquidation_evidence_low: "清算证据不足",
    liquidation_alerts_disabled: "清算提醒关闭",
    market_wide_not_top: "集体异动非 Top 强度",
    global_hourly_cap: "每小时限流",
    cooldown: "冷却中",
    duplicate: "重复信号",
    data_quality_low: "数据质量不足",
    warmup: "启动预热中",
    webhook_missing: "Webhook 未配置",
    live_send_not_enabled_for_bacm: "BACM live 发送未启用",
    disabled: "Discord gate 关闭",
    not_sent: "未推送",
    not_evaluated: "未评估",
  }[String(value || "").toLowerCase()] || value || "未推送";
}

function liquidationContextText(signal) {
  const notional = formatUsd(signal.liquidationNotionalUsd || 0);
  if (signal.liquidationSuspected) {
    return `疑似清算驱动 · 清算名义额 ${notional}`;
  }
  if (signal.forceOrderSnapshot) {
    return `有强平快照 · 清算名义额 ${notional}`;
  }
  return "未标记清算驱动";
}

function postSignalStatusLabel(value) {
  return {
    pending: "等待验证",
    validated: "已验证",
    failed: "验证失败",
    trap: "诱多/诱空失败",
  }[String(value || "pending").toLowerCase()] || "等待验证";
}

function oiQualityLabel(value) {
  return {
    fresh: "新鲜",
    stale: "过期",
    missing: "缺失",
  }[String(value || "missing").toLowerCase()] || "缺失";
}

function fundingCrowdingLabel(value) {
  return {
    long_overcrowded: "多头拥挤追多",
    short_overcrowded: "空头拥挤追空",
    anti_crowded_short_build: "反拥挤建空",
    anti_crowded_long_build: "反拥挤建多",
    neutral: "中性",
    unknown: "未知",
  }[String(value || "unknown").toLowerCase()] || "未知";
}

function marketWideText(signal) {
  const ratio = Math.round(Number(signal.marketImpulseRatio || 0) * 1000) / 10;
  if (!signal.marketWideMove) return "否";
  const rank = signal.relativeStrengthRank ? ` · 相对强度 #${signal.relativeStrengthRank}` : "";
  return `是 · 同向 ${ratio}%${rank}`;
}

function evidenceTagLabel(tag) {
  return {
    aggressive_buy_dominant: "主动买入占优",
    aggressive_sell_dominant: "主动卖出占优",
    dynamic_multiple_high: "动态倍数 High",
    dynamic_multiple_critical: "动态倍数 Critical",
    oi_expanding: "OI 扩张",
    oi_contracting: "OI 收缩",
    price_follow_through: "价格跟随",
    price_absorption: "价格吸收",
    not_liquidation_driven: "非清算驱动",
    funding_not_overcrowded: "Funding 不拥挤",
    multi_window_confirmed: "多窗口确认",
    market_relative_strength: "相对强势",
    market_relative_weakness: "相对弱势",
  }[tag] || tag;
}

function formatTime(ts) {
  if (!ts) return "N/A";
  return new Date(ts).toLocaleTimeString("zh-CN", { hour12: false });
}

function formatUsd(value) {
  const number = Number(value || 0);
  if (number >= 1_000_000) return `$${(number / 1_000_000).toFixed(1)}M`;
  return `$${Math.round(number).toLocaleString("en-US")}`;
}

function formatMcss(value) {
  const number = Number(value || 0);
  return `${Math.round(number)}/100`;
}

function formatLms(value) {
  const number = Number(value || 0);
  return `${Math.round(number)}/100`;
}

function formatCss(value) {
  const number = Number(value || 0);
  return `${Math.round(number)}/100`;
}

function formatSccScore(value) {
  const number = Number(value || 0);
  return `${Math.round(number)}/100`;
}

function sccLevelLabel(value) {
  return {
    very_high: "极高可信",
    veryhigh: "极高可信",
    high: "高可信",
    medium: "中等可信",
    weak: "弱信号",
    noise: "噪音",
  }[String(value || "noise").replace(/[\s-]/g, "").toLowerCase()] || value || "噪音";
}

function formatImpactScore(value) {
  const score = typeof value === "number" ? value : Number(value?.finalScore || 0);
  return `${Math.round(score)}/100`;
}

function formatImpactRatio(value) {
  const number = Number(value || 0);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  return `${(number * 100).toFixed(number >= 0.01 ? 2 : 3)}%`;
}

function impactReferenceLabel(value) {
  return {
    ticker_quote_volume_24h: "24h 成交量",
    synthetic_tier_volume_proxy: "Tier 代理参考",
    unavailable: "无参考源",
  }[String(value || "unavailable").toLowerCase()] || value || "无参考源";
}

function lmeBehaviorLabel(value) {
  return {
    liquiditysweepup: "上扫流动性",
    liquiditysweepdown: "下扫流动性",
    absorption_buy: "下方吸收",
    absorption_sell: "上方吸收",
    absorption: "吸收",
    spoofingdetected: "虚假挂单",
    liquiditypullup: "上方撤单",
    liquiditypulldown: "下方撤单",
    bullishimbalance: "买盘失衡",
    bearishimbalance: "卖盘失衡",
    ordinaryflow: "普通流",
  }[String(value || "ordinaryflow").replace(/[_\s-]/g, "").toLowerCase()] || value || "普通流";
}

function lmeControlLabel(value) {
  return {
    buyer_side_control: "买方控盘",
    seller_side_control: "卖方控盘",
    two_sided_absorption: "双边吸收",
    fake_liquidity_control: "虚假流动性控制",
    no_clear_control: "暂无明确控盘",
  }[String(value || "no_clear_control").toLowerCase()] || value || "暂无明确控盘";
}

function lmeSpreadLabel(value) {
  return {
    widening: "价差扩大",
    stable: "价差稳定",
    unknown: "缺少 L2 上下文",
  }[String(value || "unknown").toLowerCase()] || value || "缺少 L2 上下文";
}

function lmeSpoofingLabel(value) {
  return {
    detected: "已检测",
    watch: "观察",
    none: "未发现",
  }[String(value || "none").toLowerCase()] || value || "未发现";
}

function lmeTagLabel(tag) {
  return {
    read_only_microstructure: "只读盘口解释",
    liquidity_sweep: "流动性扫单",
    absorption: "吸收",
    spoofing_detected: "虚假挂单",
    liquidity_pull: "撤单控盘",
    bullish_imbalance: "买盘偏斜",
    bearish_imbalance: "卖盘偏斜",
    aggressive_buy_pressure: "主动买入压力",
    aggressive_sell_pressure: "主动卖出压力",
    depth_skew: "盘口偏斜",
    spread_widening: "价差扩大",
    fake_liquidity_watch: "虚假流动性观察",
    price_absorption: "价格吸收",
    aggressive_taker_flow: "主动吃单流",
  }[String(tag || "").toLowerCase()] || tag;
}

function mcgControlTypeLabel(value) {
  return {
    controlaccumulation: "控制吸筹",
    controldistribution: "控制派发",
    controlmanipulation: "操控市场",
    noclearcontrol: "未确认控盘",
  }[String(value || "NoClearControl").replace(/[_\s-]/g, "").toLowerCase()] || value || "未确认控盘";
}

function mcgSideLabel(value) {
  return {
    buy: "买方",
    sell: "卖方",
    neutral: "中性",
    absorption: "吸收",
    suppression: "压制",
  }[String(value || "neutral").toLowerCase()] || value || "中性";
}

function mcgRelationLabel(value) {
  return {
    control_relation: "控制关系",
    absorption_relation: "吸收关系",
    manipulation_relation: "操控关系",
    liquidity_transfer: "流动性转移",
    pressure_flow: "压力流",
  }[String(value || "").toLowerCase()] || value || "控制关系";
}

function mcgPathLabel(value) {
  return {
    "Bid absorption": "买盘吸收",
    "Price containment": "价格锁定",
    "Potential markup preparation": "潜在拉升准备",
    "Ask absorption": "卖盘吸收",
    "Breakout suppression": "突破压制",
    "Liquidity exit": "流动性出货",
    "Liquidity shaping": "流动性塑形",
    "Cognitive trap": "认知诱导",
    "Sweep or revert risk": "扫单/回撤风险",
    "No stable control path": "暂无稳定控盘路径",
  }[String(value || "")] || value;
}

function sccFactorLabel(factor) {
  return {
    bacm_signal_strong: "BACM 信号强",
    mcss_strong_money: "MCSS 资金强",
    smle_stable_lifecycle: "SMLE 生命周期稳定",
    smp_aligned: "SMP 预测一致",
    lme_orderbook_support: "LME 盘口支持",
    mcg_control_coherent: "MCG 控制一致",
    dynamic_multiple_confirmed: "动态倍数确认",
    data_quality_ok: "数据质量达标",
    data_quality_low: "数据质量不足",
    liquidation_interference: "清算干扰",
    oi_contracting: "OI 收缩",
    prediction_misaligned: "预测不一致",
    spoofing_or_fake_liquidity: "虚假流动性风险",
    control_manipulation_risk: "操控风险",
    market_wide_noise: "全市场噪音",
  }[String(factor || "").toLowerCase()] || factor;
}

function signedScore(value) {
  const number = Number(value || 0);
  const prefix = number > 0 ? "+" : "";
  return `${prefix}${number.toFixed(1)}`;
}

function formatConfidence(value) {
  const number = Number(value || 0);
  return `(${Math.round(number)}%)`;
}

function trendLabel(value) {
  return {
    up: "上升",
    down: "下降",
    flat: "横盘",
    slow_up: "缓慢上升",
    spike_up: "快速上冲",
    spike_down: "快速下杀",
    unknown: "未知",
  }[String(value || "unknown").toLowerCase()] || value || "未知";
}

function regimeConclusion(regime) {
  const key = String(regime?.regime || "unclear").toLowerCase();
  const subType = String(regime?.subType || "").toLowerCase();
  if (key === "accumulation") return "主力吸筹阶段";
  if (key === "distribution") return "主力派发阶段";
  if (key === "manipulation" && subType === "liquidity_trap") return "假突破诱导流动性";
  if (key === "manipulation" && subType === "manipulation_down") return "砸盘诱空 / 扫止损";
  if (key === "manipulation" && subType === "manipulation_up") return "拉升诱多 / 挤空";
  if (key === "manipulation") return "操控/诱导阶段";
  return "结构未确认";
}

function regimeTagLabel(tag) {
  return {
    oi_expanding: "OI 上升",
    oi_contracting: "OI 收缩",
    price_absorption: "价格吸收",
    price_breakout_failed: "突破失败",
    liquidity_trap: "流动性陷阱",
    stop_hunt: "扫止损",
    smart_money_accumulating: "主力吸筹",
    distribution_pressure: "派发压力",
    fake_breakout: "假突破",
    trend_following: "趋势跟随",
  }[String(tag || "").toLowerCase()] || tag;
}

function lifecycleTagLabel(tag) {
  return {
    oi_expansion: "OI 扩张",
    oi_contraction: "OI 收缩",
    flow_consistent: "行为一致",
    mcss_confirmed: "MCSS 确认",
    liquidation_disturbance: "清算扰动",
    low_price_efficiency: "价格效率下降",
    manipulation_disturbance: "操控插入事件",
  }[String(tag || "").toLowerCase()] || tag;
}

function predictionBiasLabel(value) {
  return {
    bullish: "Bullish",
    bearish: "Bearish",
    bearishrisk: "Bearish Risk",
    reboundwatch: "Rebound Watch",
    sideways: "Sideways",
  }[String(value || "sideways").toLowerCase()] || value || "Sideways";
}

function predictionFactorLabel(factor) {
  return {
    oi_mcss_expansion: "OI + MCSS 扩张",
    oi_momentum_divergence: "OI 动能背离",
    efficiency_decay: "效率衰减",
    liquidity_stress: "流动性压力",
    funding_extreme: "Funding 极端",
    market_structure_consistent: "结构一致",
    lifecycle_confidence_high: "生命周期信心高",
    mcss_acceleration: "MCSS 加速",
    manipulation_noise_filtered: "操控噪音已过滤",
  }[String(factor || "").toLowerCase()] || factor;
}

function formatDirectionProbability(value) {
  const number = Number(value || 0);
  return `(${number.toFixed(2)})`;
}

function formatPrice(value) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  if (number >= 1000) return `$${Math.round(number).toLocaleString("en-US")}`;
  if (number >= 1) return `$${number.toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  return `$${number.toLocaleString("en-US", { minimumFractionDigits: 4, maximumFractionDigits: 4 })}`;
}

function formatSignedBase(value, symbol) {
  if (value === null || value === undefined) return "N/A";
  const number = Number(value || 0);
  return `${number >= 0 ? "+" : ""}${number.toLocaleString("en-US", { maximumFractionDigits: 2 })} ${symbol}`;
}

function formatBase(value, symbol) {
  if (value === null || value === undefined) return "N/A";
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) return "N/A";
  return `${number.toLocaleString("en-US", { maximumFractionDigits: 2 })} ${symbol}`;
}

function formatSignedNumber(value) {
  const number = Number(value || 0);
  return `${number >= 0 ? "+" : ""}${number.toLocaleString("en-US", { maximumFractionDigits: 2 })}`;
}

function signedNumber(value) {
  const number = Number(value || 0);
  return `${number >= 0 ? "+" : ""}${number}`;
}

function formatSignedPct(value) {
  if (value === null || value === undefined) return "N/A";
  const number = Number(value);
  return `${number >= 0 ? "+" : ""}${number.toFixed(3)}%`;
}

function relativeAge(ts) {
  const diff = Math.max(0, Date.now() - Number(ts));
  if (diff < 60_000) return `${Math.round(diff / 1000)} 秒前`;
  if (diff < 3_600_000) return `${Math.round(diff / 60_000)} 分钟前`;
  return `${Math.round(diff / 3_600_000)} 小时前`;
}
