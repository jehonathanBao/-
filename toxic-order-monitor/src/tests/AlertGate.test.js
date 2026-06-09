import { describe, expect, it } from "vitest";
import {
  canSendDiscord,
  evaluateDiscordAlertGate,
  evaluateMarketStructureDiscordGate,
  resolveDiscordAlertFamily,
} from "../api/alertGate.js";

describe("evaluateDiscordAlertGate", () => {
  it("allows high score and high data quality candidates", () => {
    expect(evaluateDiscordAlertGate({ risk: "high", score: 88, confidence: 90, dataQuality: 90 })).toEqual({
      ok: true,
      reason: null,
    });
  });

  it("uses toxicScore before compatibility score fields", () => {
    expect(
      evaluateDiscordAlertGate({
        risk: "high",
        score: 40,
        finalRiskScore: 40,
        toxicScore: 88,
        confidence: 90,
        dataQuality: 90,
      }),
    ).toEqual({
      ok: true,
      reason: null,
    });
  });

  it("allows critical level candidates when score and data quality pass", () => {
    expect(evaluateDiscordAlertGate({ riskLevel: "critical", score: 91, confidence: 90, dataQuality: 90 })).toEqual({
      ok: true,
      reason: null,
    });
    expect(canSendDiscord({ level: "S" })).toBe(true);
  });

  it("suppresses medium candidates even when score and data quality pass", () => {
    expect(evaluateDiscordAlertGate({ risk: "medium", level: "B", score: 95, dataQuality: 95 })).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_NON_HIGH_RISK",
    });
  });

  it("suppresses low score candidates", () => {
    expect(evaluateDiscordAlertGate({ risk: "high", score: 84, confidence: 90, dataQuality: 90 })).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_SCORE",
    });
  });

  it("suppresses low confidence candidates", () => {
    expect(evaluateDiscordAlertGate({ risk: "high", score: 88, confidence: 69, dataQuality: 90 })).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_CONFIDENCE",
    });
  });

  it("suppresses low data quality candidates", () => {
    expect(evaluateDiscordAlertGate({ risk: "high", score: 88, confidence: 90, dataQuality: 69 })).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_DATA_QUALITY",
    });
  });

  it("allows market-structure alerts with main-force confirmation", () => {
    expect(
      evaluateMarketStructureDiscordGate({
        mainForceScore: 84,
        marketStructureConfidence: 76,
        marketStructureDataQuality: 74,
        extremeImpactScore: 58,
      }),
    ).toEqual({
      ok: true,
      reason: null,
    });
    expect(
      resolveDiscordAlertFamily({
        mainForceScore: 84,
        marketStructureConfidence: 76,
        marketStructureDataQuality: 74,
      }),
    ).toBe("market_structure");
  });

  it("allows market-structure alerts on extreme impact even if confidence is lower", () => {
    expect(
      evaluateMarketStructureDiscordGate({
        mainForceScore: 54,
        marketStructureConfidence: 52,
        marketStructureDataQuality: 76,
        extremeImpactScore: 91,
      }),
    ).toEqual({
      ok: true,
      reason: null,
    });
  });
});
