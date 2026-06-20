import { useEffect, useMemo, useState } from "react";
import { fetchAltcoinSignals } from "../api/liquidationCascade.js";

const DEFAULT_SYMBOL = "ASTERUSDT";

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
  const [inputSymbol, setInputSymbol] = useState(DEFAULT_SYMBOL);
  const [symbol, setSymbol] = useState(DEFAULT_SYMBOL);
  const [state, setState] = useState(null);
  const [error, setError] = useState(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    fetchAltcoinSignals(symbol)
      .then((result) => {
        if (cancelled) return;
        setState(result.data);
        setError(result.error);
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [symbol]);

  const metricRows = useMemo(() => {
    const metrics = state?.metrics || {};
    return Object.entries(metrics)
      .filter(([, value]) => Number.isFinite(Number(value)))
      .sort(([left], [right]) => left.localeCompare(right));
  }, [state]);

  function handleSubmit(event) {
    event.preventDefault();
    const next = normalizeSymbolInput(inputSymbol);
    if (next) {
      setInputSymbol(next);
      setSymbol(next);
    }
  }

  return (
    <section className="space-y-5">
      <div className="rounded-2xl border border-slate-700/70 bg-slate-900/80 p-5 shadow-glow">
        <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
          <div>
            <p className="text-xs uppercase tracking-[0.3em] text-cyan-300">Altcoin Manipulation Engine</p>
            <h3 className="mt-2 text-2xl font-black text-white">妖币控盘监控</h3>
            <p className="mt-2 max-w-4xl text-sm leading-6 text-slate-400">
              直接读取 /api/altcoin/signals，按 OI、成交、Funding、价格结构拆解山寨合约控盘风险；BTC 不进入该模型。
            </p>
          </div>
          <form className="flex w-full max-w-xl flex-col gap-2 sm:flex-row" onSubmit={handleSubmit}>
            <label className="sr-only" htmlFor="altcoin-symbol">
              山寨合约 symbol
            </label>
            <input
              className="min-h-11 flex-1 rounded-xl border border-slate-700 bg-slate-950 px-4 py-2 text-sm font-semibold text-cyan-100 outline-none transition placeholder:text-slate-600 focus:border-cyan-400"
              id="altcoin-symbol"
              onChange={(event) => setInputSymbol(event.target.value)}
              placeholder="例如 ASTERUSDT"
              value={inputSymbol}
            />
            <button
              className="min-h-11 rounded-xl border border-cyan-400/40 bg-cyan-500/15 px-5 text-sm font-black text-cyan-100 transition hover:bg-cyan-400/20 disabled:cursor-not-allowed disabled:opacity-60"
              disabled={loading}
              type="submit"
            >
              {loading ? "读取中" : "刷新"}
            </button>
          </form>
        </div>
        {error ? (
          <div className="mt-4 rounded-xl border border-yellow-400/35 bg-yellow-400/10 px-4 py-3 text-sm text-yellow-100">
            API 返回降级数据：{error}
          </div>
        ) : null}
      </div>

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <ScoreCard label="Symbol" value={state?.symbol || symbol} />
        <ScoreCard
          label="Regime"
          tone={regimeTone[state?.regime] || "border-slate-700 bg-slate-900 text-slate-100"}
          value={state?.regime || "-"}
        />
        <ScoreCard label="Bias" value={state?.bias || "NEUTRAL"} />
        <ScoreCard label="Confidence" value={percent(state?.confidence)} />
      </div>

      <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <div className="space-y-5">
          <Panel title="四组件控盘评分">
            <div className="grid gap-4 md:grid-cols-2">
              {componentLabels.map(([key, label]) => (
                <ComponentBar key={key} label={label} value={state?.[key]} />
              ))}
            </div>
            <div className="mt-5 grid gap-3 sm:grid-cols-2">
              <MiniStat label="Manipulation Score" value={percent(state?.manipulationScore)} />
              <MiniStat label="Pump / Dump Score" value={percent(state?.pumpDumpScore)} />
            </div>
          </Panel>

          <Panel title="结构信号">
            <TagList empty="暂无控盘结构信号" items={state?.signals || []} />
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
            <TagList empty="暂无风险标签" items={state?.riskTags || []} tone="risk" />
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

function clamp01(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) {
    return 0;
  }
  return Math.min(1, Math.max(0, number));
}
