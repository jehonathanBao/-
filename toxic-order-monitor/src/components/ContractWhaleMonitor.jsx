import { useEffect, useState } from "react";
import {
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
    error: null,
  });
  const [selectedSignalId, setSelectedSignalId] = useState(null);
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

  const summary = state.summary || {
    status: "calm",
    healthStatus: "disabled",
    healthReason: "contract_whale_monitor_disabled",
    thresholdProfile: "binance_bitfinex",
    activeExchangeCount: 0,
    enabledExchanges: [],
    disabledExchanges: ["binance", "okx", "bitfinex"],
    direction: "neutral",
    latestDirection: "neutral",
    latestSeverity: "calm",
    latestPushedAtMs: null,
    lastDiscordSentAt: null,
    signalCount: 0,
    enabled: false,
    dryRun: true,
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
    },
  };
  const exchangeStatuses = summary.exchanges || {};
  const visibleExchanges = visibleContractExchanges(exchangeStatuses);
  const selectedSignal = state.items.find((item) => item.id === selectedSignalId) || null;

  return (
    <section className="mb-5 rounded-2xl border border-slate-700/60 bg-slate-900/80 p-5 shadow-glow">
      <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.28em] text-cyan-300">Contract Whale Flow</p>
          <h3 className="mt-2 text-lg font-bold text-white">主力合约监控</h3>
          <p className="mt-1 text-sm text-slate-400">
            BTC / ETH 永续合约主动成交流异常，Critical / S 才进入外部告警判断。
          </p>
        </div>
        <div className="grid grid-cols-2 gap-2 text-xs text-slate-300 md:grid-cols-3 xl:grid-cols-7">
          <StatusPill label="当前状态" value={statusLabel(summary.status)} tone={statusTone(summary.status)} />
          <StatusPill label="健康状态" value={healthStatusLabel(summary.healthStatus)} tone={healthStatusTone(summary.healthStatus)} />
          <StatusPill label="当前方向" value={directionLabel(summary.latestDirection || summary.direction)} tone="cyan" />
          <StatusPill label="最新等级" value={severityLabel(summary.latestSeverity)} tone={severityTone(summary.latestSeverity)} />
          <StatusPill label="阈值模式" value={thresholdProfileLabel(summary.thresholdProfile)} tone="cyan" />
          <StatusPill label="运行模式" value={modeLabel(summary)} tone={summary.enabled ? (summary.dryRun ? "yellow" : "cyan") : "slate"} />
          <StatusPill label="最近推送" value={summary.lastDiscordSentAt ? relativeAge(summary.lastDiscordSentAt) : "暂无"} tone="slate" />
        </div>
      </div>

      <ContractWhaleTrendBar exchanges={exchangeStatuses} summary={summary} trend={summary.trend60s} />

      <div className="mt-4 grid grid-cols-1 gap-2 text-xs md:grid-cols-3">
        {visibleExchanges.map((exchange) => (
          <ExchangeStatus
            exchange={exchange}
            key={exchange}
            status={exchangeStatuses[exchange]}
          />
        ))}
      </div>

      {state.error ? (
        <p className="mt-3 rounded-lg border border-yellow-500/30 bg-yellow-500/10 px-3 py-2 text-xs text-yellow-100">
          主力合约监控数据暂时不可用，已保留上一次结果。
        </p>
      ) : null}

      <ContractWhaleFilters
        filters={filters}
        onChange={(nextFilters) => {
          setSelectedSignalId(null);
          setFilters(nextFilters);
        }}
      />

      <div className="mt-4 overflow-x-auto">
        {state.loading ? (
          <p className="rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
            主力合约监控载入中...
          </p>
        ) : state.items.length === 0 ? (
          <p className="rounded-xl border border-slate-800 bg-slate-950/50 px-4 py-5 text-sm text-slate-400">
            {summary.enabled ? "暂无主力合约异动" : "主力合约监控未启用"}
          </p>
      ) : (
          <table className="min-w-full table-fixed text-left text-xs">
            <thead className="text-slate-500">
              <tr>
                <HeaderCell>时间</HeaderCell>
                <HeaderCell>币种</HeaderCell>
                <HeaderCell>类型</HeaderCell>
                <HeaderCell>等级</HeaderCell>
                <HeaderCell>窗口</HeaderCell>
                <HeaderCell>成交量</HeaderCell>
                <HeaderCell>名义金额</HeaderCell>
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
              {state.items.map((item) => (
                <tr
                  className="cursor-pointer align-top transition hover:bg-slate-800/30"
                  data-testid={`contract-whale-row-${item.id}`}
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
                  <Cell>{item.symbol}</Cell>
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
                  <Cell>{formatBtc(item.totalVolumeBtc)}</Cell>
                  <Cell>{formatUsd(item.totalNotionalUsd)}</Cell>
                  <Cell>{netDirection(item.netVolumeBtc)}</Cell>
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
                ["等级", severityLabel(signal.severity)],
                ["窗口", `${signal.windowSec}s`],
                ["触发时间", formatTime(signal.ts)],
                ["Risk Score", `${signal.score}/100`],
                ["Data Quality", `${signal.dataQuality}/100`],
                ["Threshold Profile", thresholdProfileLabel(summary?.thresholdProfile)],
              ]}
            />
          </DetailSection>

          <DetailSection title="Discord Gate">
            <DetailGrid
              rows={[
                ["Gate Result", signal.discordEligible ? "可进入推送判断" : "仅展示"],
                ["Discord Sent", signal.discordSent ? "已推送" : "未推送"],
                ["Skip Reason", signal.discordSent ? "sent" : signal.discordReason],
                ["多平台确认", signal.multiExchangeConfirmed ? "是" : "否"],
                ["疑似强平", signal.liquidationSuspected ? "是" : "否"],
                ["合并来源", signal.mergedFrom?.length ? signal.mergedFrom.join(", ") : "无"],
              ]}
            />
          </DetailSection>
        </div>

        <DetailSection title="5s / 15s / 60s 窗口数据" className="mt-4">
          <div className="grid gap-2 md:grid-cols-3">
            {[5, 15, 60].map((windowSec, index) => {
              const item = windowRows[index];
              return (
                <div className="rounded-xl border border-slate-800 bg-slate-900/60 p-3" key={windowSec}>
                  <p className="font-bold text-slate-100">{windowSec}s</p>
                  {item ? (
                    <div className="mt-2 space-y-1 text-xs text-slate-300">
                      <p>成交量：{formatBtc(item.totalVolumeBtc)}</p>
                      <p>名义金额：{formatUsd(item.totalNotionalUsd)}</p>
                      <p>净方向：{netDirection(item.netVolumeBtc)}</p>
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
                    <p>主动买入：{formatBtc(exchange.buyVolumeBtc)}</p>
                    <p>主动卖出：{formatBtc(exchange.sellVolumeBtc)}</p>
                    <p>总量：{formatBtc(exchange.totalVolumeBtc)}</p>
                    <p>买/卖占比：{formatPct(Number(exchange.buyShare || 0) * 100)} / {formatPct(Number(exchange.sellShare || 0) * 100)}</p>
                    <p>净方向：{netDirection(exchange.netVolumeBtc)}</p>
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
                ["Percentile", formatPercentile(signal.percentileLevel)],
                ["Price Move", formatSignedPct(signal.priceMovePct)],
                ["Price Reversal", signal.priceReversalRatio === null || signal.priceReversalRatio === undefined ? "N/A" : formatPct(signal.priceReversalRatio * 100)],
                ["Dominant Net Flow", formatPct(dominantNetFlowShare(signal) * 100)],
                ["Liquidation", liquidationStatus(signal)],
                ["OI", oiStatus(signal)],
                ["Funding", fundingStatus(signal)],
              ]}
            />
          </DetailSection>

          <DetailSection title="Raw Scoring Breakdown">
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

function ContractWhaleTrendBar({ exchanges, summary, trend }) {
  const item = trend || {};
  const total = Number(item.totalVolumeBtc || 0);
  const buyRatio = total > 0 ? clampRatio(item.buyRatio) : 0;
  const sellRatio = total > 0 ? clampRatio(item.sellRatio || (1 - buyRatio)) : 0;
  const netDirectionLabel = netDirection(Number(item.netVolumeBtc || 0));
  const sourceLabel = activeContractSourceLabel(exchanges, summary);
  return (
    <div className="mt-4 rounded-xl border border-slate-800 bg-slate-950/40 px-4 py-3">
      <div className="flex flex-col gap-2 md:flex-row md:items-center md:justify-between">
        <div>
          <p className="text-[11px] uppercase tracking-[0.2em] text-slate-500">60s Contract Flow</p>
          <p className="mt-1 text-sm font-semibold text-slate-100">
            Buy {formatPct(buyRatio * 100)} / Sell {formatPct(sellRatio * 100)}
          </p>
        </div>
        <div className="text-xs text-slate-400 md:text-right">
          <p>{netDirectionLabel}</p>
          <p>总量 {formatBtc(total)} · dominance {formatPct(Number(item.dominance || 0) * 100)}</p>
        </div>
      </div>
      <div className="mt-3 h-2 overflow-hidden rounded-full bg-red-500/20">
        <div
          aria-label="最近 60 秒主动买入占比"
          className="h-full rounded-full bg-emerald-400"
          style={{ width: total > 0 ? `${Math.max(3, buyRatio * 100)}%` : "0%" }}
        />
      </div>
      <div className="mt-2 flex justify-between text-[11px] text-slate-500">
        <span>主动买入 {formatBtc(item.buyVolumeBtc)}</span>
        <span>主动卖出 {formatBtc(item.sellVolumeBtc)}</span>
      </div>
      <p className="mt-2 text-[11px] text-slate-500">
        当前统计数据源：{sourceLabel} · {thresholdProfileLabel(summary?.thresholdProfile)}
      </p>
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
      </FilterSelect>
    </div>
  );
}

function FilterSelect({ label, value, onChange, children }) {
  return (
    <label className="block rounded-xl border border-slate-800 bg-slate-950/40 px-3 py-2">
      <span className="block text-[11px] text-slate-500">{label}</span>
      <select
        className="mt-1 w-full bg-transparent font-semibold text-slate-100 outline-none"
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
    <div className="rounded-xl border border-slate-700/60 bg-slate-950/40 px-3 py-2">
      <p className="text-[11px] text-slate-500">{label}</p>
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

function ExchangeStatus({ exchange, status }) {
  const item = status || {
    connected: false,
    status: "disconnected",
    lastTradeAt: null,
    latencyMs: null,
    reconnectCount: 0,
  };
  return (
    <div className="flex items-center justify-between rounded-xl border border-slate-800 bg-slate-950/40 px-3 py-2">
      <div>
        <p className="font-semibold text-slate-200">{exchangeLabel(exchange)}</p>
        <p className="mt-1 text-slate-500">
          {item.lastTradeAt ? `最近成交 ${relativeAge(item.lastTradeAt)}` : "暂无成交"}
        </p>
        <p className="mt-1 text-slate-500">
          {item.latencyMs !== null && item.latencyMs !== undefined ? `延迟 ${formatLatency(item.latencyMs)}` : "延迟 N/A"}
        </p>
      </div>
      <div className="text-right">
        <p className={exchangeStatusClass(item)}>
          {exchangeStatusLabel(item)}
        </p>
        <p className="mt-1 text-slate-500">重连 {Number(item.reconnectCount || 0)}</p>
      </div>
    </div>
  );
}

function HeaderCell({ children }) {
  return <th className="whitespace-nowrap px-3 py-2 font-semibold">{children}</th>;
}

function Cell({ children }) {
  return <td className="whitespace-nowrap px-3 py-3">{children}</td>;
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
  if (value === "binance_bitfinex") return "Binance+Bitfinex";
  if (value === "three_exchange") return "三平台";
  return "默认";
}

function exchangeLabel(exchange) {
  const labels = {
    binance: "Binance",
    okx: "OKX",
    bitfinex: "Bitfinex",
  };
  return labels[exchange] || exchange;
}

function visibleContractExchanges(exchanges) {
  const source = exchanges && typeof exchanges === "object" ? exchanges : {};
  const visible = ["binance", "okx", "bitfinex"].filter((exchange) => source[exchange]);
  return visible.length ? visible : ["binance", "okx", "bitfinex"];
}

function exchangeStatusLabel(item) {
  const status = String(item.status || "").toLowerCase();
  if (status === "disabled") return "未启用";
  if (status === "reconnecting") return "重连中";
  if (status === "stale") return "无近期成交";
  if (status === "waiting_for_trades") return "等待成交";
  if (item.connected || status === "connected") return "在线";
  return "离线";
}

function exchangeStatusClass(item) {
  const status = String(item.status || "").toLowerCase();
  if (item.connected && status === "connected") return "font-bold text-emerald-300";
  if (status === "reconnecting" || status === "waiting_for_trades") return "font-bold text-yellow-300";
  if (status === "stale") return "font-bold text-orange-300";
  return "font-bold text-slate-400";
}

function activeContractSourceLabel(exchanges, summary) {
  const source = exchanges && typeof exchanges === "object" ? exchanges : {};
  const configured = Array.isArray(summary?.enabledExchanges) && summary.enabledExchanges.length
    ? summary.enabledExchanges
    : ["binance", "bitfinex"];
  const active = configured
    .filter((exchange) => {
      const item = source[exchange] || {};
      return item.connected && String(item.status || "connected").toLowerCase() === "connected";
    })
    .map(exchangeLabel);
  if (active.length) return active.join(" / ");
  const stale = configured
    .filter((exchange) => String(source[exchange]?.status || "").toLowerCase() === "stale")
    .map(exchangeLabel);
  if (stale.length) return `${stale.join(" / ")} 无近期成交`;
  return "暂无在线交易所";
}

function netDirection(value) {
  if (value > 0) return `净买入 ${formatBtc(Math.abs(value))}`;
  if (value < 0) return `净卖出 ${formatBtc(Math.abs(value))}`;
  return "中性";
}

function formatBtc(value) {
  return `${Math.round(Number(value || 0)).toLocaleString("en-US")} BTC`;
}

function formatUsd(value) {
  const number = Number(value || 0);
  if (number >= 1_000_000_000) return `$${(number / 1_000_000_000).toFixed(2)}B`;
  if (number >= 1_000_000) return `$${Math.round(number / 1_000_000).toLocaleString("en-US")}M`;
  return `$${Math.round(number).toLocaleString("en-US")}`;
}

function formatPct(value) {
  return `${Number(value || 0).toFixed(1)}%`;
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
  return `疑似强平 ${formatBtc(total)} / ${ratio}`;
}

function oiStatus(item) {
  const bias = oiBiasLabel(item.oiBias);
  if (item.oiChange5mBtc === null || item.oiChange5mBtc === undefined) return bias;
  const pct = item.oiChangePct === null || item.oiChangePct === undefined
    ? ""
    : ` / ${formatSignedPct(item.oiChangePct)}`;
  return `${formatSignedBtc(item.oiChange5mBtc)}${pct} ${bias}`;
}

function fundingStatus(item) {
  const bias = fundingBiasLabel(item.fundingBias);
  if (item.fundingRate === null || item.fundingRate === undefined) return bias;
  return `${formatSignedPct(Number(item.fundingRate) * 100)} ${bias}`;
}

function scoringBreakdown(item) {
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
  const number = Number(value || 0);
  const sign = number >= 0 ? "+" : "-";
  return `${sign}${formatBtc(Math.abs(number))}`;
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
