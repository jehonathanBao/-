import { Link } from "react-router-dom";
import { useState } from "react";
import Sidebar from "../components/Sidebar.jsx";
import PriceChart from "../components/PriceChart.jsx";
import MarketObservatory from "../components/observatory/MarketObservatory.jsx";
import { previewCandles, previewEvents } from "../components/observatory/previewData.js";
import { clockLabel } from "../components/observatory/model.js";

export default function InstrumentPreview() {
  const [empty, setEmpty] = useState(false);
  const candles = empty ? [] : previewCandles;
  const events = empty ? [] : previewEvents;
  return <div className="control-room-shell">
    <div className="instrument-preview-banner"><strong>设计预览 · 固定演示数据 · 非实时行情</strong><Link to="/dashboard">返回真实监控 →</Link></div>
    <Sidebar runtimeBoundary={null} />
    <main className="workspace-content" id="workspace-main">
      <header className="instrument-preview-heading"><div><h1>市场的变化，正在显形。</h1><p>MAIN FORCE OBSERVATORY / RESEARCH DESK 02</p></div><button type="button" className="monitor-flow-button" onClick={() => setEmpty(value => !value)}>{empty ? "恢复演示数据" : "查看无数据状态"}</button></header>
      <div className="instrument-preview-metrics">
        <div><p>REFERENCE PRICE / BTC</p><strong>{empty ? "—" : "$64,832.17"}</strong><small>固定样本结束价格</small></div>
        <div><p>LAST DELTA / BTC</p><strong className="ink-buy">{empty ? "—" : "+1,022"}</strong><small>最后一根 K 线 · 主动成交净差</small></div>
        <div><p>OBSERVATION WINDOW</p><strong>{empty ? "—" : "32 HOURS"}</strong><small>每层 1H · 仅用于界面演示</small></div>
        <div><p>SESSION MODE</p><strong>DESIGN LAB</strong><small>离线演示 · 无通知发送</small></div>
      </div>
      <div className="instrument-preview-top">
        <PriceChart points={candles.map(candle => ({ time: candle.time, price: candle.close }))} symbol="BTC" title="BTC 价格走势" description="参考样本 / 非实时行情" />
        <section className="instrument-panel"><header className="instrument-header"><div><i aria-hidden="true"/><h2>观测日志</h2><span>ACTIVITY LOG</span></div><span>{events.length} RECORDS</span></header>
          <ol className="instrument-activity">{events.length ? events.slice(-7).reverse().map(event => <li key={event.id}><time>{clockLabel(event.ts)}</time><strong className={`ink-${event.direction}`}>{event.direction.toUpperCase()}</strong><span>{event.mainExchange.toUpperCase()} · BTC · {event.impactLevel} 级样本</span></li>) : <li>暂无事件记录</li>}</ol>
        </section>
      </div>
      <MarketObservatory samples={candles} observations={events} demo />
      <footer className="terminal-page-footer"><span>MAIN FORCE / OBSERVATORY</span><span>01 — 04 / 成交地形 · 来源弦图 · 证据结构 · 方向关系</span></footer>
    </main>
  </div>;
}
