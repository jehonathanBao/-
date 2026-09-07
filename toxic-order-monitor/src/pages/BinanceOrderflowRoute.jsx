import Header from "../components/Header.jsx";
import Sidebar from "../components/Sidebar.jsx";
import BinanceOrderflowChart from "../components/BinanceOrderflowChart.jsx";
import WorkspacePageHeader from "../components/WorkspacePageHeader.jsx";

export default function BinanceOrderflowRoute() {
  return (
    <div className="workspace-shell control-room-shell" data-testid="workspace-shell">
      <Sidebar />
      <main className="workspace-main workspace-route-binance-orderflow w-full min-w-0 flex-1" data-testid="workspace-main" id="workspace-main" tabIndex={-1}>
        <Header discordConnected={false} highUnhandledCount={0} />
        <div className="workspace-content">
          <WorkspacePageHeader
            badge="Binance-only · 只读 · 不下单"
            description="沿用合约主力监控的 Binance 永续行情口径，按周期查看每根 K 线的主动买、主动卖与 Delta。"
            eyebrow="Binance Orderflow Delta"
            title="Binance 订单流 K 线"
          />
          <BinanceOrderflowChart />
        </div>
      </main>
    </div>
  );
}
