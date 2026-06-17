import { Navigate, Route, Routes } from "react-router-dom";
import Dashboard from "./pages/Dashboard.jsx";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Dashboard />} />
      <Route path="/dashboard" element={<Dashboard />} />
      <Route path="/contract-whale" element={<Dashboard />} />
      <Route path="/alt-contract-monitor" element={<Dashboard />} />
      <Route path="/new-token-watch" element={<Dashboard />} />
      <Route path="/spot-monitor" element={<Dashboard />} />
      <Route path="/spot-whale" element={<Dashboard />} />
      <Route path="/signals" element={<Dashboard />} />
      <Route path="/history" element={<Dashboard />} />
      <Route path="/rules" element={<Dashboard />} />
      <Route path="/usage-guide" element={<Dashboard />} />
      <Route path="/discord" element={<Dashboard />} />
      <Route path="/settings" element={<Dashboard />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
