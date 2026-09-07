import { useId, useMemo, useState } from "react";

const money = new Intl.NumberFormat("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
const timeLabel = (time) => new Date(time).toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", hour12: false });

export function normalizePricePoints(points) {
  const unique = new Map();
  for (const point of Array.isArray(points) ? points : []) {
    const time = Number(point?.time);
    const price = Number(point?.price);
    if (Number.isFinite(time) && time > 0 && time <= 8.64e15 && Number.isFinite(price) && price > 0) {
      unique.set(time, { time, price });
    }
  }
  return [...unique.values()].sort((left, right) => left.time - right.time).slice(-240);
}

/** Read-only visualization of supplied samples; this component never fetches or rates data. */
export default function PriceChart({ points, title, symbol, description, loading = false, discrete = false }) {
  const gradientId = `price-fill-${useId().replace(/:/g, "")}`;
  const [recent, setRecent] = useState(false);
  const [inspectedTime, setInspectedTime] = useState(null);
  const allPoints = useMemo(() => normalizePricePoints(points), [points]);
  const visible = recent ? allPoints.slice(-12) : allPoints;
  const last = visible.at(-1);
  const change = visible.length > 1 ? (last.price / visible[0].price - 1) * 100 : null;
  const low = visible.length ? Math.min(...visible.map(point => point.price)) : 0;
  const high = visible.length ? Math.max(...visible.map(point => point.price)) : 1;
  const spread = Math.max(high - low, high * 0.001);
  const x = (time) => visible.length <= 1 ? 375 : 14 + ((time - visible[0].time) / Math.max(1, last.time - visible[0].time)) * 720;
  const y = (price) => 180 - ((price - low + spread * 0.12) / (spread * 1.24)) * 156;
  const inspectedIndex = visible.findIndex(point => point.time === inspectedTime);
  const selected = inspectedIndex >= 0 ? visible[inspectedIndex] : last;
  const path = visible.map((point, index) => `${index === 0 ? "M" : "L"}${x(point.time).toFixed(2)},${y(point.price).toFixed(2)}`).join(" ");

  function inspectPointer(event) {
    const rect = event.currentTarget.getBoundingClientRect();
    const graphX = ((event.clientX - rect.left) / Math.max(1, rect.width)) * 800;
    const nearest = visible.reduce((best, point) => !best || Math.abs(x(point.time) - graphX) < Math.abs(x(best.time) - graphX) ? point : best, null);
    setInspectedTime(nearest?.time ?? null);
  }

  function inspectKeyboard(event) {
    const current = inspectedIndex < 0 ? visible.length - 1 : inspectedIndex;
    let next;
    if (event.key === "ArrowLeft") next = Math.max(0, current - 1);
    else if (event.key === "ArrowRight") next = Math.min(visible.length - 1, current + 1);
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = visible.length - 1;
    else return;
    event.preventDefault();
    setInspectedTime(visible[next]?.time ?? null);
  }

  return (
    <section className="terminal-price-chart" aria-label={title}>
      <header className="terminal-chart-header">
        <div className="terminal-chart-title">
          <span className={`terminal-asset-mark asset-${symbol?.toLowerCase()}`} aria-hidden="true">{symbol === "ETH" ? "Ξ" : "₿"}</span>
          <div><h2>{title}</h2><p>{description}</p></div>
        </div>
        <div className="terminal-segmented" aria-label="价格样本范围">
          <button type="button" aria-pressed={!recent} onClick={() => { setRecent(false); setInspectedTime(null); }}>全部样本</button>
          <button type="button" aria-pressed={recent} onClick={() => { setRecent(true); setInspectedTime(null); }}>最近 12 点</button>
        </div>
      </header>
      <div className="terminal-chart-readout">
        <strong><span key={last?.price} className="terminal-value-update">{last ? money.format(last.price) : "—"}</span><small>USD</small></strong>
        {change !== null ? <span className={`terminal-price-change ${change >= 0 ? "is-positive" : "is-negative"}`}>{change >= 0 ? "+" : ""}{change.toFixed(2)}% <small>样本区间</small></span> : null}
      </div>
      {visible.length ? (
        <div className="terminal-chart-interaction" role="group" aria-label={`${symbol} 图表，使用键盘左右键查看样本`} tabIndex={0} onKeyDown={inspectKeyboard}>
          <div className="terminal-chart-inspection" data-testid="chart-inspection">
            <span>{timeLabel(selected.time)}</span><strong>{money.format(selected.price)} USD</strong>
          </div>
          <svg viewBox="0 0 800 212" preserveAspectRatio="none" role="img" aria-label={discrete ? `${symbol} 事件价格散点图` : `${symbol} 价格曲线`} onPointerMove={inspectPointer} onPointerLeave={() => setInspectedTime(null)}>
            <defs><linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1"><stop offset="0" stopColor="currentColor" stopOpacity="0.16" /><stop offset="1" stopColor="currentColor" stopOpacity="0" /></linearGradient></defs>
            {[0, 1, 2, 3].map(tick => {
              const value = high - (high - low) * tick / 3;
              const lineY = y(value);
              return <g key={tick} className="terminal-chart-grid"><line x1="14" x2="738" y1={lineY} y2={lineY} /><text x="748" y={lineY + 4}>{money.format(value)}</text></g>;
            })}
            {[0, 1, 2, 3, 4, 5, 6].map(tick => <line className="terminal-chart-grid-line" key={tick} x1={14 + tick * 120} x2={14 + tick * 120} y1="14" y2="192" />)}
            {!discrete && visible.length > 1 ? <>
              <path d={`${path} L${x(last.time)},194 L${x(visible[0].time)},194 Z`} fill={`url(#${gradientId})`} />
              <path key={String(recent)} className="terminal-chart-line" data-testid="price-line" d={path} pathLength="1" />
            </> : null}
            {discrete ? visible.map(point => <circle className="terminal-chart-sample" key={point.time} cx={x(point.time)} cy={y(point.price)} r="3.5" />) : null}
            <line className="terminal-chart-crosshair" x1={x(selected.time)} x2={x(selected.time)} y1="14" y2="194" />
            <circle className="terminal-chart-current" cx={x(selected.time)} cy={y(selected.price)} r="4" />
            <text className="terminal-chart-time" x="14" y="210">{timeLabel(visible[0].time)}</text>
            <text className="terminal-chart-time" x="734" y="210" textAnchor="end">{timeLabel(last.time)}</text>
          </svg>
        </div>
      ) : (
        <div className={`terminal-chart-empty ${loading ? "is-loading" : ""}`}>
          <span className="terminal-empty-cross" aria-hidden="true">+</span>
          <strong>{loading ? "正在读取价格样本" : "暂无价格样本"}</strong>
          <p>数据接入后自动显示，不填充模拟走势。</p>
        </div>
      )}
      <footer className="terminal-chart-footer"><span>{visible.length} 个价格样本</span><span>{discrete ? "仅观察事件价格，不代表连续行情" : "按源时间绘制 · 非逐笔报价"}</span></footer>
    </section>
  );
}
