param(
    [string]$OutDir = "frontend/toxic-order-monitor",
    [switch]$Install,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$root = (Resolve-Path ".").Path
$target = Join-Path $root $OutDir

if ((Test-Path -LiteralPath $target) -and -not $Force) {
    throw "Target already exists: $target. Pass -Force to overwrite scaffold files."
}

New-Item -ItemType Directory -Force -Path $target | Out-Null

function Write-ScaffoldFile {
    param(
        [Parameter(Mandatory = $true)][string]$RelativePath,
        [Parameter(Mandatory = $true)][string]$Content
    )

    $path = Join-Path $target $RelativePath
    $parent = Split-Path -Parent $path
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    Set-Content -LiteralPath $path -Value $Content -Encoding utf8
}

Write-ScaffoldFile "package.json" @'
{
  "name": "toxic-order-monitor-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "check": "vite build"
  },
  "dependencies": {
    "@headlessui/react": "^2.2.0",
    "@heroicons/react": "^2.2.0",
    "@vitejs/plugin-react": "^5.0.0",
    "axios": "^1.7.9",
    "echarts": "^5.6.0",
    "vite": "^7.0.0",
    "zustand": "^5.0.2",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "react-router-dom": "^6.28.0"
  },
  "devDependencies": {
    "autoprefixer": "^10.4.20",
    "postcss": "^8.4.49",
    "tailwindcss": "^3.4.17"
  }
}
'@

Write-ScaffoldFile "index.html" @'
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Toxic Order Monitor</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.jsx"></script>
  </body>
</html>
'@

Write-ScaffoldFile "tailwind.config.js" @'
/** @type {import("tailwindcss").Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,jsx,ts,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: "#101820",
        sand: "#f3ead8",
        signal: "#ffb000",
        poison: "#d1495b",
        lagoon: "#2a9d8f"
      }
    }
  },
  plugins: []
};
'@

Write-ScaffoldFile "postcss.config.js" @'
export default {
  plugins: {
    tailwindcss: {},
    autoprefixer: {}
  }
};
'@

Write-ScaffoldFile "vite.config.js" @'
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      "/api": "http://127.0.0.1:3000"
    }
  }
});
'@

Write-ScaffoldFile "src/index.css" @'
@tailwind base;
@tailwind components;
@tailwind utilities;

body {
  margin: 0;
  min-width: 320px;
  background:
    radial-gradient(circle at 12% 10%, rgba(255, 176, 0, 0.22), transparent 34rem),
    radial-gradient(circle at 84% 18%, rgba(42, 157, 143, 0.22), transparent 32rem),
    #101820;
  color: #f8f4ea;
}
'@

Write-ScaffoldFile "src/main.jsx" @'
import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App.jsx";
import "./index.css";

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <BrowserRouter>
      <App />
    </BrowserRouter>
  </React.StrictMode>
);
'@

Write-ScaffoldFile "src/App.jsx" @'
import { Route, Routes } from "react-router-dom";
import Dashboard from "./pages/Dashboard.jsx";

export default function App() {
  return (
    <Routes>
      <Route path="*" element={<Dashboard />} />
    </Routes>
  );
}
'@

Write-ScaffoldFile "src/store/signalsStore.js" @'
import { create } from "zustand";

export const useSignalStore = create((set) => ({
  signals: [],
  pushLogs: [],
  selectedRisk: "all",
  discordStatus: "proxy",
  riskStats: { high: 0, medium: 0, low: 0 },
  setSignals: (signals) => set({ signals }),
  setRiskStats: (riskStats) => set({ riskStats }),
  setSelectedRisk: (selectedRisk) => set({ selectedRisk }),
  setDiscordStatus: (discordStatus) => set({ discordStatus }),
  addPushLog: (entry) =>
    set((state) => ({ pushLogs: [entry, ...state.pushLogs].slice(0, 50) }))
}));
'@

Write-ScaffoldFile "src/api/client.js" @'
import axios from "axios";

export const api = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || "/api",
  timeout: 10000
});
'@

Write-ScaffoldFile "src/api/signals.js" @'
import { api } from "./client.js";

export async function fetchSignals() {
  const [inbox, fusion] = await Promise.all([
    api.get("/toxicity/signal-inbox/recent"),
    api.get("/toxicity/fusion/recent")
  ]);

  const inboxItems = Array.isArray(inbox.data?.items) ? inbox.data.items : [];
  const fusionItems = Array.isArray(fusion.data?.items) ? fusion.data.items : [];
  return [...inboxItems, ...fusionItems].map(normalizeSignal);
}

export async function fetchRiskStats() {
  try {
    const response = await api.get("/risk-stats");
    return response.data;
  } catch {
    return null;
  }
}

function normalizeSignal(item) {
  return {
    id: item.signalId || item.id || `${item.symbol || "UNKNOWN"}-${item.createdAtMs || Date.now()}`,
    time: item.createdAt || item.createdAtMs || item.ts || "Unavailable",
    market: item.symbol || "BTC-PERP",
    type: item.kind || item.signalKind || "toxic_signal",
    reason: item.reason || item.summary || "No reason available",
    level: item.severity || item.riskLevel || "watch",
    score: Number(item.score || item.confidenceScore || item.riskScore || 0),
    status: item.status || item.alertDecision || "watch_signal_only",
    raw: item
  };
}
'@

Write-ScaffoldFile "src/api/discord.js" @'
import { api } from "./client.js";

// Safe boundary: browser code never owns a Discord webhook.
// The backend or sidecar decides whether notifications are enabled and where to send them.
export async function requestDiscordAlert(signal) {
  const response = await api.post("/notifications/discord/test", {
    signalId: signal.id,
    market: signal.market,
    type: signal.type,
    reason: signal.reason,
    score: signal.score
  });
  return response.data;
}
'@

Write-ScaffoldFile "src/components/Sidebar.jsx" @'
import { useState } from "react";
import { NavLink } from "react-router-dom";

const links = [
  { name: "Home", to: "/" },
  {
    name: "Signals",
    children: [
      { name: "Abnormal Signals", to: "/signals" },
      { name: "Signal History", to: "/history" }
    ]
  },
  {
    name: "Operations",
    children: [
      { name: "Alert Rules", to: "/rules" },
      { name: "Discord Settings", to: "/notifications" },
      { name: "System Settings", to: "/settings" }
    ]
  }
];

export default function Sidebar() {
  const [openGroups, setOpenGroups] = useState({ Signals: true, Operations: true });
  const toggleGroup = (name) => setOpenGroups((value) => ({ ...value, [name]: !value[name] }));

  return (
    <aside className="hidden min-h-screen w-72 border-r border-white/10 bg-ink/80 p-5 backdrop-blur lg:block">
      <div className="mb-8">
        <p className="text-xs uppercase tracking-[0.35em] text-signal">Monitor</p>
        <h1 className="mt-2 text-2xl font-black text-sand">Toxic Flow Console</h1>
      </div>
      <nav className="flex flex-col gap-2">
        {links.map((link) =>
          link.children ? (
            <div key={link.name} className="rounded-2xl bg-white/5 p-2">
              <button
                className="flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-sm font-bold text-sand/80 hover:bg-white/10"
                onClick={() => toggleGroup(link.name)}
                type="button"
              >
                {link.name}
                <span>{openGroups[link.name] ? "-" : "+"}</span>
              </button>
              {openGroups[link.name] ? (
                <div className="mt-2 flex flex-col gap-1">
                  {link.children.map((child) => (
                    <NavLink
                      key={child.to}
                      to={child.to}
                      className={({ isActive }) =>
                        `rounded-xl px-4 py-2 text-sm transition ${
                          isActive ? "bg-signal text-ink" : "text-sand/60 hover:bg-white/10 hover:text-sand"
                        }`
                      }
                    >
                      {child.name}
                    </NavLink>
                  ))}
                </div>
              ) : null}
            </div>
          ) : (
            <NavLink
              key={link.to}
              to={link.to}
              className={({ isActive }) =>
                `rounded-2xl px-4 py-3 text-sm transition ${
                  isActive ? "bg-signal text-ink" : "text-sand/70 hover:bg-white/10 hover:text-sand"
                }`
              }
            >
              {link.name}
            </NavLink>
          )
        )}
      </nav>
    </aside>
  );
}
'@

Write-ScaffoldFile "src/components/Header.jsx" @'
import { useEffect, useState } from "react";

export default function Header({ highRiskCount, discordStatus }) {
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    const timer = window.setInterval(() => setNow(new Date()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  return (
    <header className="mb-8 flex flex-col gap-4 rounded-[2rem] border border-white/10 bg-white/8 p-6 shadow-2xl shadow-black/20 backdrop-blur md:flex-row md:items-end md:justify-between">
      <div>
        <p className="text-sm uppercase tracking-[0.3em] text-lagoon">Read-only cockpit</p>
        <h2 className="mt-2 text-4xl font-black text-sand">Suspicious Flow Radar</h2>
        <p className="mt-2 max-w-2xl text-sand/65">
          Operator view for toxic-flow signals, risk tiers, notification requests, and audit-safe triage.
        </p>
      </div>
      <div className="grid gap-2 text-sm text-sand/75 md:min-w-72">
        <div className="rounded-full border border-poison/40 px-4 py-2">High risk: {highRiskCount}</div>
        <div className="rounded-full border border-lagoon/40 px-4 py-2">Discord: {discordStatus}</div>
        <div className="rounded-full border border-signal/40 px-4 py-2">System time: {now.toLocaleTimeString()}</div>
      </div>
    </header>
  );
}
'@

Write-ScaffoldFile "src/components/RiskCard.jsx" @'
const tones = {
  high: "from-poison to-red-700",
  medium: "from-signal to-orange-600",
  low: "from-lagoon to-emerald-700"
};

export default function RiskCard({ level, count, percentage, active, onClick }) {
  const key = String(level || "low").toLowerCase();
  return (
    <button
      className={`rounded-[1.75rem] bg-gradient-to-br ${tones[key] || tones.low} p-5 text-left text-ink shadow-xl transition hover:-translate-y-1 ${active ? "ring-4 ring-sand" : ""}`}
      onClick={onClick}
      type="button"
    >
      <p className="text-sm font-bold uppercase tracking-[0.25em]">{level} risk</p>
      <p className="mt-4 text-5xl font-black">{count}</p>
      <p className="mt-2 text-sm font-semibold">{percentage}% of visible signals</p>
    </button>
  );
}
'@

Write-ScaffoldFile "src/components/SignalDetail.jsx" @'
import { Dialog } from "@headlessui/react";
import { useState } from "react";

export default function SignalDetail({ signal }) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <button className="rounded-full border border-signal/60 px-3 py-1 text-xs text-signal" onClick={() => setOpen(true)}>
        Inspect
      </button>
      <Dialog className="relative z-50" onClose={setOpen} open={open}>
        <div className="fixed inset-0 bg-black/70" aria-hidden="true" />
        <div className="fixed inset-0 flex items-center justify-center p-4">
          <Dialog.Panel className="max-h-[80vh] w-full max-w-2xl overflow-auto rounded-3xl border border-white/10 bg-ink p-6 text-sand shadow-2xl">
            <Dialog.Title className="text-2xl font-black">{signal.market} detail</Dialog.Title>
            <p className="mt-2 text-sand/70">{signal.reason}</p>
            <pre className="mt-5 overflow-auto rounded-2xl bg-black/30 p-4 text-xs text-sand/80">
              {JSON.stringify(signal.raw || signal, null, 2)}
            </pre>
            <button className="mt-5 rounded-full bg-signal px-4 py-2 font-bold text-ink" onClick={() => setOpen(false)}>
              Close
            </button>
          </Dialog.Panel>
        </div>
      </Dialog>
    </>
  );
}
'@

Write-ScaffoldFile "src/components/SignalTable.jsx" @'
import { useMemo, useState } from "react";
import SignalDetail from "./SignalDetail.jsx";

const PAGE_SIZE = 12;

export default function SignalTable({ signals, onPushDiscord }) {
  const [page, setPage] = useState(1);
  const pageCount = Math.max(1, Math.ceil(signals.length / PAGE_SIZE));
  const visibleSignals = useMemo(
    () => signals.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE),
    [page, signals]
  );

  return (
    <div className="overflow-hidden rounded-[1.75rem] border border-white/10 bg-white/8">
      <table className="w-full border-collapse text-left text-sm">
        <thead className="bg-white/10 text-xs uppercase tracking-[0.2em] text-sand/55">
          <tr>
            <th className="p-4">Time</th>
            <th className="p-4">Market</th>
            <th className="p-4">Type</th>
            <th className="p-4">Reason</th>
            <th className="p-4">Level</th>
            <th className="p-4">Score</th>
            <th className="p-4">Status</th>
            <th className="p-4">Action</th>
          </tr>
        </thead>
        <tbody>
          {visibleSignals.map((signal) => (
            <tr key={signal.id} className="border-t border-white/10 text-sand/80">
              <td className="p-4">{String(signal.time)}</td>
              <td className="p-4 font-bold text-sand">{signal.market}</td>
              <td className="p-4">{signal.type}</td>
              <td className="max-w-sm truncate p-4">{signal.reason}</td>
              <td className="p-4">{signal.level}</td>
              <td className="p-4">{signal.score}/100</td>
              <td className="p-4">{signal.status}</td>
              <td className="flex gap-2 p-4">
                <SignalDetail signal={signal} />
                <button
                  className="rounded-full border border-lagoon/60 px-3 py-1 text-xs text-lagoon"
                  onClick={() => onPushDiscord(signal)}
                  type="button"
                >
                  Push
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div className="flex items-center justify-between border-t border-white/10 p-4 text-sm text-sand/60">
        <span>Page {page} / {pageCount}</span>
        <div className="flex gap-2">
          <button className="rounded-full border border-white/15 px-3 py-1" disabled={page <= 1} onClick={() => setPage((value) => Math.max(1, value - 1))} type="button">
            Prev
          </button>
          <button className="rounded-full border border-white/15 px-3 py-1" disabled={page >= pageCount} onClick={() => setPage((value) => Math.min(pageCount, value + 1))} type="button">
            Next
          </button>
        </div>
      </div>
    </div>
  );
}
'@

Write-ScaffoldFile "src/components/RiskChart.jsx" @'
export default function RiskChart({ stats }) {
  const total = Math.max(1, Number(stats.high || 0) + Number(stats.medium || 0) + Number(stats.low || 0));
  const rows = [
    ["High", stats.high || 0, "bg-poison"],
    ["Medium", stats.medium || 0, "bg-signal"],
    ["Low", stats.low || 0, "bg-lagoon"]
  ];

  return (
    <section className="rounded-[1.75rem] border border-white/10 bg-white/8 p-5">
      <h3 className="text-lg font-black text-sand">Risk Distribution</h3>
      <div className="mt-5 space-y-4">
        {rows.map(([label, value, color]) => (
          <div key={label}>
            <div className="mb-1 flex justify-between text-sm text-sand/65">
              <span>{label}</span>
              <span>{value}</span>
            </div>
            <div className="h-3 overflow-hidden rounded-full bg-white/10">
              <div className={`h-full ${color}`} style={{ width: `${(Number(value) / total) * 100}%` }} />
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
'@

Write-ScaffoldFile "src/components/PushLog.jsx" @'
export default function PushLog({ logs }) {
  return (
    <section className="rounded-[1.75rem] border border-white/10 bg-white/8 p-5">
      <h3 className="text-lg font-black text-sand">Notification Requests</h3>
      <div className="mt-4 space-y-3">
        {logs.length ? logs.map((log, index) => (
          <div className="rounded-2xl bg-black/25 p-3 text-sm text-sand/70" key={`${log.time}-${index}`}>
            <span className="font-bold text-lagoon">{log.status}</span> {log.message}
          </div>
        )) : <p className="text-sm text-sand/45">No notification requests yet.</p>}
      </div>
    </section>
  );
}
'@

Write-ScaffoldFile "src/pages/Dashboard.jsx" @'
import { useEffect } from "react";
import Header from "../components/Header.jsx";
import PushLog from "../components/PushLog.jsx";
import RiskCard from "../components/RiskCard.jsx";
import RiskChart from "../components/RiskChart.jsx";
import Sidebar from "../components/Sidebar.jsx";
import SignalTable from "../components/SignalTable.jsx";
import { requestDiscordAlert } from "../api/discord.js";
import { fetchSignals } from "../api/signals.js";
import { useSignalStore } from "../store/signalsStore.js";

function buildStats(signals) {
  return signals.reduce(
    (stats, signal) => {
      const level = String(signal.level || "").toLowerCase();
      if (level.includes("alert") || level.includes("high") || level.includes("critical")) stats.high += 1;
      else if (level.includes("warning") || level.includes("medium")) stats.medium += 1;
      else stats.low += 1;
      return stats;
    },
    { high: 0, medium: 0, low: 0 }
  );
}

export default function Dashboard() {
  const {
    signals,
    riskStats,
    pushLogs,
    selectedRisk,
    discordStatus,
    setSignals,
    setRiskStats,
    setSelectedRisk,
    setDiscordStatus,
    addPushLog
  } = useSignalStore();
  const filteredSignals = signals.filter((signal) => {
    if (selectedRisk === "all") return true;
    const level = String(signal.level || "").toLowerCase();
    if (selectedRisk === "high") return level.includes("alert") || level.includes("high") || level.includes("critical");
    if (selectedRisk === "medium") return level.includes("warning") || level.includes("medium");
    return !(level.includes("alert") || level.includes("high") || level.includes("critical") || level.includes("warning") || level.includes("medium"));
  });

  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const nextSignals = await fetchSignals();
        if (cancelled) return;
        setSignals(nextSignals);
        setRiskStats(buildStats(nextSignals));
      } catch {
        if (!cancelled) {
          setSignals([]);
          setRiskStats({ high: 0, medium: 0, low: 0 });
        }
      }
    }
    void load();
    const timer = window.setInterval(load, 10000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [setSignals, setRiskStats]);

  async function pushSignal(signal) {
    try {
      setDiscordStatus("proxy pending");
      await requestDiscordAlert(signal);
      setDiscordStatus("proxy accepted");
      addPushLog({
        time: new Date().toISOString(),
        status: "accepted",
        message: `${signal.market} ${signal.type} notification request accepted`
      });
    } catch (error) {
      setDiscordStatus("proxy failed");
      addPushLog({
        time: new Date().toISOString(),
        status: "failed",
        message: error?.message || "notification proxy unavailable"
      });
    }
  }

  return (
    <div className="flex min-h-screen">
      <Sidebar />
      <main className="flex-1 p-4 md:p-8">
        <Header highRiskCount={riskStats.high} discordStatus={discordStatus} />
        <div className="mb-4 flex flex-wrap gap-2 text-sm">
          <button className={`rounded-full border px-3 py-1 ${selectedRisk === "all" ? "bg-sand text-ink" : "border-white/15 text-sand/65"}`} onClick={() => setSelectedRisk("all")} type="button">
            All
          </button>
        </div>
        <div className="grid gap-4 md:grid-cols-3">
          <RiskCard active={selectedRisk === "high"} level="High" count={riskStats.high} percentage={((riskStats.high / Math.max(1, signals.length)) * 100).toFixed(1)} onClick={() => setSelectedRisk("high")} />
          <RiskCard active={selectedRisk === "medium"} level="Medium" count={riskStats.medium} percentage={((riskStats.medium / Math.max(1, signals.length)) * 100).toFixed(1)} onClick={() => setSelectedRisk("medium")} />
          <RiskCard active={selectedRisk === "low"} level="Low" count={riskStats.low} percentage={((riskStats.low / Math.max(1, signals.length)) * 100).toFixed(1)} onClick={() => setSelectedRisk("low")} />
        </div>
        <div className="mt-6 grid gap-6 xl:grid-cols-[1fr_22rem]">
          <SignalTable onPushDiscord={pushSignal} signals={filteredSignals} />
          <div className="space-y-6">
            <RiskChart stats={riskStats} />
            <PushLog logs={pushLogs} />
          </div>
        </div>
      </main>
    </div>
  );
}
'@

Write-ScaffoldFile "README.md" @'
# Toxic Order Monitor Frontend

Generated scaffold for an optional React/Vite operator console.

## Commands

```powershell
npm install
npm run dev
npm run build
```

## Security Boundary

- Browser code never stores Discord webhook URLs.
- Notification requests go through backend or sidecar APIs.
- This scaffold is not wired into the Rust monitor by default.
'@

if ($Install) {
    Push-Location $target
    try {
        npm install
    } finally {
        Pop-Location
    }
}

Write-Host "Scaffold generated at $target"
Write-Host "Next: cd `"$target`"; npm install; npm run dev"
