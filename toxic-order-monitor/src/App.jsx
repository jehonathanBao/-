import { lazy, Suspense } from "react";
import { Navigate, Route, Routes, useParams } from "react-router-dom";

const Dashboard = lazy(() => import("./pages/Dashboard.jsx"));
const BinanceOrderflowRoute = lazy(() => import("./pages/BinanceOrderflowRoute.jsx"));

const MAINSTREAM_ASSETS = new Set(["btc", "eth"]);

export default function App() {
  return (
    <Suspense fallback={<RouteLoading />}>
      <Routes>
      <Route path="/" element={<Navigate to="/dashboard" replace />} />
      <Route path="/dashboard" element={<Dashboard />} />
      <Route path="/contract-whale" element={<Navigate to="/contract-whale/btc" replace />} />
      <Route path="/contract-whale/:symbol" element={<MainstreamAssetRoute basePath="/contract-whale" />} />
      <Route path="/liquidation-cascade" element={<Dashboard />} />
      <Route path="/alt-contract-monitor" element={<Dashboard />} />
      <Route path="/new-token-watch" element={<Dashboard />} />
      <Route path="/binance-orderflow" element={<BinanceOrderflowRoute />} />
      <Route path="/spot-monitor" element={<Navigate to="/spot-monitor/btc" replace />} />
      <Route path="/spot-monitor/:symbol" element={<MainstreamAssetRoute basePath="/spot-monitor" />} />
      <Route path="/spot-whale" element={<Navigate to="/spot-monitor/btc" replace />} />
      <Route path="/signals" element={<Dashboard />} />
      <Route path="/history" element={<Dashboard />} />
      <Route path="/rules" element={<Dashboard />} />
      <Route path="/usage-guide" element={<Dashboard />} />
      <Route path="/discord" element={<Dashboard />} />
      <Route path="/settings" element={<Dashboard />} />
      <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </Suspense>
  );
}

function RouteLoading() {
  return (
    <div className="control-room-loading" role="status">
      <span className="control-room-loading-mark" aria-hidden="true">M</span>
      <strong>合约主力监控</strong>
      <span>正在加载工作区…</span>
    </div>
  );
}

function MainstreamAssetRoute({ basePath }) {
  const { symbol = "" } = useParams();
  if (!MAINSTREAM_ASSETS.has(String(symbol).toLowerCase())) {
    return <Navigate to={`${basePath}/btc`} replace />;
  }
  return <Dashboard />;
}
