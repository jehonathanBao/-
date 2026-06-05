import { Cog6ToothIcon } from "@heroicons/react/24/outline";
import { useEffect, useState } from "react";

export default function Header({ highUnhandledCount, discordConnected }) {
  const [now, setNow] = useState(new Date());

  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <header className="mb-5 flex flex-col gap-4 rounded-2xl border border-slate-700/60 bg-slate-900/80 p-5 shadow-glow md:flex-row md:items-center md:justify-between">
      <div>
        <p className="text-xs uppercase tracking-[0.3em] text-cyan-300">Toxic Order Monitor</p>
        <h2 className="mt-2 text-2xl font-bold text-white">盘口异常监控大屏</h2>
      </div>

      <div className="grid gap-3 text-sm text-slate-300 md:grid-cols-[auto_auto_auto_auto] md:items-center">
        <div className="rounded-xl border border-red-500/30 bg-red-500/10 px-4 py-2">
          高风险未处理 <span className="font-bold text-red-300">{highUnhandledCount}</span>
        </div>
        <div className="rounded-xl border border-slate-700/60 bg-slate-950/40 px-4 py-2">
          Discord <span className={discordConnected ? "text-emerald-300" : "text-slate-400"}>{discordConnected ? "已连接" : "未配置"}</span>
        </div>
        <div className="rounded-xl border border-slate-700/60 bg-slate-950/40 px-4 py-2">
          {now.toLocaleString("zh-CN", { hour12: false })}
        </div>
        <button aria-label="系统设置" className="inline-flex h-10 w-10 items-center justify-center rounded-xl border border-slate-700/60 bg-slate-950/40 text-slate-300 hover:border-cyan-400/50 hover:text-cyan-200" type="button">
          <Cog6ToothIcon className="h-5 w-5" />
        </button>
      </div>
    </header>
  );
}
