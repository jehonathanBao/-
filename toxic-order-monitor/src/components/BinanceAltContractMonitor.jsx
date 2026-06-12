import { useEffect, useMemo, useState } from "react";
import {
  fetchBinanceAltContractHistory,
  fetchBinanceAltContractLatest,
  fetchBinanceAltContractSummary,
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
  const symbolOptions = useMemo(
    () => ["all", ...(summary.monitoredSymbols || []).map((symbol) => symbol.replace(/USDT$/, ""))],
    [summary.monitoredSymbols],
  );
  const selectedSignal = state.items.find((item) => item.id === selectedSignalId) || null;

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
        ) : state.items.length === 0 ? (
          <p className="rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
            {summary.enabled ? "暂无山寨合约异常" : "山寨合约异常监控未启用"}
          </p>
        ) : (
          <table className="min-w-full table-fixed text-left text-xs">
            <thead className="bg-slate-950/80 text-slate-400">
              <tr>
                <HeaderCell>时间</HeaderCell>
                <HeaderCell>币种 / 价格</HeaderCell>
                <HeaderCell>类型</HeaderCell>
                <HeaderCell>等级</HeaderCell>
                <HeaderCell>窗口</HeaderCell>
                <HeaderCell>异常分</HeaderCell>
                <HeaderCell>建仓分</HeaderCell>
                <HeaderCell>方向</HeaderCell>
                <HeaderCell>1m 名义额</HeaderCell>
                <HeaderCell>成交额门槛</HeaderCell>
                <HeaderCell>异常倍数</HeaderCell>
                <HeaderCell>OI</HeaderCell>
                <HeaderCell>价格变化</HeaderCell>
                <HeaderCell>清算</HeaderCell>
                <HeaderCell>Discord</HeaderCell>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800 text-slate-300">
              {state.items.map((item) => (
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
                  <Cell>{signalTypeLabel(item.signalType)}</Cell>
                  <Cell><span className={`rounded-full px-2 py-1 font-bold ${severityBadgeClass(item.severity)}`}>{severityLabel(item.severity)}</span></Cell>
                  <Cell>{item.windowSec}s</Cell>
                  <Cell>{item.abnormalScore}/100</Cell>
                  <Cell>{item.buildScore}/100</Cell>
                  <Cell>{directionLabel(item.direction)} {signedNumber(item.directionBias)}</Cell>
                  <Cell>{formatUsd(item.totalNotionalUsd)}</Cell>
                  <Cell>{formatUsd(item.sGradeNotionalThresholdUsd)}</Cell>
                  <Cell>{item.dynamicMultiple ? `${item.dynamicMultiple.toFixed(1)}x` : "N/A"}</Cell>
                  <Cell>{formatSignedBase(item.oiChange1mBase ?? item.oiChange5mBase, item.symbol)}</Cell>
                  <Cell>{formatSignedPct(item.priceMovePct)}</Cell>
                  <Cell>{item.liquidationSuspected ? "疑似" : "否"}</Cell>
                  <Cell>{discordStatus(item)}</Cell>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {selectedSignal ? (
        <AltSignalDetail onClose={() => setSelectedSignalId(null)} signal={selectedSignal} />
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

function AltSignalDetail({ signal, onClose }) {
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
          <Detail label="Discord dry-run" value={discordStatus(signal)} />
          <Detail label="最终判断" value={signal.finalResult} wide />
        </div>
        <div className="mt-5 grid gap-3 md:grid-cols-2">
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

function Detail({ label, value, wide = false }) {
  return (
    <div className={`rounded-xl border border-slate-700/60 bg-slate-900/70 p-3 ${wide ? "md:col-span-3" : ""}`}>
      <p className="text-xs text-slate-500">{label}</p>
      <p className="mt-1 text-sm font-semibold text-slate-100">{value || "N/A"}</p>
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

function fallbackSummary() {
  return {
    ...{
      status: "calm",
      healthStatus: "disabled",
      latestDirection: "neutral",
      latestSeverity: "calm",
      monitoredSymbols: [],
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
    },
  };
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
  return item.discordReason || "展示";
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
