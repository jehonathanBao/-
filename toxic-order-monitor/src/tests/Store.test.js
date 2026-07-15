import { beforeEach, describe, expect, it } from "vitest";
import { mockSignals } from "../data/mockSignals.js";
import { SIGNAL_INBOX_STORAGE_KEY, useSignalsStore } from "../store/signalsStore.js";

describe("signalsStore", () => {
  beforeEach(() => {
    resetStore();
  });

  it("syncs signals, rawInboxSignals and selectedSignal when markAsPushed is called", () => {
    useSignalsStore.getState().markAsPushed("sig_001");

    const state = useSignalsStore.getState();
    const pushedSignal = state.signals.find((signal) => signal.id === "sig_001");
    const pushedInboxSignal = state.rawInboxSignals.find((signal) => signal.id === "sig_001");

    expect(pushedSignal.status).toBe("pushed");
    expect(pushedInboxSignal.status).toBe("pushed");
    expect(pushedSignal.pushedAt).toBeTruthy();
    expect(state.selectedSignal.status).toBe("pushed");
    expect(state.selectedSignal.pushedAt).toBeTruthy();
  });

  it("appends new signals to the persistent inbox", () => {
    useSignalsStore.getState().setSignals([
      {
        ...mockSignals[0],
        id: "sig_new",
        dedupeKey: "binance:BTCUSDT:new-candidate",
      },
    ]);

    expect(useSignalsStore.getState().rawInboxSignals.map((signal) => signal.id)).toContain("sig_new");
  });

  it("dedupes repeated dedupeKey values", () => {
    useSignalsStore.getState().setSignals([
      {
        ...mockSignals[0],
        id: "sig_duplicate_id",
        dedupeKey: mockSignals[0].dedupeKey,
      },
    ]);

    const repeated = useSignalsStore
      .getState()
      .rawInboxSignals.filter((signal) => signal.dedupeKey === mockSignals[0].dedupeKey);

    expect(repeated).toHaveLength(1);
  });

  it("preserves the previous inbox when a refresh snapshot fails", () => {
    const previousIds = useSignalsStore.getState().rawInboxSignals.map((signal) => signal.id);

    useSignalsStore.getState().applySignalsSnapshot({
      signals: [],
      request: {
        phase: "error",
        source: null,
        errorCode: "HTTP_500",
        fetchedAtMs: 1_700_000_000_000,
      },
      runtime: {
        phase: "unavailable",
        readOnly: null,
        monitoringStarted: null,
        executionEnabled: null,
        checkedAtMs: 1_700_000_000_000,
      },
    });

    const state = useSignalsStore.getState();
    expect(state.rawInboxSignals.map((signal) => signal.id)).toEqual(previousIds);
    expect(state.rawInboxSignals.every((signal) => signal.runtimeBoundary?.phase === "unavailable")).toBe(true);
    expect(state.signalsRequest).toMatchObject({ phase: "error", errorCode: "HTTP_500" });
    expect(state.runtimeBoundary.phase).toBe("unavailable");
  });

  it("merges a ready snapshot and stores runtime truth without persisting transient state", () => {
    useSignalsStore.getState().applySignalsSnapshot({
      signals: [{
        ...mockSignals[0],
        id: "snapshot-signal",
        dedupeKey: "snapshot-signal",
        runtimeBoundary: { phase: "confirmed", readOnly: true, monitoringStarted: true, executionEnabled: false },
      }],
      request: {
        phase: "ready",
        source: "backend",
        errorCode: null,
        fetchedAtMs: 1_700_000_000_000,
      },
      runtime: {
        phase: "confirmed",
        readOnly: true,
        monitoringStarted: true,
        executionEnabled: false,
        checkedAtMs: 1_700_000_000_000,
      },
    });

    const state = useSignalsStore.getState();
    expect(state.rawInboxSignals.map((signal) => signal.id)).toContain("snapshot-signal");
    expect(state.runtimeBoundary).toMatchObject({ phase: "confirmed", executionEnabled: false });
    const persisted = JSON.parse(window.localStorage.getItem(SIGNAL_INBOX_STORAGE_KEY));
    expect(persisted).not.toHaveProperty("signalsRequest");
    expect(persisted).not.toHaveProperty("runtimeBoundary");
    expect(persisted.rawInboxSignals.find((signal) => signal.id === "snapshot-signal")).not.toHaveProperty("runtimeBoundary");
  });

  it("demotes live signals on an unavailable runtime and never reauthorizes them from runtime alone", () => {
    useSignalsStore.setState({
      rawInboxSignals: [{ ...mockSignals[0], isLive: true }],
      signals: [{ ...mockSignals[0], isLive: true }],
      selectedSignal: { ...mockSignals[0], isLive: true },
    });

    useSignalsStore.getState().setRuntimeBoundary({
      phase: "unavailable",
      readOnly: null,
      monitoringStarted: null,
      executionEnabled: null,
      runtimeModified: null,
      analysisOnly: null,
      checkedAtMs: 1_700_000_000_000,
    });
    expect(useSignalsStore.getState().rawInboxSignals[0].isLive).toBe(false);

    useSignalsStore.getState().setRuntimeBoundary({
      phase: "confirmed",
      readOnly: true,
      monitoringStarted: true,
      executionEnabled: false,
      runtimeModified: false,
      analysisOnly: true,
      checkedAtMs: 1_700_000_000_001,
    });
    expect(useSignalsStore.getState().rawInboxSignals[0].isLive).toBe(false);
    expect(useSignalsStore.getState().selectedSignal.isLive).toBe(false);
  });

  it("marks all previous signals historical after an authoritative empty snapshot", () => {
    useSignalsStore.setState({
      rawInboxSignals: [{ ...mockSignals[0], isLive: true }],
      signals: [{ ...mockSignals[0], isLive: true }],
      selectedSignal: { ...mockSignals[0], isLive: true },
    });

    useSignalsStore.getState().applySignalsSnapshot({
      signals: [],
      request: {
        phase: "ready",
        source: "backend",
        errorCode: null,
        fetchedAtMs: 1_700_000_000_000,
      },
      runtime: {
        phase: "confirmed",
        readOnly: true,
        monitoringStarted: true,
        executionEnabled: false,
        runtimeModified: false,
        analysisOnly: true,
        checkedAtMs: 1_700_000_000_000,
      },
    });

    expect(useSignalsStore.getState().rawInboxSignals).toHaveLength(1);
    expect(useSignalsStore.getState().rawInboxSignals[0].isLive).toBe(false);
    expect(useSignalsStore.getState().selectedSignal.isLive).toBe(false);
  });

  it("does not promote an explicitly historical incoming signal under a safe runtime", () => {
    useSignalsStore.getState().setSignals([{
      ...mockSignals[0],
      id: "demo-history",
      dedupeKey: "demo-history",
      isLive: false,
      runtimeBoundary: {
        phase: "confirmed",
        readOnly: true,
        monitoringStarted: true,
        executionEnabled: false,
        runtimeModified: false,
        analysisOnly: true,
        checkedAtMs: Date.now(),
      },
    }]);

    const historical = useSignalsStore
      .getState()
      .rawInboxSignals
      .find((signal) => signal.id === "demo-history");
    expect(historical.isLive).toBe(false);
  });
});

function resetStore() {
  useSignalsStore.setState({
    rawInboxSignals: mockSignals,
    signals: mockSignals,
    selectedSignal: mockSignals[0],
    activeRiskFilter: "all",
    pushStatus: {},
    storageWarning: null,
    pushLogs: [],
    discordConnected: false,
    lastPushedAt: null,
    clearedAtMs: 0,
    clearedSignalKeys: [],
    signalsRequest: { phase: "idle", source: null, errorCode: null, fetchedAtMs: 0 },
    runtimeBoundary: {
      phase: "unavailable",
      readOnly: null,
      monitoringStarted: null,
      executionEnabled: null,
      checkedAtMs: 0,
    },
  });
}
