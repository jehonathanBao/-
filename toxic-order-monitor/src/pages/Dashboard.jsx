import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useLocation } from "react-router-dom";
import { evaluateDiscordAlertGate } from "../api/alertGate.js";
import { pushDiscordAlert, sendDiscordTestMessage } from "../api/discord.js";
import { fetchSignalsSnapshot, mapInboxItemToSignal, runtimeFromPayload } from "../api/signals.js";
import BinanceAltContractMonitor from "../components/BinanceAltContractMonitor.jsx";
import ContractWhaleMonitor from "../components/ContractWhaleMonitor.jsx";
import Header from "../components/Header.jsx";
import LiquidationCascadeDashboard from "../components/LiquidationCascadeDashboard.jsx";
import NewTokenWatch from "../components/NewTokenWatch.jsx";
import PushLog from "../components/PushLog.jsx";
import RiskCard from "../components/RiskCard.jsx";
import RiskCharts from "../components/RiskCharts.jsx";
import RiskSystemSummaryCards from "../components/RiskSystemSummaryCards.jsx";
import RuleStatus from "../components/RuleStatus.jsx";
import ScanLogPanel from "../components/ScanLogPanel.jsx";
import Sidebar from "../components/Sidebar.jsx";
import SignalDetail from "../components/SignalDetail.jsx";
import SignalTable from "../components/SignalTable.jsx";
import SpotWhaleMonitor from "../components/SpotWhaleMonitor.jsx";
import UsageGuide from "../components/UsageGuide.jsx";
import WorkspacePageHeader from "../components/WorkspacePageHeader.jsx";
import { useReconnectingWebSocket } from "../hooks/useReconnectingWebSocket.js";
import { useSignalsStore } from "../store/signalsStore.js";

const CLEAR_CACHE_CONFIRM =
  "确认清除当前页面缓存的有毒订单候选信号？操作仅清空前端页面展示，不会删除后端数据。";
const DISCORD_PUSH_CONFIRM =
  "确认推送该高风险候选信号到 Discord？\n该操作会真实发送到告警频道。";

export default function Dashboard() {
  const location = useLocation();
  const viewMode = viewModeFromPath(location.pathname);
  const mainstreamSymbol = mainstreamSymbolFromPath(location.pathname);
  const isContractWhaleView = viewMode === "contract-whale";
  const isLiquidationCascadeView = viewMode === "liquidation-cascade";
  const isSpotWhaleView = viewMode === "spot-whale";
  const isAltContractView = viewMode === "alt-contract-monitor";
  const isNewTokenWatchView = viewMode === "new-token-watch";
  const isUsageGuideView = viewMode === "usage-guide";
  const signalWsEnabled = !(
    isContractWhaleView ||
    isLiquidationCascadeView ||
    isSpotWhaleView ||
    isAltContractView ||
    isNewTokenWatchView ||
    isUsageGuideView
  );
  const {
    rawInboxSignals,
    selectedSignal,
    activeRiskFilter,
    pushLogs,
    pushStatus,
    discordConnected,
    lastPushedAt,
    signalsRequest,
    runtimeBoundary,
    applySignalsSnapshot,
    setSignals,
    setRuntimeBoundary,
    setSelectedSignal,
    setRiskFilter,
    markAsPushed,
    setSignalReviewStatus,
    addPushLog,
    setPushStatus,
    clearSignalInbox,
  } = useSignalsStore();
  const [pushNotice, setPushNotice] = useState(null);
  const [mediumExpanded, setMediumExpanded] = useState(false);
  const [pendingPushIds, setPendingPushIds] = useState(() => new Set());
  const pendingPushIdsRef = useRef(new Set());
  const signalWsRuntimeUnavailableRef = useRef(false);
  const [testPushPending, setTestPushPending] = useState(false);

  useEffect(() => {
    fetchSignalsSnapshot().then((snapshot) => {
      if (!signalWsEnabled) {
        setRuntimeBoundary(snapshot.runtime);
        return;
      }
      applySignalsSnapshot(snapshot);
      if (signalWsRuntimeUnavailableRef.current) {
        setRuntimeBoundary(runtimeFromPayload(null));
      }
      const state = useSignalsStore.getState();
      if (!state.selectedSignal && state.rawInboxSignals.length > 0) {
        const firstHighRisk =
          state.rawInboxSignals.find((signal) => signal.risk === "high") ?? state.rawInboxSignals[0];
        setSelectedSignal(firstHighRisk);
      }
    });
  }, [applySignalsSnapshot, setRuntimeBoundary, setSelectedSignal, signalWsEnabled]);

  const handleSignalWsMessage = useCallback(
    (event) => {
      try {
        const payload = JSON.parse(event.data);
        const frameRuntime = runtimeFromPayload(payload);
        signalWsRuntimeUnavailableRef.current = frameRuntime.phase !== "confirmed";
        setRuntimeBoundary(frameRuntime);
        const hasSignalSnapshot = Array.isArray(payload?.signals) || Array.isArray(payload?.items);
        if (hasSignalSnapshot) {
          const items = Array.isArray(payload?.signals) ? payload.signals : payload.items;
          setSignals(items.map((item) => ({ ...mapInboxItemToSignal(item), runtimeBoundary: frameRuntime })));
        }
      } catch {
        signalWsRuntimeUnavailableRef.current = true;
        setRuntimeBoundary(runtimeFromPayload(null));
      }
    },
    [setRuntimeBoundary, setSignals],
  );

  const { status: wsStatus } = useReconnectingWebSocket("/ws/signals", {
    enabled: signalWsEnabled,
    retryMs: 1000,
    maxRetryMs: 15000,
    onMessage: handleSignalWsMessage,
  });

  useEffect(() => {
    if (!signalWsEnabled || (wsStatus !== "reconnecting" && wsStatus !== "closed")) {
      return;
    }
    signalWsRuntimeUnavailableRef.current = true;
    setRuntimeBoundary(runtimeFromPayload(null));
  }, [setRuntimeBoundary, signalWsEnabled, wsStatus]);

  const stats = useMemo(() => {
    const base = { high: 0, medium: 0, low: 0, all: rawInboxSignals.length, total: rawInboxSignals.length };
    rawInboxSignals.forEach((signal) => {
      if (base[signal.risk] !== undefined) {
        base[signal.risk] += 1;
      }
    });
    return base;
  }, [rawInboxSignals]);

  const highRiskSignals = useMemo(
    () => rawInboxSignals.filter(isHighRiskSignal).sort(byRiskThenTimeDesc),
    [rawInboxSignals],
  );

  const sLevelSignals = useMemo(
    () => rawInboxSignals.filter(isSLevelSignal).sort(byRiskThenTimeDesc),
    [rawInboxSignals],
  );

  const mediumRiskSignals = useMemo(
    () => rawInboxSignals.filter(isMediumRiskSignal).sort(byRiskThenTimeDesc),
    [rawInboxSignals],
  );

  const primarySignalView = useMemo(() => {
    if (viewMode === "signals") {
      return {
        description: "左侧“异常信号”只显示 S/Critical 候选，优先处理最严重的盘口异常。",
        emptyHint: "新的 S/Critical 候选出现后会自动追加到这里。",
        emptyMessage: "暂无 S 级异常信号",
        signals: sLevelSignals,
        title: "S 级异常信号",
      };
    }
    if (viewMode === "history") {
      return {
        description: "中风险异常归档在信号历史；只用于观察和复盘，不触发 Discord 自动推送。",
        emptyHint: "新的中风险候选出现后会保留在信号历史。",
        emptyMessage: "暂无中风险历史信号",
        signals: mediumRiskSignals,
        title: "信号历史 · 中风险异常",
      };
    }
    if (activeRiskFilter === "low") {
      return {
        description: "低风险候选只在用户主动筛选时展示，仍保留在持久 Inbox。",
        emptyHint: "新的低风险候选出现后会继续追加。",
        emptyMessage: "暂无低风险候选信号",
        signals: rawInboxSignals.filter((signal) => signal.risk === "low").sort(byRiskThenTimeDesc),
        title: "Low Risk Candidates",
      };
    }
    return {
      description: "默认展示高风险候选信号；刷新页面只重新订阅数据，不会清空历史卡片。",
      emptyHint: "新的候选信号出现后会继续追加",
      emptyMessage: "暂无缓存的有毒订单候选信号",
      signals: highRiskSignals,
      title: "High / Critical Risk Candidates",
    };
  }, [activeRiskFilter, highRiskSignals, mediumRiskSignals, rawInboxSignals, sLevelSignals, viewMode]);
  const showMediumFoldout = viewMode !== "signals" && viewMode !== "history";

  const highUnhandledCount = countUnhandledHighRisk(rawInboxSignals);
  const focusedScoreSignal = selectedSignal ?? highRiskSignals[0] ?? rawInboxSignals[0] ?? null;
  const effectivePushStatus = useMemo(
    () => buildPushStatus(pushStatus, pendingPushIds),
    [pendingPushIds, pushStatus],
  );

  function handleClearCache() {
    if (!window.confirm(CLEAR_CACHE_CONFIRM)) {
      return;
    }
    clearSignalInbox();
    setRiskFilter("high");
    setMediumExpanded(false);
    setPushNotice({
      type: "success",
      message: "已清除当前页面缓存的候选信号，后端数据未受影响。",
    });
  }

  async function handlePush(signal) {
    if (!signal) {
      return;
    }
    if (pendingPushIdsRef.current.has(signal.id)) {
      return;
    }
    const gate = evaluateDiscordAlertGate(signal);
    if (!gate.ok) {
      setPushStatus(signal.id, "failed", gate.reason);
      addPushLog(signal, "failed", gate.reason);
      setPushNotice({
        type: "failed",
        message:
          gate.reason === "DISCORD_SUPPRESSED_NON_HIGH_RISK"
            ? "Medium 风险候选仅在折叠列表展示，不触发 Discord 推送。"
            : gate.reason === "DISCORD_SUPPRESSED_LOW_CONFIDENCE"
              ? "该短线有毒订单置信度低于 70，仅在 Dashboard 展示。"
            : "该候选信号未达到 Discord 推送门槛，仅在 Dashboard 展示。",
      });
      return;
    }
    if (!window.confirm(DISCORD_PUSH_CONFIRM)) {
      return;
    }
    setPushNotice({ type: "pending", message: "Discord 推送中..." });
    setPushStatus(signal.id, "pending");
    addPushLog(signal, "pending");
    pendingPushIdsRef.current.add(signal.id);
    setPendingPushIds(new Set(pendingPushIdsRef.current));
    try {
      const result = await pushDiscordAlert(signal);
      if (result.ok) {
        markAsPushed(signal.id);
        setPushStatus(signal.id, "success");
        addPushLog(signal, "success");
        setPushNotice({ type: "success", message: "Discord 推送成功" });
        return;
      }
      const reason = result.reason || "DISCORD_PUSH_FAILED";
      setPushStatus(signal.id, "failed", reason);
      addPushLog(signal, "failed", reason);
      setPushNotice({
        type: "failed",
        message:
          reason === "DISCORD_NOT_CONFIGURED"
            ? "Discord 未配置，推送未发送。"
            : reason === "ALERT_GATE_REJECTED"
              ? "该候选信号未达到 Discord 推送门槛，仅在 Dashboard 展示。"
              : `Discord 推送失败：${reason}`,
      });
    } catch (error) {
      const reason = error?.response?.data?.reason || error?.message || "NETWORK_ERROR";
      setPushStatus(signal.id, "failed", reason);
      addPushLog(signal, "failed", reason);
      setPushNotice({ type: "failed", message: `Discord 推送失败：${reason}` });
    } finally {
      pendingPushIdsRef.current.delete(signal.id);
      setPendingPushIds(new Set(pendingPushIdsRef.current));
    }
  }

  async function handleTestPush() {
    if (testPushPending) {
      return;
    }
    setTestPushPending(true);
    setPushNotice({ type: "pending", message: "Discord 候选预览校验中..." });
    try {
      const result = await sendDiscordTestMessage(focusedScoreSignal);
      if (result.ok) {
        setPushNotice({ type: "success", message: "Discord 候选预览已通过；未发送 Webhook。" });
        return;
      }
      setPushNotice({
        type: "failed",
        message:
          result.reason === "DISCORD_NOT_CONFIGURED"
            ? "Discord 未配置，候选预览未执行。"
            : `Discord 候选预览失败：${discordFailureHint(result.reason || "DISCORD_PREVIEW_FAILED")}`,
      });
    } catch (error) {
      const reason = discordFailureHint(error?.response?.data?.reason || error?.message || "NETWORK_ERROR", error);
      setPushNotice({ type: "failed", message: `Discord 候选预览失败：${reason}` });
    } finally {
      setTestPushPending(false);
    }
  }

  return (
    <div
      className={`workspace-shell flex min-h-screen flex-col lg:flex-row ${isContractWhaleView ? "contract-workspace-shell" : ""}`}
      data-testid="workspace-shell"
    >
      <Sidebar runtimeBoundary={runtimeBoundary} />
      <main
        className={[
          "workspace-main w-full min-w-0 flex-1",
          `workspace-route-${viewMode}`,
          isContractWhaleView ? "contract-workspace-main" : "",
        ].filter(Boolean).join(" ")}
        data-testid="workspace-main"
      >
        {isContractWhaleView ? (
          <ContractWhalePage symbol={mainstreamSymbol} />
        ) : (
          <>
            <Header discordConnected={discordConnected} highUnhandledCount={highUnhandledCount} runtimeBoundary={runtimeBoundary} />
            <div className="workspace-content">
              {isLiquidationCascadeView ? (
                <LiquidationCascadePage />
              ) : isSpotWhaleView ? (
                <SpotWhalePage symbol={mainstreamSymbol} />
              ) : isAltContractView ? (
                <AltContractPage />
              ) : isNewTokenWatchView ? (
                <NewTokenWatchPage />
              ) : isUsageGuideView ? (
                <UsageGuidePage />
              ) : (
                <>
                  {signalsRequest.phase === "error" ? (
                    <div className="mb-5 rounded-xl border border-amber-400/40 bg-amber-400/10 px-4 py-3 text-sm text-amber-100" role="status">
                      信号快照刷新失败（{signalsRequest.errorCode || "UNKNOWN"}）；已保留此前候选，运行边界视为未知，推送已关闭。
                    </div>
                  ) : null}
                  <RuleStatus
                    discordConnected={discordConnected}
                    lastPushedAt={lastPushedAt}
                    onTestPush={handleTestPush}
                    testPending={testPushPending}
                    wsStatus={wsStatus}
                  />
                  {pushNotice ? (
                    <div
                      className={[
                        "mb-5 rounded-xl border px-4 py-3 text-sm",
                        pushNotice.type === "success"
                          ? "border-emerald-400/40 bg-emerald-400/10 text-emerald-200"
                          : pushNotice.type === "pending"
                            ? "border-yellow-400/40 bg-yellow-400/10 text-yellow-200"
                            : "border-red-400/40 bg-red-400/10 text-red-200",
                      ].join(" ")}
                      role="status"
                    >
                      {pushNotice.message}
                    </div>
                  ) : null}

                  {viewMode === "dashboard" ? <ContractWhaleMonitor /> : null}
                  {viewMode === "dashboard" ? <RiskSystemSummaryCards signal={focusedScoreSignal} /> : null}

                  <section className="mb-5 grid gap-4 md:grid-cols-2 xl:grid-cols-4">
                    <RiskCard active={activeRiskFilter === "high"} count={stats.high} onClick={() => setRiskFilter("high")} percentage={ratio(stats.high, stats.all)} risk="high" />
                    <RiskCard active={activeRiskFilter === "medium"} count={stats.medium} onClick={() => { setRiskFilter("medium"); setMediumExpanded(true); }} percentage={ratio(stats.medium, stats.all)} risk="medium" />
                    <RiskCard active={activeRiskFilter === "low"} count={stats.low} onClick={() => setRiskFilter("low")} percentage={ratio(stats.low, stats.all)} risk="low" />
                    <RiskCard active={activeRiskFilter === "all"} count={stats.all} onClick={() => setRiskFilter("all")} percentage={100} risk="all" />
                  </section>

                  <div className="mb-5 flex flex-wrap items-center justify-between gap-3">
                    <div className="flex flex-wrap items-center gap-2">
                      <button className="rounded-lg border border-slate-700/60 px-3 py-2 text-sm text-slate-300 hover:border-cyan-400 hover:text-cyan-200" onClick={() => setRiskFilter("all")} type="button">
                        全部
                      </button>
                      <span className="text-sm text-slate-500">当前筛选：{filterLabel(activeRiskFilter, viewMode)}</span>
                    </div>
                    <button
                      className="rounded-lg border border-red-400/50 bg-red-500/10 px-4 py-2 text-sm font-semibold text-red-200 hover:bg-red-500/20"
                      onClick={handleClearCache}
                      type="button"
                    >
                      清除缓存
                    </button>
                  </div>

                  <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_380px]">
                    <div className="space-y-5">
                      <SignalTable
                        inboxStats={stats}
                        title={primarySignalView.title}
                        description={primarySignalView.description}
                        emptyHint={primarySignalView.emptyHint}
                        emptyMessage={primarySignalView.emptyMessage}
                        onPush={handlePush}
                        onMarkStatus={setSignalReviewStatus}
                        onSelect={setSelectedSignal}
                        pushStatus={effectivePushStatus}
                        selectedSignal={selectedSignal}
                        signals={primarySignalView.signals}
                      />
                      {showMediumFoldout ? (
                        <MediumRiskSection
                          expanded={mediumExpanded}
                          inboxStats={stats}
                          onPush={handlePush}
                          onMarkStatus={setSignalReviewStatus}
                          onSelect={setSelectedSignal}
                          pushStatus={effectivePushStatus}
                          onToggle={() => setMediumExpanded((value) => !value)}
                          selectedSignal={selectedSignal}
                          signals={mediumRiskSignals}
                        />
                      ) : null}
                      <SignalDetail signal={selectedSignal} />
                      <ScanLogPanel />
                    </div>
                    <div className="space-y-5">
                      <RiskCharts signals={rawInboxSignals} />
                      <PushLog logs={pushLogs} />
                    </div>
                  </div>
                </>
              )}
            </div>
          </>
        )}
      </main>
    </div>
  );
}

export function countUnhandledHighRisk(signals) {
  return (Array.isArray(signals) ? signals : []).filter((signal) => {
    const reviewed = signal?.reviewStatus === "acknowledged" || signal?.reviewStatus === "false_positive";
    return signal?.risk === "high" && signal?.status === "unhandled" && signal?.isLive !== false && !reviewed;
  }).length;
}

function buildPushStatus(pushStatus, pendingPushIds) {
  const next = { ...pushStatus };
  for (const signalId of pendingPushIds) {
    next[signalId] = { signalId, status: "pending" };
  }
  return next;
}

function discordFailureHint(reason, error = null) {
  const status = error?.response?.status;
  const text = String(reason || "");
  if (status === 403 || /status code 403|forbidden/i.test(text)) {
    return "Discord Webhook 被拒绝(403)：请检查 Webhook URL 是否仍有效、频道权限是否允许发送，以及后端 .env 是否使用正确的 webhook。";
  }
  if (status === 404 || /status code 404|not found/i.test(text)) {
    return "Discord Webhook 不存在(404)：请重新生成 webhook 并只写入后端 .env。";
  }
  if (status === 429 || /status code 429|rate limit/i.test(text)) {
    return "Discord rate limit(429)：请稍后重试，自动推送会继续遵守冷却。";
  }
  return text.replace(/https:\/\/discord\.com\/api\/webhooks\/[^\s]+/gi, "[redacted-discord-webhook]");
}

function ContractWhalePage({ symbol }) {
  const asset = normalizeMainstreamSymbol(symbol);
  return <ContractWhaleMonitor lockedSymbol={asset} />;
}

function LiquidationCascadePage() {
  return (
    <>
      <WorkspacePageHeader
        badge="流动性簇风险代理 · 非真实清算源 · 不推送"
        description="基于杠杆集中、流动性缺口和市场状态的估算，用于观察潜在波动窗口；不作为真实清算数据。"
        eyebrow="Liquidation Cascade Predictor"
        title="强平瀑布预测"
      />
      <LiquidationCascadeDashboard />
    </>
  );
}

function SpotWhalePage({ symbol }) {
  const asset = normalizeMainstreamSymbol(symbol);
  return (
    <>
      <WorkspacePageHeader
        badge="只读提醒 · 不下单 · Spot Discord gate 独立"
        description={`聚合 Binance、Coinbase 与 Bitfinex 现货主动成交流，识别 ${asset} 主动买入、主动卖出、下方吸收、上方压制和跨所错位。`}
        eyebrow={`${asset} SPOT WHALE FLOW`}
        title={`${asset} 现货监控`}
      />
      <SpotWhaleMonitor lockedSymbol={asset} />
    </>
  );
}

function AltContractPage() {
  return (
    <>
      <WorkspacePageHeader
        badge="只读提醒 · 不下单 · dry-run Discord"
        description="独立于 BTC/ETH CWM，只看 Binance USDT 永续山寨合约异常成交、OI、价格响应与清算上下文。"
        eyebrow="Binance Alt Contract Anomaly"
        title="山寨合约异常监控"
      />
      <BinanceAltContractMonitor />
    </>
  );
}

function NewTokenWatchPage() {
  return (
    <>
      <WorkspacePageHeader
        badge="只读提醒 · 不下单 · 外部观测扩展"
        description="最多选择 50 个 USDT 合约 symbol，独立观察吸筹、建仓、出货和中性订单流候选。"
        eyebrow="New Token Contract Flow"
        title="新币合约监控"
      />
      <NewTokenWatch />
    </>
  );
}

function UsageGuidePage() {
  return (
    <>
      <WorkspacePageHeader
        badge="只读指南 · Candidate only"
        description="面向看盘用户的信号解释、页面状态说明和 Discord 提示含义。"
        eyebrow="User Manual"
        title="用户使用指南"
      />
      <UsageGuide />
    </>
  );
}

function ratio(value, total) {
  if (!total) return 0;
  return Number(((value / total) * 100).toFixed(1));
}

function isHighRiskSignal(signal) {
  return signal.risk === "high" || signal.level === "S" || signal.level === "A" || signal.level === "CRITICAL";
}

function isSLevelSignal(signal) {
  const level = String(signal?.level || "").toUpperCase();
  return level === "S" || level === "CRITICAL";
}

function isMediumRiskSignal(signal) {
  return signal.risk === "medium" || signal.level === "B";
}

function byRiskThenTimeDesc(left, right) {
  const riskDelta = riskRank(right) - riskRank(left);
  if (riskDelta !== 0) {
    return riskDelta;
  }
  return signalTime(right) - signalTime(left);
}

function riskRank(signal) {
  const level = String(signal?.level || "").toUpperCase();
  if (level === "CRITICAL" || level === "S") return 4;
  if (signal?.risk === "high" || level === "A") return 3;
  if (signal?.risk === "medium" || level === "B") return 2;
  return 1;
}

function signalTime(signal) {
  const seenAt = Number(signal?.lastSeenAt || signal?.firstSeenAt);
  if (Number.isFinite(seenAt)) {
    return seenAt;
  }
  const parsed = Date.parse(signal?.time || "");
  return Number.isFinite(parsed) ? parsed : 0;
}

function filterLabel(activeRiskFilter, viewMode) {
  if (viewMode === "contract-whale") {
    return "主流币合约监控";
  }
  if (viewMode === "liquidation-cascade") {
    return "强平瀑布预测";
  }
  if (viewMode === "spot-whale") {
    return "主流币现货监控";
  }
  if (viewMode === "alt-contract-monitor") {
    return "山寨合约异常";
  }
  if (viewMode === "signals") {
    return "异常信号：S 级 / Critical";
  }
  if (viewMode === "history") {
    return "信号历史：中风险异常";
  }
  if (viewMode === "rules") {
    return "告警规则：当前有毒订单判断逻辑";
  }
  if (viewMode === "usage-guide") {
    return "使用指南";
  }
  if (activeRiskFilter === "medium") {
    return "高风险主列表 + 中风险折叠区";
  }
  if (activeRiskFilter === "all") {
    return "高风险主列表（中风险可展开）";
  }
  return `${activeRiskFilter} 风险`;
}

function viewModeFromPath(pathname) {
  if (pathname === "/contract-whale" || pathname.startsWith("/contract-whale/")) return "contract-whale";
  if (pathname === "/liquidation-cascade") return "liquidation-cascade";
  if (pathname === "/alt-contract-monitor") return "alt-contract-monitor";
  if (pathname === "/new-token-watch") return "new-token-watch";
  if (pathname === "/spot-whale" || pathname === "/spot-monitor" || pathname.startsWith("/spot-monitor/")) return "spot-whale";
  if (pathname === "/usage-guide") return "usage-guide";
  if (pathname === "/signals") return "signals";
  if (pathname === "/history") return "history";
  if (pathname === "/rules") return "rules";
  return "dashboard";
}

function mainstreamSymbolFromPath(pathname) {
  const match = String(pathname || "").match(/\/(?:contract-whale|spot-monitor)\/([^/]+)/i);
  return normalizeMainstreamSymbol(match?.[1]);
}

function normalizeMainstreamSymbol(symbol) {
  const normalized = String(symbol || "BTC").trim().toUpperCase();
  return normalized === "ETH" ? "ETH" : "BTC";
}

function MediumRiskSection({
  expanded,
  signals,
  selectedSignal,
  onSelect,
  onPush,
  onMarkStatus,
  inboxStats,
  onToggle,
  pushStatus,
}) {
  return (
    <section className="rounded-2xl border border-orange-400/30 bg-slate-900/70 shadow-glow">
      <button
        aria-expanded={expanded}
        aria-label="展开或隐藏 Medium Risk Candidates"
        className="flex w-full items-center justify-between gap-3 px-5 py-4 text-left"
        onClick={onToggle}
        type="button"
      >
        <div>
          <h3 className="font-bold text-orange-200">Medium Risk Candidates</h3>
          <p className="text-xs text-slate-400">
            中风险候选默认折叠保留；只在页面展示，不触发 Discord 推送。
          </p>
        </div>
        <span className="rounded-full border border-orange-300/40 px-3 py-1 text-xs font-semibold text-orange-200">
          {signals.length} 条 {expanded ? "▲" : "▼"}
        </span>
      </button>
      {expanded ? (
        <div className="border-t border-slate-700/60">
          <SignalTable
            description="Medium 风险候选不会自动删除；刷新后从 localStorage 恢复，最新快照只做合并。"
            emptyHint="新的中风险候选出现后会继续追加。"
            emptyMessage="暂无中风险候选信号"
            inboxStats={inboxStats}
            onPush={onPush}
            onMarkStatus={onMarkStatus}
            onSelect={onSelect}
            pushStatus={pushStatus}
            selectedSignal={selectedSignal}
            signals={signals}
            title="Medium Risk Candidates"
          />
        </div>
      ) : null}
    </section>
  );
}
