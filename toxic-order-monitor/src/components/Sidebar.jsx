import {
  AdjustmentsHorizontalIcon,
  BellAlertIcon,
  BookOpenIcon,
  ChartBarIcon,
  ChatBubbleLeftRightIcon,
  ClockIcon,
  Cog6ToothIcon,
  CpuChipIcon,
  HomeIcon,
  PresentationChartLineIcon,
  SignalIcon,
} from "@heroicons/react/24/outline";
import { NavLink, useLocation } from "react-router-dom";
import RuntimeBoundaryBadge from "./RuntimeBoundaryBadge.jsx";

const menuItems = [
  { label: "监控首页", path: "/dashboard", aliases: ["/"], icon: HomeIcon },
  { label: "BTC 合约监控", path: "/contract-whale/btc", aliases: ["/contract-whale"], icon: PresentationChartLineIcon },
  { label: "ETH 合约监控", path: "/contract-whale/eth", icon: PresentationChartLineIcon },
  { label: "BTC 现货监控", path: "/spot-monitor/btc", aliases: ["/spot-monitor", "/spot-whale"], icon: ChartBarIcon },
  { label: "ETH 现货监控", path: "/spot-monitor/eth", icon: ChartBarIcon },
  { label: "新币合约监控", path: "/new-token-watch", icon: CpuChipIcon },
  { label: "山寨合约异常", path: "/alt-contract-monitor", icon: SignalIcon },
  { label: "异常信号", path: "/signals", icon: BellAlertIcon },
  { label: "信号历史", path: "/history", icon: ClockIcon },
  { label: "告警规则", path: "/rules", icon: AdjustmentsHorizontalIcon },
  { label: "使用指南", path: "/usage-guide", icon: BookOpenIcon },
  { label: "Discord 设置", path: "/discord", icon: ChatBubbleLeftRightIcon },
  { label: "系统设置", path: "/settings", icon: Cog6ToothIcon },
];

export default function Sidebar({ runtimeBoundary }) {
  const location = useLocation();
  return (
    <aside
      className="workspace-sidebar contract-sidebar w-full shrink-0 border-b px-3 py-3 lg:sticky lg:top-0 lg:h-screen lg:w-[212px] lg:border-b-0 lg:border-r lg:px-2 lg:py-4"
      data-testid="workspace-sidebar"
    >
      <div className="contract-sidebar-brand mb-3 flex items-center gap-3 px-2 py-2">
        <span className="contract-sidebar-mark" aria-hidden="true">W</span>
        <div>
          <p className="text-[10px] font-semibold uppercase tracking-[0.24em] text-slate-500">
            Whale Desk
          </p>
          <h1 className="mt-1 text-sm font-semibold text-slate-100">有毒订单监控</h1>
        </div>
      </div>

      <nav aria-label="主导航" className="flex gap-1 overflow-x-auto pb-1 lg:block lg:space-y-0.5 lg:overflow-visible lg:pb-0">
        {menuItems.map((item) => {
          const Icon = item.icon;
          return (
          <NavLink
            className={({ isActive }) =>
              navLinkClass(isActive || item.aliases?.includes(location.pathname))
            }
            key={item.path}
            to={item.path}
          >
            <Icon aria-hidden="true" className="h-[17px] w-[17px] shrink-0" />
            <span>{item.label}</span>
          </NavLink>
          );
        })}
      </nav>

      <div className="contract-sidebar-health mt-4 hidden border-t px-2 pt-4 text-[10px] lg:block">
        <div className="flex items-center justify-between gap-3">
          <span className="uppercase tracking-[0.16em] text-slate-600">Workspace</span>
        </div>
        <RuntimeBoundaryBadge runtimeBoundary={runtimeBoundary} showDetail />
      </div>
    </aside>
  );
}

function navLinkClass(isActive) {
  return [
    "contract-sidebar-link flex shrink-0 items-center gap-2.5 whitespace-nowrap rounded-md border-l-2 px-2.5 py-2 text-[12px] outline-none transition focus-visible:ring-1 focus-visible:ring-cyan-300/50",
    isActive
      ? "border-cyan-300 bg-cyan-300/[0.08] text-slate-100"
      : "border-transparent text-slate-500 hover:bg-white/[0.035] hover:text-slate-200",
  ].join(" ");
}
