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
import MotionControl from "./MotionControl.jsx";

const menuItems = [
  { label: "监控首页", path: "/dashboard", aliases: ["/"], icon: HomeIcon },
  { label: "BTC 合约监控", path: "/contract-whale/btc", aliases: ["/contract-whale"], icon: PresentationChartLineIcon },
  { label: "ETH 合约监控", path: "/contract-whale/eth", icon: PresentationChartLineIcon },
  { label: "Binance Delta K线", path: "/binance-orderflow", icon: PresentationChartLineIcon },
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

const menuGroups = [
  { label: "总览", items: [0] },
  { label: "合约市场", items: [1, 2, 3, 6, 7] },
  { label: "现货市场", items: [4, 5] },
  { label: "信号中心", items: [8, 9, 10] },
  { label: "工作区", items: [11, 12, 13] },
];

export default function Sidebar({ runtimeBoundary }) {
  const location = useLocation();
  return (
    <aside
      className="workspace-sidebar contract-sidebar"
      data-testid="workspace-sidebar"
    >
      <a className="terminal-skip-link" href="#workspace-main">跳到主要内容</a>
      <div className="contract-sidebar-brand">
        <span className="contract-sidebar-mark" aria-hidden="true">
          <svg viewBox="0 0 32 32" fill="none"><path d="m4 8 6 17 6-12 6 12 6-17M4 8h5m14 0h5" stroke="currentColor" strokeWidth="2.5" strokeLinejoin="round" /></svg>
        </span>
        <div>
          <p className="terminal-brand-name">WHALE<span>DESK</span></p>
          <p className="terminal-brand-caption">有毒订单监控 · 市场观察终端</p>
        </div>
      </div>

      <nav aria-label="主导航" className="terminal-navigation">
        {menuGroups.map(group => (
          <div className="terminal-nav-group" key={group.label}>
            <p className="terminal-nav-label">{group.label}</p>
            <div className="terminal-nav-items">
        {group.items.map((index) => {
          const item = menuItems[index];
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
            </div>
          </div>
        ))}
      </nav>

      <div className="contract-sidebar-health">
        <MotionControl />
        <p className="terminal-nav-label">运行边界</p>
        <RuntimeBoundaryBadge runtimeBoundary={runtimeBoundary} showDetail />
      </div>
    </aside>
  );
}

function navLinkClass(isActive) {
  return [
    "contract-sidebar-link",
    isActive
      ? "is-active"
      : "",
  ].join(" ");
}
