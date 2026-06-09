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
            label="双评分系统"
            value="短线有毒订单评分 toxicScore 使用 1s/5s/15s/60s；现货 spotScore 按 CVD、成交量异常、吸收、盘口变化、价格响应五项计算；合约 contractScore 按 CWM 主动流、OI 冲击、清算环境、funding 拥挤、basis、enabled 交易所确认六项计算。"
          />
          <RuleItem
            label="跨市场确认"
            value="crossConfirmScore = 0.40*现货合约方向一致 + 0.25*多窗口一致 + 0.20*价格响应一致 + 0.15*数据源覆盖；SourceCoverage 只按 enabled source 算。"
          />
          <RuleItem
            label="主力确认"
            value="mainForceConfirmed 需要 mainForceScore>=75、confidence>=70、dataQuality>=70，且 7 个确认条件里至少命中 3 个；前端会显示已确认/待确认和命中数。"
          />
          <RuleItem
            label="极端行情与结构分类"
            value="extremeImpactConfirmed 只表示冲击很剧烈，不等于主力确认。regimeType 会把结构归类成主力建多、主力建空、现货吸筹、现货派发、空头挤压、多头踩踏、下方吸收、上方压制或高换手震荡。"
          />
          <RuleItem
            label="方向与置信度"
            value="structureBias 单独表示方向，范围 -100 到 +100；mainForceScore 表示主力结构强度。confidence 和 dataQuality 也分开：dataQuality 看数据健康，confidence 还要看 sourceCoverage、多窗口一致性和 signalAgreement。"
          />
          <RuleItem
            label="交易所确认"
            value="当前合约监控按 enabled 交易所计算；OKX 关闭时不参与总量、质量扣分或多平台确认。Binance 是主流动性源，Bitfinex 是确认源；只有 Binance + Bitfinex 同向确认时才给 S 级空间。"
          />
          <RuleItem
            label="短线衰减"
            value="短线 toxicScore 使用 Calm/Watch/High/Critical/S 等级，halfLifeSec 通常 30-45 秒，max TTL 约 3-5 分钟；没有持续异常时 decayedScore 会快速下降。"
          />
          <RuleItem
            label="推送边界"
            value="短线有毒订单 Discord gate 按 High/Critical/S、toxicScore >= 85、confidence >= 70、dataQuality >= 70、cooldown >= 60s；文案明确提示不代表中长线趋势。CWM 大行情提醒保留独立 gate、独立冷却和 dry-run 观察。"
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
