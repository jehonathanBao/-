export const DISCORD_ALERT_MIN_SCORE = readNumberEnv("VITE_ALERT_MIN_SCORE", 85);
export const DISCORD_ALERT_MIN_CONFIDENCE = readNumberEnv("VITE_ALERT_MIN_CONFIDENCE", 70);
export const DISCORD_ALERT_MIN_DATA_QUALITY = readNumberEnv("VITE_ALERT_MIN_DATA_QUALITY", 70);
export const MARKET_STRUCTURE_ALERT_MIN_SCORE = readNumberEnv("VITE_MARKET_STRUCTURE_ALERT_MIN_SCORE", 80);
export const MARKET_STRUCTURE_EXTREME_MIN_SCORE = readNumberEnv("VITE_MARKET_STRUCTURE_EXTREME_MIN_SCORE", 85);
export const MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE = readNumberEnv("VITE_MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE", 70);
export const MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY = readNumberEnv("VITE_MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY", 70);

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
  if (evaluateMarketStructureDiscordGate(signal).ok) {
    return { ok: true, reason: null };
  }

  const score = Number(
    signal?.toxicScore ??
      signal?.riskSystems?.shortTermToxic?.toxicScore ??
      signal?.toxicShortScore?.toxicScore ??
      signal?.finalRiskScore ??
      signal?.score ??
      0,
  );
  const confidence = Number(
    signal?.confidence ??
      signal?.riskSystems?.shortTermToxic?.confidence ??
      signal?.toxicShortScore?.confidence ??
      0,
  );
  const dataQuality = Number(
    signal?.dataQuality ??
      signal?.riskSystems?.shortTermToxic?.dataQuality ??
      signal?.toxicShortScore?.dataQuality ??
      100,
  );

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

  if (confidence < DISCORD_ALERT_MIN_CONFIDENCE) {
    return {
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_CONFIDENCE",
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

export function evaluateMarketStructureDiscordGate(signal) {
  const mainForceScore = Number(
    signal?.mainForceScore ??
      signal?.marketStructureScore?.mainForceScore ??
      signal?.riskSystems?.mainForceStructure?.mainForceScore ??
      signal?.riskSystems?.marketStructureScore?.mainForceScore ??
      0,
  );
  const extremeImpactScore = Number(
    signal?.extremeImpactScore ??
      signal?.marketStructureScore?.extremeImpactScore ??
      signal?.riskSystems?.mainForceStructure?.extremeImpactScore ??
      signal?.riskSystems?.marketStructureScore?.extremeImpactScore ??
      0,
  );
  const confidence = Number(
    signal?.marketStructureConfidence ??
      signal?.marketStructureScore?.confidence ??
      signal?.riskSystems?.mainForceStructure?.confidence ??
      signal?.riskSystems?.marketStructureScore?.confidence ??
      signal?.confidence ??
      0,
  );
  const dataQuality = Number(
    signal?.marketStructureDataQuality ??
      signal?.marketStructureScore?.dataQuality ??
      signal?.riskSystems?.mainForceStructure?.dataQuality ??
      signal?.riskSystems?.marketStructureScore?.dataQuality ??
      signal?.dataQuality ??
      100,
  );

  if (dataQuality < MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_LOW_MARKET_STRUCTURE_DATA_QUALITY" };
  }

  if (
    mainForceScore >= MARKET_STRUCTURE_ALERT_MIN_SCORE &&
    confidence >= MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE
  ) {
    return { ok: true, reason: null };
  }

  if (extremeImpactScore >= MARKET_STRUCTURE_EXTREME_MIN_SCORE) {
    return { ok: true, reason: null };
  }

  if (
    mainForceScore >= MARKET_STRUCTURE_ALERT_MIN_SCORE &&
    confidence < MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE
  ) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_LOW_MARKET_STRUCTURE_CONFIDENCE" };
  }

  return { ok: false, reason: "DISCORD_SUPPRESSED_LOW_MARKET_STRUCTURE_SCORE" };
}

export function resolveDiscordAlertFamily(signal) {
  return evaluateMarketStructureDiscordGate(signal).ok ? "market_structure" : "short_toxic_order";
}

function readNumberEnv(name, fallback) {
  const value = Number(import.meta.env[name]);
  return Number.isFinite(value) ? value : fallback;
}
