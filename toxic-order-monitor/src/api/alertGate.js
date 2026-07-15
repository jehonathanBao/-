export const DISCORD_ALERT_MIN_SCORE = readNumberEnv("VITE_ALERT_MIN_SCORE", 85);
export const DISCORD_ALERT_MIN_CONFIDENCE = readNumberEnv("VITE_ALERT_MIN_CONFIDENCE", 70);
export const DISCORD_ALERT_MIN_DATA_QUALITY = readNumberEnv("VITE_ALERT_MIN_DATA_QUALITY", 70);
export const MARKET_STRUCTURE_ALERT_MIN_SCORE = readNumberEnv("VITE_MARKET_STRUCTURE_ALERT_MIN_SCORE", 80);
export const MARKET_STRUCTURE_EXTREME_MIN_SCORE = readNumberEnv("VITE_MARKET_STRUCTURE_EXTREME_MIN_SCORE", 85);
export const MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE = readNumberEnv("VITE_MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE", 70);
export const MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY = readNumberEnv("VITE_MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY", 70);
export const RUNTIME_BOUNDARY_TTL_MS = readPositiveNumberEnv("VITE_RUNTIME_BOUNDARY_TTL_MS", 120_000);

const RUNTIME_BOUNDARY_MAX_FUTURE_SKEW_MS = 5_000;

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

export function evaluateDiscordAlertGate(signal, now = Date.now()) {
  if (!canSendDiscord(signal)) {
    return {
      ok: false,
      reason: "DISCORD_SUPPRESSED_NON_HIGH_RISK",
    };
  }
  const runtimeGate = evaluateRuntimeAndProvenanceGate(signal, now);
  if (!runtimeGate.ok) {
    return runtimeGate;
  }
  if (evaluateMarketStructureDiscordGate(signal, now).ok) {
    return { ok: true, reason: null };
  }

  const score = numberOrNull(signal?.authoritativeRiskScore ?? signal?.riskScore);
  const confidence = numberOrNull(signal?.detectorConfidence ?? signal?.confidence);
  const dataQuality = numberOrNull(signal?.authoritativeDataQuality ?? signal?.dataQualityScore);

  if (score === null) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_MISSING_AUTHORITATIVE_SCORE" };
  }

  if (score < DISCORD_ALERT_MIN_SCORE) {
    return {
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_SCORE",
    };
  }

  if (confidence === null || confidence < DISCORD_ALERT_MIN_CONFIDENCE) {
    return {
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_CONFIDENCE",
    };
  }

  if (dataQuality === null) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_MISSING_DATA_QUALITY" };
  }

  if (dataQuality < DISCORD_ALERT_MIN_DATA_QUALITY) {
    return {
      ok: false,
      reason: "DISCORD_SUPPRESSED_LOW_DATA_QUALITY",
    };
  }

  return { ok: true, reason: null };
}

export function evaluateMarketStructureDiscordGate(signal, now = Date.now()) {
  const runtimeGate = evaluateRuntimeAndProvenanceGate(signal, now);
  if (!runtimeGate.ok) {
    return runtimeGate;
  }
  const mainForceScore = numberOrZero(
    signal?.mainForceScore ??
      signal?.marketStructureScore?.mainForceScore ??
      signal?.riskSystems?.mainForceStructure?.mainForceScore ??
      signal?.riskSystems?.marketStructureScore?.mainForceScore ??
      0,
  );
  const extremeImpactScore = numberOrZero(
    signal?.extremeImpactScore ??
      signal?.marketStructureScore?.extremeImpactScore ??
      signal?.riskSystems?.mainForceStructure?.extremeImpactScore ??
      signal?.riskSystems?.marketStructureScore?.extremeImpactScore ??
      0,
  );
  const confidence = numberOrZero(
    signal?.marketStructureConfidence ??
      signal?.marketStructureScore?.confidence ??
      signal?.riskSystems?.mainForceStructure?.confidence ??
      signal?.riskSystems?.marketStructureScore?.confidence ??
      0,
  );
  const dataQuality = numberOrNull(
    signal?.marketStructureDataQuality ??
      signal?.marketStructureScore?.dataQuality ??
      signal?.riskSystems?.mainForceStructure?.dataQuality ??
      signal?.riskSystems?.marketStructureScore?.dataQuality,
  );
  const mainForceConfirmed = (
    signal?.mainForceConfirmed ??
    signal?.marketStructureScore?.mainForceConfirmed ??
    signal?.riskSystems?.mainForceStructure?.mainForceConfirmed ??
    signal?.riskSystems?.marketStructureScore?.mainForceConfirmed
  ) === true;

  if (dataQuality === null) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_MISSING_MARKET_STRUCTURE_DATA_QUALITY" };
  }

  if (dataQuality < MARKET_STRUCTURE_ALERT_MIN_DATA_QUALITY) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_LOW_MARKET_STRUCTURE_DATA_QUALITY" };
  }

  if (extremeImpactScore >= MARKET_STRUCTURE_EXTREME_MIN_SCORE) {
    return { ok: true, reason: null };
  }

  if (
    mainForceScore >= MARKET_STRUCTURE_ALERT_MIN_SCORE &&
    mainForceConfirmed &&
    confidence >= MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE
  ) {
    return { ok: true, reason: null };
  }

  if (mainForceScore >= MARKET_STRUCTURE_ALERT_MIN_SCORE && !mainForceConfirmed) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_MAIN_FORCE_UNCONFIRMED" };
  }

  if (
    mainForceScore >= MARKET_STRUCTURE_ALERT_MIN_SCORE &&
    confidence < MARKET_STRUCTURE_ALERT_MIN_CONFIDENCE
  ) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_LOW_MARKET_STRUCTURE_CONFIDENCE" };
  }

  return { ok: false, reason: "DISCORD_SUPPRESSED_LOW_MARKET_STRUCTURE_SCORE" };
}

export function resolveDiscordAlertFamily(signal, now = Date.now()) {
  return evaluateMarketStructureDiscordGate(signal, now).ok ? "market_structure" : "short_toxic_order";
}

function evaluateRuntimeAndProvenanceGate(signal, now) {
  const runtime = signal?.runtimeBoundary;
  if (!runtime || runtime.phase !== "confirmed") {
    return { ok: false, reason: "DISCORD_SUPPRESSED_RUNTIME_UNCONFIRMED" };
  }
  if (
    runtime.readOnly !== true ||
    runtime.monitoringStarted !== true ||
    runtime.executionEnabled !== false ||
    runtime.runtimeModified !== false ||
    runtime.analysisOnly !== true
  ) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_RUNTIME_CONFLICT" };
  }
  const checkedAtMs = numberOrNull(runtime.checkedAtMs);
  const nowMs = numberOrNull(now);
  if (
    checkedAtMs === null ||
    checkedAtMs <= 0 ||
    nowMs === null ||
    checkedAtMs > nowMs + RUNTIME_BOUNDARY_MAX_FUTURE_SKEW_MS ||
    nowMs - checkedAtMs > RUNTIME_BOUNDARY_TTL_MS
  ) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_RUNTIME_STALE" };
  }
  if (signal?.isLive !== true) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_NOT_LIVE" };
  }
  if (signal?.alertEligible !== true) {
    return { ok: false, reason: "DISCORD_SUPPRESSED_INELIGIBLE_PROVENANCE" };
  }
  return { ok: true, reason: null };
}

function numberOrNull(value) {
  if (value === null || value === undefined || value === "") {
    return null;
  }
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}

function numberOrZero(value) {
  return numberOrNull(value) ?? 0;
}

function readNumberEnv(name, fallback) {
  const value = Number(import.meta.env[name]);
  return Number.isFinite(value) ? value : fallback;
}

function readPositiveNumberEnv(name, fallback) {
  const value = readNumberEnv(name, fallback);
  return value > 0 ? value : fallback;
}
