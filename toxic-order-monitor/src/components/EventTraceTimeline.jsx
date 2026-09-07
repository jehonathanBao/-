const clock = new Intl.DateTimeFormat('zh-CN', { month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false });

export default function EventTraceTimeline({ items = [], onSelect, selectedId }) {
  const unique = new Map();
  for (const item of Array.isArray(items) ? items : []) {
    const ts = Number(item?.ts);
    if (!item?.id || !Number.isFinite(ts) || ts <= 0 || ts > 8.64e15) continue;
    if (!unique.has(item.id) || Number(unique.get(item.id).ts) < ts) unique.set(item.id, item);
  }
  const events = [...unique.values()].sort((a, b) => Number(a.ts) - Number(b.ts)).slice(-8);
  return (
    <section className="event-trace" aria-label="事件时间轴">
      <header><div><span className="control-room-eyebrow">EVENT SEQUENCE</span><h2>事件时间轴</h2></div><span>最近 {events.length} 个已加载事件</span></header>
      {events.length ? <ol>
        {events.map(item => {
          const direction = item.direction === 'buy' ? '买方流' : item.direction === 'sell' ? '卖方流' : '方向未知';
          const label = clock.format(new Date(Number(item.ts)));
          const content = <><time dateTime={new Date(Number(item.ts)).toISOString()}>{label}</time><span className="event-trace-node" aria-hidden="true"/><strong>{item.symbol || '市场'}</strong><small>{direction}</small></>;
          return <li className={`direction-${item.direction || 'unknown'}`} key={item.id}>{onSelect
            ? <button type="button" aria-label={`查看 ${item.symbol || '市场'} ${label} 事件`} aria-pressed={selectedId === item.id} onClick={() => onSelect(item.id)}>{content}</button>
            : <div>{content}</div>}</li>;
        })}
      </ol> : <p className="event-trace-empty">暂无可展示的事件 · 等待有效数据</p>}
    </section>
  );
}
