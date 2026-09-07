import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router-dom";
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
  const flowRootRef = useRef(null);
  const pointerFrameRef = useRef(0);

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

  useEffect(() => () => {
    if (pointerFrameRef.current) window.cancelAnimationFrame(pointerFrameRef.current);
  }, []);

  const handlePointerMove = useCallback((event) => {
    if (reducedMotion || !pageVisible || !flowRootRef.current) return;
    if (pointerFrameRef.current) window.cancelAnimationFrame(pointerFrameRef.current);
    const clientX = event.clientX;
    const clientY = event.clientY;
    pointerFrameRef.current = window.requestAnimationFrame(() => {
      const rect = flowRootRef.current?.getBoundingClientRect();
      if (!rect || !flowRootRef.current) return;
      flowRootRef.current.style.setProperty("--flow-pointer-x", `${clientX - rect.left}px`);
      flowRootRef.current.style.setProperty("--flow-pointer-y", `${clientY - rect.top}px`);
    });
  }, [pageVisible, reducedMotion]);

  return (
    <section
      className={`monitor-flow ${pageVisible ? "is-visible" : "is-backgrounded"} ${reducedMotion ? "reduce-motion" : ""}`}
      data-testid="monitor-flow-dashboard"
      onPointerMove={handlePointerMove}
      ref={flowRootRef}
    >
      <div className="monitor-flow-hero">
        <NeuralFieldCanvas
          active={pageVisible && !paused && !reducedMotion}
          intensity={Math.min(1, (events.length + health.alerts1h * 4) / 80)}
        />
        <div className="monitor-flow-hero-scan" aria-hidden="true"><i /><i /><i /></div>
        <div>
          <p className="monitor-flow-kicker">AI MARKET OBSERVATORY · GLOBAL NEURAL FLOW · READ ONLY</p>
          <div className="monitor-flow-title-row">
            <h1>全市场实时监控流</h1>
            <span className={`monitor-flow-live ${paused ? "is-paused" : ""}`}>
              <i aria-hidden="true" /> {paused ? "PAUSED" : "LIVE"}
            </span>
          </div>
          <p className="monitor-flow-subtitle">
            统一观察 BTC / ETH 合约、现货、Delta、VPIN/TOF 与告警链路；点击事件进入对应深度页面。
          </p>
        </div>
        <div className="monitor-flow-ai-core" aria-label="AI 证据融合状态">
          <div className="monitor-flow-ai-core-visual" aria-hidden="true">
            <svg className="monitor-flow-ai-core-mesh" viewBox="0 0 100 100">
              <path d="M50 9 84 29 84 70 50 91 16 70 16 29Z" />
              <path d="m50 9 18 41-18 41-18-41Z" />
              <path d="M16 29 68 50 16 70M84 29 32 50 84 70" />
            </svg>
            <i /><i /><i /><b />
          </div>
          <div className="monitor-flow-ai-core-copy">
            <span>AI SIGNAL FUSION</span>
            <strong>{health.activeLinks > 0 ? "EVIDENCE ACTIVE" : "AWAITING DATA"}</strong>
            <small>{health.activeLinks} SOURCES · {health.alerts1h} PRIORITY</small>
          </div>
        </div>
        <div className="monitor-flow-hero-actions">
          <span>{formatClock(snapshot?.fetchedAtMs)} 更新</span>
          <button className="monitor-flow-button" disabled={loading} onClick={refresh} type="button">
            {loading ? "同步中" : "立即同步"}
          </button>
          <button
            aria-pressed={paused}
            className={`monitor-flow-button ${paused ? "is-active" : ""}`}
            onClick={() => setPaused(value => !value)}
            type="button"
          >
            {paused ? "继续流" : "暂停流"}
          </button>
        </div>
      </div>

      <div className="monitor-flow-neural-bus" aria-label="AI 神经数据总线">
        <div className="monitor-flow-neural-bus-label"><i aria-hidden="true" /><span>AI NEURAL BUS</span></div>
        <div className="monitor-flow-neural-bus-window">
          <div className="monitor-flow-neural-bus-track">
            {[0, 1].map(copyIndex => (
              <div aria-hidden={copyIndex === 1} className="monitor-flow-neural-bus-segment" key={copyIndex}>
                <span>SOURCES <strong>{health.activeLinks}/{health.totalLinks}</strong></span>
                <span>EVIDENCE <strong>{events.length}</strong></span>
                <span>HIGH PRIORITY <strong>{health.alerts1h}</strong></span>
                <span>{health.qualityLabel}</span>
                <span>V3 RATING <strong>ONLINE</strong></span>
                <span>READ ONLY <strong>SAFE</strong></span>
              </div>
            ))}
          </div>
        </div>
        <span className="monitor-flow-neural-bus-clock">{formatClock(snapshot?.fetchedAtMs)}</span>
      </div>

      {signalsError || refreshError ? (
        <div className="monitor-flow-warning" role="status">
          部分数据源暂时不可用，页面保留最近一次成功快照。{signalsError ? ` 信号：${signalsError}` : ""}
        </div>
      ) : null}

      <div className="monitor-flow-hud" aria-label="监控链路状态">
        <HudCell label="采集链路" value={`${health.activeLinks}/${health.totalLinks}`} detail={health.linkDetail} tone={health.linkTone} />
        <HudCell label="最新数据延迟" value={health.latencyLabel} detail={health.qualityLabel} tone={health.qualityTone} />
        <HudCell label="近 1H 事件" value={String(health.events1h)} detail={`${health.alerts1h} 条高优先级`} tone={health.alerts1h > 0 ? "warn" : "ok"} />
        <HudCell label="Discord" value={discordConnected ? "READY" : "GATED"} detail={health.discordDetail} tone={discordConnected ? "ok" : "neutral"} />
        <HudCell label="SQLite" value={health.storageLabel} detail={health.storageDetail} tone={health.storageTone} />
      </div>

      <AiEvidenceMatrix events={events} health={health} />

      <div className="monitor-flow-layout">
        <section className="monitor-flow-panel monitor-flow-tape">
          <header className="monitor-flow-panel-header">
            <div>
              <p>UNIFIED EVENT TAPE</p>
              <h2>实时监控事件流</h2>
            </div>
            <span>{visibleEvents.length} / {events.length} EVENTS</span>
          </header>

          <div className="monitor-flow-filters" aria-label="事件类型筛选">
            {FILTERS.map(([key, label]) => (
              <button
                aria-pressed={activeFilter === key}
                className={activeFilter === key ? "is-active" : ""}
                key={key}
                onClick={() => setActiveFilter(key)}
                type="button"
              >
                {label}
                <small>{key === "all" ? events.length : events.filter(event => event.channel === key).length}</small>
              </button>
            ))}
          </div>

          <div className="monitor-flow-tape-head" aria-hidden="true">
            <span>时间</span><span>来源</span><span>事件</span><span>核心读数</span><span>状态</span>
          </div>
          <div className="monitor-flow-event-list" data-testid="monitor-flow-event-list">
            {visibleEvents.length ? visibleEvents.map(event => (
              <MonitorEventRow active={activeEventIds.has(event.id)} event={event} key={event.id} />
            )) : (
              <div className="monitor-flow-empty">
                <strong>{loading ? "正在接入监控流" : "当前筛选暂无事件"}</strong>
                <span>这不代表数据链路中断；低于阈值的数据不会生成事件卡片。</span>
              </div>
            )}
          </div>
        </section>

        <aside className="monitor-flow-side">
          <section className="monitor-flow-panel monitor-flow-pulse">
            <header className="monitor-flow-panel-header">
              <div>
                <p>BTC MARKET PULSE</p>
                <h2>BTC 当前脉冲</h2>
              </div>
              <Link to="/contract-whale/btc">深度页 →</Link>
            </header>
            <div className="monitor-flow-pulse-grid">
              {pulse.map(item => <PulseCell item={item} key={item.label} />)}
            </div>
          </section>

          <section className="monitor-flow-panel monitor-flow-routes">
            <header className="monitor-flow-panel-header">
              <div>
                <p>QUICK ROUTES</p>
                <h2>深度监控入口</h2>
              </div>
            </header>
            <div className="monitor-flow-route-grid">
              <Link to="/contract-whale/btc"><span>BTC</span>合约事件带<small>V3 评级 · OI · Funding</small></Link>
              <Link to="/spot-monitor/btc"><span>SPOT</span>现货鲸鱼流<small>净方向 · 跨所确认</small></Link>
              <Link to="/binance-orderflow"><span>FLOW</span>订单流 K 线<small>Delta · VPIN · TOF</small></Link>
              <Link to="/signals"><span>RISK</span>异常候选<small>S 级 · Discord Gate</small></Link>
            </div>
          </section>
        </aside>
      </div>

      <section className="monitor-flow-panel monitor-flow-pipeline">
        <header className="monitor-flow-panel-header">
          <div>
            <p>OBSERVABILITY PIPELINE</p>
            <h2>监控链路</h2>
          </div>
          <span>只读聚合 · 无额外持久化</span>
        </header>
        <div className="monitor-flow-pipeline-track">
          {[
            ["01", "交易所", health.activeLinks > 0 ? "ACTIVE" : "WAIT"],
            ["02", "采集器", health.activeLinks > 0 ? "STREAMING" : "WAIT"],
            ["03", "检测器", health.qualityLabel],
            ["04", "V3 评级", "EVIDENCE"],
            ["05", "Discord", discordConnected ? "READY" : "GATED"],
            ["06", "冷热存储", health.storageLabel],
          ].map(([index, label, status], itemIndex, items) => (
              <div className={`monitor-flow-pipeline-node ${health.activeLinks > 0 ? "is-active" : ""}`} key={index}>
              <div><small>{index}</small><strong>{label}</strong><span>{status}</span></div>
              {itemIndex < items.length - 1 ? <i aria-hidden="true">→</i> : null}
            </div>
          ))}
        </div>
      </section>
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

function NeuralFieldCanvas({ active, intensity = 0.5 }) {
  const canvasRef = useRef(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || (typeof navigator !== "undefined" && /jsdom/i.test(navigator.userAgent))) return undefined;
    const context = canvas.getContext?.("2d");
    if (!context) return undefined;

    let animationFrame = 0;
    let width = 0;
    let height = 0;
    let pixelRatio = 1;
    let nodes = [];

    const buildNodes = () => {
      const count = Math.round(22 + intensity * 18);
      nodes = Array.from({ length: count }, (_, index) => ({
        x: ((index * 47) % 101) / 100,
        y: ((index * 71 + 13) % 97) / 96,
        phase: index * 0.73,
        speed: 0.55 + (index % 7) * 0.08,
        radius: index % 9 === 0 ? 1.8 : 0.8 + (index % 3) * 0.22,
      }));
    };

    const resize = () => {
      const rect = canvas.getBoundingClientRect();
      width = Math.max(1, rect.width);
      height = Math.max(1, rect.height);
      pixelRatio = Math.min(window.devicePixelRatio || 1, 2);
      canvas.width = Math.round(width * pixelRatio);
      canvas.height = Math.round(height * pixelRatio);
      context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
      buildNodes();
    };

    const draw = (time = 0) => {
      context.clearRect(0, 0, width, height);
      const points = nodes.map(node => ({
        ...node,
        px: node.x * width + Math.sin(time * 0.00018 * node.speed + node.phase) * 14,
        py: node.y * height + Math.cos(time * 0.00015 * node.speed + node.phase) * 8,
      }));

      for (let leftIndex = 0; leftIndex < points.length; leftIndex += 1) {
        const left = points[leftIndex];
        for (let rightIndex = leftIndex + 1; rightIndex < points.length; rightIndex += 1) {
          const right = points[rightIndex];
          const distance = Math.hypot(left.px - right.px, left.py - right.py);
          if (distance > 118) continue;
          context.beginPath();
          context.moveTo(left.px, left.py);
          context.lineTo(right.px, right.py);
          context.strokeStyle = `rgba(84, 218, 246, ${Math.max(0, (1 - distance / 118) * 0.13)})`;
          context.lineWidth = 0.55;
          context.stroke();
        }
      }

      points.forEach((point, index) => {
        const pulse = 0.65 + Math.sin(time * 0.0018 + point.phase) * 0.28;
        context.beginPath();
        context.arc(point.px, point.py, point.radius * pulse, 0, Math.PI * 2);
        context.fillStyle = index % 8 === 0 ? "rgba(91, 232, 178, .78)" : "rgba(113, 229, 248, .62)";
        context.shadowColor = index % 8 === 0 ? "rgba(83, 230, 177, .75)" : "rgba(100, 231, 245, .75)";
        context.shadowBlur = point.radius > 1.5 ? 10 : 5;
        context.fill();
      });
      context.shadowBlur = 0;

      if (active) animationFrame = window.requestAnimationFrame(draw);
    };

    resize();
    draw(performance.now());
    const resizeObserver = typeof ResizeObserver === "function" ? new ResizeObserver(resize) : null;
    resizeObserver?.observe(canvas);
    window.addEventListener("resize", resize);

    return () => {
      if (animationFrame) window.cancelAnimationFrame(animationFrame);
      resizeObserver?.disconnect();
      window.removeEventListener("resize", resize);
    };
  }, [active, intensity]);

  return <canvas aria-hidden="true" className="monitor-flow-neural-field" data-testid="ai-neural-field" ref={canvasRef} />;
}

function AiEvidenceMatrix({ events, health }) {
  const recent = events.slice(0, 30).reverse();
  const directionCounts = recent.reduce((counts, event) => {
    const key = event.direction === "buy" || event.direction === "sell" ? event.direction : "neutral";
    counts[key] += 1;
    return counts;
  }, { buy: 0, sell: 0, neutral: 0 });
  const severityCounts = recent.reduce((counts, event) => {
    const key = ["critical", "high", "medium"].includes(event.severity) ? event.severity : "info";
    counts[key] += 1;
    return counts;
  }, { critical: 0, high: 0, medium: 0, info: 0 });
  const sourceCount = new Set(recent.map(event => event.channel)).size;
  const directionalTotal = directionCounts.buy + directionCounts.sell;
  const buyShare = directionalTotal ? directionCounts.buy / directionalTotal : 0.5;
  const riskShare = recent.length ? (severityCounts.critical + severityCounts.high) / recent.length : 0;
  const vectorPoints = recent.map((event, index) => {
    const x = recent.length <= 1 ? 50 : 3 + (index / (recent.length - 1)) * 94;
    const direction = event.direction === "buy" ? 1 : event.direction === "sell" ? -1 : 0;
    const severity = event.severity === "critical" ? 1 : event.severity === "high" ? 0.72 : event.severity === "medium" ? 0.42 : 0.18;
    const y = 30 - direction * (7 + severity * 12) + (index % 3 - 1) * 1.2;
    return { ...event, x, y: Math.max(5, Math.min(55, y)) };
  });
  const polyline = vectorPoints.map(point => `${point.x.toFixed(1)},${point.y.toFixed(1)}`).join(" ");
  const bias = buyShare > 0.58 ? "BUY VECTOR" : buyShare < 0.42 ? "SELL VECTOR" : "BALANCED";

  return (
    <section className="monitor-flow-panel monitor-flow-intelligence" data-testid="ai-evidence-matrix">
      <header className="monitor-flow-intelligence-header">
        <div>
          <p>AI EVIDENCE MATRIX</p>
          <strong>实时证据向量融合</strong>
        </div>
        <span><i aria-hidden="true" /> SYNTHESIS LIVE</span>
      </header>

      <div className="monitor-flow-intelligence-chart">
        <div className="monitor-flow-vector-labels" aria-hidden="true"><span>BUY</span><span>NEUTRAL</span><span>SELL</span></div>
        <svg aria-label="最近事件方向与等级向量图" preserveAspectRatio="none" viewBox="0 0 100 60">
          <defs>
            <linearGradient id="flow-vector-gradient" x1="0" x2="1">
              <stop offset="0" stopColor="#5ea7ff" />
              <stop offset="0.5" stopColor="#64e7f5" />
              <stop offset="1" stopColor="#53e6b1" />
            </linearGradient>
            <linearGradient id="flow-vector-fill" x1="0" x2="0" y1="0" y2="1">
              <stop offset="0" stopColor="#64e7f5" stopOpacity=".18" />
              <stop offset="1" stopColor="#64e7f5" stopOpacity="0" />
            </linearGradient>
          </defs>
          <g className="monitor-flow-vector-grid"><path d="M0 12H100M0 30H100M0 48H100" /><path d="M12 0V60M25 0V60M38 0V60M51 0V60M64 0V60M77 0V60M90 0V60" /></g>
          {polyline ? <polygon className="monitor-flow-vector-fill" points={`3,60 ${polyline} 97,60`} /> : null}
          {polyline ? <polyline className="monitor-flow-vector-line" points={polyline} /> : null}
          {vectorPoints.map(point => (
            <circle className={`vector-${point.direction} severity-${point.severity}`} cx={point.x} cy={point.y} key={point.id} r={point.severity === "critical" ? 1.35 : 0.82} />
          ))}
          <line className="monitor-flow-vector-scan" x1="0" x2="0" y1="0" y2="60" />
        </svg>
        <div className="monitor-flow-vector-readout">
          <span>{recent.length} EVIDENCE</span><strong>{bias}</strong><small>{sourceCount} ACTIVE LAYERS</small>
        </div>
      </div>

      <div className="monitor-flow-consensus">
        <div className="monitor-flow-consensus-title"><span>MARKET CONSENSUS</span><strong>{Math.round(buyShare * 100)}%</strong></div>
        <div className="monitor-flow-consensus-bar" aria-label={`买向证据 ${directionCounts.buy}，卖向证据 ${directionCounts.sell}`}>
          <i className="is-buy" style={{ width: `${buyShare * 100}%` }} />
          <i className="is-sell" style={{ width: `${(1 - buyShare) * 100}%` }} />
        </div>
        <div className="monitor-flow-consensus-counts">
          <span>BUY <strong>{directionCounts.buy}</strong></span>
          <span>NEUTRAL <strong>{directionCounts.neutral}</strong></span>
          <span>SELL <strong>{directionCounts.sell}</strong></span>
        </div>
      </div>

      <div className="monitor-flow-risk-radar">
        <div className="monitor-flow-risk-orbit" style={{ "--risk-angle": `${Math.max(8, riskShare * 360)}deg` }}>
          <i /><b>{health.alerts1h}</b>
        </div>
        <div>
          <span>THREAT RADAR</span>
          <strong>{riskShare >= 0.5 ? "ELEVATED" : riskShare > 0 ? "TRACKING" : "CLEAR"}</strong>
          <small>S {severityCounts.critical} · A/HIGH {severityCounts.high}</small>
        </div>
      </div>
    </section>
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
  const contract = snapshot?.contract?.BTC?.summary;
  const spot = snapshot?.spot?.BTC?.summary;
  const candle = latestFlowCandle(snapshot?.orderflow?.candles);
  const contractNet = finiteNumber(contract?.trend60s?.netVolumeBtc);
  const spotNet = finiteNumber(spot?.trend60s?.netVolumeBase);
  const delta = candle ? finiteNumber(candle.deltaBase) : null;
  const vpin = candle?.vpin;
  return [
    { label: "合约净流 60S", value: signedVolume(contractNet, "BTC"), detail: directionLabel(contractNet === 0 ? contract?.latestDirection : contractNet > 0 ? "buy" : "sell"), tone: valueTone(contractNet) },
    { label: "现货净流 60S", value: signedVolume(spotNet, "BTC"), detail: `主导 ${(finiteNumber(spot?.trend60s?.dominance) * 100).toFixed(0)}%`, tone: valueTone(spotNet) },
    { label: "1H DELTA", value: delta === null ? "等待数据" : signedVolume(delta, "BTC"), detail: candle ? signedPercent(candle.deltaPct) : "—", tone: valueTone(delta) },
    { label: "VPIN", value: vpin === null || vpin === undefined ? "等待基线" : vpin.toFixed(3), detail: candle?.vpinExtreme ? "EXTREME" : candle?.vpinHigh ? "HIGH" : candle?.vpinSpike ? "SPIKE" : "NORMAL", tone: candle?.vpinHigh || candle?.vpinExtreme ? "warn" : "ok" },
    { label: "TOF", value: candle?.tofAlert ? "ALERT" : "NORMAL", detail: !candle?.tofVolumeBtc ? "尚无异常桶" : formatVolume(candle.tofVolumeBtc, "BTC"), tone: candle?.tofAlert ? "warn" : "ok" },
    { label: "综合质量", value: `${Math.round(finiteNumber(contract?.overallDataQuality)) || "—"}`, detail: `${contract?.activeExchangeCount || 0} 个合约源`, tone: finiteNumber(contract?.overallDataQuality) >= 70 ? "ok" : "warn" },
  ];
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
