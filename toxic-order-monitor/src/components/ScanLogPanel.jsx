import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fetchScanLogs, normalizeScanLogItem } from "../api/scanLogs.js";
import { useReconnectingWebSocket } from "../hooks/useReconnectingWebSocket.js";

const MAX_LOCAL_SCAN_LOGS = 200;

export default function ScanLogPanel() {
  const [items, setItems] = useState([]);
  const [autoScroll, setAutoScroll] = useState(true);
  const scrollRef = useRef(null);

  useEffect(() => {
    let active = true;
    fetchScanLogs(100).then((logs) => {
      if (active) {
        setItems(mergeScanLogs([], logs));
      }
    });
    return () => {
      active = false;
    };
  }, []);

  const handleMessage = useCallback((event) => {
    try {
      const payload = JSON.parse(event.data);
      const item = normalizeScanLogItem(payload?.item || payload);
      if (!item) {
        return;
      }
      setItems((current) => mergeScanLogs(current, [item]));
    } catch {
      // Ignore malformed scan-log frames; the initial GET remains the fallback.
    }
  }, []);

  const { status } = useReconnectingWebSocket("/ws/scan-logs", {
    retryMs: 1000,
    maxRetryMs: 15000,
    onMessage: handleMessage,
  });

  useEffect(() => {
    if (!autoScroll || !scrollRef.current) {
      return;
    }
    scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
  }, [autoScroll, items]);

  const statusText = useMemo(() => scanLogStatusLabel(status), [status]);

  return (
    <section
      aria-label="扫描日志"
      className="rounded-2xl border border-cyan-400/25 bg-slate-950/80 p-5 shadow-glow"
    >
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="font-bold text-white">扫描日志</h3>
          <p className="mt-1 text-xs text-slate-400">实时扫描、候选信号和 Discord 推送状态</p>
        </div>
        <span className={statusClass(status)}>{statusText}</span>
      </div>

      <div className="mt-4 flex flex-wrap gap-2">
        <button
          className="rounded-lg border border-slate-700/70 px-3 py-1.5 text-xs font-semibold text-slate-300 hover:border-cyan-400 hover:text-cyan-200"
          onClick={() => setAutoScroll((value) => !value)}
          type="button"
        >
          {autoScroll ? "暂停滚动" : "继续滚动"}
        </button>
        <button
          className="rounded-lg border border-slate-700/70 px-3 py-1.5 text-xs font-semibold text-slate-300 hover:border-red-400 hover:text-red-200"
          onClick={() => setItems([])}
          type="button"
        >
          清空显示
        </button>
      </div>

      <div
        className="mt-4 max-h-72 space-y-2 overflow-y-auto pr-1"
        data-testid="scan-log-list"
        ref={scrollRef}
      >
        {items.length === 0 ? (
          <p className="rounded-xl border border-slate-800 bg-slate-900/60 px-3 py-4 text-sm text-slate-500">
            暂无扫描日志
          </p>
        ) : (
          items.map((item) => (
            <article className="rounded-xl border border-slate-800 bg-slate-900/70 p-3" key={item.id}>
              <div className="flex flex-wrap items-center justify-between gap-2">
                <span className={levelClass(item.level)}>{levelLabel(item.level)}</span>
                <time className="text-xs text-slate-500">{item.ts || "runtime"}</time>
              </div>
              <p className="mt-2 text-sm text-slate-200">{item.message}</p>
              <div className="mt-2 flex flex-wrap gap-2 text-xs text-slate-500">
                {item.symbol ? <span>{item.symbol}</span> : null}
                {item.candidateId ? <span>{item.candidateId}</span> : null}
                <span>{item.kind}</span>
              </div>
            </article>
          ))
        )}
      </div>
    </section>
  );
}

function mergeScanLogs(current, incoming) {
  const byId = new Map();
  for (const item of [...current, ...incoming]) {
    if (item?.id) {
      byId.set(item.id, item);
    }
  }
  return Array.from(byId.values()).slice(-MAX_LOCAL_SCAN_LOGS);
}

function scanLogStatusLabel(status) {
  if (status === "open") return "connected";
  if (status === "reconnecting") return "reconnecting";
  if (status === "connecting") return "connecting";
  if (status === "closed") return "disconnected";
  return "idle";
}

function statusClass(status) {
  const base = "rounded-full border px-3 py-1 text-xs font-semibold";
  if (status === "open") return `${base} border-emerald-400/40 text-emerald-300`;
  if (status === "reconnecting" || status === "connecting") {
    return `${base} border-yellow-400/40 text-yellow-300`;
  }
  return `${base} border-slate-700 text-slate-400`;
}

function levelClass(level) {
  const base = "rounded-full border px-2 py-0.5 text-[11px] font-bold uppercase";
  if (level === "error") return `${base} border-red-400/40 text-red-300`;
  if (level === "warn") return `${base} border-yellow-400/40 text-yellow-300`;
  return `${base} border-cyan-400/40 text-cyan-300`;
}

function levelLabel(level) {
  return level || "info";
}
