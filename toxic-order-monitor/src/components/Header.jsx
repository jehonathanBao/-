import { Cog6ToothIcon } from "@heroicons/react/24/outline";
import { useEffect, useState } from "react";

export default function Header({ highUnhandledCount, discordConnected }) {
  const [now, setNow] = useState(new Date());

  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <header className="workspace-command-header" data-testid="workspace-command-header">
      <div className="workspace-command-brand">
        <p>Toxic Order Monitor</p>
        <h2>盘口异常监控大屏</h2>
        <span>READ-ONLY RISK WORKSPACE</span>
      </div>

      <div className="workspace-command-metrics">
        <div className="workspace-command-metric workspace-command-metric-danger">
          高风险未处理 <span className="font-bold text-red-300">{highUnhandledCount}</span>
        </div>
        <div className="workspace-command-metric">
          Discord <span className={discordConnected ? "text-emerald-300" : "text-slate-400"}>{discordConnected ? "已连接" : "未配置"}</span>
        </div>
        <div className="workspace-command-metric workspace-command-clock">
          {now.toLocaleString("zh-CN", { hour12: false })}
        </div>
        <button aria-label="系统设置" className="workspace-command-settings" type="button">
          <Cog6ToothIcon className="h-5 w-5" />
        </button>
      </div>
    </header>
  );
}
