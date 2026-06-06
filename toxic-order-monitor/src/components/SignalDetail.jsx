import { finalResultDescription } from "../utils/signalResult.js";
import CandidateExplanation from "./CandidateExplanation.jsx";
import TofMetricsPanel from "./TofMetricsPanel.jsx";

export default function SignalDetail({ signal }) {
  if (!signal) {
    return (
      <section className="rounded-2xl border border-slate-700/60 bg-slate-900/80 p-5 text-slate-400">
        暂无选中信号
      </section>
    );
  }

  const finalResult = finalResultDescription(signal);
  const preview = [
    "疑似异常候选（基于盘口风险信号）",
    "",
    `交易所：${signal.exchange}`,
    `交易对：${signal.symbol}`,
    `信号类型：${signal.type}`,
    `方向：${signal.side || "N/A"}`,
    `最终结果：${finalResult}`,
    `时间：${signal.time}`,
  ].join("\n");

  return (
    <section className="rounded-2xl border border-slate-700/60 bg-slate-900/80 p-5 shadow-glow">
      <div className="mb-4 flex items-start justify-between gap-4">
        <div>
          <p className="text-xs uppercase tracking-[0.28em] text-cyan-300">Signal Result</p>
          <h3 className="mt-2 text-xl font-bold text-white">{signal.exchange} / {signal.symbol}</h3>
          <p className="mt-1 text-sm text-slate-400">{signal.type} · {signal.time}</p>
        </div>
        <span className="rounded-full border border-slate-600/70 px-3 py-1 text-xs font-bold text-slate-200">
          {signal.level}
        </span>
      </div>

      <div className="rounded-xl border border-slate-700/60 bg-slate-950/40 p-4">
        <h4 className="mb-2 font-semibold text-white">最终结果描述</h4>
        <p className="text-base font-semibold leading-7 text-slate-100">{finalResult}</p>
      </div>

      <div className="mt-5 space-y-4">
        <CandidateExplanation signal={signal} />
        <TofMetricsPanel metrics={signal.tofMetrics} />
      </div>

      <div className="mt-5 rounded-xl border border-slate-700/60 bg-slate-950/40 p-4">
        <h4 className="mb-2 font-semibold text-white">推送内容预览</h4>
        <pre className="whitespace-pre-wrap text-xs leading-5 text-slate-300">{preview}</pre>
      </div>
    </section>
  );
}
