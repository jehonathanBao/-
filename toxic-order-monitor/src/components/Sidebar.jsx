import { NavLink, useLocation } from "react-router-dom";

const menuItems = [
  { label: "监控首页", path: "/dashboard", aliases: ["/"] },
  { label: "BTC/ETH 合约监控", path: "/contract-whale" },
  { label: "BTC 清算监控", path: "/btc-liquidation" },
  { label: "BTC/ETH 现货监控", path: "/spot-monitor", aliases: ["/spot-whale"] },
  { label: "山寨合约异常", path: "/alt-contract-monitor" },
  { label: "异常信号", path: "/signals" },
  { label: "信号历史", path: "/history" },
  { label: "告警规则", path: "/rules" },
  { label: "使用指南", path: "/usage-guide" },
  { label: "Discord 设置", path: "/discord" },
  { label: "系统设置", path: "/settings" },
];

export default function Sidebar() {
  const location = useLocation();
  return (
    <aside className="w-full shrink-0 border-b border-slate-700/60 bg-slate-950/90 px-4 py-4 shadow-xl shadow-slate-950/30 backdrop-blur lg:min-h-screen lg:w-64 lg:border-b-0 lg:border-r lg:py-6">
      <div className="mb-4 rounded-xl border border-cyan-500/20 bg-cyan-500/10 p-4 lg:mb-7">
        <p className="text-xs uppercase tracking-[0.28em] text-cyan-300">Orderbook</p>
        <h1 className="mt-2 text-xl font-bold text-white">有毒订单监控</h1>
        <p className="mt-2 text-xs text-slate-400">异常盘口信号雷达</p>
      </div>

      <nav aria-label="主导航" className="flex gap-2 overflow-x-auto pb-1 lg:block lg:space-y-2 lg:overflow-visible lg:pb-0">
        {menuItems.map((item) => (
          <NavLink
            className={({ isActive }) =>
              navLinkClass(isActive || item.aliases?.includes(location.pathname))
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

function navLinkClass(isActive) {
  return [
    "block shrink-0 whitespace-nowrap rounded-xl px-4 py-3 text-sm outline-none transition focus-visible:ring-2 focus-visible:ring-cyan-500/35",
    isActive
      ? "bg-cyan-500/15 text-cyan-100 ring-1 ring-cyan-400/35"
      : "text-slate-400 hover:bg-slate-800/80 hover:text-slate-100",
  ].join(" ");
}
