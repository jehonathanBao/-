export default function RuleStatus({ discordConnected, lastPushedAt, onTestPush, testPending = false, wsStatus = "idle" }) {
  return (
    <section className="mb-5 rounded-2xl border border-slate-700/60 bg-slate-900/80 p-5 shadow-glow">
      <div className="flex flex-col gap-4 xl:flex-row xl:items-center xl:justify-between">
        <div>
          <h3 className="font-bold text-white">Discord 推送状态</h3>
          <p className="mt-1 text-sm text-slate-400">高风险候选信号可手动推送到 #toxic-order-alerts</p>
        </div>
        <div className="grid gap-3 text-sm md:grid-cols-5">
          <Status label="推送状态" value={discordConnected ? "已连接" : "未配置"} ok={discordConnected} />
          <Status label="Webhook" value={discordConnected ? "后端已配置" : "未配置"} ok={discordConnected} />
          <Status label="频道" value="#toxic-order-alerts" ok />
          <Status label="Live" value={liveStatusLabel(wsStatus)} ok={wsStatus === "open"} />
          <Status label="最后推送" value={lastPushedAt || "暂无"} ok={Boolean(lastPushedAt)} />
        </div>
        <button
          aria-label="测试 Discord 推送"
          className="rounded-xl border border-emerald-400/40 bg-emerald-400/10 px-4 py-2 text-sm font-semibold text-emerald-200 hover:bg-emerald-400/20 disabled:cursor-not-allowed disabled:opacity-50"
          disabled={testPending}
          onClick={onTestPush}
          type="button"
        >
          {testPending ? "测试中" : "测试推送"}
        </button>
      </div>
      <div className="mt-5 border-t border-slate-800 pt-4">
        <div className="flex flex-col gap-1">
          <h4 className="text-sm font-bold text-white">当前有毒订单判断逻辑</h4>
          <p className="text-xs text-slate-400">
            Candidate only，系统只做盘口/成交异常提醒，不执行下单、拦截、封禁或资金操作。
          </p>
        </div>
        <div className="mt-3 grid gap-3 text-xs text-slate-300 lg:grid-cols-4">
          <RuleItem
            label="候选生成"
            value="L2 撤单/挂单、trade imbalance、depth withdrawal、spread widening、VPIN-lite 触发现货 TOF 候选。"
          />
          <RuleItem
            label="合约增强"
            value="OI、Funding、Liquidation pressure、Aggressive order flow 与现货候选按 symbol 合并。"
          />
          <RuleItem
            label="风险评分"
            value="finalRiskScore = 0.4 * 现货风险 + 0.3 * TOF-lite + 0.3 * 合约指标；dataQuality 看指标完整性和新鲜度。"
          />
          <RuleItem
            label="推送边界"
            value="S/Critical/High 且 score >= 80、dataQuality >= 70 才进入 Discord gate；Medium 进入信号历史，Low 仅主动筛选展示。"
          />
        </div>
      </div>
    </section>
  );
}

function liveStatusLabel(status) {
  if (status === "open") return "connected";
  if (status === "reconnecting") return "reconnecting";
  if (status === "connecting") return "connecting";
  if (status === "closed") return "disconnected";
  return "idle";
}

function Status({ label, value, ok }) {
  return (
    <div className="rounded-xl border border-slate-700/60 bg-slate-950/40 px-4 py-3">
      <p className="text-xs text-slate-500">{label}</p>
      <p className={ok ? "mt-1 font-semibold text-emerald-300" : "mt-1 font-semibold text-slate-400"}>{value}</p>
    </div>
  );
}

function RuleItem({ label, value }) {
  return (
    <div className="border-l border-cyan-400/30 pl-3">
      <p className="font-semibold text-cyan-200">{label}</p>
      <p className="mt-1 leading-5 text-slate-400">{value}</p>
    </div>
  );
}
