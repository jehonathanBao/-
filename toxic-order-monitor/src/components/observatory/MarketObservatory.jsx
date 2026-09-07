import { useEffect, useId, useMemo, useRef, useState } from "react";
import { buildRidgePath, clockLabel, formatReading, groupObservations, normalizeFlowSamples, normalizeObservations } from "./model.js";

const LABELS = { buy: "买方", sell: "卖方", neutral: "中性" };

function InstrumentHeader({ number, title, code, children }) {
  return <header className="instrument-header"><div><i aria-hidden="true" /><h2>{title}</h2><span>{code}</span></div><div>{children}<small>{number}</small></div></header>;
}

export default function MarketObservatory({ samples = [], observations = [], symbol = "BTC", mode = "candles", paused = false, stale = false, demo = false }) {
  const rows = useMemo(() => normalizeFlowSamples(samples, symbol), [samples, symbol]);
  const events = useMemo(() => normalizeObservations(observations, symbol), [observations, symbol]);
  const [direction, setDirection] = useState("all");
  const filtered = useMemo(() => direction === "all" ? events : events.filter(event => event.direction === direction), [direction, events]);
  return <section className="market-observatory" aria-label={`${symbol} 动态观测仪表`} data-paused={paused ? "true" : "false"}>
    <div className="instrument-section-label"><span><b>02</b> FLOW LABORATORY <i>/</i> 成交与证据观测</span><span>{demo ? "固定演示样本" : stale ? "保留快照 · 数据待更新" : "已加载样本"} · {symbol}</span></div>
    <FlowRidge rows={rows} symbol={symbol} mode={mode} />
    <div className="instrument-pair">
      <VenueChord events={events} />
      <EvidenceLattice events={events} paused={paused} />
    </div>
    <section className="instrument-panel instrument-clusters">
      <InstrumentHeader number="04" title="方向关系图" code="DIRECTION CLUSTERS">
        <div className="instrument-segments" aria-label="方向关系筛选">{[["all", "全部方向"], ["buy", "买方"], ["sell", "卖方"]].map(([key, label]) => <button type="button" aria-pressed={direction === key} key={key} onClick={() => setDirection(key)}>{label}</button>)}</div>
      </InstrumentHeader>
      <DirectionClusters events={filtered} total={events.length} />
      <footer className="instrument-footer"><span>节点 = 已加载事件 · 连线 = 方向分组</span><span>不表示资金转移或账户关系</span></footer>
    </section>
  </section>;
}

function FlowRidge({ rows, symbol, mode }) {
  const [selectedId, setSelectedId] = useState(null);
  const selected = rows.find(row => row.id === selectedId) || rows.at(-1);
  const maximum = Math.max(1, ...rows.flatMap(row => [row.buy, row.sell]));
  const rendered = rows.length ? rows : Array.from({ length: 24 }, () => null);
  const totalBuy = rows.reduce((sum, row) => sum + row.buy, 0);
  const totalSell = rows.reduce((sum, row) => sum + row.sell, 0);
  const buyShare = totalBuy + totalSell > 0 ? totalBuy / (totalBuy + totalSell) * 100 : null;
  const gradient = `ridge-${useId().replace(/:/g, "")}`;
  return <section className="instrument-panel instrument-ridge">
    <InstrumentHeader number="01" title="主动成交地形" code="FLOW TOPOGRAPHY"><span>{mode === "candles" ? "1H KLINE" : "EVENT SAMPLES"} / {symbol}</span></InstrumentHeader>
    <div className="instrument-ridge-body">
      <aside className="instrument-readouts">
        <p>BUY / SELL LANDSCAPE</p>
        <strong className="instrument-hero-number">{formatReading(buyShare)}<small>{buyShare === null ? "" : "%"}</small></strong>
        <span>样本买入占比</span>
        <dl><div><dt>样本数量</dt><dd>{rows.length.toString().padStart(2, "0")}</dd></div><div><dt>主动买入</dt><dd className="ink-buy">{rows.length ? formatReading(totalBuy) : "—"}</dd></div><div><dt>主动卖出</dt><dd className="ink-sell">{rows.length ? formatReading(totalSell) : "—"}</dd></div><div><dt>计量单位</dt><dd>{symbol}</dd></div></dl>
        <span className="instrument-micro-note">{mode === "candles" ? "各层代表一根 K 线" : "事件窗口可能重叠，不代表区间总量"}</span>
      </aside>
      <div className="instrument-ridge-plot">
        <div className="instrument-plot-meta"><span>SELL PRESSURE ←</span><span>→ BUY PRESSURE</span></div>
        <svg viewBox="0 0 800 295" role="img" aria-label={rows.length ? `${symbol} 主动成交买卖量分层曲面` : "暂无成交样本，显示空白坐标网格"}>
          <defs><linearGradient id={gradient}><stop offset="0" stopColor="#c47482" /><stop offset=".5" stopColor="#aeb6b3" /><stop offset="1" stopColor="#278d77" /></linearGradient></defs>
          <g className="ridge-camera">
            {Array.from({ length: 11 }, (_, i) => <line className="instrument-guide" key={i} x1={45 + i * 53} x2={200 + i * 53} y1="250" y2="148" />)}
            {[...rendered].reverse().map((_, reversed) => {
              const index = rendered.length - 1 - reversed;
              const row = rendered[index];
              const active = Boolean(selected && row?.id === selected.id);
              return <path key={row?.id ?? index} d={buildRidgePath(row, index, rendered.length, maximum)} fill="none" stroke={active ? `url(#${gradient})` : "currentColor"} strokeWidth={active ? "2" : ".7"} className={`ridge-ribbon ${active ? "is-selected" : ""}`} />;
            })}
            {selected ? <path className="ridge-scanner" d={buildRidgePath(selected, rows.indexOf(selected), rows.length, maximum)} fill="none" stroke={`url(#${gradient})`} strokeWidth="2" pathLength="1" /> : null}
          </g>
          <text x="46" y="278">卖方 / SELL</text><text x="373" y="278">成交方向</text><text x="678" y="278">买方 / BUY</text>
        </svg>
        <div className="instrument-probe"><i aria-hidden="true"/><span>{selected ? `${clockLabel(selected.time)} / Δ` : "等待成交数据"}</span><strong className={selected?.delta >= 0 ? "ink-buy" : "ink-sell"}>{selected ? `${selected.delta > 0 ? "+" : ""}${formatReading(selected.delta)} ${symbol}` : "—"}</strong></div>
        <label className="instrument-sample-control"><span>样本探针</span><input type="range" min="0" max={Math.max(0, rows.length - 1)} value={Math.max(0, rows.indexOf(selected))} disabled={!rows.length} onChange={event => setSelectedId(rows[Number(event.target.value)]?.id ?? null)} aria-label="选择成交地形样本" /><span>{selected ? clockLabel(selected.time) : "NO DATA"}</span></label>
      </div>
    </div>
    <footer className="instrument-footer"><span>高度按买卖量统一归一化 · 轮廓为平滑显示</span><span>形态动效不改变数据读数</span></footer>
  </section>;
}

function VenueChord({ events }) {
  const venues = [...new Set(events.map(event => event.venue))].sort();
  const nodes = [...venues.map(venue => ({ name: venue.toUpperCase(), key: venue, type: "venue" })), ...["buy", "neutral", "sell"].map(direction => ({ name: LABELS[direction], key: direction, type: "direction" }))];
  const positions = nodes.map((node, i) => ({ ...node, x: 154 + Math.cos(i / nodes.length * Math.PI * 2 - Math.PI / 2) * 105, y: 133 + Math.sin(i / nodes.length * Math.PI * 2 - Math.PI / 2) * 105 }));
  return <section className="instrument-panel">
    <InstrumentHeader number="02" title="交易所弦图" code="VENUE CHORD"><span>{events.length} EVENTS</span></InstrumentHeader>
    <div className="instrument-split-body">
      <svg className="instrument-chord" viewBox="0 0 315 270" role="img" aria-label="事件来源与成交方向分组弦图">
        <circle className="instrument-guide" cx="154" cy="133" r="105" fill="none" />
        <circle className="instrument-guide instrument-orbit" cx="154" cy="133" r="119" fill="none" strokeDasharray="1 12" />
        {positions.filter(node => node.type === "venue").flatMap(venue => positions.filter(node => node.type === "direction").map(target => {
          const count = events.filter(event => event.venue === venue.key && event.direction === target.key).length;
          const path = `M${venue.x},${venue.y} Q154,133 ${target.x},${target.y}`;
          return count ? <g key={`${venue.key}-${target.key}`} className={`ink-${target.key}`}><path d={path} fill="none" stroke="currentColor" opacity=".25" strokeWidth={Math.min(7, 1 + count / 3)} /><path className="chord-flow" d={path} fill="none" stroke="currentColor" strokeWidth="1.4" pathLength="1" /><title>{venue.name} · {LABELS[target.key]}：{count} 条</title></g> : null;
        }))}
        {positions.map(node => <g key={node.key} className={`ink-${node.type === "direction" ? node.key : "neutral"}`}><circle cx={node.x} cy={node.y} r="3" fill="currentColor" /><text x={node.x} y={node.y + (node.y < 133 ? -10 : 15)} textAnchor="middle">{node.name}</text></g>)}
        <text x="154" y="130" textAnchor="middle" className="instrument-svg-number">{events.length}</text><text x="154" y="146" textAnchor="middle">OBSERVATIONS</text>
      </svg>
      <div className="instrument-side-readouts"><p>SOURCE → DIRECTION</p><dl>{groupObservations(events).map(group => <div key={group.direction}><dt>{LABELS[group.direction]}事件</dt><dd className={`ink-${group.direction}`}>{group.items.length}</dd></div>)}<div><dt>来源数量</dt><dd>{venues.length}</dd></div></dl><p className="instrument-micro-note">弦宽表示事件数量；<br/>流光仅用于关系导览。</p></div>
    </div>
  </section>;
}

// Imperative SVG camera updates avoid rerendering the dashboard on every frame.
// The root motion preference, system preference, tab visibility and intersection all stop the RAF.
function useLatticeCamera(ref, paused) {
  const angleRef = useRef(.35);
  useEffect(() => {
    const svg = ref.current;
    if (!svg) return undefined;
    let frame = 0, angle = angleRef.current, previous = 0, onscreen = true, disposed = false;
    const media = window.matchMedia?.("(prefers-reduced-motion: reduce)");
    const vertices = Array.from({ length: 16 }, (_, i) => [(i & 1 ? 1 : -1) * (i & 8 ? .53 : 1), (i & 2 ? 1 : -1) * (i & 8 ? .53 : 1), (i & 4 ? 1 : -1) * (i & 8 ? .53 : 1)]);
    function draw() {
      angleRef.current = angle;
      const points = vertices.map(([x, y, z]) => {
        const rx = x * Math.cos(angle) - z * Math.sin(angle);
        const rz = x * Math.sin(angle) + z * Math.cos(angle);
        const ry = y * Math.cos(.4) - rz * Math.sin(.4);
        return [160 + rx * 70, 126 + ry * 70];
      });
      svg.querySelectorAll("[data-edge]").forEach(line => {
        const [a, b] = line.dataset.edge.split(",").map(Number);
        line.setAttribute("x1", points[a][0]); line.setAttribute("y1", points[a][1]);
        line.setAttribute("x2", points[b][0]); line.setAttribute("y2", points[b][1]);
      });
      svg.querySelectorAll("[data-vertex]").forEach(dot => { const point = points[Number(dot.dataset.vertex)]; dot.setAttribute("cx", point[0]); dot.setAttribute("cy", point[1]); });
    }
    function allowed() { return !disposed && !paused && onscreen && !media?.matches && document.visibilityState !== "hidden" && document.documentElement.dataset.motion !== "off"; }
    function tick(now) {
      frame = 0;
      if (!allowed()) return;
      if (!previous || now - previous >= 40) { angle += Math.min(previous ? now - previous : 40, 80) * .00016; previous = now; draw(); }
      frame = window.requestAnimationFrame(tick);
    }
    function sync() { window.cancelAnimationFrame?.(frame); frame = 0; previous = 0; if (allowed() && window.requestAnimationFrame) frame = window.requestAnimationFrame(tick); }
    draw();
    const observer = new MutationObserver(sync);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-motion"] });
    const intersection = typeof IntersectionObserver !== "undefined" ? new IntersectionObserver(entries => { onscreen = entries[0].isIntersecting; sync(); }) : null;
    intersection?.observe(svg);
    document.addEventListener("visibilitychange", sync); media?.addEventListener?.("change", sync);
    sync();
    return () => { disposed = true; window.cancelAnimationFrame?.(frame); observer.disconnect(); intersection?.disconnect(); document.removeEventListener("visibilitychange", sync); media?.removeEventListener?.("change", sync); };
  }, [ref, paused]);
}

function EvidenceLattice({ events, paused }) {
  const ref = useRef(null);
  const [frozen, setFrozen] = useState(false);
  useLatticeCamera(ref, paused || frozen);
  const latest = events.at(-1);
  const edges = [];
  for (let i = 0; i < 16; i++) for (const step of [1, 2, 4, 8]) if ((i ^ step) > i) edges.push([i, i ^ step]);
  return <section className="instrument-panel">
    <InstrumentHeader number="03" title="证据结构" code="EVIDENCE LATTICE"><button className="instrument-text-button" type="button" onClick={() => setFrozen(value => !value)} aria-pressed={frozen}>{frozen ? "继续旋转" : "冻结视角"}</button></InstrumentHeader>
    <div className="instrument-split-body instrument-lattice-body">
      <div className="instrument-side-readouts"><p>LATEST OBSERVATION</p><dl><div><dt>事件评级</dt><dd>{latest?.grade ?? "—"}</dd></div><div><dt>风险分数</dt><dd>{formatReading(latest?.score)}</dd></div><div><dt>数据质量</dt><dd>{formatReading(latest?.quality)}</dd></div><div><dt>事件方向</dt><dd className={`ink-${latest?.direction || "neutral"}`}>{latest ? LABELS[latest.direction] : "—"}</dd></div></dl><span className="instrument-micro-note">几何结构示意<br/>数值沿用事件原始评级</span></div>
      <svg ref={ref} className="instrument-lattice" viewBox="0 0 320 270" role="img" aria-label="旋转线框结构，旁列为最新事件原始读数">
        <ellipse cx="160" cy="236" rx="107" ry="9" fill="#233c3610" />
        {edges.map(([a, b], i) => <line key={`${a}-${b}`} data-edge={`${a},${b}`} stroke={["#508bca", "#d795b0", "#c9ac5c", "#77ac99"][i % 4]} strokeWidth=".8" opacity=".65" />)}
        {Array.from({ length: 16 }, (_, i) => <circle data-vertex={i} key={i} r={i < 8 ? 2.6 : 1.8} fill={i % 2 ? "#289d83" : "#cb7b99"} />)}
        <text x="16" y="258">CAMERA / 3D → 2D</text><text x="300" y="258" textAnchor="end">STRUCTURE VIEW</text>
      </svg>
    </div>
  </section>;
}

function DirectionClusters({ events, total }) {
  const groups = groupObservations(events);
  return <div className="instrument-cluster-body">
    <aside className="instrument-readouts"><p>OBSERVATION MAP</p><strong className="instrument-hero-number">{String(events.length).padStart(2, "0")}</strong><span>已显示 / {total} 个事件</span><dl>{groups.map(group => <div key={group.direction}><dt><i className={`legend-dot ink-${group.direction}`} />{LABELS[group.direction]}</dt><dd>{group.items.length}</dd></div>)}</dl></aside>
    <svg viewBox="0 0 900 230" role="img" aria-label={`方向关系图，${events.length} 个已加载事件`}>
      <line className="instrument-guide" x1="20" x2="880" y1="202" y2="202" strokeDasharray="2 5" />
      {groups.map((group, groupIndex) => {
        const cx = 160 + groupIndex * 280, cy = groupIndex === 1 ? 137 : 94;
        return <g key={group.direction} className={`ink-${group.direction}`}>
          <ellipse className="instrument-guide" cx={cx} cy={cy} rx="120" ry="65" fill="none" strokeDasharray="2 6" />
          {group.items.map((event, i) => {
            const theta = i * 2.39996;
            const radius = 26 + Math.sqrt((i + 1) / Math.max(1, group.items.length)) * 83;
            const x = cx + Math.cos(theta) * radius, y = cy + Math.sin(theta) * radius * .53;
            return <g key={event.id} className="cluster-satellite" style={{ "--satellite-delay": `${-(i % 9)}s` }}><line x1={cx} y1={cy} x2={x} y2={y} stroke="currentColor" opacity=".16" strokeWidth=".65"/><circle cx={x} cy={y} r="2.2" fill="currentColor" opacity=".8"/><title>{event.symbol} · {clockLabel(event.time)} · {LABELS[event.direction]} · {event.grade}</title></g>;
          })}
          <circle className={group.items.length ? "cluster-halo" : ""} cx={cx} cy={cy} r="19" fill="none" stroke="currentColor" opacity=".35" />
          <circle cx={cx} cy={cy} r="10" fill={group.items.length ? "currentColor" : "#fff"} stroke="currentColor" opacity={group.items.length ? 1 : .4} />
          <text x={cx} y={cy + 34} textAnchor="middle" className="cluster-label">{LABELS[group.direction]} / {group.direction.toUpperCase()}</text>
          <text x={cx} y="224" textAnchor="middle">{group.items.length} EVENTS</text>
        </g>;
      })}
    </svg>
  </div>;
}
