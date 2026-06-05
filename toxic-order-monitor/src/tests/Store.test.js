import { beforeEach, describe, expect, it } from "vitest";
import { mockSignals } from "../data/mockSignals.js";
import { useSignalsStore } from "../store/signalsStore.js";

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
  });
}
