import { beforeEach, describe, expect, it, vi } from "vitest";

const STORAGE_KEY = "toxic-order-monitor.signal-inbox.v1";
const MIGRATED_STORAGE_KEY = "toxic-order-monitor.signal-inbox.v2";

describe("signal inbox persistence safety", () => {
  beforeEach(() => {
    window.localStorage.clear();
    vi.resetModules();
  });

  it("loads legacy persisted live flags as historical and runtime truth cannot restore them", async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({
      rawInboxSignals: [{
        id: "legacy-live",
        dedupeKey: "legacy-live",
        risk: "high",
        alertEligible: true,
        isLive: true,
      }],
      clearedAtMs: 0,
      clearedSignalKeys: [],
    }));

    const { useSignalsStore } = await import("../store/signalsStore.js");
    expect(useSignalsStore.getState().rawInboxSignals[0].isLive).toBe(false);

    useSignalsStore.getState().setRuntimeBoundary(safeRuntimeBoundary());
    expect(useSignalsStore.getState().rawInboxSignals[0].isLive).toBe(false);
  });

  it("persists newly received live signals only as historical cache entries", async () => {
    const { useSignalsStore } = await import("../store/signalsStore.js");
    useSignalsStore.getState().setSignals([{
      id: "current-live",
      dedupeKey: "current-live",
      risk: "high",
      alertEligible: true,
      isLive: true,
      runtimeBoundary: safeRuntimeBoundary(),
    }]);

    const persisted = JSON.parse(window.localStorage.getItem(MIGRATED_STORAGE_KEY));
    expect(persisted.rawInboxSignals[0].isLive).toBe(false);
  });

  it("migrates v1 cache without legacy synthetic market-structure zeroes", async () => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({
      rawInboxSignals: [{
        id: "legacy-synthetic-structure",
        dedupeKey: "legacy-synthetic-structure",
        risk: "high",
        riskScore: 83,
        dataQualityScore: 79,
        confidence: 82,
        shortPressure: -83,
        mainForceScore: 0,
        mainForceConfirmed: false,
        mainForceConfirmationCount: 0,
        extremeImpactScore: 0,
        extremeImpactConfirmed: false,
        structureBias: 0,
        marketStructureSeverity: "Calm",
        marketStructureConfidence: 0,
        marketStructureDataQuality: 0,
        marketStructureScore: {
          mainForceScore: 0,
          mainForceConfirmed: false,
          extremeImpactScore: 0,
          extremeImpactConfirmed: false,
          structureBias: 0,
          confidence: 0,
          dataQuality: 0,
          severity: "Calm",
        },
        riskSystems: {
          shortTermToxic: {
            toxicScore: 83,
            shortPressure: -83,
            confidence: 82,
            dataQuality: 79,
          },
          marketStructureScore: {
            mainForceScore: 0,
            mainForceConfirmed: false,
            confidence: 0,
            dataQuality: 0,
          },
          mainForceStructure: {
            mainForceScore: 0,
            mainForceConfirmed: false,
            confidence: 0,
            dataQuality: 0,
          },
        },
        isLive: true,
      }],
      clearedAtMs: 0,
      clearedSignalKeys: [],
    }));

    const { useSignalsStore } = await import("../store/signalsStore.js");
    const signal = useSignalsStore.getState().rawInboxSignals[0];

    expect(signal).toMatchObject({
      riskScore: 83,
      dataQualityScore: 79,
      confidence: 82,
      shortPressure: -83,
      mainForceScore: null,
      mainForceConfirmed: null,
      extremeImpactScore: null,
      extremeImpactConfirmed: null,
      structureBias: null,
      marketStructureSeverity: null,
      marketStructureConfidence: null,
      marketStructureDataQuality: null,
      marketStructureScore: null,
      isLive: false,
    });
    expect(signal.riskSystems.shortTermToxic.toxicScore).toBe(83);
    expect(signal.riskSystems.marketStructureScore).toBeNull();
    expect(signal.riskSystems.mainForceStructure).toBeNull();
    expect(JSON.parse(window.localStorage.getItem(MIGRATED_STORAGE_KEY)).rawInboxSignals[0].mainForceScore).toBeNull();
  });
});

function safeRuntimeBoundary() {
  return {
    phase: "confirmed",
    readOnly: true,
    monitoringStarted: true,
    executionEnabled: false,
    runtimeModified: false,
    analysisOnly: true,
    checkedAtMs: Date.now(),
  };
}
