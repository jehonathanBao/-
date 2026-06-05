export default function PushLog({ logs }) {
  return (
    <section className="rounded-2xl border border-slate-700/60 bg-slate-900/80 p-5 shadow-glow">
      <h3 className="font-bold text-white">Discord 推送记录</h3>
      <div className="mt-4 space-y-3">
        {logs.length === 0 ? (
          <p className="text-sm text-slate-500">暂无推送记录</p>
        ) : (
          logs.map((log) => (
            <div className="rounded-xl border border-slate-700/60 bg-slate-950/40 p-3" key={log.id}>
              <div className="flex items-center justify-between gap-3">
                <p className="font-semibold text-slate-100">{log.exchange} / {log.symbol}</p>
                <span className={statusClass(log.status)}>{statusLabel(log.status)}</span>
              </div>
              {log.reason ? <p className="mt-1 text-xs text-red-300">{log.reason}</p> : null}
              <p className="mt-1 text-xs text-slate-400">{log.time} · {log.type} · 等级 {log.level}</p>
            </div>
          ))
        )}
      </div>
    </section>
  );
}

function statusClass(status) {
  if (status === "success") return "text-emerald-300";
  if (status === "failed") return "text-red-300";
  return "text-yellow-300";
}

function statusLabel(status) {
  if (status === "success") return "成功";
  if (status === "failed") return "失败";
  return "待发送";
}
