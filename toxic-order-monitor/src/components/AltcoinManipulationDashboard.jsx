import { useEffect, useMemo, useState } from "react";
import { fetchAltcoinSignals } from "../api/liquidationCascade.js";

const DEFAULT_SYMBOLS = ["ASTERUSDT"];
const MAX_SYMBOLS = 10;
const STORAGE_KEY = "altcoin_manipulation_watchlist_v1";

const regimeTone = {
  MANIPULATION_HIGH: "border-red-400/40 bg-red-500/10 text-red-100",
  MANIPULATION_MEDIUM: "border-orange-400/40 bg-orange-500/10 text-orange-100",
  CLEAN_MARKET: "border-emerald-400/35 bg-emerald-500/10 text-emerald-100",
};

const componentLabels = [
  ["oiSignalScore", "OI 结构"],
  ["volumeSignalScore", "成交结构"],
  ["fundingSignalScore", "Funding 压力"],
  ["priceSignalScore", "价格操控"],
];

export default function AltcoinManipulationDashboard() {
  const [inputSymbol, setInputSymbol] = useState("");
  const [symbols, setSymbols] = useState(loadInitialSymbols);
  const [selectedSymbol, setSelectedSymbol] = useState(() => symbols[0] || DEFAULT_SYMBOLS[0]);
  const [results, setResults] = useState([]);
  const [loading, setLoading] = useState(false);
  const [notice, setNotice] = useState(null);
  const [refreshNonce, setRefreshNonce] = useState(0);

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(symbols));
  }, [symbols]);

  useEffect(() => {
    if (!symbols.includes(selectedSymbol)) {
      setSelectedSymbol(symbols[0] || DEFAULT_SYMBOLS[0]);
    }
  }, [selectedSymbol, symbols]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    Promise.all(
      symbols.map((symbol) =>
        fetchAltcoinSignals(symbol).then((result) => ({
          data: result.data,
          error: result.error,
          requestedSymbol: symbol,
        })),
      ),
    )
      .then((items) => {
        if (cancelled) return;
        setResults(items);
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [refreshNonce, symbols]);

  const sortedResults = useMemo(
    () =>
      [...results].sort(
        (left, right) =>
          scoreValue(right.data?.manipulationScore) - scoreValue(left.data?.manipulationScore) ||
          String(left.data?.symbol || left.requestedSymbol).localeCompare(String(right.data?.symbol || right.requestedSymbol)),
      ),
    [results],
  );

  const selectedResult = useMemo(
    () =>
      results.find(
        (item) => normalizeSymbolInput(item.data?.symbol || item.requestedSymbol) === normalizeSymbolInput(selectedSymbol),
      ) || results[0],
    [results, selectedSymbol],
  );

  const selectedState = selectedResult?.data || null;
  const metricRows = useMemo(() => {
    const metrics = selectedState?.metrics || {};
    return Object.entries(metrics)
      .filter(([, value]) => Number.isFinite(Number(value)))
      .sort(([left], [right]) => left.localeCompare(right));
  }, [selectedState]);

  const summary = useMemo(() => {
    const highCount = results.filter((item) => item.data?.regime === "MANIPULATION_HIGH").length;
    const top = sortedResults[0]?.data;
    return {
      highCount,
      topRegime: top?.regime || "-",
      topScore: top?.manipulationScore,
      total: symbols.length,
    };
  }, [results, sortedResults, symbols.length]);

  function handleSubmit(event) {
    event.preventDefault();
    const additions = parseSymbolInput(inputSymbol);
    if (!additions.length) {
      setNotice({ type: "warning", message: "请输入至少一个 USDT 合约 symbol。" });
      return;
    }
    const existing = new Set(symbols);
    const nextSymbols = [...symbols];
    for (const symbol of additions) {
      if (nextSymbols.length >= MAX_SYMBOLS) break;
      if (!existing.has(symbol)) {
        existing.add(symbol);
        nextSymbols.push(symbol);
      }
    }
    if (nextSymbols.length === symbols.length) {
      setNotice({ type: "warning", message: "没有新增 symbol，可能已在监控列表或超过最多 10 个限制。" });
      return;
    }
    setSymbols(nextSymbols);
    setSelectedSymbol(additions.find((symbol) => existing.has(symbol)) || nextSymbols[nextSymbols.length - 1]);
    setInputSymbol("");
    setNotice({ type: "success", message: `已加入 ${nextSymbols.length} / ${MAX_SYMBOLS} 个妖币控盘监控 symbol。` });
  }

  function removeSymbol(symbol) {
    if (symbols.length <= 1) {
      setNotice({ type: "warning", message: "至少保留 1 个 symbol 作为监控对象。" });
      return;
    }
    const nextSymbols = symbols.filter((item) => item !== symbol);
    setSymbols(nextSymbols);
    if (selectedSymbol === symbol) {
      setSelectedSymbol(nextSymbols[0]);
    }
    setNotice({ type: "success", message: `已从本页监控列表移除 ${symbol}。` });
  }

  function refreshAll() {
    setRefreshNonce((value) => value + 1);
    setNotice({ type: "success", message: "已刷新全部山寨控盘监控 symbol。" });
  }

  return (
    <section className="space-y-5">
      <div className="rounded-2xl border border-slate-700/70 bg-slate-900/80 p-5 shadow-glow">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div>
            <p className="text-xs uppercase tracking-[0.3em] text-cyan-300">Altcoin Manipulation Engine</p>
            <h3 className="mt-2 text-2xl font-black text-white">妖币控盘监控</h3>
            <p className="mt-2 max-w-4xl text-sm leading-6 text-slate-400">
              可同时读取多个 /api/altcoin/signals 输出，按 OI、成交、Funding、价格结构拆解山寨合约控盘风险；BTC 不进入该模型。
            </p>
          </div>
          <form className="flex w-full max-w-2xl flex-col gap-2 sm:flex-row" onSubmit={handleSubmit}>
            <label className="sr-only" htmlFor="altcoin-symbol">
              山寨合约 symbols
            </label>
            <input
              className="min-h-11 flex-1 rounded-xl border border-slate-700 bg-slate-950 px-4 py-2 text-sm font-semibold text-cyan-100 outline-none transition placeholder:text-slate-600 focus:border-cyan-400"
              id="altcoin-symbol"
              onChange={(event) => setInputSymbol(event.target.value)}
              placeholder="例如 ASTERUSDT, JTOUSDT, ZECUSDT"
              value={inputSymbol}
            />
            <button
              className="min-h-11 rounded-xl border border-cyan-400/40 bg-cyan-500/15 px-5 text-sm font-black text-cyan-100 transition hover:bg-cyan-400/20 disabled:cursor-not-allowed disabled:opacity-60"
              disabled={loading || symbols.length >= MAX_SYMBOLS}
              type="submit"
            >
              加入监控
            </button>
            <button
              className="min-h-11 rounded-xl border border-slate-600 bg-slate-800/70 px-5 text-sm font-black text-slate-100 transition hover:border-cyan-400 hover:text-cyan-100 disabled:cursor-not-allowed disabled:opacity-60"
              disabled={loading}
              onClick={refreshAll}
              type="button"
            >
              {loading ? "刷新中" : "刷新全部"}
            </button>
          </form>
        </div>
        <div className="mt-4 flex flex-wrap items-center gap-2 text-xs text-slate-400">
          <span className="rounded-full border border-slate-700 px-3 py-1">最多 {MAX_SYMBOLS} 个</span>
          <span className="rounded-full border border-slate-700 px-3 py-1">逗号 / 空格可一次加入多个</span>
          <span className="rounded-full border border-slate-700 px-3 py-1">仅保存浏览器本页 watchlist</span>
        </div>
        {notice ? (
          <div
            className={[
              "mt-4 rounded-xl border px-4 py-3 text-sm",
              notice.type === "success"
                ? "border-emerald-400/35 bg-emerald-400/10 text-emerald-100"
                : "border-yellow-400/35 bg-yellow-400/10 text-yellow-100",
            ].join(" ")}
            role="status"
          >
            {notice.message}
          </div>
        ) : null}
      </div>

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <ScoreCard label="Symbols" value={`${summary.total} / ${MAX_SYMBOLS}`} />
        <ScoreCard label="High Risk" value={summary.highCount} />
        <ScoreCard
          label="Top Regime"
          tone={regimeTone[summary.topRegime] || "border-slate-700 bg-slate-900 text-slate-100"}
          value={summary.topRegime}
        />
        <ScoreCard label="Top Score" value={percent(summary.topScore)} />
      </div>

      <Panel title="多币种监控列表">
        <div className="grid gap-3 lg:grid-cols-2 2xl:grid-cols-3">
          {sortedResults.map((item) => (
            <SymbolCard
              error={item.error}
              key={item.requestedSymbol}
              onRemove={() => removeSymbol(item.requestedSymbol)}
              onSelect={() => setSelectedSymbol(item.requestedSymbol)}
              selected={normalizeSymbolInput(selectedSymbol) === normalizeSymbolInput(item.requestedSymbol)}
              state={item.data}
              symbol={item.requestedSymbol}
            />
          ))}
        </div>
      </Panel>

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-5">
          <Panel title={`四组件控盘评分 · ${selectedState?.symbol || selectedSymbol}`}>
            <div className="grid gap-4 md:grid-cols-2">
              {componentLabels.map(([key, label]) => (
                <ComponentBar key={key} label={label} value={selectedState?.[key]} />
              ))}
            </div>
            <div className="mt-5 grid gap-3 sm:grid-cols-2">
              <MiniStat label="Manipulation Score" value={percent(selectedState?.manipulationScore)} />
              <MiniStat label="Pump / Dump Score" value={percent(selectedState?.pumpDumpScore)} />
            </div>
          </Panel>

          <Panel title="结构信号">
            <TagList empty="暂无控盘结构信号" items={selectedState?.signals || []} />
          </Panel>

          <Panel title="底层指标">
            {metricRows.length > 0 ? (
              <div className="grid gap-2 md:grid-cols-2">
                {metricRows.map(([key, value]) => (
                  <div
                    className="flex items-center justify-between rounded-xl border border-slate-700/70 bg-slate-950/45 px-3 py-2 text-sm"
                    key={key}
                  >
                    <span className="text-slate-400">{metricLabel(key)}</span>
                    <span className="font-bold text-slate-100">{formatMetric(value)}</span>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-slate-500">暂无可展示指标。</p>
            )}
          </Panel>
        </div>

        <aside className="space-y-5">
          <Panel title="风险标签">
            <TagList empty="暂无风险标签" items={selectedState?.riskTags || []} tone="risk" />
          </Panel>
          <Panel title="模型边界">
            <div className="space-y-3 text-sm leading-6 text-slate-300">
              <p>该页面只用于山寨 / 新币 / 低流动性合约控盘观察。</p>
              <p>BTC 已隔离到 BTC Structure Engine，不使用 fake breakout 或控盘评分作为主逻辑。</p>
              <p className="rounded-xl border border-cyan-400/25 bg-cyan-500/10 px-3 py-2 text-cyan-100">
                只读输出，不推送 Discord，不下单，不修改运行时状态。
              </p>
            </div>
          </Panel>
        </aside>
      </div>
    </section>
  );
}

function SymbolCard({ error, onRemove, onSelect, selected, state, symbol }) {
  return (
    <article
      className={[
        "rounded-xl border bg-slate-950/45 p-4 transition",
        selected ? "border-cyan-400/70 shadow-glow" : "border-slate-700/70 hover:border-cyan-400/45",
      ].join(" ")}
    >
      <button className="w-full text-left" onClick={onSelect} type="button">
        <div className="flex items-start justify-between gap-3">
          <div>
            <p className="text-lg font-black text-white">{state?.symbol || symbol}</p>
            <p className="mt-1 text-xs text-slate-500">requested {symbol}</p>
          </div>
          <span
            className={[
              "rounded-full border px-2.5 py-1 text-xs font-black",
              regimeTone[state?.regime] || "border-slate-700 bg-slate-900 text-slate-200",
            ].join(" ")}
          >
            {state?.regime || "-"}
          </span>
        </div>
        <div className="mt-4 grid grid-cols-3 gap-2 text-xs">
          <MiniInline label="Score" value={percent(state?.manipulationScore)} />
          <MiniInline label="Bias" value={state?.bias || "NEUTRAL"} />
          <MiniInline label="Conf" value={percent(state?.confidence)} />
        </div>
        {error ? (
          <p className="mt-3 rounded-lg border border-yellow-400/30 bg-yellow-400/10 px-3 py-2 text-xs text-yellow-100">
            降级数据：{error}
          </p>
        ) : null}
      </button>
      <button
        className="mt-4 w-full rounded-lg border border-red-400/30 bg-red-500/10 px-3 py-2 text-xs font-black text-red-100 transition hover:bg-red-500/15"
        onClick={onRemove}
        type="button"
      >
        停止展示
      </button>
    </article>
  );
}

function MiniInline({ label, value }) {
  return (
    <div className="rounded-lg border border-slate-800 bg-slate-900/80 px-2 py-1.5">
      <p className="text-[10px] uppercase tracking-[0.12em] text-slate-500">{label}</p>
      <p className="mt-0.5 truncate font-black text-cyan-100">{value}</p>
    </div>
  );
}

function ScoreCard({ label, value, tone = "border-slate-700/70 bg-slate-900/80 text-slate-100" }) {
  return (
    <div className={`rounded-2xl border p-4 shadow-glow ${tone}`}>
      <p className="text-xs uppercase tracking-[0.22em] opacity-70">{label}</p>
      <p className="mt-2 text-xl font-black">{value}</p>
    </div>
  );
}

function Panel({ title, children }) {
  return (
    <section className="rounded-2xl border border-slate-700/70 bg-slate-900/80 p-5 shadow-glow">
      <h4 className="text-sm font-black uppercase tracking-[0.22em] text-cyan-300">{title}</h4>
      <div className="mt-4">{children}</div>
    </section>
  );
}

function ComponentBar({ label, value }) {
  const pct = clamp01(value) * 100;
  return (
    <div className="rounded-xl border border-slate-700/70 bg-slate-950/45 p-4">
      <div className="flex items-center justify-between gap-3">
        <span className="text-sm font-bold text-slate-200">{label}</span>
        <span className="text-sm font-black text-cyan-100">{percent(value)}</span>
      </div>
      <div className="mt-3 h-2 overflow-hidden rounded-full bg-slate-800">
        <div
          className="h-full rounded-full bg-gradient-to-r from-cyan-400 via-yellow-300 to-red-400"
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

function MiniStat({ label, value }) {
  return (
    <div className="rounded-xl border border-slate-700/70 bg-slate-950/45 px-4 py-3">
      <p className="text-xs uppercase tracking-[0.18em] text-slate-500">{label}</p>
      <p className="mt-1 text-lg font-black text-white">{value}</p>
    </div>
  );
}

function TagList({ items, empty, tone = "signal" }) {
  if (!items.length) {
    return <p className="text-sm text-slate-500">{empty}</p>;
  }
  return (
    <div className="flex flex-wrap gap-2">
      {items.map((item) => (
        <span
          className={[
            "rounded-full border px-3 py-1 text-xs font-black",
            tone === "risk"
              ? "border-orange-400/40 bg-orange-500/10 text-orange-100"
              : "border-cyan-400/35 bg-cyan-500/10 text-cyan-100",
          ].join(" ")}
          key={item}
        >
          {item}
        </span>
      ))}
    </div>
  );
}

function loadInitialSymbols() {
  try {
    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY) || "[]");
    const symbols = Array.isArray(stored) ? uniqueSymbols(stored.map(normalizeSymbolInput)) : [];
    return symbols.length ? symbols.slice(0, MAX_SYMBOLS) : DEFAULT_SYMBOLS;
  } catch {
    return DEFAULT_SYMBOLS;
  }
}

function parseSymbolInput(value) {
  return uniqueSymbols(String(value || "").split(/[\s,，;；]+/).map(normalizeSymbolInput)).slice(0, MAX_SYMBOLS);
}

function uniqueSymbols(values) {
  const seen = new Set();
  const symbols = [];
  for (const value of values) {
    if (!value || seen.has(value)) continue;
    seen.add(value);
    symbols.push(value);
  }
  return symbols;
}

function normalizeSymbolInput(value) {
  const compact = String(value || "")
    .trim()
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "");
  if (!compact) {
    return "";
  }
  return compact.endsWith("USDT") ? compact : `${compact}USDT`;
}

function metricLabel(key) {
  return String(key)
    .replace(/_/g, " ")
    .replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function formatMetric(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return "-";
  }
  if (Math.abs(number) < 0.001 && number !== 0) {
    return number.toExponential(2);
  }
  return number.toLocaleString(undefined, { maximumFractionDigits: 4 });
}

function percent(value) {
  return `${Math.round(clamp01(value) * 100)}%`;
}

function scoreValue(value) {
  return clamp01(value);
}

function clamp01(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return 0;
  }
  return Math.min(1, Math.max(0, number));
}
