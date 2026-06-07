export const DISCORD_ALERT_MIN_SCORE = readNumberEnv("VITE_ALERT_MIN_SCORE", 80);
export const DISCORD_ALERT_MIN_DATA_QUALITY = readNumberEnv("VITE_ALERT_MIN_DATA_QUALITY", 70);

export function canSendDiscord(signal) {
  const riskLevel = String(signal?.riskLevel ?? signal?.risk ?? "").toLowerCase();
  const level = String(signal?.level ?? "").toUpperCase();
  return (
    riskLevel === "high" ||
    riskLevel === "critical" ||
    level === "S" ||
    level === "A" ||
    level === "CRITICAL"
  );
}

export function evaluateDiscordAlertGate(signal) {
  const score = Number(signal?.finalRiskScore ?? signal?.score ?? 0);
  const dataQuality = Number(signal?.dataQuality ?? 100);

  if (!canSendDiscord(signal)) {
    return {
      ok: false,
      reason: "DISCORD_SUPPRESSED_NON_HIGH_RISK",
    };
  }

  if (score < DISCORD_ALERT_MIN_SCORE) {
    return {
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_SCORE",
    };
  }

  if (dataQuality < DISCORD_ALERT_MIN_DATA_QUALITY) {
    return {
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_DATA_QUALITY",
    };
  }

  return { ok: true, reason: null };
}

function readNumberEnv(name, fallback) {
  const value = Number(import.meta.env[name]);
  return Number.isFinite(value) ? value : fallback;
}
