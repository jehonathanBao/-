import { describe, expect, it } from "vitest";
import { canSendDiscord, evaluateDiscordAlertGate } from "../api/alertGate.js";

describe("evaluateDiscordAlertGate", () => {
  it("allows high score and high data quality candidates", () => {
    expect(evaluateDiscordAlertGate({ risk: "high", score: 88, dataQuality: 90 })).toEqual({
      ok: true,
      reason: null,
    });
  });

  it("uses finalRiskScore before legacy score", () => {
    expect(
      evaluateDiscordAlertGate({
        risk: "high",
        score: 40,
        finalRiskScore: 88,
        dataQuality: 90,
      }),
    ).toEqual({
      ok: true,
      reason: null,
    });
  });

  it("allows critical level candidates when score and data quality pass", () => {
    expect(evaluateDiscordAlertGate({ riskLevel: "critical", score: 91, dataQuality: 90 })).toEqual({
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
    expect(evaluateDiscordAlertGate({ risk: "high", score: 79, dataQuality: 90 })).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_SCORE",
    });
  });

  it("suppresses low data quality candidates", () => {
    expect(evaluateDiscordAlertGate({ risk: "high", score: 88, dataQuality: 69 })).toEqual({
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_DATA_QUALITY",
    });
  });
});
