import { Navigate, useParams } from "react-router-dom";
import ContractWhaleMonitor from "../components/ContractWhaleMonitor.jsx";
import Sidebar from "../components/Sidebar.jsx";

const MAINSTREAM_ASSETS = new Set(["btc", "eth"]);

export default function ContractWhaleRoute() {
  const { symbol = "" } = useParams();
  const normalized = String(symbol).toLowerCase();
  if (!MAINSTREAM_ASSETS.has(normalized)) {
    return <Navigate to="/contract-whale/btc" replace />;
  }

  return (
    <div className="workspace-shell contract-workspace-shell flex min-h-screen flex-col lg:flex-row" data-testid="workspace-shell">
      <Sidebar />
      <main className="workspace-main contract-workspace-main workspace-route-contract-whale w-full min-w-0 flex-1" data-testid="workspace-main">
        <ContractWhaleMonitor lockedSymbol={normalized.toUpperCase()} />
      </main>
    </div>
  );
}
