import { create } from "zustand";

export const SIGNAL_INBOX_STORAGE_KEY = "toxic-order-monitor.signal-inbox.v1";

function signalKey(signal) {
  return signal?.dedupeKey || signal?.dedupe_key || signal?.id;
}

function normalizeSignal(signal, now = Date.now()) {
  return {
    ...signal,
    dedupeKey: signalKey(signal),
    firstSeenAt: signal?.firstSeenAt ?? now,
    lastSeenAt: signal?.lastSeenAt ?? now,
    isLive: signal?.isLive ?? true,
  };
}

function loadInboxState() {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    const raw = window.localStorage.getItem(SIGNAL_INBOX_STORAGE_KEY);
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw);
    if (!hasPersistedInboxPayload(parsed)) {
      return null;
    }
    return {
      rawInboxSignals: parsed.rawInboxSignals.map((signal) => normalizeSignal(signal)),
      clearedAtMs: Number(parsed.clearedAtMs || 0),
      clearedSignalKeys: Array.isArray(parsed.clearedSignalKeys)
        ? parsed.clearedSignalKeys
        : [],
    };
  } catch {
    return null;
  }
}

function hasPersistedInboxPayload(parsed) {
  return Boolean(parsed && Array.isArray(parsed.rawInboxSignals));
}

function persistInboxState(state) {
  if (typeof window === "undefined") {
    return null;
  }

  const payload = {
    rawInboxSignals: state.rawInboxSignals,
    clearedAtMs: state.clearedAtMs,
    clearedSignalKeys: state.clearedSignalKeys,
  };
  try {
    window.localStorage.setItem(SIGNAL_INBOX_STORAGE_KEY, JSON.stringify(payload));
    return null;
  } catch {
    return "LOCAL_STORAGE_WRITE_FAILED";
  }
}

function mergeIncomingSignals(currentSignals, incomingSignals, clearedSignalKeys = []) {
  const now = Date.now();
  const cleared = new Set(clearedSignalKeys);
  const byKey = new Map();
  const currentList = Array.isArray(currentSignals) ? currentSignals : [];
  const incomingList = Array.isArray(incomingSignals) ? incomingSignals : [];

  currentList.map((signal) => normalizeSignal(signal, now)).forEach((signal) => {
    const key = signalKey(signal);
    if (key && !cleared.has(key)) {
      byKey.set(key, { ...signal, isLive: false });
    }
  });

  incomingList.map((signal) => normalizeSignal(signal, now)).forEach((signal) => {
    const key = signalKey(signal);
    if (!key || cleared.has(key)) {
      return;
    }
    const existing = byKey.get(key);
    byKey.set(key, {
      ...existing,
      ...signal,
      firstSeenAt: existing?.firstSeenAt ?? signal.firstSeenAt ?? now,
      lastSeenAt: now,
      isLive: true,
    });
  });

  return Array.from(byKey.values()).sort(bySignalTimeDesc);
}

function bySignalTimeDesc(left, right) {
  return signalTime(right) - signalTime(left);
}

function signalTime(signal) {
  const lastSeenAt = Number(signal?.lastSeenAt);
  if (Number.isFinite(lastSeenAt)) {
    return lastSeenAt;
  }
  const parsedTime = Date.parse(signal?.time || "");
  return Number.isFinite(parsedTime) ? parsedTime : 0;
}

const persistedInbox = loadInboxState();
const initialRawInboxSignals =
  persistedInbox
    ? persistedInbox.rawInboxSignals
    : [];
const firstHighRiskSignal =
  initialRawInboxSignals.find((signal) => signal.risk === "high") ?? initialRawInboxSignals[0] ?? null;

export const useSignalsStore = create((set, get) => ({
  rawInboxSignals: initialRawInboxSignals,
  signals: initialRawInboxSignals,
  selectedSignal: firstHighRiskSignal,
  activeRiskFilter: "high",
  pushStatus: {},
  storageWarning: null,
  clearedAtMs: persistedInbox?.clearedAtMs ?? 0,
  clearedSignalKeys: persistedInbox?.clearedSignalKeys ?? [],
  pushLogs: initialRawInboxSignals
    .filter((signal) => signal.status === "pushed")
    .map((signal) => ({
      id: `log_${signal.id}`,
      time: signal.pushedAt,
      exchange: signal.exchange,
      symbol: signal.symbol,
      type: signal.type,
      level: signal.level,
      status: "success",
    })),
  discordConnected: false,
  lastPushedAt: firstHighRiskSignal?.pushedAt ?? null,
  setSignals: (signals) =>
    set((state) => {
      const rawInboxSignals = mergeIncomingSignals(
        state.rawInboxSignals,
        signals,
        state.clearedSignalKeys,
      );
      const selectedKey = signalKey(state.selectedSignal);
      const selectedSignal =
        (selectedKey ? rawInboxSignals.find((signal) => signalKey(signal) === selectedKey) : null) ??
        rawInboxSignals.find((signal) => signal.risk === "high") ??
        rawInboxSignals[0] ??
        null;
      const nextState = {
        rawInboxSignals,
        signals: rawInboxSignals,
        selectedSignal,
      };
      const storageWarning = persistInboxState({ ...state, ...nextState });
      return { ...nextState, storageWarning };
    }),
  setSelectedSignal: (selectedSignal) => set({ selectedSignal }),
  setRiskFilter: (activeRiskFilter) => set({ activeRiskFilter }),
  setPushStatus: (signalId, status, reason = null) =>
    set((state) => ({
      pushStatus: {
        ...state.pushStatus,
        [signalId]: { signalId, status, reason },
      },
    })),
  markAsPushed: (signalId) => {
    const pushedAt = new Date().toLocaleString("zh-CN", { hour12: false }).replace(/\//g, "-");
    set((state) => {
      const updateSignal = (signal) =>
        signal.id === signalId ? { ...signal, status: "pushed", pushedAt } : signal;
      const rawInboxSignals = state.rawInboxSignals.map(updateSignal);
      const selectedSignal =
        state.selectedSignal?.id === signalId
          ? { ...state.selectedSignal, status: "pushed", pushedAt }
          : state.selectedSignal;
      const nextState = {
        rawInboxSignals,
        signals: rawInboxSignals,
        lastPushedAt: pushedAt,
        selectedSignal,
        discordConnected: true,
      };
      const storageWarning = persistInboxState({ ...state, ...nextState });
      return { ...nextState, storageWarning };
    });
  },
  clearSignalInbox: () =>
    set((state) => {
      const clearedSignalKeys = [
        ...new Set([
          ...state.clearedSignalKeys,
          ...state.rawInboxSignals.map(signalKey).filter(Boolean),
        ]),
      ];
      const nextState = {
        rawInboxSignals: [],
        signals: [],
        selectedSignal: null,
        clearedAtMs: Date.now(),
        clearedSignalKeys,
      };
      const storageWarning = persistInboxState({ ...state, ...nextState });
      return { ...nextState, storageWarning };
    }),
  addPushLog: (signal, status = "success", reason = null) =>
    set((state) => ({
      pushLogs: [
        {
          id: `log_${signal.id}_${Date.now()}`,
          time: new Date().toLocaleString("zh-CN", { hour12: false }).replace(/\//g, "-"),
          exchange: signal.exchange,
          symbol: signal.symbol,
          type: signal.type,
          level: signal.level,
          status,
          reason,
        },
        ...state.pushLogs,
      ].slice(0, 12),
    })),
}));
