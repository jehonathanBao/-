import { create } from "zustand";

export const SIGNAL_INBOX_STORAGE_KEY = "toxic-order-monitor.signal-inbox.v2";
const LEGACY_SIGNAL_INBOX_STORAGE_KEY = "toxic-order-monitor.signal-inbox.v1";

const LEGACY_MARKET_STRUCTURE_FIELDS = [
  "mainForceScore",
  "mainForceConfirmed",
  "mainForceConfirmationCount",
  "mainForceConfirmationTotal",
  "mainForceConfirmationThreshold",
  "structureBias",
  "extremeImpactScore",
  "extremeImpactConfirmed",
  "regimeType",
  "marketStructureSeverity",
  "marketStructureConfidence",
  "marketStructureDataQuality",
  "structureRaw",
  "spotContractFloor",
  "durationScore",
  "liquidationPenalty",
  "crowdingPenalty",
  "spotScore",
  "spotCvdScore",
  "spotVolumeAnomaly",
  "spotAbsorption",
  "spotLiquidityShift",
  "spotPriceResponse",
  "contractScore",
  "cwmAggressiveFlow",
  "oiImpulse",
  "liquidationContext",
  "fundingCrowding",
  "basisPremium",
  "activeExchangeConfirmation",
  "crossConfirmScore",
  "spotContractDirectionConsistency",
  "multiWindowConsistency",
  "priceResponseConsistency",
  "sourceCoverage",
  "signalAgreement",
  "oiScore",
  "liquidationScore",
  "fundingCrowdingScore",
  "cwmScore",
  "marketStructureReasons",
];

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
    reviewStatus: signal?.reviewStatus ?? null,
  };
}

function loadInboxState() {
  if (typeof window === "undefined") {
    return null;
  }

  try {
    const current = readPersistedInbox(SIGNAL_INBOX_STORAGE_KEY);
    const legacy = current ? null : readPersistedInbox(LEGACY_SIGNAL_INBOX_STORAGE_KEY);
    const parsed = current ?? legacy;
    if (!parsed) {
      return null;
    }
    const state = {
      rawInboxSignals: parsed.rawInboxSignals.map((signal) => ({
        ...normalizeSignal(legacy ? sanitizeLegacySignal(signal) : signal),
        isLive: false,
      })),
      clearedAtMs: Number(parsed.clearedAtMs || 0),
      clearedSignalKeys: Array.isArray(parsed.clearedSignalKeys)
        ? parsed.clearedSignalKeys
        : [],
    };
    if (legacy) {
      persistInboxState(state);
    }
    return state;
  } catch {
    return null;
  }
}

function readPersistedInbox(storageKey) {
  try {
    const raw = window.localStorage.getItem(storageKey);
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw);
    return hasPersistedInboxPayload(parsed) ? parsed : null;
  } catch {
    return null;
  }
}

function sanitizeLegacySignal(signal) {
  const sanitized = {
    ...(signal && typeof signal === "object" ? signal : {}),
    marketStructureScore: null,
    mainForceStructure: null,
  };
  for (const field of LEGACY_MARKET_STRUCTURE_FIELDS) {
    sanitized[field] = null;
  }
  if (sanitized.riskSystems && typeof sanitized.riskSystems === "object") {
    sanitized.riskSystems = {
      ...sanitized.riskSystems,
      marketStructureScore: null,
      mainForceStructure: null,
    };
  }
  return sanitized;
}

function hasPersistedInboxPayload(parsed) {
  return Boolean(parsed && Array.isArray(parsed.rawInboxSignals));
}

function persistInboxState(state) {
  if (typeof window === "undefined") {
    return null;
  }

  const payload = {
    rawInboxSignals: state.rawInboxSignals.map(stripTransientSignalState),
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

function stripTransientSignalState(signal) {
  const {
    runtimeBoundary: _runtimeBoundary,
    request: _request,
    isLive: _isLive,
    ...persisted
  } = signal || {};
  return { ...persisted, isLive: false };
}

function runtimeAllowsLiveSignals(runtimeBoundary) {
  return Boolean(
    runtimeBoundary &&
      runtimeBoundary.phase === "confirmed" &&
      runtimeBoundary.readOnly === true &&
      runtimeBoundary.monitoringStarted === true &&
      runtimeBoundary.executionEnabled === false &&
      runtimeBoundary.runtimeModified === false &&
      runtimeBoundary.analysisOnly === true,
  );
}

function attachRuntimeBoundary(signal, runtimeBoundary) {
  return {
    ...signal,
    runtimeBoundary,
    isLive: runtimeAllowsLiveSignals(runtimeBoundary) ? signal?.isLive === true : false,
  };
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
    const hasExplicitRuntimeBoundary = Boolean(
      signal?.runtimeBoundary && typeof signal.runtimeBoundary === "object",
    );
    byKey.set(key, {
      ...existing,
      ...signal,
      firstSeenAt: existing?.firstSeenAt ?? signal.firstSeenAt ?? now,
      lastSeenAt: now,
      isLive: hasExplicitRuntimeBoundary
        ? runtimeAllowsLiveSignals(signal.runtimeBoundary) && signal.isLive === true
        : true,
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
  signalsRequest: { phase: "idle", source: null, errorCode: null, fetchedAtMs: 0 },
  runtimeBoundary: {
    phase: "unavailable",
    readOnly: null,
    monitoringStarted: null,
    executionEnabled: null,
    runtimeModified: null,
    analysisOnly: null,
    checkedAtMs: 0,
  },
  applySignalsSnapshot: (snapshot) =>
    set((state) => {
      const request = snapshot?.request && typeof snapshot.request === "object"
        ? snapshot.request
        : { phase: "error", source: null, errorCode: "MALFORMED_SNAPSHOT", fetchedAtMs: Date.now() };
      const runtimeBoundary = snapshot?.runtime && typeof snapshot.runtime === "object"
        ? snapshot.runtime
        : {
            phase: "unavailable",
            readOnly: null,
            monitoringStarted: null,
            executionEnabled: null,
            checkedAtMs: Date.now(),
      };
      if (request.phase !== "ready" || !Array.isArray(snapshot?.signals)) {
        const rawInboxSignals = state.rawInboxSignals.map((signal) => attachRuntimeBoundary(signal, runtimeBoundary));
        const selectedSignal = state.selectedSignal
          ? attachRuntimeBoundary(state.selectedSignal, runtimeBoundary)
          : state.selectedSignal;
        return {
          rawInboxSignals,
          signals: rawInboxSignals,
          selectedSignal,
          signalsRequest: request,
          runtimeBoundary,
        };
      }
      const rawInboxSignals = mergeIncomingSignals(
        state.rawInboxSignals,
        snapshot.signals,
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
        signalsRequest: request,
        runtimeBoundary,
      };
      const storageWarning = persistInboxState({ ...state, ...nextState });
      return { ...nextState, storageWarning };
    }),
  setRuntimeBoundary: (runtimeBoundary) =>
    set((state) => {
      const rawInboxSignals = state.rawInboxSignals.map((signal) =>
        attachRuntimeBoundary(signal, runtimeBoundary),
      );
      const selectedKey = signalKey(state.selectedSignal);
      const selectedSignal = selectedKey
        ? rawInboxSignals.find((signal) => signalKey(signal) === selectedKey) ?? null
        : state.selectedSignal
          ? attachRuntimeBoundary(state.selectedSignal, runtimeBoundary)
          : null;
      return {
        runtimeBoundary,
        rawInboxSignals,
        signals: rawInboxSignals,
        selectedSignal,
      };
    }),
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
        signal.id === signalId
          ? {
              ...signal,
              status: "pushed",
              pushedAt,
              alertStatus: "sent",
              alertReason: "manual_sent",
              discordAlert: {
                ...(signal.discordAlert || {}),
                lastDecision: "sent",
                reason: "manual_sent",
                manualSentAt: pushedAt,
              },
            }
          : signal;
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
  setSignalReviewStatus: (signalId, reviewStatus) =>
    set((state) => {
      const updateSignal = (signal) =>
        signal.id === signalId || signalKey(signal) === signalId
          ? {
              ...signal,
              reviewStatus,
              reviewStatusUpdatedAt: new Date().toISOString(),
            }
          : signal;
      const rawInboxSignals = state.rawInboxSignals.map(updateSignal);
      const selectedSignal =
        state.selectedSignal && (state.selectedSignal.id === signalId || signalKey(state.selectedSignal) === signalId)
          ? updateSignal(state.selectedSignal)
          : state.selectedSignal;
      const nextState = {
        rawInboxSignals,
        signals: rawInboxSignals,
        selectedSignal,
      };
      const storageWarning = persistInboxState({ ...state, ...nextState });
      return { ...nextState, storageWarning };
    }),
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
