import { useEffect, useState } from "react";
import {
  fetchSpotWhaleHistory,
  fetchSpotWhaleLatest,
  fetchSpotWhaleSummary,
} from "../api/spotWhale.js";

const SUMMARY_REFRESH_MS = 5_000;
const LATEST_REFRESH_MS = 10_000;
const DEFAULT_FILTERS = {
  symbol: "BTC",
  severity: "all",
  signal_type: "all",
  discord_sent: "all",
  net_direction: "all",
};

export default function SpotWhaleMonitor() {
  const [state, setState] = useState({ loading: true, summary: null, items: [], error: null });
  const [selectedSignalId, setSelectedSignalId] = useState(null);
  const [filters, setFilters] = useState(DEFAULT_FILTERS);

  useEffect(() => {
    let cancelled = false;
    let summaryTimer = null;
    let latestTimer = null;

    const refreshSummary = () => {
      fetchSpotWhaleSummary(filters.symbol).then((payload) => {
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
        ? fetchSpotWhaleHistory({ ...filters, limit: 50 })
        : fetchSpotWhaleLatest(50, filters.symbol);
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

  useEffect(() => {
    if (selectedSignalId && !state.items.some((item) => item.id === selectedSignalId)) {
      setSelectedSignalId(null);
    }
  }, [selectedSignalId, state.items]);

  const summary = state.summary || fallbackSummary(filters.symbol);
  const selectedSignal = state.items.find((item) => item.id === selectedSignalId) || null;
  const exchanges = summary.exchanges || {};
  const visibleItems = filterByNetDirection(state.items, filters.net_direction);

  return (
    <section className="mb-5 rounded-2xl border border-slate-700/60 bg-slate-900/80 p-5 shadow-glow">
      <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.28em] text-cyan-300">Spot Whale Flow</p>
          <h3 className="mt-2 text-lg font-bold text-white">BTC / ETH 现货监控</h3>
          <p className="mt-1 text-sm text-slate-400">
            聚合 Binance、Coinbase 与 Bitfinex 现货主动成交流，Critical / S 才进入 Discord gate。
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-3 xl:grid-cols-6">
          <StatusPill label="当前状态" value={statusLabel(summary.status)} tone={statusTone(summary.status)} />
          <StatusPill label="健康状态" value={healthStatusLabel(summary.healthStatus)} tone={healthStatusTone(summary.healthStatus)} />
          <StatusPill label="当前方向" value={directionLabel(summary.latestDirection || summary.direction)} tone="cyan" />
          <StatusPill label="最新等级" value={severityLabel(summary.latestSeverity)} tone={severityTone(summary.latestSeverity)} />
          <StatusPill label="运行模式" value={modeLabel(summary)} tone={summary.enabled ? (summary.dryRun ? "yellow" : "cyan") : "slate"} />
          <StatusPill label="最近推送" value={summary.lastDiscordSentAt ? relativeAge(summary.lastDiscordSentAt) : "暂无"} tone="slate" />
        </div>
      </div>

      <SpotTrendBar symbol={filters.symbol} trend={summary.trend60s} />

      <div className="mt-4 grid grid-cols-1 gap-2 text-xs md:grid-cols-2">
        {["binance", "coinbase", "bitfinex"].map((exchange) => (
          <ExchangeStatus exchange={exchange} key={exchange} status={exchanges[exchange]} />
        ))}
      </div>

      {state.error ? (
        <p className="mt-3 rounded-lg border border-yellow-500/30 bg-yellow-500/10 px-3 py-2 text-xs text-yellow-100">
          现货监控数据暂时不可用，已保留上一次结果。
        </p>
      ) : null}

      <SpotWhaleFilters
        filters={filters}
        onChange={(nextFilters) => {
          setSelectedSignalId(null);
          setFilters(nextFilters);
        }}
      />

      <div className="mt-4 overflow-x-auto">
        {state.loading ? (
          <p className="rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
            现货监控载入中...
          </p>
        ) : visibleItems.length === 0 ? (
          <p className="rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
            {summary.enabled
              ? filters.net_direction === "all"
                ? "暂无 BTC/ETH 现货异动"
                : "暂无匹配净方向阈值的现货异动"
              : "现货监控未启用"}
          </p>
        ) : (
          <table className="min-w-full table-fixed text-left text-xs">
            <thead className="text-slate-500">
              <tr>
                <HeaderCell>时间</HeaderCell>
                <HeaderCell>币种 / 价格</HeaderCell>
                <HeaderCell>类型</HeaderCell>
                <HeaderCell>等级</HeaderCell>
                <HeaderCell>窗口</HeaderCell>
                <HeaderCell>成交量</HeaderCell>
                <HeaderCell>名义金额</HeaderCell>
                <HeaderCell>价格</HeaderCell>
                <HeaderCell>净方向</HeaderCell>
                <HeaderCell>方向占比</HeaderCell>
                <HeaderCell>Coinbase 溢价</HeaderCell>
                <HeaderCell>主导平台</HeaderCell>
                <HeaderCell>价格变化</HeaderCell>
                <HeaderCell>Discord</HeaderCell>
                <HeaderCell>详情</HeaderCell>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800 text-slate-300">
              {visibleItems.map((item) => (
                <tr
                  className="cursor-pointer align-top transition hover:bg-slate-800/30"
                  data-testid={`spot-whale-row-${item.id}`}
                  key={item.id}
                  onClick={() => setSelectedSignalId(item.id)}
                  tabIndex={0}
                >
                  <Cell>{formatTime(item.ts)}</Cell>
                  <Cell>
                    <SymbolWithPrice item={item} />
                  </Cell>
                  <Cell>{signalTypeLabel(item.signalType)}</Cell>
                  <Cell>
                    <span className={`rounded-full px-2 py-1 font-bold ${severityBadgeClass(item.severity)}`}>
                      {severityLabel(item.severity)}
                    </span>
                  </Cell>
                  <Cell>{item.windowSec}s</Cell>
                  <Cell>{formatBase(item.totalVolumeBase, item.symbol)}</Cell>
                  <Cell>{formatUsd(item.totalNotionalUsd)}</Cell>
                  <Cell>{formatPrice(signalTriggerPrice(item))}</Cell>
                  <Cell>{formatSignedBase(item.netVolumeBase, item.symbol)}</Cell>
                  <Cell>{formatPct(item.dominance * 100)}</Cell>
                  <Cell>{formatSignedPct(item.coinbasePremiumPct)}</Cell>
                  <Cell>{exchangeLabel(item.mainExchange)}</Cell>
                  <Cell>{formatSignedPct(item.priceMovePct)}</Cell>
                  <Cell>{discordStatus(item)}</Cell>
                  <Cell>
                    <button
                      aria-label={`查看现货信号详情 ${item.id}`}
                      className="rounded-lg border border-cyan-500/40 px-2 py-1 text-cyan-100 transition hover:border-cyan-300 hover:bg-cyan-500/10"
                      onClick={(event) => {
                        event.stopPropagation();
                        setSelectedSignalId(item.id);
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
        )}
      </div>

      {selectedSignal ? (
        <SpotSignalDetail
          onClose={() => setSelectedSignalId(null)}
          signal={selectedSignal}
        />
      ) : null}
    </section>
  );
}

function SpotWhaleFilters({ filters, onChange }) {
  return (
    <div className="mt-5 grid gap-3 text-xs md:grid-cols-5">
      <label className="space-y-1">
        <span className="text-slate-500">币种</span>
        <select
          className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-slate-100"
          onChange={(event) => onChange({ ...filters, symbol: event.target.value })}
          value={filters.symbol}
        >
          <option value="BTC">BTC</option>
          <option value="ETH">ETH</option>
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-500">等级</span>
        <select
          className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-slate-100"
          onChange={(event) => onChange({ ...filters, severity: event.target.value })}
          value={filters.severity}
        >
          <option value="all">全部</option>
          <option value="s">S</option>
          <option value="critical">Critical</option>
          <option value="high">High</option>
          <option value="medium">Medium</option>
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-500">类型</span>
        <select
          className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-slate-100"
          onChange={(event) => onChange({ ...filters, signal_type: event.target.value })}
          value={filters.signal_type}
        >
          <option value="all">全部</option>
          <option value="spotaggressivebuy">主动买入</option>
          <option value="spotaggressivesell">主动卖出</option>
          <option value="spotdownsideabsorption">下方吸收</option>
          <option value="spotupsidesuppression">上方压制</option>
          <option value="spotexchangedislocation">跨所错位</option>
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-500">Discord</span>
        <select
          className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-slate-100"
          onChange={(event) => onChange({ ...filters, discord_sent: event.target.value })}
          value={filters.discord_sent}
        >
          <option value="all">全部</option>
          <option value="true">已推送</option>
          <option value="false">未推送</option>
        </select>
      </label>
      <label className="space-y-1">
        <span className="text-slate-500">净方向</span>
        <select
          className="w-full rounded-xl border border-slate-700 bg-slate-950 px-3 py-2 text-slate-100"
          onChange={(event) => onChange({ ...filters, net_direction: event.target.value })}
          value={filters.net_direction}
        >
          <option value="all">全部</option>
          <option value="abs50">大于 50（正负）</option>
          <option value="abs100">大于 100（正负）</option>
          <option value="abs200">大于 200（正负）</option>
          <option value="abs500">大于 500（正负）</option>
        </select>
      </label>
    </div>
  );
}

function SymbolWithPrice({ item }) {
  return (
    <span className="flex min-w-[96px] flex-col leading-tight">
      <span className="font-semibold text-slate-100">{item.symbol}</span>
      <span className="mt-1 text-[11px] font-semibold text-cyan-200">{formatPrice(signalTriggerPrice(item))}</span>
    </span>
  );
}

function SpotTrendBar({ symbol, trend }) {
  const buyPct = Math.round((trend?.buyRatio || 0) * 1000) / 10;
  const sellPct = Math.round((trend?.sellRatio || 0) * 1000) / 10;
  const total = Number(trend?.totalVolumeBase || 0);
  return (
    <div className="mt-4 rounded-xl border border-slate-700/60 bg-slate-950/60 p-4">
      <p className="text-xs uppercase tracking-[0.25em] text-slate-500">60s Spot Flow</p>
      <div className="mt-2 flex flex-wrap items-center justify-between gap-2 text-sm font-semibold text-white">
        <span className="text-emerald-300">Buy {buyPct}%</span>
        <span className="text-red-300">Sell {sellPct}%</span>
      </div>
      <div className="mt-3 flex h-2 overflow-hidden rounded-full bg-slate-800">
        <div className="bg-emerald-400" style={{ width: `${Math.max(0, Math.min(100, buyPct))}%` }} />
        <div className="bg-red-400" style={{ width: `${Math.max(0, Math.min(100, sellPct))}%` }} />
      </div>
      <p className="mt-2 text-xs text-slate-500">
        60s 总成交 {formatBase(total, symbol)} · 净方向 {formatSignedBase(trend?.netVolumeBase || 0, symbol)}
      </p>
    </div>
  );
}

function ExchangeStatus({ exchange, status }) {
  const item = status || {};
  return (
    <div className="rounded-xl border border-slate-700/60 bg-slate-950/60 p-3">
      <div className="flex items-center justify-between gap-2">
        <span className="font-semibold text-white">{exchangeLabel(exchange)}</span>
        <span className={exchangeStatusClass(item)}>
          {exchangeStatusLabel(item.status)}
        </span>
      </div>
      <p className="mt-1 text-slate-500">最近成交 {item.lastTradeAt ? relativeAge(item.lastTradeAt) : "暂无"}</p>
      {item.latencyMs !== null && item.latencyMs !== undefined ? (
        <p className="mt-1 text-slate-500">延迟 {formatLatency(item.latencyMs)}</p>
      ) : null}
    </div>
  );
}

function SpotSignalDetail({ signal, onClose }) {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 p-4">
      <div className="max-h-[90vh] w-full max-w-3xl overflow-y-auto rounded-2xl border border-cyan-400/30 bg-slate-950 p-5 shadow-glow">
        <div className="flex items-start justify-between gap-4">
          <div>
            <p className="text-xs uppercase tracking-[0.25em] text-cyan-300">Spot Candidate Review</p>
            <h3 className="mt-2 text-xl font-bold text-white">{signal.symbol} · {signalTypeLabel(signal.signalType)}</h3>
          </div>
          <button className="rounded-lg border border-slate-600 px-3 py-1 text-sm text-slate-200" onClick={onClose} type="button">
            关闭
          </button>
        </div>
        <div className="mt-5 grid gap-3 text-sm md:grid-cols-2">
          <Detail label="Direction" value={directionLabel(signal.direction)} />
          <Detail label="Risk Score" value={`${signal.score}/100`} />
          <Detail label="Data Quality" value={`${signal.dataQuality}/100`} />
          <Detail label="Window" value={`${signal.windowSec}s`} />
          <Detail label="Total Volume" value={formatBase(signal.totalVolumeBase, signal.symbol)} />
          <Detail label="Net Direction" value={formatSignedBase(signal.netVolumeBase, signal.symbol)} />
          <Detail label="Notional" value={formatUsd(signal.totalNotionalUsd)} />
          <Detail label="Trigger Price" value={formatPrice(signalTriggerPrice(signal))} />
          <Detail label="Coinbase Premium" value={formatSignedPct(signal.coinbasePremiumPct)} />
          <Detail label="Discord Alert Status" value={discordStatus(signal)} />
          <Detail label="Core Reason" value={signal.finalResult} wide />
        </div>
        <div className="mt-5 rounded-xl border border-slate-700 bg-slate-900/70 p-4">
          <p className="text-xs uppercase tracking-[0.22em] text-slate-500">Exchange Breakdown</p>
          <div className="mt-3 grid gap-2 md:grid-cols-2">
            {signal.exchanges.map((exchange) => (
              <div className="rounded-lg border border-slate-700/60 bg-slate-950/60 p-3" key={exchange.exchange}>
                <p className="font-semibold text-white">{exchangeLabel(exchange.exchange)}</p>
                <p className="mt-1 text-xs text-slate-400">
                  Buy {formatBase(exchange.buyVolumeBase, signal.symbol)} / Sell {formatBase(exchange.sellVolumeBase, signal.symbol)}
                </p>
                <p className="mt-1 text-xs text-slate-500">Dominance {formatPct(exchange.dominance * 100)}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

function Detail({ label, value, wide = false }) {
  return (
    <div className={`rounded-xl border border-slate-700/60 bg-slate-900/70 p-3 ${wide ? "md:col-span-2" : ""}`}>
      <p className="text-xs text-slate-500">{label}</p>
      <p className="mt-1 text-sm font-semibold text-slate-100">{value || "n/a"}</p>
    </div>
  );
}

function StatusPill({ label, value, tone }) {
  const toneClass = {
    cyan: "border-cyan-400/30 text-cyan-100",
    emerald: "border-emerald-400/30 text-emerald-100",
    red: "border-red-400/30 text-red-100",
    yellow: "border-yellow-400/30 text-yellow-100",
    slate: "border-slate-700 text-slate-300",
    fuchsia: "border-fuchsia-400/30 text-fuchsia-100",
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
  return (
    filters.severity !== "all" ||
    filters.signal_type !== "all" ||
    filters.discord_sent !== "all" ||
    filters.net_direction !== "all"
  );
}

function filterByNetDirection(items, filter) {
  if (!Array.isArray(items)) return [];
  const value = String(filter || "all").toLowerCase();
  if (value === "all") return items;
  const absoluteMatch = value.match(/^abs(50|100|200|500)$/);
  if (absoluteMatch) {
    const threshold = Number(absoluteMatch[1]);
    return items.filter((item) => Math.abs(Number(item?.netVolumeBase || 0)) >= threshold);
  }
  const match = value.match(/^(pos|neg)(50|100|200)$/);
  if (!match) return items;
  const threshold = Number(match[2]);
  return items.filter((item) => {
    const net = Number(item?.netVolumeBase || 0);
    return match[1] === "pos" ? net >= threshold : net <= -threshold;
  });
}

function fallbackSummary(symbol) {
  return {
    status: "calm",
    healthStatus: "disabled",
    healthReason: "spot_whale_monitor_disabled",
    direction: "neutral",
    latestDirection: "neutral",
    latestSeverity: "calm",
    lastDiscordSentAt: null,
    signalCount: 0,
    enabled: false,
    dryRun: true,
    symbol,
    trend60s: {},
    exchanges: {},
  };
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
  return { s: "S", critical: "Critical", high: "High", medium: "Medium", calm: "Calm" }[String(value || "").toLowerCase()] || "Medium";
}

function severityTone(value) {
  return { s: "fuchsia", critical: "red", high: "yellow", medium: "yellow", calm: "slate" }[String(value || "").toLowerCase()] || "slate";
}

function severityBadgeClass(value) {
  return {
    s: "bg-fuchsia-400/15 text-fuchsia-200",
    critical: "bg-red-400/15 text-red-200",
    high: "bg-orange-400/15 text-orange-200",
    medium: "bg-yellow-400/15 text-yellow-200",
  }[String(value || "").toLowerCase()] || "bg-slate-700 text-slate-200";
}

function modeLabel(summary) {
  if (!summary.enabled) return "Disabled";
  return summary.dryRun ? "Dry-run" : "Live";
}

function signalTypeLabel(value) {
  return {
    spot_aggressive_buy: "现货主动买入",
    spotaggressivebuy: "现货主动买入",
    spot_aggressive_sell: "现货主动卖出",
    spotaggressivesell: "现货主动卖出",
    spot_downside_absorption: "下方吸收",
    spotdownsideabsorption: "下方吸收",
    spot_upside_suppression: "上方压制",
    spotupsidesuppression: "上方压制",
    spot_exchange_dislocation: "跨所错位",
    spotexchangedislocation: "跨所错位",
  }[String(value || "").toLowerCase()] || "现货候选";
}

function directionLabel(value) {
  return {
    buy: "主动买入",
    sell: "主动卖出",
    absorption: "下方吸收",
    suppression: "上方压制",
    dislocation: "跨所错位",
    neutral: "中性",
  }[String(value || "").toLowerCase()] || "中性";
}

function exchangeLabel(value) {
  return { binance: "Binance", coinbase: "Coinbase", bitfinex: "Bitfinex", multi: "Multi" }[
    String(value || "").toLowerCase()
  ] || value || "Multi";
}

function exchangeStatusLabel(value) {
  return {
    connected: "在线",
    connecting: "连接中",
    reconnecting: "重连中",
    disabled: "未启用",
    degraded: "降级",
    stale: "无近期成交",
    waiting_for_trades: "等待成交",
  }[String(value || "").toLowerCase()] || "离线";
}

function exchangeStatusClass(item) {
  const status = String(item?.status || "").toLowerCase();
  if (status === "connected" && item?.connected) return "font-bold text-emerald-300";
  if (status === "reconnecting" || status === "connecting" || status === "waiting_for_trades") {
    return "font-bold text-yellow-300";
  }
  if (status === "stale" || status === "degraded") return "font-bold text-orange-300";
  return "font-bold text-slate-400";
}

function discordStatus(item) {
  if (item.discordSent) return "已推送";
  if (item.discordEligible) return "符合 gate";
  return item.discordReason || "展示";
}

function formatTime(ts) {
  if (!ts) return "n/a";
  return new Date(ts).toLocaleTimeString("zh-CN", { hour12: false });
}

function formatBase(value, symbol) {
  return `${Number(value || 0).toLocaleString("en-US", { maximumFractionDigits: 2 })} ${symbol}`;
}

function formatSignedBase(value, symbol) {
  const number = Number(value || 0);
  return `${number >= 0 ? "+" : ""}${number.toLocaleString("en-US", { maximumFractionDigits: 2 })} ${symbol}`;
}

function formatUsd(value) {
  const number = Number(value || 0);
  if (number >= 1_000_000) return `$${(number / 1_000_000).toFixed(1)}M`;
  return `$${Math.round(number).toLocaleString("en-US")}`;
}

function signalTriggerPrice(item) {
  const explicit = Number(
    item?.triggerPriceUsd ??
      item?.triggerPrice ??
      item?.avgPriceUsd ??
      item?.priceUsd ??
      item?.price,
  );
  if (Number.isFinite(explicit) && explicit > 0) {
    return explicit;
  }
  const totalVolumeBase = Number(item?.totalVolumeBase || 0);
  const totalNotionalUsd = Number(item?.totalNotionalUsd || 0);
  if (totalVolumeBase > 0 && totalNotionalUsd > 0) {
    return totalNotionalUsd / totalVolumeBase;
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
  if (value === null || value === undefined) return "n/a";
  return `${Number(value).toFixed(1)}%`;
}

function formatSignedPct(value) {
  if (value === null || value === undefined) return "n/a";
  const number = Number(value);
  return `${number >= 0 ? "+" : ""}${number.toFixed(3)}%`;
}

function relativeAge(ts) {
  const diff = Math.max(0, Date.now() - Number(ts));
  if (diff < 60_000) return `${Math.round(diff / 1000)} 秒前`;
  if (diff < 3_600_000) return `${Math.round(diff / 60_000)} 分钟前`;
  return `${Math.round(diff / 3_600_000)} 小时前`;
}

function formatLatency(value) {
  const ms = Number(value);
  if (!Number.isFinite(ms) || ms < 0) return "N/A";
  if (ms < 1000) return `${Math.round(ms)}ms`;
  return `${Math.round(ms / 1000)}s`;
}
