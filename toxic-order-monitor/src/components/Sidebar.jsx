import { Link, NavLink, useLocation } from "react-router-dom";
import { Squares2X2Icon, ChevronDownIcon, ArrowUpRightIcon } from "@heroicons/react/24/outline";
import RuntimeBoundaryBadge from "./RuntimeBoundaryBadge.jsx";
import MotionControl from "./MotionControl.jsx";

const workspaces = [
  { name: "总览", code: "01", items: [["监控首页", "/dashboard"]] },
  { name: "合约市场", code: "02", items: [
    ["BTC 合约监控", "/contract-whale/btc"], ["ETH 合约监控", "/contract-whale/eth"],
    ["Binance Delta K线", "/binance-orderflow"], ["新币合约监控", "/new-token-watch"],
    ["山寨合约异常", "/alt-contract-monitor"], ["强平观察", "/liquidation-cascade"],
  ] },
  { name: "现货市场", code: "03", items: [["BTC 现货监控", "/spot-monitor/btc"], ["ETH 现货监控", "/spot-monitor/eth"]] },
  { name: "信号中心", code: "04", items: [["异常信号", "/signals"], ["信号历史", "/history"], ["告警规则", "/rules"]] },
  { name: "工作区", code: "05", items: [["使用指南", "/usage-guide"], ["Discord 设置", "/discord"], ["系统设置", "/settings"]] },
];
const aliases = { "/": "/dashboard", "/contract-whale": "/contract-whale/btc", "/spot-monitor": "/spot-monitor/btc", "/spot-whale": "/spot-monitor/btc" };

// Keep the legacy export for route compatibility; the rendered surface is top navigation.
export default function Sidebar({ runtimeBoundary }) {
  const { pathname } = useLocation();
  const path = aliases[pathname] || pathname;
  const active = workspaces.find(group => group.items.some(item => item[1] === path)) || workspaces[0];
  return (
    <header className="control-room-chrome" data-testid="workspace-topbar">
      <a className="terminal-skip-link" href="#workspace-main">跳到主要内容</a>
      <div className="control-room-masthead">
        <Link to="/dashboard" className="control-room-brand" aria-label="合约主力监控">
          <span className="control-room-monogram" aria-hidden="true">
            <svg viewBox="0 0 36 36" fill="none"><path d="M5 27V10l13-5 13 5v17l-13 5-13-5Z" stroke="currentColor" strokeWidth="1.4"/><path d="M11 24V13l7 8 7-8v11" stroke="currentColor" strokeWidth="2.5" strokeLinejoin="round"/></svg>
          </span>
          <span><strong>合约主力监控</strong><small>MAIN FORCE <i>/</i> INTELLIGENCE</small></span>
        </Link>
        <nav className="control-room-primary-nav" aria-label="主导航">
          {workspaces.map(group => <Link key={group.code} to={group.items[0][1]} className={active === group ? "is-active" : ""} aria-current={active === group ? "true" : undefined}>
            <span aria-hidden="true">{group.code}</span>{group.name}
          </Link>)}
        </nav>
        <div className="control-room-tools">
          <MotionControl />
          <details className="control-room-directory" key={pathname}>
            <summary aria-label="全部模块" title="全部模块"><Squares2X2Icon aria-hidden="true"/><span>全部模块</span><ChevronDownIcon aria-hidden="true"/></summary>
            <nav aria-label="全部模块目录">
              {workspaces.map(group => <section key={group.code}><h2>{group.name}</h2>{group.items.map(([label, to]) => <Link key={to} to={to}>{label}<ArrowUpRightIcon aria-hidden="true"/></Link>)}</section>)}
            </nav>
          </details>
        </div>
      </div>
      <div className="control-room-context-bar">
        <span className="control-room-workspace-code">WORKSPACE <b>{active.code}</b></span>
        <nav className="control-room-context-nav" aria-label="当前工作区">
          {active.items.map(([label, to]) => <NavLink key={to} to={to} end className={({ isActive }) => isActive || path === to ? "is-active" : ""} aria-current={path === to ? "page" : undefined}>{label}</NavLink>)}
        </nav>
        <div className="control-room-boundary"><RuntimeBoundaryBadge runtimeBoundary={runtimeBoundary} showDetail /></div>
      </div>
    </header>
  );
}
