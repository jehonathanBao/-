import { RUNTIME_BOUNDARY_TTL_MS } from "../api/alertGate.js";
import useRuntimeBoundaryClock from "../hooks/useRuntimeBoundaryClock.js";

const RUNTIME_BOUNDARY_MAX_FUTURE_SKEW_MS = 5_000;

export default function RuntimeBoundaryBadge({ runtimeBoundary, showDetail = false }) {
  const nowMs = useRuntimeBoundaryClock(runtimeBoundary);
  const display = runtimeDisplay(runtimeBoundary, nowMs);
  return (
    <div data-testid="runtime-boundary-status">
      <span className={display.className}>{display.label}</span>
      {showDetail ? <p className="mt-2 leading-4 text-slate-600">{display.detail}</p> : null}
    </div>
  );
}

export function runtimeDisplay(runtimeBoundary, now = Date.now()) {
  if (!runtimeBoundary || runtimeBoundary.phase !== "confirmed") {
    return {
      label: "RUNTIME UNKNOWN",
      detail: "Runtime status unavailable · Push disabled",
      className: "inline-flex items-center gap-1.5 text-amber-300",
    };
  }
  const safe =
    runtimeBoundary.readOnly === true &&
    runtimeBoundary.monitoringStarted === true &&
    runtimeBoundary.executionEnabled === false &&
    runtimeBoundary.runtimeModified === false &&
    runtimeBoundary.analysisOnly === true;
  if (!safe) {
    return {
      label: "RUNTIME CONFLICT",
      detail: "Runtime boundary conflict · Push disabled",
      className: "inline-flex items-center gap-1.5 text-red-300",
    };
  }
  const checkedAtMs = numberOrNull(runtimeBoundary.checkedAtMs);
  const nowMs = numberOrNull(now);
  if (
    checkedAtMs === null ||
    checkedAtMs <= 0 ||
    nowMs === null ||
    checkedAtMs > nowMs + RUNTIME_BOUNDARY_MAX_FUTURE_SKEW_MS ||
    nowMs - checkedAtMs > RUNTIME_BOUNDARY_TTL_MS
  ) {
    return {
      label: "RUNTIME UNKNOWN",
      detail: "Runtime status stale · Push disabled",
      className: "inline-flex items-center gap-1.5 text-amber-300",
    };
  }
  return {
    label: "READ ONLY",
    detail: "Monitoring active · Execution disabled",
    className: "inline-flex items-center gap-1.5 text-emerald-300",
  };
}

function numberOrNull(value) {
  if (value === null || value === undefined || value === "") {
    return null;
  }
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}
