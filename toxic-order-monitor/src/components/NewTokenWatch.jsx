import { useCallback, useEffect, useMemo, useState } from "react";
import {
  addNewTokenWatch,
  fetchNewTokenWatchList,
  normalizeNewTokenWatchList,
  removeNewTokenWatch,
} from "../api/newTokenWatch.js";
import { useReconnectingWebSocket } from "../hooks/useReconnectingWebSocket.js";

const regimeLabels = {
  accumulation: "吸筹",
  building: "建仓",
  distribution: "出货",
  neutral: "中性",
};

const regimeClass = {
  accumulation: "border-emerald-400/35 bg-emerald-400/10 text-emerald-200",
  building: "border-yellow-400/35 bg-yellow-400/10 text-yellow-200",
  distribution: "border-red-400/35 bg-red-400/10 text-red-200",
  neutral: "border-slate-500/35 bg-slate-500/10 text-slate-300",
};

const actorLabels = {
  liquidity_provider: "LP",
  momentum_chaser: "Momentum",
  smart_money: "Smart Money",
  mixed: "Mixed",
  unknown: "Unknown",
};

const stabilityRegimeLabels = {
  trend: "趋势",
  chop: "震荡",
  liquidity_expansion: "流动扩张",
  liquidity_stress: "流动压力",
  manipulation: "操控风险",
  neutral: "中性",
};

const advisoryDirectionLabels = {
  long: "偏多",
  short: "偏空",
  no_trade: "观望",
};

export default function NewTokenWatch() {
  const [items, setItems] = useState([]);
  const [maxActiveTokens, setMaxActiveTokens] = useState(10);
  const [symbol, setSymbol] = useState("");
  const [notice, setNotice] = useState(null);
  const [loading, setLoading] = useState(true);
  const [busySymbol, setBusySymbol] = useState(null);

  const load = useCallback(async () => {
    const result = await fetchNewTokenWatchList();
    setItems(result.items);
    setMaxActiveTokens(result.maxActiveTokens);
    setLoading(false);
  }, []);

  useEffect(() => {
    load().catch((error) => {
      setLoading(false);
      setNotice({ type: "error", message: `加载失败：${error?.message || "NETWORK_ERROR"}` });
    });
  }, [load]);

  const handleWsMessage = useCallback((event) => {
    try {
      const snapshot = normalizeNewTokenWatchList(JSON.parse(event.data));
      setItems(snapshot.items);
      setMaxActiveTokens(snapshot.maxActiveTokens);
    } catch {
      // HTTP operations remain the fallback if a snapshot frame is malformed.
    }
  }, []);

  const { status: wsStatus } = useReconnectingWebSocket("/ws/new-token-flow", {
    enabled: true,
    retryMs: 1500,
    maxRetryMs: 10000,
    onMessage: handleWsMessage,
  });

  async function handleAdd(event) {
    event.preventDefault();
    const raw = symbol.trim();
    if (!raw || busySymbol) return;
    setBusySymbol(raw);
    setNotice(null);
    try {
      const result = await addNewTokenWatch(raw);
      if (!result.ok) {
        setNotice({ type: "error", message: errorMessage(result.error) });
        return;
      }
      setItems(result.items);
      setMaxActiveTokens(result.maxActiveTokens);
      setSymbol("");
      setNotice({ type: "success", message: `${result.item?.symbol || raw.toUpperCase()} 已加入观察` });
    } catch (error) {
      setNotice({ type: "error", message: errorMessage(error?.response?.data?.error || error?.message) });
    } finally {
      setBusySymbol(null);
    }
  }

  async function handleRemove(item) {
    if (!item?.symbol || busySymbol) return;
    setBusySymbol(item.symbol);
    setNotice(null);
    try {
      const result = await removeNewTokenWatch(item.symbol);
      if (!result.ok) {
        setNotice({ type: "error", message: errorMessage(result.error) });
        return;
      }
      setItems(result.items);
      setMaxActiveTokens(result.maxActiveTokens);
      setNotice({ type: "success", message: `${item.symbol} 已停止观察` });
    } catch (error) {
      setNotice({ type: "error", message: errorMessage(error?.response?.data?.error || error?.message) });
    } finally {
      setBusySymbol(null);
    }
  }

  const activeCount = items.length;
  const capacityReached = activeCount >= maxActiveTokens;
  const sortedItems = useMemo(
    () =>
      [...items].sort((left, right) => {
        const signalDelta = Number(right.lastSignal?.strength || 0) - Number(left.lastSignal?.strength || 0);
        if (Math.abs(signalDelta) > 0.001) return signalDelta;
        return String(left.symbol).localeCompare(String(right.symbol));
      }),
    [items],
  );

  return (
    <section className="rounded-2xl border border-slate-700/70 bg-slate-900/80 shadow-glow">
      <div className="border-b border-slate-700/60 p-5">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div>
            <p className="text-xs uppercase tracking-[0.3em] text-cyan-300">New Token Contract Flow</p>
            <h3 className="mt-2 text-xl font-bold text-white">新币合约行为探针</h3>
            <p className="mt-2 max-w-3xl text-sm leading-6 text-slate-400">
              对用户选择的合约 symbol 做独立订单流行为观察，输出吸筹、建仓、出货和中性候选状态。
            </p>
          </div>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <MetricPill label="活动币种" value={`${activeCount}/${maxActiveTokens}`} />
            <MetricPill label="数据通道" value={wsStatusLabel(wsStatus)} tone={wsStatus === "open" ? "emerald" : "yellow"} />
            <MetricPill label="安全边界" value="只读" tone="cyan" />
          </div>
        </div>

        <form className="mt-5 flex flex-col gap-3 sm:flex-row" onSubmit={handleAdd}>
          <label className="min-w-0 flex-1">
            <span className="mb-2 block text-xs text-slate-400">Symbol</span>
            <input
              aria-label="新币合约 symbol"
              className="w-full rounded-xl border border-slate-700 bg-slate-950 px-4 py-3 text-sm font-semibold text-white outline-none transition placeholder:text-slate-600 focus:border-cyan-400/70 focus:ring-2 focus:ring-cyan-500/20"
              disabled={capacityReached}
              onChange={(event) => setSymbol(event.target.value)}
              placeholder="例如 ABCUSDT"
              value={symbol}
            />
          </label>
          <button
            className="rounded-xl border border-cyan-400/40 bg-cyan-400/10 px-5 py-3 text-sm font-bold text-cyan-100 transition hover:bg-cyan-400/20 disabled:cursor-not-allowed disabled:border-slate-700 disabled:bg-slate-800/60 disabled:text-slate-500 sm:self-end"
            disabled={!symbol.trim() || Boolean(busySymbol) || capacityReached}
            type="submit"
          >
            加入监控
          </button>
        </form>
        {notice ? (
          <div
            className={[
              "mt-4 rounded-xl border px-4 py-3 text-sm",
              notice.type === "success"
                ? "border-emerald-400/35 bg-emerald-400/10 text-emerald-200"
                : "border-red-400/35 bg-red-400/10 text-red-200",
            ].join(" ")}
            role="status"
          >
            {notice.message}
          </div>
        ) : null}
      </div>

      <div className="p-5">
        {loading ? (
          <div className="rounded-xl border border-slate-700/60 bg-slate-950/70 p-5 text-sm text-slate-400">
            新币合约探针加载中...
          </div>
        ) : sortedItems.length === 0 ? (
          <div className="rounded-xl border border-slate-700/60 bg-slate-950/70 p-5 text-sm text-slate-400">
            暂无活动 symbol。
          </div>
        ) : (
          <div className="grid gap-3">
            {sortedItems.map((item) => (
              <TokenWatchRow
                busy={busySymbol === item.symbol}
                item={item}
                key={item.symbol}
                onRemove={() => handleRemove(item)}
              />
            ))}
          </div>
        )}
      </div>
    </section>
  );
}

function TokenWatchRow({ item, busy, onRemove }) {
  const signal = item.lastSignal || {};
  const regime = signal.regime || "neutral";
  return (
    <article className="rounded-xl border border-slate-700/60 bg-slate-950/70 p-4">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-3">
            <h4 className="text-lg font-black text-white">{item.symbol}</h4>
            <span className={`rounded-full border px-3 py-1 text-xs font-bold ${regimeClass[regime] || regimeClass.neutral}`}>
              {regimeLabels[regime] || "中性"}
            </span>
            <span className="rounded-full border border-slate-700 bg-slate-900 px-3 py-1 text-xs text-slate-400">
              {item.streamStatus}
            </span>
          </div>
          <div className="mt-3 flex flex-wrap gap-2 text-xs text-slate-400">
            {signal.evidence?.slice(0, 3).map((entry) => (
              <span className="rounded-lg border border-slate-700/70 bg-slate-900 px-2 py-1" key={entry}>
                {entry}
              </span>
            ))}
          </div>
        </div>
        <div className="grid gap-3 sm:grid-cols-[120px_120px_auto] sm:items-center">
          <SmallMeter label="Strength" value={signal.strength || 0} />
          <SmallMeter label="Confidence" value={signal.confidence || 0} />
          <button
            className="rounded-xl border border-red-400/40 bg-red-400/10 px-4 py-2 text-sm font-bold text-red-100 transition hover:bg-red-400/20 disabled:cursor-not-allowed disabled:opacity-50"
            disabled={busy}
            onClick={onRemove}
            type="button"
          >
            Stop
          </button>
        </div>
      </div>
      <div className="mt-4 grid gap-3 border-t border-slate-800 pt-4 md:grid-cols-6">
        <MetricStack
          label="Actor"
          value={actorLabel(signal.actorDecomposition)}
          detail={`SM ${percent(signal.actorDecomposition?.smartMoneyProbability)} · LP ${percent(signal.actorDecomposition?.liquidityProviderProbability)}`}
        />
        <MetricStack
          label="Rolling OFI"
          value={ofiSummary(signal.ofiWindows)}
          detail={`Persistence ${percent(signal.flowPersistence)}`}
        />
        <MetricStack
          label="Impact"
          value={impactLabel(signal.impactResponse)}
          detail={`Abs ${percent(signal.impactResponse?.absorptionScore)} · Thin ${percent(signal.impactResponse?.thinLiquidityScore)}`}
        />
        <MetricStack
          label="Liquidity"
          value={`Dep ${percent(signal.liquidityDepletion?.depletionPressure)}`}
          detail={`Bid ${percent(signal.liquidityDepletion?.bidDepletionRate)} · Ask ${percent(signal.liquidityDepletion?.askDepletionRate)}`}
        />
        <MetricStack
          label="SCL"
          value={pvgLabel(signal.signalCompression)}
          detail={sclSummary(signal.signalCompression)}
        />
        <MetricStack
          label="Kernel"
          value={kernelLabel(signal.signalCompression)}
          detail={kernelDetail(signal.signalCompression)}
        />
      </div>
    </article>
  );
}

function MetricStack({ label, value, detail }) {
  return (
    <div className="rounded-xl border border-slate-800 bg-slate-900/70 px-3 py-2">
      <p className="text-[11px] uppercase tracking-[0.16em] text-slate-500">{label}</p>
      <p className="mt-1 text-sm font-bold text-slate-100">{value}</p>
      <p className="mt-1 text-xs text-slate-400">{detail}</p>
    </div>
  );
}

function SmallMeter({ label, value }) {
  const pct = Math.round(Number(value || 0) * 100);
  return (
    <div>
      <div className="mb-1 flex items-center justify-between text-[11px] uppercase tracking-[0.16em] text-slate-500">
        <span>{label}</span>
        <span className="text-slate-300">{pct}%</span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-slate-800">
        <div className="h-full rounded-full bg-cyan-300" style={{ width: `${Math.max(0, Math.min(100, pct))}%` }} />
      </div>
    </div>
  );
}

function MetricPill({ label, value, tone = "slate" }) {
  const toneClass =
    tone === "emerald"
      ? "text-emerald-200"
      : tone === "yellow"
        ? "text-yellow-200"
        : tone === "cyan"
          ? "text-cyan-200"
          : "text-white";
  return (
    <div className="rounded-xl border border-slate-700/70 bg-slate-950/70 px-4 py-3">
      <p className="text-xs text-slate-500">{label}</p>
      <p className={`mt-1 text-sm font-black ${toneClass}`}>{value}</p>
    </div>
  );
}

function wsStatusLabel(status) {
  if (status === "open") return "实时";
  if (status === "connecting") return "连接中";
  if (status === "reconnecting") return "重连";
  return "待连接";
}

function ofiSummary(windows = []) {
  const primary = windows.find((window) => window.windowSec === 30) || windows[0];
  if (!primary) return "N/A";
  const prefix = primary.normalizedOfi >= 0 ? "+" : "";
  return `${primary.windowSec}s ${prefix}${primary.normalizedOfi.toFixed(2)}`;
}

function impactLabel(impact = {}) {
  const label = {
    absorption: "吸收",
    thin_liquidity: "薄流动性",
    balanced_response: "均衡",
    insufficient_window: "样本不足",
  }[impact.classification] || "未知";
  return `${label} ${(Number(impact.priceMovePct || 0) * 100).toFixed(2)}%`;
}

function actorLabel(actor = {}) {
  const label = actorLabels[actor.dominantActor] || "Unknown";
  return `${label} ${percent(actor.confidence)}`;
}

function pvgLabel(compression = {}) {
  const gate = compression.positionValidityGate || {};
  const status = gate.tradePermission ? "建议允许" : "建议阻断";
  return `PVG ${status} ${percent(gate.riskScore)}`;
}

function sclSummary(compression = {}) {
  return `SMP ${signedValue(compression.smartMoneyPressure)} · MFE ${signedValue(compression.momentumFlowExhaustion)} · LSM ${signedValue(compression.liquidityStressManipulation)}`;
}

function kernelLabel(compression = {}) {
  const kernel = compression.stabilityKernel || {};
  const signal = kernel.tradeSignal || {};
  return `${stabilityRegimeLabels[kernel.regime] || "中性"} · ${advisoryDirectionLabels[signal.direction] || "观望"}`;
}

function kernelDetail(compression = {}) {
  const kernel = compression.stabilityKernel || {};
  const smoothing = kernel.positionSmoothing || {};
  return `Q ${percent(kernel.regimeQuality)} · Size ${percent(smoothing.suggestedSizeMultiplier)}`;
}

function signedValue(value) {
  const numeric = Number(value || 0);
  const prefix = numeric > 0 ? "+" : "";
  return `${prefix}${numeric.toFixed(2)}`;
}

function percent(value) {
  return `${Math.round(Number(value || 0) * 100)}%`;
}

function errorMessage(error) {
  if (error === "max_active_tokens_reached") return "最多只能同时监控 10 个币。";
  if (error === "invalid_symbol") return "Symbol 格式无效。";
  if (error === "token_not_found") return "该 symbol 当前未在监控列表。";
  return error || "操作失败";
}
