import { describe, expect, it } from "vitest";
import {
  canSendDiscord,
  evaluateDiscordAlertGate,
  evaluateMarketStructureDiscordGate,
  resolveDiscordAlertFamily,
} from "../api/alertGate.js";

describe("evaluateDiscordAlertGate", () => {
  it("allows high score and high data quality candidates", () => {
    expect(evaluateDiscordAlertGate(safeSignal({ riskScore: 88, confidence: 90, dataQualityScore: 90 }))).toEqual({
      ok: true,
      reason: null,
    });
  });

  it("uses only the authoritative detector score and ignores synthetic toxic scores", () => {
    expect(
      evaluateDiscordAlertGate(safeSignal({
        riskScore: 40,
        score: 40,
        finalRiskScore: 99,
        toxicScore: 99,
        confidence: 90,
        dataQualityScore: 90,
      })),
    ).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_SCORE",
    });
  });

  it("allows critical level candidates when score and data quality pass", () => {
    expect(evaluateDiscordAlertGate(safeSignal({ riskLevel: "critical", riskScore: 91, confidence: 90, dataQualityScore: 90 }))).toEqual({
      ok: true,
      reason: null,
    });
    expect(canSendDiscord({ level: "S" })).toBe(true);
  });

  it("suppresses medium candidates even when score and data quality pass", () => {
    expect(evaluateDiscordAlertGate(safeSignal({ risk: "medium", level: "B", riskScore: 95, dataQualityScore: 95 }))).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_NON_HIGH_RISK",
    });
  });

  it("suppresses low score candidates", () => {
    expect(evaluateDiscordAlertGate(safeSignal({ riskScore: 84, confidence: 90, dataQualityScore: 90 }))).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_SCORE",
    });
  });

  it("suppresses low confidence candidates", () => {
    expect(evaluateDiscordAlertGate(safeSignal({ riskScore: 88, confidence: 69, dataQualityScore: 90 }))).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_CONFIDENCE",
    });
  });

  it("suppresses low data quality candidates", () => {
    expect(evaluateDiscordAlertGate(safeSignal({ riskScore: 88, confidence: 90, dataQualityScore: 69 }))).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_DATA_QUALITY",
    });
  });

  it("fails closed when runtime truth or alert-eligible provenance is missing", () => {
    expect(
      evaluateDiscordAlertGate({
        risk: "high",
        riskScore: 91,
        confidence: 90,
        dataQualityScore: 90,
        alertEligible: true,
      }),
    ).toEqual({ ok: false, reason: "DISCORD_SUPPRESSED_RUNTIME_UNCONFIRMED" });
    expect(evaluateDiscordAlertGate(safeSignal({ alertEligible: false }))).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_INELIGIBLE_PROVENANCE",
    });
    expect(evaluateDiscordAlertGate(safeSignal({
      runtimeBoundary: {
        phase: "confirmed",
        readOnly: true,
        monitoringStarted: true,
        executionEnabled: false,
        runtimeModified: null,
        analysisOnly: null,
      },
    }))).toEqual({ ok: false, reason: "DISCORD_SUPPRESSED_RUNTIME_CONFLICT" });
  });

  it("fails closed for cached signals even when a newer runtime boundary is confirmed", () => {
    expect(
      evaluateDiscordAlertGate(safeSignal({
        isLive: false,
        runtimeBoundary: safeRuntimeBoundary(Date.now()),
      })),
    ).toEqual({ ok: false, reason: "DISCORD_SUPPRESSED_NOT_LIVE" });
  });

  it("fails closed when the runtime boundary has exceeded its TTL", () => {
    const checkedAtMs = 1_700_000_000_000;
    expect(
      evaluateDiscordAlertGate(
        safeSignal({ runtimeBoundary: safeRuntimeBoundary(checkedAtMs) }),
        checkedAtMs + 120_001,
      ),
    ).toEqual({ ok: false, reason: "DISCORD_SUPPRESSED_RUNTIME_STALE" });
    expect(
      evaluateDiscordAlertGate(
        safeSignal({ runtimeBoundary: safeRuntimeBoundary(undefined) }),
        checkedAtMs,
      ),
    ).toEqual({ ok: false, reason: "DISCORD_SUPPRESSED_RUNTIME_STALE" });
  });

  it("allows market-structure alerts with main-force confirmation", () => {
    expect(
      evaluateMarketStructureDiscordGate(safeSignal({
        mainForceScore: 84,
        marketStructureConfidence: 76,
        marketStructureDataQuality: 74,
        extremeImpactScore: 58,
      })),
    ).toEqual({
      ok: true,
      reason: null,
    });
    expect(
      resolveDiscordAlertFamily(safeSignal({
        mainForceScore: 84,
        marketStructureConfidence: 76,
        marketStructureDataQuality: 74,
      })),
    ).toBe("market_structure");
  });

  it("requires an explicit main-force confirmation for the main-force path", () => {
    expect(
      evaluateMarketStructureDiscordGate(safeSignal({
        mainForceScore: 84,
        mainForceConfirmed: false,
        marketStructureConfidence: 76,
        marketStructureDataQuality: 74,
        extremeImpactScore: 58,
      })),
    ).toEqual({ ok: false, reason: "DISCORD_SUPPRESSED_MAIN_FORCE_UNCONFIRMED" });
  });

  it("allows market-structure alerts on extreme impact even if confidence is lower", () => {
    expect(
      evaluateMarketStructureDiscordGate(safeSignal({
        mainForceScore: 54,
        mainForceConfirmed: false,
        marketStructureConfidence: 52,
        marketStructureDataQuality: 76,
        extremeImpactScore: 91,
      })),
    ).toEqual({
      ok: true,
      reason: null,
    });
  });

  it("does not borrow short-term confidence for a market-structure alert", () => {
    expect(
      evaluateMarketStructureDiscordGate(safeSignal({
        mainForceScore: 84,
        confidence: 99,
        marketStructureConfidence: undefined,
        marketStructureDataQuality: 90,
        extremeImpactScore: 50,
      })),
    ).toEqual({ ok: false, reason: "DISCORD_SUPPRESSED_LOW_MARKET_STRUCTURE_CONFIDENCE" });
  });
});

function safeSignal(overrides = {}) {
  return {
    risk: "high",
    riskScore: 88,
    confidence: 90,
    dataQualityScore: 90,
    alertEligible: true,
    isLive: true,
    mainForceConfirmed: true,
    runtimeBoundary: safeRuntimeBoundary(Date.now()),
    ...overrides,
  };
}

function safeRuntimeBoundary(checkedAtMs) {
  return {
    phase: "confirmed",
    readOnly: true,
    monitoringStarted: true,
    executionEnabled: false,
    runtimeModified: false,
    analysisOnly: true,
    checkedAtMs,
  };
}
