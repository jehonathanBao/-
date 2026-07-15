import { useEffect, useState } from "react";
import { RUNTIME_BOUNDARY_TTL_MS } from "../api/alertGate.js";

export default function useRuntimeBoundaryClock(runtimeBoundaries) {
  const [, setClockTick] = useState(0);
  const nowMs = Date.now();
  const deadlineMs = earliestFutureExpiry(runtimeBoundaries, nowMs);

  useEffect(() => {
    if (deadlineMs === null) {
      return undefined;
    }
    const timer = window.setTimeout(
      () => setClockTick((tick) => tick + 1),
      Math.max(0, deadlineMs - Date.now()),
    );
    return () => window.clearTimeout(timer);
  }, [deadlineMs]);

  return nowMs;
}

function earliestFutureExpiry(value, nowMs) {
  const boundaries = Array.isArray(value) ? value : [value];
  const deadlines = boundaries
    .map(runtimeExpiry)
    .filter((deadline) => deadline !== null && deadline > nowMs);
  return deadlines.length > 0 ? Math.min(...deadlines) : null;
}

function runtimeExpiry(runtimeBoundary) {
  if (
    !runtimeBoundary ||
    runtimeBoundary.phase !== "confirmed" ||
    runtimeBoundary.readOnly !== true ||
    runtimeBoundary.monitoringStarted !== true ||
    runtimeBoundary.executionEnabled !== false ||
    runtimeBoundary.runtimeModified !== false ||
    runtimeBoundary.analysisOnly !== true
  ) {
    return null;
  }
  const checkedAtMs = Number(runtimeBoundary.checkedAtMs);
  return Number.isFinite(checkedAtMs) && checkedAtMs > 0
    ? checkedAtMs + RUNTIME_BOUNDARY_TTL_MS + 1
    : null;
}
