import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { ArrowUpRightIcon, ArrowPathIcon, PauseIcon, PlayIcon } from "@heroicons/react/24/outline";
import PriceChart from "./PriceChart.jsx";
import { fetchMonitorFlowSnapshot } from "../api/monitorFlow.js";

const REFRESH_INTERVAL_MS = 8_000;
const MAX_VISIBLE_EVENTS = 80;
const FILTERS = [
  ["all", "全部"],
  ["contract", "合约"],
  ["spot", "现货"],
  ["micro", "Delta · VPIN/TOF"],
  ["signal", "异常候选"],
  ["system", "系统"],
];

export default function MonitorFlowDashboard({
  discordConnected = false,
  rawInboxSignals = [],
  runtimeBoundary = null,
  signalsError = null,
  wsStatus = "idle",
}) {
  const [snapshot, setSnapshot] = useState(null);
  const [activeFilter, setActiveFilter] = useState("all");
  const [paused, setPaused] = useState(false);
  const [loading, setLoading] = useState(true);
  const [refreshError, setRefreshError] = useState(null);
  const [pageVisible, setPageVisible] = useState(() => typeof document === "undefined" || document.visibilityState !== "hidden");
  const [reducedMotion, setReducedMotion] = useState(false);
  const [activeEventIds, setActiveEventIds] = useState(() => new Set());
  const eventSignaturesRef = useRef(new Map());
  const activityReadyRef = useRef(false);

  const refresh = useCallback(async () => {
    setRefreshError(null);
    try {
      const next = await fetchMonitorFlowSnapshot();
      setSnapshot(next);
    } catch (error) {
      setRefreshError(error?.message || "monitor_flow_unavailable");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  useEffect(() => {
    if (paused || !pageVisible) return undefined;
    const timer = window.setInterval(refresh, REFRESH_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [pageVisible, paused, refresh]);

  useEffect(() => {
    const onVisibilityChange = () => setPageVisible(document.visibilityState !== "hidden");
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => document.removeEventListener("visibilitychange", onVisibilityChange);
  }, []);

  useEffect(() => {
    const media = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    if (!media) return undefined;
    const update = () => setReducedMotion(media.matches);
    update();
    media.addEventListener?.("change", update);
    return () => media.removeEventListener?.("change", update);
  }, []);

  const events = useMemo(
    () => buildMonitorFlowEvents(snapshot, rawInboxSignals).slice(0, MAX_VISIBLE_EVENTS),
    [rawInboxSignals, snapshot],
  );
  const visibleEvents = useMemo(
    () => (activeFilter === "all" ? events : events.filter((event) => event.channel === activeFilter)),
    [activeFilter, events],
  );
  const pulse = useMemo(() => buildBtcPulse(snapshot), [snapshot]);
  const health = useMemo(
    () => buildMonitorHealth(snapshot, events, { discordConnected, runtimeBoundary, wsStatus }),
    [discordConnected, events, runtimeBoundary, snapshot, wsStatus],
  );

  useEffect(() => {
    const currentSignatures = new Map(events.map(event => [event.id, `${event.value}|${event.state}|${event.secondary}`]));
    if (!activityReadyRef.current) {
      activityReadyRef.current = true;
      eventSignaturesRef.current = currentSignatures;
      return undefined;
    }
    const changedIds = new Set();
    currentSignatures.forEach((signature, id) => {
      if (eventSignaturesRef.current.get(id) !== signature) changedIds.add(id);
    });
    eventSignaturesRef.current = currentSignatures;
    if (!changedIds.size || reducedMotion) {
      setActiveEventIds(new Set());
      return undefined;
    }
    setActiveEventIds(changedIds);
    const timer = window.setTimeout(() => setActiveEventIds(new Set()), 2_200);
    return () => window.clearTimeout(timer);
  }, [events, reducedMotion]);


  const pricePoints = useMemo(() => (snapshot?.orderflow?.candles || []).map(candle => ({
    time: candle.time,
    price: candle.close,
  })), [snapshot]);
  const focus = events.find(event => event.channel === "contract") || null;
  const streamState = paused ? "PAUSED" : loading ? "CONNECTING" : refreshError || !health.activeLinks ? "WAITING" : "LIVE";

  return (
    <section className={`monitor-flow ${pageVisible ? "is-visible" : "is-backgrounded"} ${reducedMotion ? "reduce-motion" : ""}`} data-testid="monitor-flow-dashboard">
      <div className="monitor-flow-hero">
        <div>
          <p className="monitor-flow-kicker"><span aria-hidden="true">01</span> MARKET OVERVIEW</p>
          <div className="monitor-flow-title-row">
            <h1>全市场实时监控流</h1>
            <span className={`monitor-flow-live ${streamState !== "LIVE" ? "is-paused" : ""}`}><i aria-hidden="true" />{streamState}</span>
          </div>
          <p className="monitor-flow-subtitle">跟踪资金方向，连接市场证据。BTC / ETH 合约、现货与订单流的统一视角。</p>
        </div>
        <div className="monitor-flow-hero-actions">
          <span>最近同步 <time>{formatClock(snapshot?.fetchedAtMs)}</time></span>
          <button className="monitor-flow-button" disabled={loading} onClick={refresh} type="button"><ArrowPathIcon aria-hidden="true" />{loading ? "同步中" : "立即同步"}</button>
          <button aria-pressed={paused} className={`monitor-flow-button ${paused ? "is-active" : ""}`} onClick={() => setPaused(value => !value)} type="button">
            {paused ? <PlayIcon aria-hidden="true" /> : <PauseIcon aria-hidden="true" />}{paused ? "继续流" : "暂停流"}
          </button>
        </div>
      </div>

      {signalsError || refreshError ? (
        <div className="monitor-flow-warning" role="status">
          部分数据源暂时不可用，页面保留最近一次成功快照。{signalsError ? ` 信号：${signalsError}` : ""}
        </div>
      ) : null}

      <div className="monitor-flow-hud" aria-label="监控链路状态">
        <HudCell label="采集链路" value={`${health.activeLinks}/${health.totalLinks}`} detail={health.linkDetail} tone={health.linkTone} />
        <HudCell label="最新数据延迟" value={health.latencyLabel} detail={health.qualityLabel} tone={health.qualityTone} />
        <HudCell label="近 1H 样本事件" value={String(health.events1h)} detail={`${health.alerts1h} 条高优先级 · 当前快照`} tone={health.alerts1h > 0 ? "warn" : "neutral"} />
        <HudCell label="Discord 通知" value={discordConnected ? "已配置" : "独立门控"} detail={health.discordDetail} tone={discordConnected ? "ok" : "neutral"} />
      </div>

      <div className="terminal-overview-grid">
        <PriceChart points={pricePoints} title="BTC 价格走势" symbol="BTC" loading={loading} description="Binance 永续 · 1H K 线收盘价" />
        <aside className="terminal-focus-card">
          <div className="terminal-section-title"><div><p>ON THE RADAR</p><h2>重点观察</h2></div><span className="terminal-section-number">02</span></div>
          {focus ? <>
            <div className="terminal-focus-asset"><span className="terminal-focus-symbol">{focus.symbol}</span><span className={`severity-${focus.severity}`}>{focus.state}</span></div>
            <h3>最新合约 · {focus.title}</h3>
            <p className="terminal-focus-copy">{focus.detail}</p>
            <div className={`terminal-focus-value direction-${focus.direction}`}><strong>{focus.value}</strong><span>{focus.secondary}</span></div>
            <div className="terminal-focus-foot"><span>{formatRelativeTime(focus.ts)} · 最近合约事件</span><span>{focus.pushState}</span></div>
          </> : <div className="terminal-focus-empty"><span aria-hidden="true">⌁</span><h3>等待主力行为线索</h3><p>符合展示标准的合约事件会出现在这里。</p></div>}
          <Link className="terminal-focus-link" to={focus?.href || "/contract-whale/btc"}>打开合约监控<ArrowUpRightIcon aria-hidden="true" /></Link>
          <p className="terminal-disclaimer">公开成交证据推断 · 不代表账户身份确认</p>
        </aside>
      </div>

      <section className="monitor-flow-panel monitor-flow-pulse">
        <header className="monitor-flow-panel-header">
          <div><p>FLOW SNAPSHOT</p><h2>BTC 当前脉冲</h2></div><Link to="/binance-orderflow">查看订单流 <ArrowUpRightIcon aria-hidden="true" /></Link>
        </header>
        <div className="monitor-flow-pulse-grid">{pulse.map(item => <PulseCell item={item} key={item.label} />)}</div>
      </section>

      <div className="monitor-flow-layout">
        <section className="monitor-flow-panel monitor-flow-tape">
          <header className="monitor-flow-panel-header">
            <div><p>UNIFIED EVENT TAPE</p><h2>实时监控事件流</h2></div>
            <span className="terminal-event-count">{visibleEvents.length}<small> / {events.length} 条</small></span>
          </header>
          <div className="monitor-flow-filters" aria-label="事件类型筛选">
            {FILTERS.map(([key, label]) => (
              <button aria-pressed={activeFilter === key} className={activeFilter === key ? "is-active" : ""} key={key} onClick={() => setActiveFilter(key)} type="button">
                {label}<small>{key === "all" ? events.length : events.filter(event => event.channel === key).length}</small>
              </button>
            ))}
          </div>
          <div className="monitor-flow-tape-head" aria-hidden="true"><span>时间</span><span>市场</span><span>行为 / 证据</span><span>核心读数</span><span>状态</span></div>
          <div className="monitor-flow-event-list" data-testid="monitor-flow-event-list">
            {visibleEvents.length ? visibleEvents.map(event => <MonitorEventRow active={activeEventIds.has(event.id)} event={event} key={event.id} />) : (
              <div className="monitor-flow-empty"><strong>{loading ? "正在接入监控流" : "当前筛选暂无事件"}</strong><span>这不代表数据链路中断；低于阈值的数据不会生成事件卡片。</span></div>
            )}
          </div>
        </section>
        <aside className="monitor-flow-side">
          <section className="monitor-flow-panel monitor-flow-routes">
            <header className="monitor-flow-panel-header"><div><p>EXPLORE MARKETS</p><h2>深度监控入口</h2></div></header>
            <div className="monitor-flow-route-grid">
              <Link to="/contract-whale/btc"><span>BTC</span><div>合约事件带<small>统一评级 · OI · Funding</small></div><ArrowUpRightIcon aria-hidden="true" /></Link>
              <Link to="/contract-whale/eth"><span>ETH</span><div>ETH 合约<small>事件追踪 · 行为证据</small></div><ArrowUpRightIcon aria-hidden="true" /></Link>
              <Link to="/spot-monitor/btc"><span>SPOT</span><div>现货鲸鱼流<small>净方向 · 跨所确认</small></div><ArrowUpRightIcon aria-hidden="true" /></Link>
              <Link to="/binance-orderflow"><span>FLOW</span><div>订单流 K 线<small>Delta · VPIN · TOF</small></div><ArrowUpRightIcon aria-hidden="true" /></Link>
              <Link to="/signals"><span>RISK</span><div>异常候选<small>S 级 · Discord Gate</small></div><ArrowUpRightIcon aria-hidden="true" /></Link>
            </div>
          </section>
          <div className="terminal-observation-note"><span>观察，不执行。</span><p>评级描述市场事件强度，不等于交易指令。请结合数据质量和详细证据判断。</p></div>
        </aside>
      </div>

      <section className="monitor-flow-panel monitor-flow-pipeline">
        <header className="monitor-flow-panel-header"><div><p>SYSTEM TELEMETRY</p><h2>监控链路</h2></div><span>只读聚合 · 无额外持久化</span></header>
        <div className="monitor-flow-pipeline-track">
          {[
            ["01", "交易所", health.activeLinks > 0 ? "ACTIVE" : "WAIT"],
            ["02", "采集器", health.activeLinks > 0 ? "STREAMING" : "WAIT"],
            ["03", "检测器", health.qualityLabel],
            ["04", "统一评级", "EVIDENCE"],
            ["05", "Discord", discordConnected ? "READY" : "GATED"],
            ["06", "冷热存储", health.storageLabel],
          ].map(([index, label, status]) => (
            <div className="monitor-flow-pipeline-node" key={index}><small>{index}</small><div><strong>{label}</strong><span>{status}</span></div><i aria-hidden="true">→</i></div>
          ))}
        </div>
      </section>
      <footer className="terminal-page-footer"><span>WHALE DESK / MARKET INTELLIGENCE</span><span>行情仅供观察 · 通知沿用原有门控</span></footer>
    </section>
  );
}

function HudCell({ label, value, detail, tone = "neutral" }) {
  return (
    <div className={`monitor-flow-hud-cell tone-${tone}`}>
      <span>{label}</span>
      <strong key={value} className="monitor-flow-metric-value">{value}</strong>
      <small>{detail}</small>
    </div>
  );
}

function MonitorEventRow({ event, active = false }) {
  const content = (
    <>
      <time dateTime={new Date(event.ts).toISOString()}>
        <strong>{formatEventTime(event.ts)}</strong>
        <small>{formatRelativeTime(event.ts)}</small>
      </time>
      <div className="monitor-flow-event-source">
        <span className={`monitor-flow-channel channel-${event.channel}`}>{event.channelLabel}</span>
        <small>{event.symbol}</small>
      </div>
      <div className="monitor-flow-event-copy">
        <strong>{event.title}</strong>
        <span>{event.detail}</span>
      </div>
      <div className={`monitor-flow-event-value direction-${event.direction}`}>
        <strong key={event.value} className="monitor-flow-metric-value">{event.value}</strong>
        <small>{event.secondary}</small>
      </div>
      <div className="monitor-flow-event-state">
        <span className={`severity-${event.severity}`}>{event.state}</span>
        <small>{event.pushState}</small>
      </div>
    </>
  );
  return event.href ? (
    <Link className={`monitor-flow-event ${active ? "is-active" : ""}`} to={event.href}>{content}</Link>
  ) : (
    <div className={`monitor-flow-event ${active ? "is-active" : ""}`}>{content}</div>
  );
}

function PulseCell({ item }) {
  return (
    <div className={`monitor-flow-pulse-cell tone-${item.tone || "neutral"}`}>
      <span>{item.label}</span>
      <strong key={item.value} className="monitor-flow-metric-value">{item.value}</strong>
      <small>{item.detail}</small>
    </div>
  );
}

export function buildMonitorFlowEvents(snapshot, rawInboxSignals = []) {
  const items = [];
  const contractEvents = Array.isArray(snapshot?.contract?.events) ? snapshot.contract.events : [];
  contractEvents.forEach(item => {
    const symbol = baseSymbol(item.symbol);
    const value = finiteNumber(item.netVolumeBtc);
    items.push({
      id: `contract:${item.eventId || item.id}`,
      ts: epochMs(item.ts),
      channel: "contract",
      channelLabel: "PERP",
      symbol,
      title: contractTitle(item),
      detail: `${windowLabel(item.windowSec)} · ${item.mainExchange || "MULTI"} · ${directionLabel(item.direction)}`,
      value: signedVolume(value, symbol),
      secondary: `${formatCompactUsd(item.totalNotionalUsd)} · 总量 ${formatVolume(item.displayVolumeBtc ?? item.totalVolumeBtc, symbol)}`,
      state: ratingLabel(item),
      severity: severityKey(item.impactGrade || item.signalLevel || item.severity),
      direction: directionKey(value, item.direction),
      pushState: item.discordSent ? "Discord 已推送" : item.discordEligible ? "Discord 待门控" : "页面观察",
      href: `/contract-whale/${symbol.toLowerCase()}`,
    });
  });

  [snapshot?.spot?.BTC, snapshot?.spot?.ETH].forEach(payload => {
    (payload?.items || []).forEach(item => {
      const symbol = baseSymbol(item.symbol);
      const value = finiteNumber(item.netVolumeBase);
      items.push({
        id: `spot:${item.id}`,
        ts: epochMs(item.ts),
        channel: "spot",
        channelLabel: "SPOT",
        symbol,
        title: spotTitle(item),
        detail: `${windowLabel(item.windowSec)} · ${String(item.mainExchange || "MULTI").toUpperCase()} · 主导 ${(finiteNumber(item.dominance) * 100).toFixed(0)}%`,
        value: signedVolume(value, symbol),
        secondary: `${formatCompactUsd(item.totalNotionalUsd)} · 质量 ${Math.round(finiteNumber(item.dataQuality))}`,
        state: String(item.severity || "WATCH").toUpperCase(),
        severity: severityKey(item.severity),
        direction: directionKey(value, item.direction),
        pushState: item.discordSent ? "Discord 已推送" : item.discordEligible ? "Discord 待门控" : "阈值内",
        href: `/spot-monitor/${symbol.toLowerCase()}`,
      });
    });
  });

  const candles = Array.isArray(snapshot?.orderflow?.candles) ? snapshot.orderflow.candles : [];
  const flowCandles = candles.filter(candle => candle.buyBase > 0 || candle.sellBase > 0).slice(-8);
  flowCandles.forEach(candle => {
    const delta = finiteNumber(candle.deltaBase);
    items.push({
      id: `delta:${candle.time}`,
      ts: epochMs(candle.closeTime || candle.time),
      channel: "micro",
      channelLabel: "DELTA",
      symbol: "BTC",
      title: `${snapshot?.orderflow?.interval || "1h"} 主动买卖差 ${delta >= 0 ? "偏买" : "偏卖"}`,
      detail: `成交 ${formatVolume(candle.volumeBase, "BTC")} · ${candle.tradeCount.toLocaleString("zh-CN")} 笔`,
      value: signedVolume(delta, "BTC"),
      secondary: `Delta ${signedPercent(candle.deltaPct)}`,
      state: Math.abs(delta) >= 1_000 ? "EXTREME" : Math.abs(delta) >= 200 ? "WATCH" : "NORMAL",
      severity: Math.abs(delta) >= 1_000 ? "critical" : Math.abs(delta) >= 200 ? "high" : "info",
      direction: directionKey(delta),
      pushState: candle.closed ? "小时已收盘" : "小时进行中",
      href: "/binance-orderflow",
    });

    if (candle.vpinHigh || candle.vpinExtreme || candle.vpinSpike || candle.tofAlert) {
      const vpinValue = candle.vpin === null ? "—" : candle.vpin.toFixed(3);
      items.push({
        id: `micro:${candle.time}`,
        ts: epochMs(candle.closeTime || candle.time) + 1,
        channel: "micro",
        channelLabel: candle.tofAlert ? "TOF" : "VPIN",
        symbol: "BTC",
        title: candle.tofAlert ? "订单流异常达到 TOF 阈值" : "VPIN 流量失衡异常",
        detail: candle.tofReasons?.length ? candle.tofReasons.slice(0, 2).join(" · ") : `VPIN 百分位 ${formatPercentile(candle.vpinPercentile)}`,
        value: candle.tofAlert ? formatVolume(candle.tofVolumeBtc, "BTC") : vpinValue,
        secondary: `z ${formatNumber(candle.vpinZscore, 2)}`,
        state: candle.vpinExtreme ? "EXTREME" : "ALERT",
        severity: candle.vpinExtreme || String(candle.tofSeverity).toLowerCase() === "critical" ? "critical" : "high",
        direction: "neutral",
        pushState: "独立 Discord 门控",
        href: "/binance-orderflow",
      });
    }
  });

  (Array.isArray(rawInboxSignals) ? rawInboxSignals : [])
    .map((signal, index) => ({ signal, ts: signalTimestamp(signal, index) }))
    .filter(({ ts }) => ts >= Date.now() - 24 * 60 * 60 * 1_000)
    .slice(0, 24)
    .forEach(({ signal, ts }, index) => {
    const symbol = baseSymbol(signal.symbol);
    items.push({
      id: `signal:${signal.id || index}`,
      ts,
      channel: "signal",
      channelLabel: signal.level || "RISK",
      symbol,
      title: readableToken(signal.type || "异常候选"),
      detail: signal.reason || signal.impact || "盘口 / 成交异常候选",
      value: `SCORE ${Math.round(finiteNumber(signal.score))}`,
      secondary: `置信 ${Math.round(finiteNumber(signal.confidence))} · 质量 ${Math.round(finiteNumber(signal.dataQuality))}`,
      state: String(signal.level || signal.risk || "WATCH").toUpperCase(),
      severity: severityKey(signal.level || signal.risk),
      direction: sideDirection(signal.side),
      pushState: signal.pushedAt ? "Discord 已推送" : signal.risk === "high" ? "待处理" : "页面观察",
      href: "/signals",
    });
  });

  const seenLogKinds = new Set();
  (snapshot?.scanLogs || [])
    .filter(log => log.level === "warn" || log.level === "error")
    .slice()
    .reverse()
    .forEach(log => {
      const dedupeKey = `${log.kind}:${log.symbol}`;
      if (seenLogKinds.has(dedupeKey) || seenLogKinds.size >= 6) return;
      seenLogKinds.add(dedupeKey);
      items.push({
        id: `system:${log.id}`,
        ts: epochMs(log.tsMs),
        channel: "system",
        channelLabel: log.level === "error" ? "ERROR" : "WARN",
        symbol: baseSymbol(log.symbol) || "SYS",
        title: readableToken(log.kind),
        detail: log.message,
        value: log.level.toUpperCase(),
        secondary: log.candidateId ? `ID ${String(log.candidateId).slice(-10)}` : "运行日志",
        state: log.level === "error" ? "ACTION" : "CHECK",
        severity: log.level === "error" ? "critical" : "high",
        direction: "neutral",
        pushState: "系统可观测",
        href: null,
      });
    });

  const quality = snapshot?.system?.marketDataQuality;
  if (quality?.latestTradeTs || quality?.lastMessageTs) {
    items.push({
      id: `health:${quality.latestTradeTs || quality.lastMessageTs}`,
      ts: epochMs(quality.latestTradeTs || quality.lastMessageTs),
      channel: "system",
      channelLabel: "HEALTH",
      symbol: "SYS",
      title: "市场数据链路心跳",
      detail: quality.operatorWarning || `事件总线 ${String(quality.status || "unknown").toUpperCase()}`,
      value: String(quality.status || "UNKNOWN").toUpperCase(),
      secondary: `${finiteNumber(quality.recentLaggedEvents)} recent lag`,
      state: String(quality.status || "CHECK").toUpperCase(),
      severity: quality.status === "healthy" ? "info" : "high",
      direction: "neutral",
      pushState: "实时状态",
      href: null,
    });
  }

  const deduped = new Map();
  items
    .filter(item => Number.isFinite(item.ts) && item.ts > 0)
    .forEach(item => {
      if (!deduped.has(item.id)) deduped.set(item.id, item);
    });
  return Array.from(deduped.values()).sort((left, right) => right.ts - left.ts);
}

function buildBtcPulse(snapshot) {
  const contract = snapshot?.contract?.BTC?.error || snapshot?.contract?.BTC?.summary?.enabled === false ? null : snapshot?.contract?.BTC?.summary;
  const spot = snapshot?.spot?.BTC?.error || snapshot?.spot?.BTC?.summary?.enabled === false ? null : snapshot?.spot?.BTC?.summary;
  const candle = latestFlowCandle(snapshot?.orderflow?.candles);
  const contractNet = availableNumber(contract?.trend60s?.netVolumeBtc);
  const spotNet = availableNumber(spot?.trend60s?.netVolumeBase);
  const spotDominance = availableNumber(spot?.trend60s?.dominance);
  const hasFlow = Number(snapshot?.orderflow?.monitorFlowCandles) > 0 || candle?.buyBase > 0 || candle?.sellBase > 0;
  const delta = hasFlow ? availableNumber(candle?.deltaBase) : null;
  const vpin = hasFlow ? availableNumber(candle?.vpin) : null;
  const tofVolume = hasFlow ? availableNumber(candle?.tofVolumeBtc) : null;
  return [
    { label: "合约净流 60S", value: contractNet === null ? "等待数据" : signedVolume(contractNet, "BTC"), detail: contractNet === null ? "合约成交证据未接入" : directionLabel(contractNet === 0 ? contract?.latestDirection : contractNet > 0 ? "buy" : "sell"), tone: valueTone(contractNet) },
    { label: "现货净流 60S", value: spotNet === null ? "等待数据" : signedVolume(spotNet, "BTC"), detail: spotDominance === null ? "等待现货确认" : `主导 ${(spotDominance * 100).toFixed(0)}%`, tone: valueTone(spotNet) },
    { label: "1H DELTA", value: delta === null ? "等待数据" : signedVolume(delta, "BTC"), detail: delta === null ? "缺少主动买卖量" : signedPercent(candle.deltaPct), tone: valueTone(delta) },
    { label: "VPIN", value: vpin === null ? "等待基线" : vpin.toFixed(3), detail: vpin === null ? "尚无有效读数" : candle?.vpinExtreme ? "EXTREME" : candle?.vpinHigh ? "HIGH" : candle?.vpinSpike ? "SPIKE" : "NORMAL", tone: vpin === null ? "neutral" : candle?.vpinHigh || candle?.vpinExtreme ? "warn" : "ok" },
    { label: "TOF", value: tofVolume === null ? "等待数据" : candle?.tofAlert ? "ALERT" : "NORMAL", detail: tofVolume === null ? "尚无有效订单流" : tofVolume === 0 ? "尚无异常桶" : formatVolume(tofVolume, "BTC"), tone: tofVolume === null ? "neutral" : candle?.tofAlert ? "warn" : "ok" },
    { label: "综合质量", value: `${Math.round(finiteNumber(contract?.overallDataQuality)) || "—"}`, detail: `${contract?.activeExchangeCount || 0} 个合约源`, tone: finiteNumber(contract?.overallDataQuality) >= 70 ? "ok" : "warn" },
  ];
}

function availableNumber(value) {
  if (value === null || value === undefined || value === "") return null;
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function buildMonitorHealth(snapshot, events, context) {
  const venues = Object.values(snapshot?.system?.venues || {});
  const enabledVenues = venues.filter(venue => venue?.enabled);
  const activePerp = enabledVenues.filter(venue => venue?.tradeActive || venue?.status === "connected").length;
  const spotExchanges = Object.values(snapshot?.spot?.BTC?.summary?.exchanges || {});
  const enabledSpot = snapshot?.spot?.BTC?.summary?.enabled ? spotExchanges.length : 0;
  const activeSpot = spotExchanges.filter(exchange => exchange?.connected).length;
  const activeLinks = activePerp + activeSpot;
  const totalLinks = enabledVenues.length + enabledSpot;
  const messageTimes = venues.map(venue => epochMs(venue?.lastMessageTs)).filter(value => value > 0);
  const latestMessageAt = messageTimes.length ? Math.max(...messageTimes) : 0;
  const lagMs = latestMessageAt ? Math.max(0, Date.now() - latestMessageAt) : null;
  const qualityStatus = String(snapshot?.system?.marketDataQuality?.status || "unknown").toLowerCase();
  const now = Date.now();
  const hourAgo = now - 60 * 60 * 1000;
  const recentEvents = events.filter(event => event.ts >= hourAgo && event.channel !== "system");
  const alerts1h = recentEvents.filter(event => event.severity === "critical" || event.severity === "high").length;
  const storage = snapshot?.system?.storage;
  const runtimeKnown = context.runtimeBoundary?.phase === "confirmed";
  return {
    activeLinks,
    totalLinks: totalLinks || activeLinks,
    linkDetail: totalLinks ? `${activePerp} 合约 · ${activeSpot} 现货` : "等待状态快照",
    linkTone: activeLinks > 0 && activeLinks === totalLinks ? "ok" : activeLinks > 0 ? "warn" : "bad",
    latencyLabel: lagMs === null ? "—" : lagMs < 1_000 ? `${Math.round(lagMs)} ms` : `${(lagMs / 1_000).toFixed(1)} s`,
    qualityLabel: qualityStatus === "unknown" ? "质量待确认" : `数据质量 ${qualityStatus.toUpperCase()}`,
    qualityTone: qualityStatus === "healthy" ? "ok" : qualityStatus === "unknown" ? "neutral" : "warn",
    events1h: recentEvents.length,
    alerts1h,
    discordDetail: `${context.discordConnected ? "通道已配置" : "各通道独立门控"} · WS ${String(context.wsStatus).toUpperCase()}`,
    storageLabel: String(storage?.status || (storage?.enabled ? "READY" : "CHECK")).toUpperCase(),
    storageDetail: storage?.lastWriteTs ? `写入 ${formatRelativeTime(epochMs(storage.lastWriteTs))}` : storage?.enabled ? "等待首次写入" : "状态待确认",
    storageTone: storage?.enabled && !storage?.lastError ? "ok" : "warn",
    runtimeKnown,
  };
}

function latestFlowCandle(candles) {
  const rows = Array.isArray(candles) ? candles : [];
  return rows.slice().reverse().find(candle => candle.buyBase > 0 || candle.sellBase > 0) || rows.at(-1) || null;
}

function contractTitle(item) {
  if (item.finalResult && !/candidate/i.test(item.finalResult)) return item.finalResult;
  return readableToken(item.displaySignalType || item.signalType || "合约主力事件");
}

function spotTitle(item) {
  if (item.finalResult && !/candidate/i.test(item.finalResult)) return item.finalResult;
  return readableToken(item.signalType || "现货鲸鱼事件");
}

function ratingLabel(item) {
  const grade = String(item.impactGrade || "").toUpperCase();
  if (grade && !["UNRATED", "N/A", "NA"].includes(grade)) return `V3 ${grade}`;
  const level = String(item.signalLevel || "").toUpperCase();
  return level && !["UNRATED", "N/A", "NA"].includes(level) ? level : "V3 观察";
}

function severityKey(value) {
  const normalized = String(value || "").toLowerCase();
  if (["s", "critical", "extreme"].includes(normalized)) return "critical";
  if (["a", "high", "strong"].includes(normalized)) return "high";
  if (["b", "medium", "moderate", "watch"].includes(normalized)) return "medium";
  return "info";
}

function directionKey(value, fallback = "") {
  const numeric = Number(value);
  if (Number.isFinite(numeric) && numeric > 0) return "buy";
  if (Number.isFinite(numeric) && numeric < 0) return "sell";
  const text = String(fallback || "").toLowerCase();
  if (/buy|long|up|bull|absorption/.test(text)) return "buy";
  if (/sell|short|down|bear|suppression/.test(text)) return "sell";
  return "neutral";
}

function sideDirection(side) {
  return directionKey(Number.NaN, side);
}

function valueTone(value) {
  if (value === null || value === undefined || Number(value) === 0) return "neutral";
  return Number(value) > 0 ? "buy" : "sell";
}

function directionLabel(value) {
  const text = String(value || "neutral").toLowerCase();
  if (/buy|long|up|bull/.test(text)) return "主动买入占优";
  if (/sell|short|down|bear/.test(text)) return "主动卖出占优";
  if (text === "absorption") return "下方吸收";
  if (text === "suppression") return "上方压制";
  return "方向中性";
}

function baseSymbol(value) {
  const text = String(value || "").toUpperCase();
  if (text.includes("ETH")) return "ETH";
  if (text.includes("BTC")) return "BTC";
  return text.replace(/[-_/]?(USDT|USD|PERP|SWAP).*$/, "") || "SYS";
}

function epochMs(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric <= 0) return 0;
  return numeric < 10_000_000_000 ? numeric * 1_000 : numeric;
}

function signalTimestamp(signal, index = 0) {
  const explicit = epochMs(signal?.ts ?? signal?.timestamp ?? signal?.createdAtMs);
  if (explicit) return explicit;
  const parsed = Date.parse(signal?.time || signal?.createdAt || "");
  if (Number.isFinite(parsed)) return parsed;
  return Date.now() - index;
}

function finiteNumber(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : 0;
}

function signedVolume(value, symbol) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return "—";
  const sign = numeric > 0 ? "+" : numeric < 0 ? "−" : "";
  const absolute = Math.abs(numeric);
  const digits = absolute >= 1_000 ? 0 : absolute >= 100 ? 1 : absolute >= 10 ? 2 : 3;
  return `${sign}${absolute.toLocaleString("zh-CN", { maximumFractionDigits: digits })} ${symbol}`;
}

function formatVolume(value, symbol) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return "—";
  return `${numeric.toLocaleString("zh-CN", { maximumFractionDigits: numeric >= 100 ? 0 : 2 })} ${symbol}`;
}

function formatCompactUsd(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric <= 0) return "$—";
  if (numeric >= 1_000_000_000) return `$${(numeric / 1_000_000_000).toFixed(2)}B`;
  if (numeric >= 1_000_000) return `$${(numeric / 1_000_000).toFixed(1)}M`;
  if (numeric >= 1_000) return `$${(numeric / 1_000).toFixed(0)}K`;
  return `$${numeric.toFixed(0)}`;
}

function signedPercent(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return "—";
  return `${numeric > 0 ? "+" : ""}${numeric.toFixed(1)}%`;
}

function formatNumber(value, digits = 2) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric.toFixed(digits) : "—";
}

function formatPercentile(value) {
  const numeric = Number(value);
  if (!Number.isFinite(numeric)) return "—";
  return `${Math.round((numeric <= 1 ? numeric * 100 : numeric))}%`;
}

function windowLabel(seconds) {
  const value = Number(seconds);
  if (!Number.isFinite(value) || value <= 0) return "实时窗口";
  if (value >= 3_600) return `${Math.round(value / 3_600)}H`;
  if (value >= 60) return `${Math.round(value / 60)}M`;
  return `${Math.round(value)}S`;
}

function formatClock(value) {
  const ms = epochMs(value);
  if (!ms) return "等待首次";
  return new Date(ms).toLocaleTimeString("zh-CN", { hour12: false });
}

function formatEventTime(value) {
  const date = new Date(value);
  return date.toLocaleTimeString("zh-CN", { hour12: false });
}

function formatRelativeTime(value) {
  const seconds = Math.max(0, Math.round((Date.now() - value) / 1_000));
  if (seconds < 5) return "刚刚";
  if (seconds < 60) return `${seconds} 秒前`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} 分钟前`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} 小时前`;
  return `${Math.round(hours / 24)} 天前`;
}

function readableToken(value) {
  return String(value || "")
    .replace(/_/g, " ")
    .replace(/\b\w/g, character => character.toUpperCase());
}
