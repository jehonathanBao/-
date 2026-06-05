import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { evaluateDiscordAlertGate } from "../api/alertGate.js";
import { pushDiscordAlert, sendDiscordTestMessage } from "../api/discord.js";
import { fetchSignals, mapInboxItemToSignal } from "../api/signals.js";
import Header from "../components/Header.jsx";
import PushLog from "../components/PushLog.jsx";
import RiskCard from "../components/RiskCard.jsx";
import RiskCharts from "../components/RiskCharts.jsx";
import RuleStatus from "../components/RuleStatus.jsx";
import Sidebar from "../components/Sidebar.jsx";
import SignalDetail from "../components/SignalDetail.jsx";
import SignalTable from "../components/SignalTable.jsx";
import { useReconnectingWebSocket } from "../hooks/useReconnectingWebSocket.js";
import { useSignalsStore } from "../store/signalsStore.js";

const CLEAR_CACHE_CONFIRM =
  "确认清除当前页面缓存的有毒订单候选信号？操作仅清空前端页面展示，不会删除后端数据。";
const DISCORD_PUSH_CONFIRM =
  "确认推送该高风险候选信号到 Discord？\n该操作会真实发送到告警频道。";

export default function Dashboard() {
  const {
    rawInboxSignals,
    selectedSignal,
    activeRiskFilter,
    pushLogs,
    pushStatus,
    discordConnected,
    lastPushedAt,
    setSignals,
    setSelectedSignal,
    setRiskFilter,
    markAsPushed,
    addPushLog,
    setPushStatus,
    clearSignalInbox,
  } = useSignalsStore();
  const [pushNotice, setPushNotice] = useState(null);
  const [mediumExpanded, setMediumExpanded] = useState(false);
  const [pendingPushIds, setPendingPushIds] = useState(() => new Set());
  const pendingPushIdsRef = useRef(new Set());
  const [testPushPending, setTestPushPending] = useState(false);

  useEffect(() => {
    fetchSignals().then((items) => {
      setSignals(items);
      const state = useSignalsStore.getState();
      if (!state.selectedSignal && state.rawInboxSignals.length > 0) {
        const firstHighRisk =
          state.rawInboxSignals.find((signal) => signal.risk === "high") ?? state.rawInboxSignals[0];
        setSelectedSignal(firstHighRisk);
      }
    });
  }, [setSelectedSignal, setSignals]);

  const handleSignalWsMessage = useCallback(
    (event) => {
      try {
        const payload = JSON.parse(event.data);
        const items = Array.isArray(payload?.signals)
          ? payload.signals
          : Array.isArray(payload?.items)
            ? payload.items
            : [];
        if (items.length > 0) {
          setSignals(items.map(mapInboxItemToSignal));
        }
      } catch {
        // Ignore malformed dashboard stream frames; HTTP polling remains the fallback.
      }
    },
    [setSignals],
  );

  const { status: wsStatus } = useReconnectingWebSocket("/ws/signals", {
    retryMs: 1000,
    maxRetryMs: 15000,
    onMessage: handleSignalWsMessage,
  });

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

  const mediumRiskSignals = useMemo(
    () => rawInboxSignals.filter(isMediumRiskSignal).sort(byRiskThenTimeDesc),
    [rawInboxSignals],
  );

  const primarySignals = useMemo(() => {
    if (activeRiskFilter === "low") {
      return rawInboxSignals.filter((signal) => signal.risk === "low").sort(byRiskThenTimeDesc);
    }
    return highRiskSignals;
  }, [activeRiskFilter, highRiskSignals, rawInboxSignals]);

  const highUnhandledCount = rawInboxSignals.filter(
    (signal) => signal.risk === "high" && signal.status === "unhandled",
  ).length;
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
    setPushNotice({ type: "pending", message: "Discord 测试消息发送中..." });
    try {
      const result = await sendDiscordTestMessage();
      if (result.ok) {
        setPushNotice({ type: "success", message: "Discord 测试消息发送成功" });
        return;
      }
      setPushNotice({
        type: "failed",
        message:
          result.reason === "DISCORD_NOT_CONFIGURED"
            ? "Discord 未配置，测试消息未发送。"
            : `Discord 测试消息失败：${result.reason || "DISCORD_TEST_FAILED"}`,
      });
    } catch (error) {
      const reason = error?.response?.data?.reason || error?.message || "NETWORK_ERROR";
      setPushNotice({ type: "failed", message: `Discord 测试消息失败：${reason}` });
    } finally {
      setTestPushPending(false);
    }
  }

  return (
    <div className="flex min-h-screen bg-[#07111f]">
      <Sidebar />
      <main className="min-w-0 flex-1 p-4 lg:p-6">
        <Header discordConnected={discordConnected} highUnhandledCount={highUnhandledCount} />
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
            <span className="text-sm text-slate-500">当前筛选：{filterLabel(activeRiskFilter)}</span>
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
              title={activeRiskFilter === "low" ? "Low Risk Candidates" : "High / Critical Risk Candidates"}
              description={
                activeRiskFilter === "low"
                  ? "低风险候选只在用户主动筛选时展示，仍保留在持久 Inbox。"
                  : "默认展示高风险候选信号；刷新页面只重新订阅数据，不会清空历史卡片。"
              }
              onPush={handlePush}
              onSelect={setSelectedSignal}
              pushStatus={effectivePushStatus}
              selectedSignal={selectedSignal}
              signals={primarySignals}
            />
            <MediumRiskSection
              expanded={mediumExpanded}
              inboxStats={stats}
              onPush={handlePush}
              onSelect={setSelectedSignal}
              pushStatus={effectivePushStatus}
              onToggle={() => setMediumExpanded((value) => !value)}
              selectedSignal={selectedSignal}
              signals={mediumRiskSignals}
            />
            <SignalDetail signal={selectedSignal} />
          </div>
          <div className="space-y-5">
            <RiskCharts signals={rawInboxSignals} />
            <PushLog logs={pushLogs} />
          </div>
        </div>
      </main>
    </div>
  );
}

function buildPushStatus(pushStatus, pendingPushIds) {
  const next = { ...pushStatus };
  for (const signalId of pendingPushIds) {
    next[signalId] = { signalId, status: "pending" };
  }
  return next;
}

function ratio(value, total) {
  if (!total) return 0;
  return Number(((value / total) * 100).toFixed(1));
}

function isHighRiskSignal(signal) {
  return signal.risk === "high" || signal.level === "S" || signal.level === "A" || signal.level === "CRITICAL";
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

function filterLabel(activeRiskFilter) {
  if (activeRiskFilter === "medium") {
    return "高风险主列表 + 中风险折叠区";
  }
  if (activeRiskFilter === "all") {
    return "高风险主列表（中风险可展开）";
  }
  return `${activeRiskFilter} 风险`;
}

function MediumRiskSection({
  expanded,
  signals,
  selectedSignal,
  onSelect,
  onPush,
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
