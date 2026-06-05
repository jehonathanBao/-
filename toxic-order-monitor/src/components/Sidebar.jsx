import { NavLink } from "react-router-dom";

const menuItems = [
  { label: "监控首页", path: "/" },
  { label: "异常信号", path: "/signals" },
  { label: "信号历史", path: "/history" },
  { label: "告警规则", path: "/rules" },
  { label: "Discord 设置", path: "/discord" },
  { label: "系统设置", path: "/settings" },
];

export default function Sidebar() {
  return (
    <aside className="hidden min-h-screen w-64 shrink-0 border-r border-slate-700/60 bg-slate-950/70 px-4 py-6 shadow-glow backdrop-blur lg:block">
      <div className="mb-8 rounded-2xl border border-cyan-500/20 bg-cyan-500/10 p-4">
        <p className="text-xs uppercase tracking-[0.32em] text-cyan-300">Orderbook</p>
        <h1 className="mt-2 text-xl font-bold text-white">有毒订单监控</h1>
        <p className="mt-2 text-xs text-slate-400">异常盘口信号雷达</p>
      </div>

      <nav className="space-y-2">
        {menuItems.map((item) => (
          <NavLink
            className={({ isActive }) =>
              [
                "block rounded-xl px-4 py-3 text-sm transition",
                isActive
                  ? "bg-cyan-500/15 text-cyan-200 ring-1 ring-cyan-400/30"
                  : "text-slate-400 hover:bg-slate-800/80 hover:text-slate-100",
              ].join(" ")
            }
            key={item.path}
            to={item.path}
          >
            {item.label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
