import type { RiskLevel } from "./use-risk-orders";

const RISK_LABELS: Record<RiskLevel, string> = {
  low: "Low",
  medium: "Medium",
  high: "High",
  critical: "Critical",
  data_insufficient: "Data insufficient",
};

const RISK_CLASSES: Record<RiskLevel, string> = {
  low: "bg-emerald-100 text-emerald-900",
  medium: "bg-amber-100 text-amber-900",
  high: "bg-orange-100 text-orange-950",
  critical: "bg-red-100 text-red-950",
  data_insufficient: "bg-slate-100 text-slate-800",
};

export function RiskBadge({ level }: { level: RiskLevel }) {
  return (
    <span className={`rounded-full px-2 py-1 text-xs font-semibold ${RISK_CLASSES[level]}`}>
      {RISK_LABELS[level]}
    </span>
  );
}
