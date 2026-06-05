import axios from "axios";

const FORBIDDEN_PATTERNS = [
  /discord(?:app)?\.com\/api\/webhooks\/[^\s]+/gi,
  /authorization/gi,
  /bearer\s+[^\s]+/gi,
  /rawPayload/gi,
  /raw_payload/gi,
  /raw payload/gi,
  /markout/gi,
  /evidence/gi,
  /webhook/gi,
  /token/gi,
  /apiKey/gi,
  /api key/gi,
];

export async function fetchScanLogs(limit = 100) {
  const baseURL = (import.meta.env.VITE_API_BASE_URL || "").replace(/\/$/, "");
  try {
    const response = await axios.get(`${baseURL}/api/runtime/scan-log/recent`, {
      params: { limit },
    });
    const items = Array.isArray(response.data?.items) ? response.data.items : [];
    return items.map(normalizeScanLogItem).filter(Boolean);
  } catch {
    return [];
  }
}

export function normalizeScanLogItem(item) {
  if (!item || typeof item !== "object") {
    return null;
  }
  const id = item.id ?? `${item.tsMs || item.ts || Date.now()}-${item.kind || "scan"}`;
  return {
    id: String(id),
    ts: redactScanLogText(item.ts || formatScanLogTime(item.tsMs)),
    tsMs: Number(item.tsMs || 0),
    level: normalizeLevel(item.level),
    kind: redactScanLogText(item.kind || "scan_event"),
    message: redactScanLogText(item.message || "Scan event received"),
    symbol: item.symbol ? redactScanLogText(item.symbol) : "",
    candidateId: item.candidateId ? redactScanLogText(item.candidateId) : "",
  };
}

export function redactScanLogText(value) {
  let text = String(value ?? "").slice(0, 500);
  for (const pattern of FORBIDDEN_PATTERNS) {
    text = text.replace(pattern, "[redacted]");
  }
  return text;
}

function normalizeLevel(level) {
  const value = String(level || "info").toLowerCase();
  if (["error", "warn", "info", "debug"].includes(value)) {
    return value;
  }
  return "info";
}

function formatScanLogTime(tsMs) {
  const value = Number(tsMs);
  if (!Number.isFinite(value) || value <= 0) {
    return "";
  }
  return new Date(value).toISOString();
}
