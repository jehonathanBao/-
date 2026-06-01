const fastEndpoints = [
  "/api/status",
  "/api/venues/diagnostics",
  "/api/toxic-state",
  "/api/toxicity/active-trade/status",
  "/api/toxicity/active-trade/recent",
  "/api/toxicity/liquidation/status",
  "/api/toxicity/liquidation/recent",
  "/api/toxicity/orderbook-walls/status",
  "/api/toxicity/orderbook-walls/recent",
  "/api/toxicity/orderbook-wall-interpretation/status",
  "/api/toxicity/orderbook-wall-interpretation/recent",
  "/api/toxicity/structural/status",
  "/api/toxicity/structural/recent",
  "/api/toxicity/whale-flow/status",
  "/api/toxicity/whale-flow/recent",
  "/api/toxicity/whale-flow/calibration/status",
  "/api/toxicity/whale-flow/calibration/report",
  "/api/toxicity/whale-flow/history/status",
  "/api/toxicity/whale-flow/history/recent",
  "/api/toxicity/fusion/status",
  "/api/toxicity/fusion/recent",
  "/api/toxicity/signal-inbox/status",
  "/api/toxicity/signal-inbox/recent",
  "/api/toxicity/signal-groups/status",
  "/api/toxicity/signal-groups/recent",
  "/api/toxicity/signal-detail/status",
  "/api/toxicity/signal-history/status",
  "/api/toxicity/signal-history/recent",
  "/api/toxicity/signal-history/alerts/recent",
  "/api/toxicity/signal-history/reports/recent",
  "/api/toxicity/signal-health/status",
  "/api/toxicity/signal-health/summary",
  "/api/toxicity/signal-report/status",
  "/api/toxicity/signal-report/daily",
  "/api/toxicity/signal-report/rolling",
  "/api/toxicity/signal-alert-preview/status",
  "/api/toxicity/signal-alert-preview/recent",
  "/api/toxicity/replay/status",
  "/api/toxicity/replay/recent",
  "/api/toxicity/markout/status",
  "/api/toxicity/markout/recent",
  "/api/toxicity/quality-scorecard/status",
  "/api/toxicity/quality-scorecard/summary",
  "/api/toxicity/weight-recommendation/status",
  "/api/toxicity/weight-recommendation/summary",
  "/api/toxicity/weight-review/status",
  "/api/toxicity/weight-review/latest",
  "/api/toxicity/governance-ledger/status",
  "/api/toxicity/governance-ledger/recent",
  "/api/toxicity/governance-proposal/status",
  "/api/toxicity/governance-proposal/summary",
  "/api/toxicity/governance-review-pack/status",
  "/api/toxicity/governance-review-pack/summary",
  "/api/toxicity/governance-signoff-pack/status",
  "/api/toxicity/governance-signoff-pack/summary",
  "/api/liq-hunt-state",
  "/api/vpin-state",
  "/api/liquidation-state",
];

const slowEndpoints = [
  "/api/flow-state",
  "/api/markout-state",
  "/api/sweep-state",
  "/api/toxic-events?limit=50",
  "/api/storage/status",
];

const replayEndpoints = ["/api/replay-reports"];
const calibrationEndpoints = ["/api/calibration/reports", "/api/calibration/reports/latest"];
const parameterReviewEndpoints = [
  "/api/parameter-review/recommendations",
  "/api/parameter-review/recommendations/latest",
  "/api/parameter-review/reviews",
  "/api/parameter-review/exports",
  "/api/parameter-review/exports/latest",
];
const patchDiffEndpoints = [
  "/api/parameter-review/exports/latest/diff",
  "/api/parameter-review/exports/latest/audit",
];
const runbookEndpoints = ["/api/parameter-review/exports/latest/runbook"];
const dryRunEndpoints = ["/api/parameter-review/exports/latest/dry-run"];
const evidencePackEndpoints = ["/api/parameter-review/exports/latest/evidence-pack"];
const startupCheckEndpoints = ["/api/calibration/manual-startup/check"];
const signoffEndpoints = [
  "/api/calibration/manual-signoff/status",
  "/api/calibration/manual-signoff/history",
];
const evidenceFreshnessEndpoints = ["/api/calibration/manual-evidence/freshness"];
const auditStoryEndpoints = ["/api/calibration/manual-audit-story"];
const governanceEndpoints = ["/api/calibration/manual-governance/index"];

const state = {
  data: {},
  selectedReport: null,
  selectedReportContent: null,
  selectedCalibrationReport: null,
  selectedCalibrationReportContent: null,
  selectedRunbookExportId: null,
  selectedRunbook: null,
  selectedRunbookMarkdown: null,
  selectedDryRunExportId: null,
  selectedDryRun: null,
  selectedDryRunMarkdown: null,
  selectedEvidencePackExportId: null,
  selectedEvidencePack: null,
  selectedEvidencePackMarkdown: null,
  latestManualSignoffAction: null,
  latestManualAuditStoryAction: null,
  latestGovernanceAction: null,
  activeTradeToxicitySymbol: null,
  latestActiveTradeToxicityAction: null,
  orderbookWallSymbol: null,
  latestOrderbookWallAction: null,
  orderbookWallInterpretationSymbol: null,
  latestOrderbookWallInterpretationAction: null,
  structuralToxicitySymbol: null,
  latestStructuralToxicityAction: null,
  latestWhaleFlowAction: null,
  latestVenueDiagnosticsAction: null,
  whaleFlowCompactPreset: "all",
  latestWhaleFlowCompactAction: null,
  whaleFlowCalibrationSymbol: null,
  latestWhaleFlowCalibrationAction: null,
  whaleFlowCandidateHistorySymbol: null,
  latestWhaleFlowCandidateHistoryAction: null,
  toxicSignalFusionSymbol: null,
  latestToxicSignalFusionAction: null,
  signalSymbolFilter: null,
  latestSignalSymbolFilterAction: null,
  toxicSignalHealthSymbol: null,
  latestToxicSignalHealthAction: null,
  toxicSignalInboxSymbol: null,
  latestToxicSignalInboxAction: null,
  toxicSignalGroupSymbol: null,
  latestToxicSignalGroupAction: null,
  toxicSignalDetailSignalId: null,
  toxicSignalDetailGroupId: null,
  toxicSignalDetailPayload: null,
  latestToxicSignalDetailAction: null,
  toxicSignalHistorySymbol: null,
  toxicSignalHistorySignalId: null,
  toxicSignalHistoryLookupPayload: null,
  toxicSignalHistorySortMode: "severity",
  latestToxicSignalHistoryAction: null,
  latestToxicSignalReportAction: null,
  latestToxicSignalRollingAction: null,
  toxicSignalAlertExplainSignalId: null,
  toxicSignalAlertExplainPayload: null,
  latestToxicSignalAlertPreviewAction: null,
  latestDurableArchiveDryRunAction: null,
  latestDurableArchiveDryRunReviewPackAction: null,
  latestDurableArchiveWriteGateAction: null,
  latestDurableArchiveWriteAuditAction: null,
  toxicReplayDetail: null,
  latestToxicReplayAction: null,
  latestToxicMarkoutAction: null,
  latestToxicQualityScorecardAction: null,
  latestToxicWeightRecommendationAction: null,
  latestToxicWeightReviewAction: null,
  latestToxicGovernanceLedgerAction: null,
  latestToxicGovernanceProposalAction: null,
  latestToxicGovernanceReviewPackAction: null,
  latestToxicGovernanceSignoffPackAction: null,
  latestRuntimeAction: null,
  suspiciousReplaySymbol: null,
  suspiciousReplaySignalId: null,
  suspiciousReplayStatusPayload: null,
  suspiciousReplayHistoryPayload: null,
  suspiciousReplayLookupPayload: null,
  suspiciousReplayDetailPayload: null,
  suspiciousReplayExplainPayload: null,
  suspiciousReplayError: null,
  latestSuspiciousReplayAction: null,
  suspiciousOrdersSortMode: "severity",
  suspiciousOrdersFilterSymbol: "",
  suspiciousOrdersFilterAlertDecision: "",
  suspiciousOrdersHideNotEnoughData: false,
  suspiciousOrdersHighSeverityOnly: false,
  suspiciousOrdersLastSeen: {},
  latestSuspiciousOrdersAction: null,
  replayHeatmapSymbolFilter: "",
  replayHeatmapSignalKindFilter: "",
  replayHeatmapDirectionFilter: "",
  replayHeatmapHistoryPayload: null,
  replayHeatmapRollingPayload: null,
  replayHeatmapBuiltPayload: null,
  replayHeatmapLastHistoryUrl: null,
  replayHeatmapLastRollingUrl: null,
  replayHeatmapError: null,
  latestReplayHeatmapAction: null,
};

const SUSPICIOUS_ORDERS_LAST_SEEN_WINDOW_MS = 5 * 60 * 1000;

function $(id) {
  return document.getElementById(id);
}

function formatNumber(value, digits = 1) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "Unavailable";
  }
  return Number(value).toLocaleString(undefined, {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  });
}

function formatPercent(value, digits = 1) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "Unavailable";
  }
  return `${(Number(value) * 100).toFixed(digits)}%`;
}

function formatInteger(value) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "Unavailable";
  }
  return Number(value).toLocaleString();
}

function formatBool(value) {
  return value ? "true" : "false";
}

function formatTime(ts) {
  if (!ts) {
    return "Unavailable";
  }
  return new Date(ts).toLocaleTimeString();
}

function formatDateTime(ts) {
  if (!ts) {
    return "Unavailable";
  }
  return new Date(ts).toLocaleString();
}

function badgeClass(kind) {
  switch ((kind || "").toString().toLowerCase()) {
    case "ok":
    case "connected":
    case "normal":
    case "watch":
    case "buy":
    case "short_squeeze":
    case "true":
      return "badge-green";
    case "warning":
    case "degraded":
    case "yellow":
    case "likely":
      return "badge-yellow";
    case "alert":
    case "orange":
      return "badge-orange";
    case "error":
    case "extreme":
    case "active":
    case "disconnected":
    case "reconnecting":
      return "badge-red";
    case "none":
    case "disabled":
    case "neutral":
    case "balanced":
      return "badge-gray";
    default:
      return "badge-blue";
  }
}

function setBadge(id, label, tone) {
  const el = $(id);
  if (!el) return;
  el.textContent = label;
  el.className = `badge ${badgeClass(tone || label)}`;
}

function renderMetrics(items) {
  return `<div class="metric-grid">${items
    .map(
      ({ label, value }) => `
        <div class="metric">
          <div class="metric-label">${label}</div>
          <div class="metric-value">${value}</div>
        </div>`
    )
    .join("")}</div>`;
}

function renderKeyValueGrid(items) {
  return renderMetrics(items);
}

function renderReasons(reasons = []) {
  if (!reasons.length) {
    return `<div class="muted">No reasons</div>`;
  }
  return `<div class="reason-list">${reasons
    .map((reason) => `<span class="chip">${reason}</span>`)
    .join("")}</div>`;
}

function renderSignalChip(label, tone = "gray") {
  return `<span class="signal-chip signal-chip-${escapeHtml(tone)}">${escapeHtml(label)}</span>`;
}

function renderSignalChipRow(chips = []) {
  if (!chips.length) {
    return "";
  }
  return `<div class="signal-chip-row">${chips.join("")}</div>`;
}

function downloadJsonFile(filename, payload) {
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

function severityRank(severity) {
  switch ((severity || "").toString().toLowerCase()) {
    case "critical":
    case "high":
      return 3;
    case "medium":
    case "moderate":
      return 2;
    case "low":
      return 1;
    default:
      return 0;
  }
}

function severityTone(severity) {
  switch ((severity || "").toString().toLowerCase()) {
    case "critical":
    case "high":
      return "danger";
    case "medium":
    case "moderate":
      return "warning";
    case "low":
      return "muted";
    default:
      return "neutral";
  }
}

function monitorQualityTone(status) {
  switch ((status || "").toString().toLowerCase()) {
    case "healthy":
      return "healthy";
    case "degraded":
      return "degraded";
    case "stale":
      return "stale";
    default:
      return "no-data";
  }
}

function previewStatusTone(status) {
  switch ((status || "").toString().toLowerCase()) {
    case "notify_candidate":
      return "success";
    case "review_candidate":
      return "warning";
    case "suppressed_no_trade_only":
      return "danger";
    case "not_enough_data":
      return "muted";
    case "not_found":
      return "muted";
    default:
      return "neutral";
  }
}

function yesNoTone(value) {
  return value ? "success" : "muted";
}

function sortRecentSignalItems(items, mode) {
  return [...items].sort((a, b) => {
    if ((mode || "").toLowerCase() === "newest") {
      return (b.historyRecordedAtMs || 0) - (a.historyRecordedAtMs || 0);
    }
    if ((mode || "").toLowerCase() === "symbol") {
      return String(a.symbol || "").localeCompare(String(b.symbol || ""))
        || severityRank(b.severity) - severityRank(a.severity)
        || (b.historyRecordedAtMs || 0) - (a.historyRecordedAtMs || 0);
    }
    return severityRank(b.severity) - severityRank(a.severity)
      || (b.confidence || 0) - (a.confidence || 0)
      || (b.historyRecordedAtMs || 0) - (a.historyRecordedAtMs || 0);
  });
}

function sortGroupHistoryItems(items, mode) {
  return [...items].sort((a, b) => {
    if ((mode || "").toLowerCase() === "newest") {
      return (b.lastSeenAtMs || 0) - (a.lastSeenAtMs || 0);
    }
    if ((mode || "").toLowerCase() === "count") {
      return (b.count || 0) - (a.count || 0)
        || severityRank(b.maxSeverity) - severityRank(a.maxSeverity)
        || (b.lastSeenAtMs || 0) - (a.lastSeenAtMs || 0);
    }
    return severityRank(b.maxSeverity) - severityRank(a.maxSeverity)
      || (b.count || 0) - (a.count || 0)
      || (b.lastSeenAtMs || 0) - (a.lastSeenAtMs || 0);
  });
}

function sortAlertHistoryItems(items, mode) {
  return [...items].sort((a, b) => {
    if ((mode || "").toLowerCase() === "newest") {
      return (b.historyRecordedAtMs || 0) - (a.historyRecordedAtMs || 0);
    }
    if ((mode || "").toLowerCase() === "status") {
      return String(a.previewStatus || "").localeCompare(String(b.previewStatus || ""))
        || (b.historyRecordedAtMs || 0) - (a.historyRecordedAtMs || 0);
    }
    return previewStatusTone(a.previewStatus).localeCompare(previewStatusTone(b.previewStatus))
      || (b.historyRecordedAtMs || 0) - (a.historyRecordedAtMs || 0);
  });
}

function renderTable(headers, rows) {
  return `<div class="table-wrap"><table><thead><tr>${headers
    .map((header) => `<th>${header}</th>`)
    .join("")}</tr></thead><tbody>${
      rows.length
        ? rows
            .map(
              (row) =>
                `<tr>${row.map((cell) => `<td>${cell ?? "Unavailable"}</td>`).join("")}</tr>`
            )
            .join("")
        : `<tr><td colspan="${headers.length}" class="muted">Unavailable</td></tr>`
    }</tbody></table></div>`;
}

function formatAgeFromNow(ts) {
  if (!ts) {
    return "Unavailable";
  }
  const deltaMs = Math.max(0, Date.now() - Number(ts));
  if (deltaMs < 1000) {
    return "just now";
  }
  if (deltaMs < 60_000) {
    return `${Math.round(deltaMs / 1000)}s ago`;
  }
  if (deltaMs < 3_600_000) {
    return `${Math.round(deltaMs / 60_000)}m ago`;
  }
  return `${Math.round(deltaMs / 3_600_000)}h ago`;
}

async function fetchJson(url) {
  const response = await fetch(url, { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`${url} -> ${response.status}`);
  }
  return response.json();
}

async function postJson(url, body = null) {
  const response = await fetch(url, {
    method: "POST",
    cache: "no-store",
    headers: { "Content-Type": "application/json" },
    body: body === null ? null : JSON.stringify(body),
  });
  if (!response.ok) {
    throw new Error(`${url} -> ${response.status}`);
  }
  return response.json();
}

function rememberFetchSuccess(url, data) {
  state.data[url] = { ok: true, data };
}

function rememberFetchFailure(url, error) {
  state.data[url] = {
    ok: false,
    error: error.message,
    data: state.data[url]?.data,
  };
}

async function refreshGroup(urls) {
  const entries = await Promise.all(
    urls.map(async (url) => {
      try {
        return [url, { ok: true, data: await fetchJson(url) }];
      } catch (error) {
        return [
          url,
          {
            ok: false,
            error: error.message,
            data: state.data[url]?.data,
          },
        ];
      }
    })
  );

  for (const [url, payload] of entries) {
    state.data[url] = payload;
  }
}

async function refreshActiveTradeToxicity() {
  const url = activeTradeToxicityUrl();
  const statusUrl = activeTradeToxicityStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[url] = { ok: false, error: error.message };
  }
  renderActiveTradeToxicity();
}

async function refreshLiquidationToxicity() {
  await refreshGroup([
    "/api/toxicity/liquidation/status",
    "/api/toxicity/liquidation/recent",
  ]);
  renderLiquidationToxicity();
}

async function refreshOrderbookWallLifecycle() {
  const url = orderbookWallRecentUrl();
  const statusUrl = orderbookWallStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[url] = { ok: false, error: error.message };
  }
  renderOrderbookWallLifecycle();
}

async function refreshOrderbookWallInterpretation() {
  const url = orderbookWallInterpretationRecentUrl();
  const statusUrl = orderbookWallInterpretationStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[url] = { ok: false, error: error.message };
  }
  renderOrderbookWallInterpretation();
}

async function refreshStructuralToxicity() {
  const url = structuralToxicityRecentUrl();
  const statusUrl = structuralToxicityStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[url] = { ok: false, error: error.message };
  }
  renderStructuralToxicity();
}

async function refreshWhaleFlowMonitor() {
  const url = whaleFlowRecentUrl();
  const statusUrl = whaleFlowStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[url] = { ok: false, error: error.message };
  }
  renderWhaleFlowMonitor();
  renderWhaleFlowCompactMode();
}

async function refreshVenueDiagnostics() {
  try {
    state.data["/api/venues/diagnostics"] = {
      ok: true,
      data: await fetchJson("/api/venues/diagnostics"),
    };
    state.latestVenueDiagnosticsAction = "Venue diagnostics refreshed.";
  } catch (error) {
    state.data["/api/venues/diagnostics"] = { ok: false, error: error.message };
    state.latestVenueDiagnosticsAction = `Venue diagnostics refresh failed: ${error.message}`;
  }
  renderVenueStreamDiagnostics();
}

async function refreshWhaleFlowCalibration() {
  const url = whaleFlowCalibrationReportUrl();
  const statusUrl = whaleFlowCalibrationStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[url] = { ok: false, error: error.message };
  }
  renderWhaleFlowCalibration();
  renderWhaleFlowCompactMode();
}

async function refreshWhaleFlowCandidateHistory() {
  const url = whaleFlowCandidateHistoryRecentUrl();
  const statusUrl = whaleFlowCandidateHistoryStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[url] = { ok: false, error: error.message };
  }
  renderWhaleFlowCandidateHistory();
  renderWhaleFlowCompactMode();
}

async function refreshToxicSignalFusion() {
  const url = toxicSignalFusionRecentUrl();
  const statusUrl = toxicSignalFusionStatusUrl();
  try {
    rememberFetchSuccess(statusUrl, await fetchJson(statusUrl));
    rememberFetchSuccess(url, await fetchJson(url));
  } catch (error) {
    rememberFetchFailure(statusUrl, error);
    rememberFetchFailure(url, error);
  }
  renderToxicSignalFusion();
}

async function refreshToxicSignalInbox() {
  const url = toxicSignalInboxRecentUrl();
  const statusUrl = toxicSignalInboxStatusUrl();
  try {
    rememberFetchSuccess(statusUrl, await fetchJson(statusUrl));
    rememberFetchSuccess(url, await fetchJson(url));
  } catch (error) {
    rememberFetchFailure(statusUrl, error);
    rememberFetchFailure(url, error);
  }
  renderToxicSignalInbox();
}

async function refreshToxicSignalGroups() {
  const url = toxicSignalGroupsRecentUrl();
  const statusUrl = toxicSignalGroupsStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[url] = { ok: false, error: error.message };
  }
  renderToxicSignalGroups();
}

async function refreshToxicSignalDetail() {
  const statusUrl = toxicSignalDetailStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
  }
  renderToxicSignalDetail();
}

async function refreshToxicSignalHistory() {
  const recentUrl = toxicSignalHistoryRecentUrl();
  const statusUrl = toxicSignalHistoryStatusUrl();
  const alertsUrl = toxicSignalHistoryAlertsUrl();
  const reportsUrl = toxicSignalHistoryReportsUrl();
  const lookupUrl = toxicSignalHistorySignalUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[recentUrl] = { ok: true, data: await fetchJson(recentUrl) };
    state.data[alertsUrl] = { ok: true, data: await fetchJson(alertsUrl) };
    state.data[reportsUrl] = { ok: true, data: await fetchJson(reportsUrl) };
    if (lookupUrl) {
      state.toxicSignalHistoryLookupPayload = await fetchJson(lookupUrl);
    } else {
      state.toxicSignalHistoryLookupPayload = null;
    }
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[recentUrl] = { ok: false, error: error.message };
    state.data[alertsUrl] = { ok: false, error: error.message };
    state.data[reportsUrl] = { ok: false, error: error.message };
    if (lookupUrl) {
      state.toxicSignalHistoryLookupPayload = {
        found: false,
        source: "signal_history",
        retentionMode: "in_memory_bounded",
        reason: error.message,
      };
    }
  }
  renderToxicSignalHistory();
}

async function refreshToxicSignalHealth() {
  const summaryUrl = toxicSignalHealthSummaryUrl();
  const statusUrl = toxicSignalHealthStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[summaryUrl] = { ok: true, data: await fetchJson(summaryUrl) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[summaryUrl] = { ok: false, error: error.message };
  }
  renderToxicSignalHealth();
}

async function refreshToxicSignalReport() {
  const url = toxicSignalReportDailyUrl();
  const statusUrl = toxicSignalReportStatusUrl();
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[url] = { ok: false, error: error.message };
  }
  renderToxicSignalReport();
}

async function refreshToxicSignalRolling() {
  const url = toxicSignalReportRollingUrl();
  try {
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[url] = { ok: false, error: error.message };
  }
  renderToxicSignalRolling();
}

async function refreshToxicSignalAlertPreview() {
  const url = toxicSignalAlertPreviewRecentUrl();
  const statusUrl = toxicSignalAlertPreviewStatusUrl();
  const explainUrl = toxicSignalAlertPreviewExplainUrl();
  if (!explainUrl) {
    state.toxicSignalAlertExplainPayload = null;
  }
  try {
    state.data[statusUrl] = { ok: true, data: await fetchJson(statusUrl) };
    state.data[url] = { ok: true, data: await fetchJson(url) };
    if (explainUrl) {
      state.toxicSignalAlertExplainPayload = await fetchJson(explainUrl);
    }
  } catch (error) {
    state.data[statusUrl] = { ok: false, error: error.message };
    state.data[url] = { ok: false, error: error.message };
    if (explainUrl) {
      state.toxicSignalAlertExplainPayload = {
        found: false,
        alertDecision: "not_found",
        reason: error.message,
      };
    }
  }
  renderToxicSignalAlertPreview();
}

async function refreshDurableArchiveDryRun() {
  const url = durableArchiveDryRunUrl();
  try {
    state.data[url] = { ok: true, data: await postJson(url) };
  } catch (error) {
    state.data[url] = { ok: false, error: error.message };
  }
  renderDurableArchiveDryRun();
}

async function refreshDurableArchiveDryRunReviewPack() {
  const url = durableArchiveDryRunReviewPackLatestUrl();
  try {
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[url] = { ok: false, error: error.message };
  }
  renderDurableArchiveDryRunReviewPack();
}

async function refreshDurableArchiveWriteGate() {
  const url = durableArchiveWriteGateStatusUrl();
  try {
    state.data[url] = { ok: true, data: await fetchJson(url) };
  } catch (error) {
    state.data[url] = { ok: false, error: error.message };
  }
  renderDurableArchiveWriteGate();
}

async function refreshDurableArchiveWriteAudit() {
  const urls = [
    durableArchiveWriteAuditStatusUrl(),
    durableArchiveWriteAuditRecentUrl(),
    durableArchiveWriteAuditLatestUrl(),
  ];
  await Promise.all(
    urls.map(async (url) => {
      try {
        state.data[url] = { ok: true, data: await fetchJson(url) };
      } catch (error) {
        state.data[url] = { ok: false, error: error.message };
      }
    })
  );
  renderDurableArchiveWriteAudit();
}

async function loadLatestDurableArchiveWriteAttempt() {
  const url = durableArchiveWriteAuditLatestUrl();
  try {
    state.data[url] = { ok: true, data: await fetchJson(url) };
    state.latestDurableArchiveWriteAuditAction =
      "Latest write attempt preview loaded.";
  } catch (error) {
    state.data[url] = { ok: false, error: error.message };
    state.latestDurableArchiveWriteAuditAction =
      `Latest write attempt preview failed: ${error.message}`;
  }
  renderDurableArchiveWriteAudit();
}

async function refreshToxicReplay() {
  await refreshGroup([
    "/api/toxicity/replay/status",
    "/api/toxicity/replay/recent",
  ]);
  renderToxicReplay();
}

async function refreshToxicMarkout() {
  await refreshGroup([
    "/api/toxicity/markout/status",
    "/api/toxicity/markout/recent",
  ]);
  renderToxicMarkout();
}

async function refreshToxicQualityScorecard() {
  await refreshGroup([
    "/api/toxicity/quality-scorecard/status",
    "/api/toxicity/quality-scorecard/summary",
  ]);
  renderToxicQualityScorecard();
}

async function refreshToxicWeightRecommendation() {
  await refreshGroup([
    "/api/toxicity/weight-recommendation/status",
    "/api/toxicity/weight-recommendation/summary",
  ]);
  renderToxicWeightRecommendation();
}

async function refreshToxicWeightReview() {
  await refreshGroup([
    "/api/toxicity/weight-review/status",
    "/api/toxicity/weight-review/summary",
  ]);
  renderToxicWeightReview();
}

async function refreshToxicGovernanceLedger() {
  await refreshGroup([
    "/api/toxicity/governance-ledger/status",
    "/api/toxicity/governance-ledger/recent",
  ]);
  renderToxicGovernanceLedger();
}

async function refreshToxicGovernanceProposal() {
  await refreshGroup([
    "/api/toxicity/governance-proposal/status",
    "/api/toxicity/governance-proposal/summary",
  ]);
  renderToxicGovernanceProposal();
}

async function refreshToxicGovernanceReviewPack() {
  await refreshGroup([
    "/api/toxicity/governance-review-pack/status",
    "/api/toxicity/governance-review-pack/summary",
  ]);
  renderToxicGovernanceReviewPack();
}

async function refreshToxicGovernanceSignoffPack() {
  await refreshGroup([
    "/api/toxicity/governance-signoff-pack/status",
    "/api/toxicity/governance-signoff-pack/summary",
  ]);
  renderToxicGovernanceSignoffPack();
}

function getData(url) {
  return state.data[url]?.data;
}

function getError(url) {
  return state.data[url] && !state.data[url].ok ? state.data[url].error : null;
}

function strongestToxicResult(toxicState) {
  if (!toxicState?.results) return null;
  return Object.values(toxicState.results).sort(
    (a, b) => (b.toxicVolumeBtc || 0) - (a.toxicVolumeBtc || 0)
  )[0];
}

function renderOperatorHomeSummary() {
  const content = $("operatorHomeContent");
  if (!content) {
    return;
  }

  const status = getData("/api/status");
  const fusionStatus = getData("/api/toxicity/fusion/status");
  const fusionRecent = getData("/api/toxicity/fusion/recent");
  const replayStatus = getData("/api/toxicity/replay/status");
  const markoutStatus = getData("/api/toxicity/markout/status");
  const qualitySummary = getData("/api/toxicity/quality-scorecard/summary");
  const recommendationSummary = getData("/api/toxicity/weight-recommendation/summary");
  const reviewSummary = getData("/api/toxicity/weight-review/latest");
  const ledgerStatus = getData("/api/toxicity/governance-ledger/status");
  const manualGovernance = getData("/api/calibration/manual-governance/index");
  const error =
    getError("/api/status") ||
    getError("/api/toxicity/fusion/status") ||
    getError("/api/toxicity/fusion/recent") ||
    getError("/api/toxicity/replay/status") ||
    getError("/api/toxicity/markout/status") ||
    getError("/api/toxicity/quality-scorecard/summary") ||
    getError("/api/toxicity/weight-recommendation/summary") ||
    getError("/api/toxicity/weight-review/latest") ||
    getError("/api/toxicity/governance-ledger/status") ||
    getError("/api/calibration/manual-governance/index");

  if (error) {
    setBadge("operatorHomeBadge", "API Error", "error");
    content.innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!status) {
    setBadge("operatorHomeBadge", "Loading", "none");
    content.innerHTML =
      `<div class="muted">Operator home summary will appear after status and toxicity evidence load.</div>`;
    return;
  }

  const runtimeControl = status.runtimeControl || {};
  const startState =
    runtimeControl.startState || (runtimeControl.monitoringStarted ? "started" : "stopped");
  const fusionSignals = fusionRecent?.signals || [];
  const governanceGate = manualGovernance?.finalGate || "Unavailable";
  const safetyOk =
    Boolean(status.readOnly) &&
    Boolean(fusionStatus?.readOnly ?? true) &&
    !Boolean(fusionStatus?.runtimeModified) &&
    !Boolean(fusionStatus?.executionEnabled) &&
    !Boolean(recommendationSummary?.runtimeWeightModified) &&
    !Boolean(recommendationSummary?.configModified);
  const operatorTone = startState === "failed" ? "error" : safetyOk ? "ok" : "warning";

  setBadge(
    "operatorHomeBadge",
    safetyOk ? "Safety Locked" : "Needs Review",
    operatorTone
  );
  content.innerHTML =
    renderMetrics([
      { label: "Runtime State", value: startState },
      { label: "Monitoring Started", value: formatBool(Boolean(runtimeControl.monitoringStarted)) },
      { label: "Symbol", value: status.symbol || "Unavailable" },
      { label: "Toxic Fusion Signals", value: formatInteger(fusionStatus?.signalCount ?? fusionSignals.length) },
      { label: "Fusion Mode", value: fusionStatus?.mode || fusionRecent?.mode || "analysis_only" },
      { label: "Replay Signals", value: formatInteger(replayStatus?.signalCount) },
      { label: "Markout Signals", value: formatInteger(markoutStatus?.signalCount) },
      { label: "Quality Evaluations", value: formatInteger(qualitySummary?.totalEvaluations) },
      { label: "Weight Recommendations", value: formatInteger(recommendationSummary?.totalRecommendations) },
      { label: "Review Items", value: formatInteger(reviewSummary?.totalItems) },
      { label: "Governance Ledger", value: ledgerStatus?.status || "Unavailable" },
      { label: "Manual Governance Gate", value: governanceGate },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshOperatorHomeButton">Refresh Operator Home</button>
    </div>` +
    `<div class="muted">Read-only. Analysis only. Monitoring only. Manual review required. Recommendation only. No order placement. No cancel/amend. No wallet/signing. No live trading. No transaction construction. No runtime config mutation.</div>` +
    `<div class="muted">Operator Home Summary aggregates existing status only; it does not change runtime config, weights, or trading state.</div>`;
}

function renderSystemHealth() {
  const status = getData("/api/status");
  const storage = getData("/api/storage/status");
  const error = getError("/api/status") || getError("/api/storage/status");
  if (error) {
    setBadge("systemHealthBadge", "API Error", "error");
    $("systemHealthContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const activeVenues = Object.entries(status?.venues || {})
    .filter(([, venue]) => venue.status === "connected")
    .map(([name]) => name)
    .join(", ") || "None";
  const storageStatus = storage?.status || status?.storage?.status || "Unavailable";
  const runtimeControl = status?.runtimeControl || {};

  setBadge("systemHealthBadge", status?.readOnly ? "Read Only" : "Unsafe", status?.readOnly ? "ok" : "error");
  $("systemHealthContent").innerHTML = renderMetrics([
    { label: "Read Only", value: formatBool(Boolean(status?.readOnly)) },
    {
      label: "Monitoring Started",
      value: formatBool(Boolean(runtimeControl.monitoringStarted)),
    },
    { label: "Storage", value: storageStatus },
    { label: "Telegram", value: formatBool(Boolean(status?.alerts?.telegramEnabled)) },
    { label: "Active Venues", value: activeVenues },
    { label: "Last Sent Alert", value: formatTime(status?.alerts?.lastSentTs) },
    { label: "Snapshot Write", value: formatTime(status?.storage?.lastWriteTs) },
  ]);
}

function renderOperatorConsole() {
  const status = getData("/api/status");
  const error = getError("/api/status");
  const content = $("operatorConsoleContent");
  if (!content) {
    return;
  }
  if (error) {
    setBadge("operatorRuntimeBadge", "API Error", "error");
    content.innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const runtimeControl = status?.runtimeControl || {};
  const started = Boolean(runtimeControl.monitoringStarted);
  const startState = runtimeControl.startState || (started ? "started" : "stopped");
  const lastStartResult = runtimeControl.lastStartResult || "none";
  const lastStartError = runtimeControl.lastStartError || "None";
  const startAttemptCount = runtimeControl.startAttemptCount || 0;
  const lastStartAtMs = runtimeControl.lastStartAtMs || null;
  const stopState = runtimeControl.stopState || (started ? "stopped" : "stopped");
  const lastStopResult = runtimeControl.lastStopResult || "none";
  const lastStopError = runtimeControl.lastStopError || "None";
  const stopAttemptCount = runtimeControl.stopAttemptCount || 0;
  const startButton = $("oneClickStartButton");
  if (startButton) {
    startButton.textContent =
      startState === "starting"
        ? "启动中..."
        : startState === "started"
          ? "监控中"
          : startState === "failed"
            ? "重新开始监控"
            : "开始监控订单流";
    startButton.disabled = startState === "starting" || startState === "started";
  }
  const operatorStopButton = $("operatorStopButton");
  if (operatorStopButton) {
    operatorStopButton.textContent =
      stopState === "stopping"
        ? "停止中..."
        : !started && stopState === "stopped"
          ? "已停止"
          : stopState === "failed"
            ? "重新停止"
            : "停止监控";
    operatorStopButton.disabled = stopState === "stopping" || (!started && stopState === "stopped");
  }
  const operatorRefreshButton = $("operatorRefreshButton");
  if (operatorRefreshButton) {
    operatorRefreshButton.disabled = startState === "starting" || stopState === "stopping";
  }
  const monitoringStartedStatus = $("monitoringStartedStatus");
  if (monitoringStartedStatus) {
    monitoringStartedStatus.textContent = `monitoringStarted=${formatBool(started)}`;
  }
  const runtimeActionStatus = $("runtimeActionStatus");
  if (runtimeActionStatus) {
    runtimeActionStatus.textContent = state.latestRuntimeAction
      ? `runtimeState=${state.latestRuntimeAction}`
      : `runtimeState=${startState}`;
  }

  setBadge(
    "operatorRuntimeBadge",
    startState.toUpperCase(),
    startState === "started"
      ? "ok"
      : startState === "starting"
        ? "warning"
        : startState === "failed"
          ? "error"
          : "gray"
  );
  const lastStartResultLabel =
    lastStartResult === "already_started"
      ? "already_started"
      : lastStartResult === "started"
        ? "started"
        : lastStartResult === "failed"
        ? "failed"
          : "none";
  const lastStopResultLabel =
    lastStopResult === "already_stopped"
      ? "already_stopped"
      : lastStopResult === "stopped"
        ? "stopped"
        : lastStopResult === "failed"
          ? "failed"
          : "none";
  const runtimeStatusClass =
    startState === "started" ? "badge-green" :
    startState === "starting" ? "badge-yellow" :
    startState === "failed" ? "badge-red" :
    "badge-gray";
  const stopStatusClass =
    stopState === "stopped" ? "badge-gray" :
    stopState === "stopping" ? "badge-yellow" :
    stopState === "failed" ? "badge-red" :
    "badge-gray";
  const stopButtonLabel =
    stopState === "stopping"
      ? "停止中..."
      : !started && stopState === "stopped"
        ? "已停止"
        : stopState === "failed"
          ? "重新停止"
          : "停止监控";
  const actionHtml = started || stopState === "failed"
    ? `<div class="action-row">
        <button type="button" class="small-button button-danger" id="operatorStopButton"${stopState === "stopping" || (!started && stopState === "stopped") ? " disabled" : ""}>${stopButtonLabel}</button>
        <button type="button" class="small-button" id="operatorRefreshButton"${startState === "starting" || stopState === "stopping" ? " disabled" : ""}>刷新列表</button>
      </div>`
    : "";
  content.innerHTML =
    `<div class="operator-status">
      <span class="badge ${runtimeStatusClass}">${startState.toUpperCase()}</span>
      <span class="badge ${stopStatusClass}">${stopState.toUpperCase()}</span>
    </div>` +
    `<div class="operator-status-line">
      <span class="status-code">monitoringStarted=${formatBool(started)}</span>
      <span class="status-code">startState=${escapeHtml(startState)}</span>
      <span class="status-code">stopState=${escapeHtml(stopState)}</span>
    </div>` +
    `<div class="muted">点击页面顶部“一键启动”后才会连接交易所公开成交与盘口流。Dashboard 只读，不会下单、撤单、签名、apply、reload。</div>` +
    `<div class="muted">最近一次 start result=${escapeHtml(lastStartResultLabel)}; attempts=${formatInteger(startAttemptCount)}; lastStartAt=${escapeHtml(formatDateTime(lastStartAtMs))}; lastError=${escapeHtml(lastStartError === "None" ? lastStopError : lastStartError)}</div>` +
    actionHtml +
    (state.latestRuntimeAction
      ? `<div class="muted">${escapeHtml(state.latestRuntimeAction)}</div>`
      : "");
}

function renderSuspiciousToxicOrders() {
  const content = $("suspiciousToxicOrdersContent");
  if (!content) {
    return;
  }

  const status = getData("/api/status");
  const inboxPayload = getToxicSignalInboxPayload();
  const fusionPayload = getToxicSignalFusionPayload();
  const inboxError = getError(toxicSignalInboxRecentUrl()) || getError("/api/toxicity/signal-inbox/recent");
  const fusionError = getError(toxicSignalFusionRecentUrl()) || getError("/api/toxicity/fusion/recent");
  const statusError = getError("/api/status");
  const error = statusError || inboxError || fusionError;
  const hasUsableSnapshot = Boolean(status) && Boolean(inboxPayload || fusionPayload);
  if (error && !hasUsableSnapshot) {
    setBadge("suspiciousToxicOrdersBadge", "API Error", "error");
    content.innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const started = Boolean(status?.runtimeControl?.monitoringStarted);
  const liveItems = currentSuspiciousToxicOrderItems();
  const canRefreshSnapshot = !inboxError && !fusionError;
  if (canRefreshSnapshot) {
    syncSuspiciousOrdersLastSeen(liveItems);
  } else {
    pruneSuspiciousOrdersLastSeen();
  }
  const allItems = getSuspiciousToxicOrderItems();
  const items = suspiciousOrdersVisibleItems(allItems);
  const liveVisibleItems = suspiciousOrdersVisibleItems(
    allItems.filter((item) => item.snapshotState !== "stale")
  );
  const staleVisibleItems = items.filter((item) => item.snapshotState === "stale");
  const maxVisibleSeverityRank = items.reduce(
    (max, item) => Math.max(max, suspiciousSeverityRank(item.severity)),
    0
  );
  const hasFilters =
    suspiciousOrdersFilterSymbol() ||
    suspiciousOrdersFilterAlertDecision() ||
    Boolean(state.suspiciousOrdersHideNotEnoughData) ||
    Boolean(state.suspiciousOrdersHighSeverityOnly) ||
    suspiciousOrdersSortMode() !== "severity";

  setBadge(
    "suspiciousToxicOrdersBadge",
    allItems.length
      ? liveVisibleItems.length
        ? `${liveVisibleItems.length}/${allItems.length} MATCHES`
        : staleVisibleItems.length
          ? `${staleVisibleItems.length} RECENT`
          : items.length
            ? `${items.length}/${allItems.length} MATCHES`
        : hasFilters
          ? "FILTERED"
          : started
            ? `${allItems.length} SUSPICIOUS`
            : `${allItems.length} CACHED`
      : started
        ? "WATCHING"
        : "WAITING",
    items.length ? (maxVisibleSeverityRank >= 4 ? "alert" : "warning") : started ? "ok" : "gray"
  );

  const summaryText = suspiciousOrdersSummaryText();
  const emptyState =
    !started && !allItems.length
      ? `<div class="suspicious-empty">
          <div>
            <div class="metric-value">监控尚未开始</div>
            <div class="muted">点击“开始监控订单流”后，系统会监听交易所公开订单流并在这里展示可疑信号。</div>
          </div>
        </div>`
      : started && !allItems.length
        ? `<div class="suspicious-empty">
            <div>
              <div class="metric-value">暂无可疑有毒订单</div>
              <div class="muted">系统正在监听公开成交与盘口流；出现可疑有毒订单流信号后会显示在这里。</div>
            </div>
          </div>`
        : started && !liveVisibleItems.length && staleVisibleItems.length
          ? `<div class="suspicious-empty">
              <div>
                <div class="metric-value">暂无当前 live 信号</div>
                <div class="muted">下面显示最近 5 分钟内观察到的历史候选，已标记为 stale / last seen。</div>
              </div>
            </div>`
        : !items.length
          ? `<div class="suspicious-empty">
              <div>
                <div class="metric-value">No matches</div>
                <div class="muted">当前筛选条件没有匹配项，清空筛选或重置排序后可恢复列表。</div>
              </div>
            </div>`
          : "";

  content.innerHTML = `
    <div class="suspicious-list">
      <div class="muted suspicious-summary">${escapeHtml(summaryText)}</div>
      <div class="suspicious-controls">
        <div class="control-grid">
          <label class="form-field">
            <span>Sort by</span>
            <select id="suspiciousOrdersSortSelect" class="signal-sort-select">
              <option value="severity"${suspiciousOrdersSortMode() === "severity" ? " selected" : ""}>Severity</option>
              <option value="confidence"${suspiciousOrdersSortMode() === "confidence" ? " selected" : ""}>Confidence</option>
              <option value="createdAtMs"${suspiciousOrdersSortMode() === "createdAtMs" ? " selected" : ""}>CreatedAt</option>
            </select>
          </label>
          <label class="form-field">
            <span>Symbol filter</span>
            <input id="suspiciousOrdersFilterSymbolInput" placeholder="symbol" value="${escapeHtml(state.suspiciousOrdersFilterSymbol || "")}" />
          </label>
          <label class="form-field">
            <span>AlertDecision filter</span>
            <input id="suspiciousOrdersFilterAlertDecisionInput" placeholder="alertDecision" value="${escapeHtml(state.suspiciousOrdersFilterAlertDecision || "")}" />
          </label>
          <label class="checkbox-field">
            <input type="checkbox" id="suspiciousOrdersHideNotEnoughDataCheckbox"${state.suspiciousOrdersHideNotEnoughData ? " checked" : ""} />
            <span>Hide not_enough_data</span>
          </label>
          <label class="checkbox-field">
            <input type="checkbox" id="suspiciousOrdersHighSeverityOnlyCheckbox"${state.suspiciousOrdersHighSeverityOnly ? " checked" : ""} />
            <span>High severity only</span>
          </label>
        </div>
        <div class="action-row">
          <button type="button" class="small-button" id="clearSuspiciousOrdersFilterButton">Clear Filter</button>
          <button type="button" class="small-button" id="resetSuspiciousOrdersSortButton">Reset Sort</button>
        </div>
      </div>
      ${emptyState ||
        `<div class="suspicious-list">
          ${!started ? `<div class="muted">监控未启动，以下为历史/缓存中的可疑有毒订单，仅供只读审阅。</div>` : ""}
          ${items.map((item) => renderSuspiciousToxicOrderItem(item)).join("")}
        </div>`}
      ${error && hasUsableSnapshot
        ? `<div class="muted">Latest refresh failed: ${escapeHtml(error)}. Showing the last successful suspicious-order snapshot.</div>`
        : ""}
      ${state.latestSuspiciousOrdersAction
        ? `<div class="muted">${escapeHtml(state.latestSuspiciousOrdersAction)}</div>`
        : ""}
    </div>`;
}

function updateSuspiciousOrdersViewStateFromControls() {
  const sortSelect = $("suspiciousOrdersSortSelect");
  const symbolInput = $("suspiciousOrdersFilterSymbolInput");
  const alertDecisionInput = $("suspiciousOrdersFilterAlertDecisionInput");
  const hideNotEnoughDataCheckbox = $("suspiciousOrdersHideNotEnoughDataCheckbox");
  const highSeverityOnlyCheckbox = $("suspiciousOrdersHighSeverityOnlyCheckbox");

  state.suspiciousOrdersSortMode = sortSelect?.value || "severity";
  state.suspiciousOrdersFilterSymbol = symbolInput?.value?.trim() || "";
  state.suspiciousOrdersFilterAlertDecision = alertDecisionInput?.value?.trim() || "";
  state.suspiciousOrdersHideNotEnoughData = Boolean(hideNotEnoughDataCheckbox?.checked);
  state.suspiciousOrdersHighSeverityOnly = Boolean(highSeverityOnlyCheckbox?.checked);
  state.latestSuspiciousOrdersAction = `sort=${state.suspiciousOrdersSortMode}; symbol=${
    state.suspiciousOrdersFilterSymbol || "ALL"
  }; alertDecision=${state.suspiciousOrdersFilterAlertDecision || "ALL"}; hideNotEnoughData=${formatBool(
    state.suspiciousOrdersHideNotEnoughData
  )}; highSeverityOnly=${formatBool(state.suspiciousOrdersHighSeverityOnly)}`;
}

function clearSuspiciousOrdersFilters() {
  state.suspiciousOrdersFilterSymbol = "";
  state.suspiciousOrdersFilterAlertDecision = "";
  state.suspiciousOrdersHideNotEnoughData = false;
  state.suspiciousOrdersHighSeverityOnly = false;
  state.latestSuspiciousOrdersAction = "Suspicious order filters cleared.";
  renderSuspiciousToxicOrders();
}

function resetSuspiciousOrdersSort() {
  state.suspiciousOrdersSortMode = "severity";
  state.latestSuspiciousOrdersAction = "Suspicious order sort reset to severity.";
  renderSuspiciousToxicOrders();
}

function flowDirection(netAggressiveBtc) {
  if (netAggressiveBtc > 0) {
    return "BUY";
  }
  if (netAggressiveBtc < 0) {
    return "SELL";
  }
  return "FLAT";
}

function venueStatusTone(status) {
  switch ((status || "").toString().toLowerCase()) {
    case "connected":
      return "success";
    case "reconnecting":
      return "warning";
    case "configuration_error":
    case "disconnected":
      return "danger";
    default:
      return "muted";
  }
}

function venueStatusSummary(venue) {
  const status = (venue?.status || "unknown").toString().toLowerCase();
  if (status === "disabled") {
    const source = venue?.enableSource ? `source=${venue.enableSource}` : "source=unknown";
    const reason = ["env_flag_missing_or_false", "env_or_config_flag_false"].includes(venue?.disabledReason)
      ? `${venue?.enableFlagName || "ENABLE_FLAG"} not true in current process (${source})`
      : venue?.disabledReason || "disabled";
    return `${venue?.venue || "venue"} disabled: ${reason}`;
  }
  if (venue?.lastError) {
    return `${venue?.venue || "venue"} ${status} / lastError=${venue.lastError}`;
  }
  return `${venue?.venue || "venue"} ${status} / trade ${formatTime(venue?.lastTradeTs)}`;
}

function renderMonitorFlow() {
  const content = $("monitorFlowContent");
  if (!content) {
    return;
  }

  const flow = getData("/api/flow-state");
  const status = getData("/api/status");
  const error = getError("/api/flow-state") || getError("/api/status");
  const hasUsableSnapshot = Boolean(flow) && Boolean(status);
  if (error && !hasUsableSnapshot) {
    setBadge("monitorFlowBadge", "API Error", "error");
    content.innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const started = Boolean(status?.runtimeControl?.monitoringStarted);
  const windows = Object.values(flow?.windows || {}).sort(
    (a, b) => (a.windowMs || 0) - (b.windowMs || 0)
  );
  const venues = Object.entries(status?.venues || {});
  const connectedVenues = venues.filter(([, venue]) => venue.status === "connected");
  const marketDataQuality = status?.marketDataQuality || {};
  const qualityStatus = (marketDataQuality.status || "no_data").toString();
  const laggedEvents = Number(marketDataQuality.recentLaggedEvents || 0);
  const historicalLaggedEvents = Number(marketDataQuality.historicalLaggedEvents || 0);
  const droppedEvents = Number(marketDataQuality.eventBusDroppedEvents || 0);
  const lastTradeTs = venues.reduce(
    (max, [, venue]) => Math.max(max, Number(venue?.lastTradeTs || 0)),
    0
  );
  const lastMessageTs = Number(marketDataQuality.lastMessageTs || 0);
  const updatedAt = Number(marketDataQuality.flowUpdatedAt || flow?.updatedAt || 0);
  const latestBookForQuality = Number(marketDataQuality.latestBookTs || 0);
  const lagSources = Array.isArray(marketDataQuality.lagSources) ? marketDataQuality.lagSources : [];
  const degradedReason = marketDataQuality.degradedReason || "none";
  const recentWindow = windows[0] || null;
  const anyTrades = windows.some((window) => Number(window.tradeCount || 0) > 0);
  const activeVenueNames = [...new Set(
    windows.flatMap((window) => window.dataQuality?.activeVenues || [])
  )];

  let badgeLabel = "IDLE";
  let badgeTone = "gray";
  if (["degraded", "stale"].includes(qualityStatus)) {
    badgeLabel = qualityStatus.toUpperCase();
    badgeTone = "warning";
  } else if (qualityStatus === "no_data") {
    badgeLabel = started ? "NO DATA" : "IDLE";
    badgeTone = "gray";
  } else if (started) {
    badgeLabel = updatedAt ? "LIVE" : "WATCHING";
    badgeTone = updatedAt && Date.now() - updatedAt <= 10_000 ? "ok" : "warning";
  }
  setBadge("monitorFlowBadge", badgeLabel, badgeTone);

  const venueChips = venues.length
    ? renderSignalChipRow(
        venues.map(([name, venue]) =>
          renderSignalChip(
            venueStatusSummary({ ...venue, venue: name }),
            venueStatusTone(venue.status)
          )
        )
      )
    : `<div class="muted">暂无交易所连接状态。</div>`;

  const tableHtml = renderTable(
    ["窗口", "方向", "净主动 BTC", "绝对主动 BTC", "成交笔数", "价格变动 bps", "活跃交易所"],
    windows.map((window) => [
      `${Math.round((window.windowMs || 0) / 1000)}s`,
      flowDirection(Number(window.netAggressiveBtc || 0)),
      formatNumber(window.netAggressiveBtc),
      formatNumber(window.absAggressiveBtc),
      formatInteger(window.tradeCount),
      formatNumber(window.priceMoveBps, 1),
      escapeHtml((window.dataQuality?.activeVenues || []).join(", ") || "None"),
    ])
  );

  let hint = "点击开始监控订单流后，这里会持续显示公开流是否在刷新。";
  if (started && qualityStatus === "degraded") {
    hint = "数据质量降级，当前空列表不能直接理解为无有毒订单。";
  } else if (started && qualityStatus === "stale") {
    hint = "行情数据已变旧，当前空列表可能只是监控流没有及时刷新。";
  } else if (started && qualityStatus === "no_data") {
    hint = "监控已启动但流窗口尚未填充，当前空列表不能证明没有有毒订单。";
  } else if (started && anyTrades) {
    hint = "这里有实时成交/盘口窗口在刷新；如果上面还是空，更可能是暂时没有形成可疑有毒信号。";
  } else if (started) {
    hint = "监控已经在跑，但当前窗口内几乎没有成交或波动，暂时没抓到可疑单也可能只是行情偏平。";
  }
  const latestTradeForQuality = Number(marketDataQuality.latestTradeTs || lastTradeTs);
  const qualityWarning = marketDataQuality.operatorWarning
    ? `<div class="monitor-quality-warning">${escapeHtml(marketDataQuality.operatorWarning)}<br/>数据质量降级，当前空列表不能直接理解为无有毒订单。</div>`
    : "";
  const qualityTone = monitorQualityTone(qualityStatus);
  const qualityStrip = `
    <div class="monitor-quality-strip monitor-quality-${qualityTone}">
      <div class="monitor-quality-orb" aria-hidden="true"></div>
      <div class="monitor-quality-copy">
        <div class="monitor-quality-kicker">Market Data Quality</div>
        <div class="monitor-quality-title">${escapeHtml(qualityStatus)}</div>
      </div>
      <div class="monitor-quality-facts">
        <span>lag ${formatInteger(laggedEvents)}</span>
        <span>hist ${formatInteger(historicalLaggedEvents)}</span>
        <span>drop ${formatInteger(droppedEvents)}</span>
        <span>last ${escapeHtml(formatAgeFromNow(lastMessageTs))}</span>
      </div>
    </div>`;

  content.innerHTML = `
    <div class="monitor-flow-card">
      <div class="muted">${escapeHtml(hint)}</div>
      ${qualityStrip}
      ${qualityWarning}
      ${renderMetrics([
        { label: "Monitoring Started", value: formatBool(started) },
        { label: "Market Data Quality", value: escapeHtml(qualityStatus) },
        { label: "Symbol", value: escapeHtml(flow?.symbol || status?.symbol || "Unavailable") },
        { label: "Lagged Events", value: formatInteger(laggedEvents) },
        { label: "Dropped Events", value: formatInteger(droppedEvents) },
        {
          label: "Flow Updated",
          value: `${escapeHtml(formatDateTime(updatedAt))}<br/><span class="muted">${escapeHtml(formatAgeFromNow(updatedAt))}</span>`,
        },
        {
          label: "Last Message",
          value: `${escapeHtml(formatDateTime(lastMessageTs))}<br/><span class="muted">${escapeHtml(formatAgeFromNow(lastMessageTs))}</span>`,
        },
        {
          label: "Latest Venue Trade",
          value: `${escapeHtml(formatDateTime(latestTradeForQuality))}<br/><span class="muted">${escapeHtml(formatAgeFromNow(latestTradeForQuality))}</span>`,
        },
        {
          label: "Latest Venue Book",
          value: `${escapeHtml(formatDateTime(latestBookForQuality))}<br/><span class="muted">${escapeHtml(formatAgeFromNow(latestBookForQuality))}</span>`,
        },
        { label: "Lag Sources", value: escapeHtml(lagSources.join(", ") || "none") },
        { label: "Lag Reason", value: escapeHtml(degradedReason) },
        { label: "Flow Windows Populated", value: formatBool(Boolean(marketDataQuality.flowWindowsPopulated)) },
        { label: "Connected Venues", value: formatInteger(connectedVenues.length) },
        { label: "Active Venues", value: formatInteger(activeVenueNames.length) },
        { label: "Visible Windows", value: formatInteger(windows.length) },
        { label: "Shortest Window Trades", value: formatInteger(recentWindow?.tradeCount || 0) },
      ])}
      <div class="monitor-flow-venues">${venueChips}</div>
      <div class="monitor-flow-table">${tableHtml}</div>
      ${error && hasUsableSnapshot
        ? `<div class="muted">Latest refresh failed: ${escapeHtml(error)}. Showing the last successful monitor-flow snapshot.</div>`
        : ""}
    </div>`;
}

function venueDiagnosticsTone(summary) {
  const status = (summary?.diagnosticStatus || "").toString();
  if (status === "public_stream_active") {
    return "ok";
  }
  if ([
    "enabled_but_not_connected",
    "connected_but_no_events",
    "events_seen_but_flow_empty",
    "stream_subscribe_failed",
    "symbol_mapping_failed",
    "network_error",
    "ws_not_attempted",
  ].includes(status)) {
    return "warning";
  }
  return "gray";
}

function renderVenueStreamDiagnostics() {
  const content = $("venueStreamDiagnosticsContent");
  if (!content) {
    return;
  }

  const diagnostics = getData("/api/venues/diagnostics");
  const error = getError("/api/venues/diagnostics");
  if (error) {
    setBadge("venueStreamDiagnosticsBadge", "API Error", "error");
    content.innerHTML = `<div class="error">${escapeHtml(error)}</div>`;
    return;
  }

  const summary = diagnostics?.summary || {};
  const status = summary.diagnosticStatus || "unavailable";
  setBadge("venueStreamDiagnosticsBadge", status.toString().toUpperCase(), venueDiagnosticsTone(summary));

  const notes = (diagnostics?.operatorNotes || [])
    .map((note) => `<li>${escapeHtml(note)}</li>`)
    .join("");
  const venues = diagnostics?.venues || [];
  const venueRows = venues.map((venue) => [
    escapeHtml(venue.venue || "Unavailable"),
    escapeHtml(venue.status || "unavailable"),
    formatBool(Boolean(venue.enabled)),
    escapeHtml(venue.enableFlagName || "Unavailable"),
    formatBool(Boolean(venue.enableFlagValue)),
    escapeHtml(venue.enableSource || "Unavailable"),
    escapeHtml(venue.disabledReason || "None"),
    formatBool(Boolean(venue.connectorConstructed)),
    formatBool(Boolean(venue.startAttempted)),
    formatBool(Boolean(venue.wsConfigured)),
    formatBool(Boolean(venue.wsConnectAttempted)),
    formatBool(Boolean(venue.wsConnected)),
    formatInteger(venue.wsReconnectCount),
    escapeHtml(venue.wsErrorClass || "none"),
    escapeHtml(venue.requestedSymbol || "Unavailable"),
    escapeHtml(venue.venueSymbol || "Unavailable"),
    escapeHtml(venue.venueMarketType || "Unavailable"),
    escapeHtml(venue.symbolMappingStatus || "unavailable"),
    escapeHtml(venue.symbolMappingError || "None"),
    formatBool(Boolean(venue.tradeSubscribeAttempted)),
    formatBool(Boolean(venue.bookSubscribeAttempted)),
    `${formatBool(Boolean(venue.tradeSubscribeAcked))} / ${formatBool(Boolean(venue.bookSubscribeAcked))}`,
    escapeHtml(venue.ackMode || "unavailable"),
    formatInteger(venue.tradeMessageCount),
    formatInteger(venue.bookMessageCount),
    formatBool(Boolean(venue.tradeActive)),
    formatBool(Boolean(venue.bookActive)),
    escapeHtml(venue.activityStatus || "unavailable"),
    formatBool(Boolean(venue.proxyEnabled)),
    formatBool(Boolean(venue.proxySupported)),
    escapeHtml(venue.proxySource || "None"),
    escapeHtml(venue.proxyScheme || "None"),
    escapeHtml(venue.proxyHostMasked || "None"),
    escapeHtml(venue.proxyPortMasked || "None"),
    escapeHtml(venue.lastParseError || "None"),
    escapeHtml(formatAgeFromNow(venue.lastTradeTs)),
    escapeHtml(formatAgeFromNow(venue.lastBookTs)),
    escapeHtml(venue.lastError || "None"),
  ]);

  content.innerHTML = `
    <div class="venue-diagnostics-panel">
      <div class="muted">Explains why monitoring can be started while public trade/orderbook streams are still inactive.</div>
      ${renderMetrics([
        { label: "Monitoring Started", value: formatBool(Boolean(diagnostics?.monitoringStarted)) },
        { label: "Diagnostic Status", value: escapeHtml(status) },
        { label: "Configured Venues", value: formatInteger(summary.configuredVenues) },
        { label: "Enabled Venues", value: formatInteger(summary.enabledVenues) },
        { label: "Connector Constructed", value: formatInteger(summary.connectorConstructedVenues) },
        { label: "Start Attempted", value: formatInteger(summary.startAttemptedVenues) },
        { label: "Connected Venues", value: formatInteger(summary.connectedVenues) },
        { label: "WS Connect Attempted", value: formatInteger(summary.wsConnectAttemptedVenues) },
        { label: "WS Connected", value: formatInteger(summary.wsConnectedVenues) },
        { label: "Symbol Mapped", value: formatInteger(summary.symbolMappedVenues) },
        { label: "Network Errors", value: formatInteger(summary.venuesWithNetworkErrors) },
        { label: "Active Trade Venues", value: formatInteger(summary.activeTradeVenues) },
        { label: "Active Book Venues", value: formatInteger(summary.activeBookVenues) },
        { label: "Trade Active", value: formatInteger(summary.tradeActiveVenues) },
        { label: "Book Active", value: formatInteger(summary.bookActiveVenues) },
        { label: "Latest Book Available", value: formatBool(Boolean(summary.latestVenueBookAvailable)) },
        { label: "Flow Windows Populated", value: formatBool(Boolean(summary.flowWindowsPopulated)) },
      ])}
      <div class="venue-diagnostics-sections">
        <span>WebSocket</span>
        <span>Subscription</span>
        <span>Symbol Mapping</span>
        <span>Activity</span>
        <span>Proxy / Network</span>
      </div>
      <div class="actions">
        <button type="button" class="small-button" id="refreshVenueDiagnosticsButton">Refresh Venue Diagnostics</button>
        <button type="button" class="small-button" id="copyVenueDiagnosticsJsonButton">Copy Venue Diagnostics JSON</button>
      </div>
      ${state.latestVenueDiagnosticsAction ? `<div class="muted">${escapeHtml(state.latestVenueDiagnosticsAction)}</div>` : ""}
      ${notes ? `<ul class="venue-diagnostics-notes">${notes}</ul>` : ""}
      <div class="venue-diagnostics-table">
        ${renderTable(
          [
            "Venue",
            "Status",
            "Enabled",
            "Flag",
            "Flag Value",
            "Source",
            "Disabled Reason",
            "Constructed",
            "Start Attempted",
            "WS Configured",
            "WS Attempted",
            "WS Connected",
            "WS Reconnects",
            "WS Error Class",
            "Requested Symbol",
            "Venue Symbol",
            "Market Type",
            "Mapping",
            "Mapping Error",
            "Trade Sub",
            "Book Sub",
            "Ack Trade/Book",
            "Ack Mode",
            "Trade Msgs",
            "Book Msgs",
            "Trade Active",
            "Book Active",
            "Activity",
            "Proxy Enabled",
            "Proxy Supported",
            "Proxy Source",
            "Proxy Scheme",
            "Proxy Host",
            "Proxy Port",
            "Parse Error",
            "Last Trade",
            "Last Book",
            "Last Error",
          ],
          venueRows
        )}
      </div>
    </div>`;
}

function renderToxicFlow() {
  const toxicState = getData("/api/toxic-state");
  const error = getError("/api/toxic-state");
  if (error) {
    setBadge("toxicFlowBadge", "API Error", "error");
    $("toxicFlowContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const result = strongestToxicResult(toxicState);
  const severity = result?.severity || "normal";
  setBadge("toxicFlowBadge", severity.toUpperCase(), severity);
  $("toxicFlowContent").innerHTML =
    renderMetrics([
      { label: "Toxic Volume BTC", value: formatNumber(result?.toxicVolumeBtc) },
      { label: "Direction", value: result?.direction || "Unavailable" },
      { label: "Window", value: result?.windowMs ? `${result.windowMs / 1000}s` : "Unavailable" },
      { label: "Threshold BTC", value: formatNumber(toxicState?.thresholdBtc) },
      { label: "Toxic Ratio", value: formatNumber(result?.toxicRatio, 2) },
      { label: "Leader Venue", value: result?.leaderVenue || "Unavailable" },
    ]) +
    renderReasons(result?.reasonCodes || []);
}

function activeTradeToxicityUrl() {
  const symbol = (state.activeTradeToxicitySymbol || "").trim();
  return symbol
    ? `/api/toxicity/active-trade/recent?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/active-trade/recent";
}

function activeTradeToxicityStatusUrl() {
  const symbol = (state.activeTradeToxicitySymbol || "").trim();
  return symbol
    ? `/api/toxicity/active-trade/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/active-trade/status";
}

function orderbookWallRecentUrl() {
  const symbol = (state.orderbookWallSymbol || "").trim();
  return symbol
    ? `/api/toxicity/orderbook-walls/recent?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/orderbook-walls/recent";
}

function orderbookWallStatusUrl() {
  const symbol = (state.orderbookWallSymbol || "").trim();
  return symbol
    ? `/api/toxicity/orderbook-walls/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/orderbook-walls/status";
}

const orderbookWallInterpretationSymbolEndpointTemplate =
  "/api/toxicity/orderbook-wall-interpretation/:symbol";

function orderbookWallInterpretationRecentUrl() {
  const symbol = (state.orderbookWallInterpretationSymbol || "").trim();
  return symbol
    ? orderbookWallInterpretationSymbolEndpointTemplate.replace(
        ":symbol",
        encodeURIComponent(symbol)
      )
    : "/api/toxicity/orderbook-wall-interpretation/recent";
}

function orderbookWallInterpretationStatusUrl() {
  const symbol = (state.orderbookWallInterpretationSymbol || "").trim();
  return symbol
    ? `/api/toxicity/orderbook-wall-interpretation/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/orderbook-wall-interpretation/status";
}

const structuralToxicitySymbolEndpointTemplate = "/api/toxicity/structural/:symbol";
const whaleFlowSymbolEndpointTemplate = "/api/toxicity/whale-flow/:symbol";
const whaleFlowCalibrationSymbolEndpointTemplate = "/api/toxicity/whale-flow/calibration/:symbol";
const whaleFlowCandidateHistorySymbolEndpointTemplate =
  "/api/toxicity/whale-flow/history/:symbol";
const toxicSignalFusionSymbolEndpointTemplate = "/api/toxicity/fusion/:symbol";
const toxicSignalInboxSymbolEndpointTemplate = "/api/toxicity/signal-inbox/:symbol";
const toxicSignalInboxSignalEndpointTemplate = "/api/toxicity/signal-inbox/signal/:signal_id";
const toxicSignalGroupsSymbolEndpointTemplate = "/api/toxicity/signal-groups/:symbol";
const toxicSignalGroupsGroupEndpointTemplate = "/api/toxicity/signal-groups/group/:group_id";
const toxicSignalDetailSignalEndpointTemplate = "/api/toxicity/signal-detail/:signal_id";
const toxicSignalDetailGroupEndpointTemplate = "/api/toxicity/signal-detail/group/:group_id";
const toxicSignalHistorySymbolEndpointTemplate = "/api/toxicity/signal-history/:symbol";
const toxicSignalHistorySignalEndpointTemplate =
  "/api/toxicity/signal-history/signal/:signal_id";
const toxicSignalHealthSymbolEndpointTemplate = "/api/toxicity/signal-health/:symbol";
const toxicSignalAlertPreviewExplainEndpointTemplate =
  "/api/toxicity/signal-alert-preview/explain/:signal_id";
const toxicReplaySymbolEndpointTemplate = "/api/toxicity/replay/:symbol";
const toxicReplayLatestEndpointTemplate = "/api/toxicity/replay/:symbol/latest";
const toxicReplaySignalEndpointTemplate = "/api/toxicity/replay/:symbol/:signal_id";
const toxicMarkoutSymbolEndpointTemplate = "/api/toxicity/markout/:symbol";
const toxicMarkoutSignalEndpointTemplate = "/api/toxicity/markout/signal/:signal_id";
const toxicQualityScorecardSymbolEndpointTemplate = "/api/toxicity/quality-scorecard/:symbol";
const toxicWeightRecommendationSymbolEndpointTemplate =
  "/api/toxicity/weight-recommendation/:symbol";
const toxicWeightReviewSymbolEndpointTemplate = "/api/toxicity/weight-review/:symbol";
const toxicWeightReviewSymbolExportEndpointTemplate = "/api/toxicity/weight-review/:symbol/export";
const toxicGovernanceLedgerSymbolEndpointTemplate =
  "/api/toxicity/governance-ledger/:symbol";
const toxicGovernanceProposalSymbolEndpointTemplate =
  "/api/toxicity/governance-proposal/:symbol";
const toxicGovernanceReviewPackSymbolEndpointTemplate =
  "/api/toxicity/governance-review-pack/:symbol";
const toxicGovernanceSignoffPackSymbolEndpointTemplate =
  "/api/toxicity/governance-signoff-pack/:symbol";

function structuralToxicityRecentUrl() {
  const symbol = (state.structuralToxicitySymbol || "").trim();
  return symbol
    ? structuralToxicitySymbolEndpointTemplate.replace(":symbol", encodeURIComponent(symbol))
    : "/api/toxicity/structural/recent";
}

function structuralToxicityStatusUrl() {
  const symbol = (state.structuralToxicitySymbol || "").trim();
  return symbol
    ? `/api/toxicity/structural/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/structural/status";
}

function whaleFlowRecentUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? whaleFlowSymbolEndpointTemplate.replace(":symbol", encodeURIComponent(symbol))
    : "/api/toxicity/whale-flow/recent";
}

function whaleFlowStatusUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? `/api/toxicity/whale-flow/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/whale-flow/status";
}

function whaleFlowCalibrationSelectedSymbol() {
  const explicit = (state.whaleFlowCalibrationSymbol || "").trim();
  if (explicit) {
    return explicit.toUpperCase();
  }
  return signalSymbolFilterValue();
}

function whaleFlowCalibrationReportUrl() {
  const symbol = whaleFlowCalibrationSelectedSymbol();
  return symbol
    ? whaleFlowCalibrationSymbolEndpointTemplate.replace(":symbol", encodeURIComponent(symbol))
    : "/api/toxicity/whale-flow/calibration/report";
}

function whaleFlowCalibrationStatusUrl() {
  const symbol = whaleFlowCalibrationSelectedSymbol();
  return symbol
    ? `/api/toxicity/whale-flow/calibration/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/whale-flow/calibration/status";
}

function whaleFlowCandidateHistorySelectedSymbol() {
  const explicit = (state.whaleFlowCandidateHistorySymbol || "").trim();
  if (explicit) {
    return explicit.toUpperCase();
  }
  return signalSymbolFilterValue();
}

function whaleFlowCandidateHistoryRecentUrl() {
  const symbol = whaleFlowCandidateHistorySelectedSymbol();
  return symbol
    ? whaleFlowCandidateHistorySymbolEndpointTemplate.replace(
        ":symbol",
        encodeURIComponent(symbol)
      )
    : "/api/toxicity/whale-flow/history/recent";
}

function whaleFlowCandidateHistoryStatusUrl() {
  const symbol = whaleFlowCandidateHistorySelectedSymbol();
  return symbol
    ? `/api/toxicity/whale-flow/history/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/whale-flow/history/status";
}

function toxicSignalFusionRecentUrl() {
  const symbol = (state.toxicSignalFusionSymbol || "").trim();
  return symbol
    ? toxicSignalFusionSymbolEndpointTemplate.replace(":symbol", encodeURIComponent(symbol))
    : "/api/toxicity/fusion/recent";
}

function toxicSignalFusionStatusUrl() {
  const symbol = (state.toxicSignalFusionSymbol || "").trim();
  return symbol
    ? `/api/toxicity/fusion/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/fusion/status";
}

function toxicSignalInboxRecentUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? toxicSignalInboxSymbolEndpointTemplate.replace(":symbol", encodeURIComponent(symbol))
    : "/api/toxicity/signal-inbox/recent";
}

function toxicSignalInboxStatusUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? `/api/toxicity/signal-inbox/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-inbox/status";
}

function toxicSignalGroupsRecentUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? toxicSignalGroupsSymbolEndpointTemplate.replace(":symbol", encodeURIComponent(symbol))
    : "/api/toxicity/signal-groups/recent";
}

function toxicSignalGroupsStatusUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? `/api/toxicity/signal-groups/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-groups/status";
}

function toxicSignalDetailSelectedSymbol() {
  return (
    signalSymbolFilterValue() ||
    getData("/api/status")?.symbol ||
    "BTC-PERP"
  );
}

function signalSymbolFilterValue() {
  const explicit = (state.signalSymbolFilter || "").trim();
  if (explicit) {
    return explicit.toUpperCase();
  }
  const inboxFallback = (state.toxicSignalInboxSymbol || "").trim();
  if (inboxFallback) {
    return inboxFallback.toUpperCase();
  }
  const groupFallback = (state.toxicSignalGroupSymbol || "").trim();
  if (groupFallback) {
    return groupFallback.toUpperCase();
  }
  return "";
}

function toxicSignalDetailStatusUrl() {
  const symbol = toxicSignalDetailSelectedSymbol();
  return `/api/toxicity/signal-detail/status?symbol=${encodeURIComponent(symbol)}`;
}

function toxicSignalHistorySelectedSymbol() {
  const explicit = (state.toxicSignalHistorySymbol || "").trim();
  if (explicit) {
    return explicit.toUpperCase();
  }
  return signalSymbolFilterValue();
}

function toxicSignalHistoryRecentUrl() {
  const symbol = toxicSignalHistorySelectedSymbol();
  return symbol
    ? `/api/toxicity/signal-history/recent?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-history/recent";
}

function toxicSignalHistoryStatusUrl() {
  const symbol = toxicSignalHistorySelectedSymbol();
  return symbol
    ? `/api/toxicity/signal-history/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-history/status";
}

function toxicSignalHistoryAlertsUrl() {
  const symbol = toxicSignalHistorySelectedSymbol();
  return symbol
    ? `/api/toxicity/signal-history/alerts/recent?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-history/alerts/recent";
}

function toxicSignalHistoryReportsUrl() {
  const symbol = toxicSignalHistorySelectedSymbol();
  return symbol
    ? `/api/toxicity/signal-history/reports/recent?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-history/reports/recent";
}

function toxicSignalHistorySignalUrl() {
  const signalId = (state.toxicSignalHistorySignalId || "").trim();
  if (!signalId) {
    return null;
  }
  const symbol = toxicSignalHistorySelectedSymbol();
  return symbol
    ? `${toxicSignalHistorySignalEndpointTemplate.replace(
        ":signal_id",
        encodeURIComponent(signalId)
      )}?symbol=${encodeURIComponent(symbol)}`
    : toxicSignalHistorySignalEndpointTemplate.replace(
        ":signal_id",
        encodeURIComponent(signalId)
      );
}

function toxicSignalHealthSelectedSymbol() {
  const explicit = (state.toxicSignalHealthSymbol || "").trim();
  if (explicit) {
    return explicit.toUpperCase();
  }
  return signalSymbolFilterValue();
}

function toxicSignalHealthSummaryUrl() {
  const symbol = toxicSignalHealthSelectedSymbol();
  return symbol
    ? toxicSignalHealthSymbolEndpointTemplate.replace(":symbol", encodeURIComponent(symbol))
    : "/api/toxicity/signal-health/summary";
}

function toxicSignalHealthStatusUrl() {
  const symbol = toxicSignalHealthSelectedSymbol();
  return symbol
    ? `/api/toxicity/signal-health/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-health/status";
}

function toxicSignalReportDailyUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? `/api/toxicity/signal-report/daily?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-report/daily";
}

function toxicSignalReportStatusUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? `/api/toxicity/signal-report/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-report/status";
}

function toxicSignalReportRollingUrl() {
  const symbol = signalSymbolFilterValue();
  const params = new URLSearchParams({ window: "7d" });
  if (symbol) {
    params.set("symbol", symbol);
  }
  return `/api/toxicity/signal-report/rolling?${params.toString()}`;
}

function toxicSignalAlertPreviewRecentUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? `/api/toxicity/signal-alert-preview/recent?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-alert-preview/recent";
}

function toxicSignalAlertPreviewStatusUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? `/api/toxicity/signal-alert-preview/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-alert-preview/status";
}

function toxicSignalAlertPreviewExplainUrl() {
  const signalId = (state.toxicSignalAlertExplainSignalId || "").trim();
  if (!signalId) {
    return null;
  }
  const symbol = signalSymbolFilterValue();
  const base = toxicSignalAlertPreviewExplainEndpointTemplate.replace(
    ":signal_id",
    encodeURIComponent(signalId)
  );
  return symbol ? `${base}?symbol=${encodeURIComponent(symbol)}` : base;
}

function durableArchiveDryRunUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? `/api/archive/dry-run/write?symbol=${encodeURIComponent(symbol)}`
    : "/api/archive/dry-run/write";
}

function durableArchiveDryRunReviewPackLatestUrl() {
  const symbol = signalSymbolFilterValue();
  return symbol
    ? `/api/archive/dry-run/review-pack/latest?symbol=${encodeURIComponent(symbol)}`
    : "/api/archive/dry-run/review-pack/latest";
}

function durableArchiveWriteGateStatusUrl() {
  return "/api/archive/write/status";
}

function durableArchiveWriteAuditStatusUrl() {
  return "/api/archive/write/audit/status";
}

function durableArchiveWriteAuditRecentUrl() {
  return "/api/archive/write/audit/recent";
}

function durableArchiveWriteAuditLatestUrl() {
  return "/api/archive/write/audit/latest";
}

function getActiveTradeToxicityPayload() {
  return (
    state.data[activeTradeToxicityUrl()]?.data ||
    getData("/api/toxicity/active-trade/recent")
  );
}

function getActiveTradeToxicityStatusPayload() {
  return (
    state.data[activeTradeToxicityStatusUrl()]?.data ||
    getData("/api/toxicity/active-trade/status")
  );
}

function getWhaleFlowPayload() {
  return state.data[whaleFlowRecentUrl()]?.data || getData("/api/toxicity/whale-flow/recent");
}

function getWhaleFlowStatusPayload() {
  return state.data[whaleFlowStatusUrl()]?.data || getData("/api/toxicity/whale-flow/status");
}

function getWhaleFlowCalibrationPayload() {
  return (
    state.data[whaleFlowCalibrationReportUrl()]?.data ||
    getData("/api/toxicity/whale-flow/calibration/report")
  );
}

function getWhaleFlowCalibrationStatusPayload() {
  return (
    state.data[whaleFlowCalibrationStatusUrl()]?.data ||
    getData("/api/toxicity/whale-flow/calibration/status")
  );
}

function getWhaleFlowCandidateHistoryPayload() {
  return (
    state.data[whaleFlowCandidateHistoryRecentUrl()]?.data ||
    getData("/api/toxicity/whale-flow/history/recent")
  );
}

function getWhaleFlowCandidateHistoryStatusPayload() {
  return (
    state.data[whaleFlowCandidateHistoryStatusUrl()]?.data ||
    getData("/api/toxicity/whale-flow/history/status")
  );
}

const whaleFlowCompactPresetDefinitions = [
  ["all", "All"],
  ["high_volume", "High Volume"],
  ["venue_confluence_satisfied", "Venue Confluence"],
  ["degraded_or_partial_data", "Degraded Data"],
  ["calibration_not_ready", "Calibration Not Ready"],
  ["needs_more_data", "Needs More Data"],
  ["not_enough_data", "Not Enough Data"],
];

function whaleFlowCompactPresetLabel(preset) {
  return (
    whaleFlowCompactPresetDefinitions.find(([key]) => key === preset)?.[1] ||
    "All"
  );
}

function whaleFlowCompactPresetTone(preset) {
  switch (preset) {
    case "high_volume":
      return "warning";
    case "venue_confluence_satisfied":
      return "success";
    case "degraded_or_partial_data":
      return "orange";
    case "calibration_not_ready":
      return "danger";
    case "needs_more_data":
      return "warning";
    case "not_enough_data":
      return "muted";
    default:
      return "blue";
  }
}

function whaleFlowCompactEmptyText(preset) {
  switch (preset) {
    case "high_volume":
      return "No high volume candidates";
    case "venue_confluence_satisfied":
      return "No venue confluence candidates";
    case "degraded_or_partial_data":
      return "No degraded data quality candidates";
    case "calibration_not_ready":
      return "No calibration blocked candidates";
    case "needs_more_data":
      return "No needs_more_data candidates";
    case "not_enough_data":
      return "No not_enough_data candidates";
    default:
      return "No whale flow items matched this preset";
  }
}

function whaleFlowCompactThresholdForWindow(windowMs, thresholds = {}) {
  switch (Number(windowMs)) {
    case 1000:
      return thresholds.oneSecondBtc ?? 100;
    case 5000:
      return thresholds.fiveSecondBtc ?? 300;
    case 15000:
      return thresholds.fifteenSecondBtc ?? 800;
    case 60000:
      return thresholds.sixtySecondBtc ?? 2000;
    default:
      return Number.POSITIVE_INFINITY;
  }
}

function whaleFlowCompactIsFallbackBaseline(source) {
  return [
    "sixty_second_fallback",
    "longer_window_fallback",
    "insufficient_history",
  ].includes(String(source || "").toLowerCase());
}

function whaleFlowCompactReasonSummary(reasons = []) {
  return reasons.length ? reasons.map((item) => escapeHtml(item)).join("<br/>") : "None";
}

function whaleFlowCompactCandidateMatchesPreset(candidate, whaleReport, whaleStatus, preset) {
  if (preset === "all") {
    return true;
  }
  const thresholds = whaleReport?.thresholds || whaleStatus?.thresholds || {};
  const minVenueConfirmations = thresholds.minVenueConfirmations ?? 2;
  const baselineSource =
    whaleReport?.baselineQuality?.baselineSource ||
    whaleStatus?.baselineQuality?.baselineSource ||
    "insufficient_history";
  const dataQuality = String(candidate?.diagnostics?.dataQuality || "").toLowerCase();
  const degradationReasons = candidate?.diagnostics?.degradationReasons || [];
  switch (preset) {
    case "high_volume":
      return (candidate?.volumeBtc ?? 0) >= whaleFlowCompactThresholdForWindow(candidate?.windowMs, thresholds);
    case "venue_confluence_satisfied":
      return (candidate?.sameDirectionVenues ?? 0) >= minVenueConfirmations;
    case "degraded_or_partial_data":
      return (
        dataQuality === "partial" ||
        dataQuality === "degraded" ||
        whaleFlowCompactIsFallbackBaseline(baselineSource) ||
        Boolean(degradationReasons.length) ||
        Boolean((whaleReport?.degradationWarnings || []).length)
      );
    case "not_enough_data":
      return (
        whaleFlowCompactIsFallbackBaseline(baselineSource) ||
        Boolean((candidate?.diagnostics?.missingInputs || []).length)
      );
    default:
      return false;
  }
}

function whaleFlowCompactHistoryMatchesPreset(item, whaleStatus, historyStatus, preset) {
  if (preset === "all") {
    return true;
  }
  const minVenueConfirmations =
    whaleStatus?.thresholds?.minVenueConfirmations ??
    historyStatus?.minCandidatesRequired ??
    2;
  switch (preset) {
    case "high_volume":
      return (item?.volumeBtc ?? 0) >= whaleFlowCompactThresholdForWindow(item?.windowMs, whaleStatus?.thresholds || {});
    case "venue_confluence_satisfied":
      return (item?.venueConfluenceCount ?? 0) >= minVenueConfirmations;
    case "degraded_or_partial_data":
      return (
        ["partial", "degraded"].includes(String(item?.dataQuality || "").toLowerCase()) ||
        whaleFlowCompactIsFallbackBaseline(item?.baselineSource)
      );
    case "not_enough_data":
      return (
        String(item?.markoutStatus || "").toLowerCase() === "not_enough_data" ||
        whaleFlowCompactIsFallbackBaseline(item?.baselineSource)
      );
    default:
      return false;
  }
}

function buildWhaleFlowCompactCalibrationItems(calibrationReport, calibrationStatus, historyStatus, preset) {
  const items = [];
  const sampleStatus = calibrationReport?.sampleStatus || {};
  const manualTuningNotes = calibrationReport?.manualTuningNotes || [];
  const blockedReasons = [
    ...(sampleStatus.blockedReasons || []),
    ...(historyStatus?.calibrationBlockedReasons || []),
  ].filter(Boolean);
  const uniqueBlockedReasons = [...new Set(blockedReasons)];
  const calibrationReady =
    historyStatus?.calibrationReady ?? sampleStatus.enoughData ?? calibrationStatus?.enoughData;
  const unresolvedMarkoutCount =
    sampleStatus.unresolvedMarkoutCount ?? historyStatus?.notEnoughDataCount ?? 0;
  const warningText = calibrationReport?.warnings || [];

  if (
    preset === "all" ||
    preset === "calibration_not_ready"
  ) {
    if (calibrationReady === false || uniqueBlockedReasons.length) {
      items.push({
        id: "calibration-not-ready",
        source: "Calibration Gate",
        title: "Calibration NOT READY",
        tone: "danger",
        summary: "Manual review required. Preset view is display-only.",
        chips: [
          renderSignalChip("calibrationReady=false", "danger"),
          renderSignalChip("view-only", "blue"),
          renderSignalChip("Persistent preset disabled", "muted"),
        ],
        details: uniqueBlockedReasons.length
          ? uniqueBlockedReasons
          : ["Calibration is blocked but no explicit blocked reason was returned."],
      });
    }
  }

  if (preset === "all" || preset === "needs_more_data") {
    const noteMatches = manualTuningNotes.filter(
      (note) => String(note?.suggestedAction || "").toLowerCase() === "needs_more_data"
    );
    if (noteMatches.length || sampleStatus.enoughData === false || uniqueBlockedReasons.length) {
      items.push({
        id: "needs-more-data",
        source: "Calibration Report",
        title: "Needs More Data",
        tone: "warning",
        summary: "Tuning notes stay locked until resolved evidence gates pass.",
        chips: [
          renderSignalChip("needs_more_data", "warning"),
          renderSignalChip(`resolved ${formatInteger(sampleStatus.resolvedMarkoutEvidenceCount)}`, "blue"),
          renderSignalChip(`blocked ${formatInteger(uniqueBlockedReasons.length)}`, "muted"),
        ],
        details: [
          ...noteMatches.map(
            (note) => `${note.target || "threshold"} -> ${note.reason || "needs_more_data"}`
          ),
          ...uniqueBlockedReasons,
        ],
      });
    }
  }

  if (preset === "all" || preset === "not_enough_data") {
    if (
      unresolvedMarkoutCount > 0 ||
      warningText.some((item) => String(item).toLowerCase().includes("not_enough_data")) ||
      warningText.some((item) => String(item).toLowerCase().includes("baseline insufficient"))
    ) {
      items.push({
        id: "not-enough-data",
        source: "Evidence Quality",
        title: "Not Enough Data",
        tone: "muted",
        summary: "Unresolved markout outcomes or thin baseline inputs require caution.",
        chips: [
          renderSignalChip(`unresolved ${formatInteger(unresolvedMarkoutCount)}`, "muted"),
          renderSignalChip(
            `rate ${formatPercent(sampleStatus.notEnoughDataRate ?? historyStatus?.maxNotEnoughDataRateForTuning, 0)}`,
            "muted"
          ),
        ],
        details: warningText.length
          ? warningText
          : ["not_enough_data must not be treated as aligned."],
      });
    }
  }

  if (preset === "all" || preset === "degraded_or_partial_data") {
    const degradedBaselineItems = (calibrationReport?.baselineSourceQuality || []).filter((item) =>
      whaleFlowCompactIsFallbackBaseline(item?.baselineSource)
    );
    if (degradedBaselineItems.length) {
      items.push({
        id: "degraded-baseline",
        source: "Baseline Source",
        title: "Fallback Baseline Detected",
        tone: "orange",
        summary: "Compact preset is highlighting fallback or insufficient baseline evidence.",
        chips: degradedBaselineItems.map((item) =>
          renderSignalChip(item.baselineSource || "insufficient_history", "warning")
        ),
        details: degradedBaselineItems.map(
          (item) =>
            `${item.baselineSource || "insufficient_history"} · sampleCount=${item.sampleCount ?? 0} · notEnoughDataRate=${formatPercent(item.notEnoughDataRate, 0)}`
        ),
      });
    }
  }

  return items;
}

function buildWhaleFlowCompactCards() {
  const preset = state.whaleFlowCompactPreset || "all";
  const whaleReport = getWhaleFlowPayload();
  const whaleStatus = getWhaleFlowStatusPayload();
  const calibrationReport = getWhaleFlowCalibrationPayload();
  const calibrationStatus = getWhaleFlowCalibrationStatusPayload();
  const historyReport = getWhaleFlowCandidateHistoryPayload();
  const historyStatus = getWhaleFlowCandidateHistoryStatusPayload();
  const candidateCards = (whaleReport?.candidates || [])
    .filter((candidate) =>
      whaleFlowCompactCandidateMatchesPreset(candidate, whaleReport, whaleStatus, preset)
    )
    .map((candidate) => ({
      id: `candidate-${candidate.candidateId}`,
      source: "Whale Candidate",
      title: `${formatWhaleFlowCandidateType(candidate.candidateType)} · ${candidate.symbol || "Unavailable"}`,
      tone:
        preset === "degraded_or_partial_data"
          ? "orange"
          : preset === "venue_confluence_satisfied"
            ? "success"
            : preset === "not_enough_data"
              ? "muted"
              : "warning",
      summary: candidate.primaryReason || "No primary reason",
      chips: [
        renderSignalChip(`${formatNumber(candidate.volumeBtc, 1)} BTC`, "warning"),
        renderSignalChip(`window ${candidate.window || formatInteger(candidate.windowMs)}`, "blue"),
        renderSignalChip(`venues ${formatInteger(candidate.sameDirectionVenues)}`, "blue"),
        renderSignalChip(
          `quality ${formatWhaleFlowQualityStatus(candidate.diagnostics?.dataQuality)}`,
          whaleFlowQualityTone(candidate.diagnostics?.dataQuality)
        ),
      ],
      details: [
        `directionBias=${formatPercent(candidate.directionBias, 0)}`,
        `priceImpactBps=${formatNumber(candidate.priceImpactBps, 2)}`,
        `depthDropRatio=${formatPercent(candidate.depthDropRatio, 0)}`,
        `whyCandidate=${(candidate.diagnostics?.whyCandidate || []).join(" | ") || "None"}`,
        `missingInputs=${(candidate.diagnostics?.missingInputs || []).join(" | ") || "None"}`,
        `confidenceModifiers=${(candidate.diagnostics?.confidenceModifiers || []).join(" | ") || "None"}`,
      ],
    }));

  const historyCards = (historyReport?.items || [])
    .filter((item) => whaleFlowCompactHistoryMatchesPreset(item, whaleStatus, historyStatus, preset))
    .map((item) => ({
      id: `history-${item.candidateId}`,
      source: "Candidate History",
      title: `${formatWhaleFlowCandidateType(item.classification)} · ${item.symbol || "Unavailable"}`,
      tone:
        String(item.markoutStatus || "").toLowerCase() === "not_enough_data"
          ? "muted"
          : preset === "degraded_or_partial_data"
            ? "orange"
            : "blue",
      summary: `Markout: ${item.markoutStatus || "not_enough_data"}`,
      chips: [
        renderSignalChip(`${formatNumber(item.volumeBtc, 1)} BTC`, "blue"),
        renderSignalChip(`baseline ${item.baselineSource || "insufficient_history"}`, "muted"),
        renderSignalChip(`dataQuality ${item.dataQuality || "no_data"}`, whaleFlowQualityTone(item.dataQuality)),
        renderSignalChip(`venues ${formatInteger(item.venueConfluenceCount)}`, "blue"),
      ],
      details: [
        `windowMs=${formatInteger(item.windowMs)}`,
        `direction=${item.directionBias || "neutral"}`,
        `relativeVolumeMultiple=${item.relativeVolumeMultiple == null ? "Unavailable" : `${formatNumber(item.relativeVolumeMultiple, 2)}x`}`,
        `outcomeStatus=${item.outcomeStatus || "not_enough_data"}`,
        `createdAt=${formatDateTime(item.createdAtMs)}`,
      ],
    }));

  const calibrationCards = buildWhaleFlowCompactCalibrationItems(
    calibrationReport,
    calibrationStatus,
    historyStatus,
    preset
  );

  return {
    preset,
    whaleReport,
    whaleStatus,
    calibrationReport,
    calibrationStatus,
    historyReport,
    historyStatus,
    candidateCards,
    historyCards,
    calibrationCards,
    matchedItems:
      candidateCards.length + historyCards.length + calibrationCards.length,
  };
}

function buildWhaleFlowCompactPresetFilters(preset, compactData) {
  const filters = {
    viewOnly: true,
    persistentPresetEnabled: false,
    runtimePresetModified: false,
  };
  switch (preset) {
    case "high_volume":
      filters.highVolumeOnly = true;
      break;
    case "venue_confluence_satisfied":
      filters.venueConfluenceSatisfied = true;
      filters.minVenueConfirmations =
        compactData.whaleStatus?.thresholds?.minVenueConfirmations ??
        compactData.whaleReport?.thresholds?.minVenueConfirmations ??
        2;
      break;
    case "degraded_or_partial_data":
      filters.dataQuality = ["degraded", "partial"];
      filters.fallbackBaselineOnly = true;
      break;
    case "calibration_not_ready":
      filters.calibrationReady = false;
      filters.requiresBlockedReasons = true;
      break;
    case "needs_more_data":
      filters.requiresNeedsMoreData = true;
      filters.enoughData = false;
      break;
    case "not_enough_data":
      filters.markout = "not_enough_data";
      filters.baselineInsufficientAllowed = true;
      break;
    default:
      filters.all = true;
      break;
  }
  return filters;
}

function buildWhaleFlowCompactCopyPayload() {
  const compactData = buildWhaleFlowCompactCards();
  return {
    readOnly: true,
    analysisOnly: true,
    executionEnabled: false,
    runtimeModified: false,
    viewOnly: true,
    persistentPresetEnabled: false,
    runtimePresetModified: false,
    selectedPreset: compactData.preset,
    matchedItems: compactData.matchedItems,
    filters: buildWhaleFlowCompactPresetFilters(compactData.preset, compactData),
    operatorNote:
      "Preset view is display-only. It does not modify thresholds, runtime config, or monitoring scope.",
  };
}

function renderWhaleFlowCompactCard(item) {
  return `<div class="recommendation-card">
    <div class="metric-grid">
      <div class="metric"><div class="metric-label">Source</div><div class="metric-value">${escapeHtml(item.source)}</div></div>
      <div class="metric"><div class="metric-label">Title</div><div class="metric-value">${escapeHtml(item.title)}</div></div>
      <div class="metric"><div class="metric-label">Summary</div><div class="metric-value">${escapeHtml(item.summary)}</div></div>
    </div>
    ${renderSignalChipRow(item.chips || [])}
    <div class="muted">${whaleFlowCompactReasonSummary(item.details || [])}</div>
  </div>`;
}

function renderWhaleFlowCompactMode() {
  const compactData = buildWhaleFlowCompactCards();
  const {
    preset,
    whaleReport,
    whaleStatus,
    calibrationReport,
    calibrationStatus,
    historyStatus,
    candidateCards,
    historyCards,
    calibrationCards,
    matchedItems,
  } = compactData;
  const monitorError =
    getError(whaleFlowRecentUrl()) ||
    getError(whaleFlowStatusUrl()) ||
    (!signalSymbolFilterValue()
      ? getError("/api/toxicity/whale-flow/recent") || getError("/api/toxicity/whale-flow/status")
      : null);
  const calibrationError =
    getError(whaleFlowCalibrationReportUrl()) ||
    getError(whaleFlowCalibrationStatusUrl()) ||
    (!whaleFlowCalibrationSelectedSymbol()
      ? getError("/api/toxicity/whale-flow/calibration/report") ||
        getError("/api/toxicity/whale-flow/calibration/status")
      : null);
  const historyError =
    getError(whaleFlowCandidateHistoryRecentUrl()) ||
    getError(whaleFlowCandidateHistoryStatusUrl()) ||
    (!whaleFlowCandidateHistorySelectedSymbol()
      ? getError("/api/toxicity/whale-flow/history/recent") ||
        getError("/api/toxicity/whale-flow/history/status")
      : null);
  const error = monitorError || calibrationError || historyError;
  if (error) {
    setBadge("whaleFlowCompactModeBadge", "API Error", "error");
    $("whaleFlowCompactModeContent").innerHTML = `<div class="error">${escapeHtml(error)}</div>`;
    return;
  }

  if (!whaleReport || !calibrationReport || !historyStatus || !calibrationStatus) {
    setBadge("whaleFlowCompactModeBadge", "Loading", "none");
    $("whaleFlowCompactModeContent").innerHTML =
      `<div class="muted">Whale Flow Compact View will appear after read-only whale-flow, calibration, and candidate history payloads are available.</div>`;
    return;
  }

  const compactCards = [...candidateCards, ...historyCards, ...calibrationCards];
  setBadge(
    "whaleFlowCompactModeBadge",
    whaleFlowCompactPresetLabel(preset),
    whaleFlowCompactPresetTone(preset)
  );

  const presetButtons = whaleFlowCompactPresetDefinitions
    .map(
      ([key, label]) =>
        `<button type="button" class="small-button ${key === preset ? "active" : ""}" data-whale-flow-preset="${key}">Preset: ${escapeHtml(label)}</button>`
    )
    .join("");

  const emptyState = `<div class="whale-flow-empty">${escapeHtml(
    whaleFlowCompactEmptyText(preset)
  )}</div>`;

  $("whaleFlowCompactModeContent").innerHTML =
    renderMetrics([
      { label: "Current Preset", value: whaleFlowCompactPresetLabel(preset) },
      { label: "Mode", value: "view-only" },
      { label: "Matched Items", value: formatInteger(matchedItems) },
      { label: "Persistent Preset", value: "disabled" },
      { label: "Runtime Modified", value: "false" },
      {
        label: "Calibration Ready",
        value: formatBool(Boolean(historyStatus.calibrationReady ?? calibrationStatus.enoughData)),
      },
      {
        label: "Selected Symbol",
        value:
          whaleReport.selectedSymbol ||
          calibrationReport.selectedSymbol ||
          historyStatus.selectedSymbol ||
          "Unavailable",
      },
      { label: "Read Only", value: "true" },
    ]) +
    `<div class="compact-preset-grid">${presetButtons}</div>` +
    `<div class="action-row">
      <button type="button" class="small-button" id="resetWhaleFlowCompactPresetButton">Reset Preset</button>
      <button type="button" class="small-button" id="copyWhaleFlowCompactPresetJsonButton">Copy Preset View JSON</button>
    </div>` +
    `<div class="compact-summary">
      Current Preset: ${escapeHtml(whaleFlowCompactPresetLabel(preset))}<br/>
      Mode: view-only<br/>
      Persistent preset: disabled<br/>
      Runtime modified: false<br/>
      No threshold modified<br/>
      No config write<br/>
      No apply/reload
    </div>` +
    renderSignalChipRow([
      renderSignalChip("view-only", "blue"),
      renderSignalChip("Persistent preset disabled", "muted"),
      renderSignalChip("Runtime modified: false", "muted"),
      renderSignalChip("No threshold modified", "muted"),
      renderSignalChip("No config write", "muted"),
      renderSignalChip("No apply/reload", "muted"),
    ]) +
    `<div class="compact-list">${compactCards.length ? compactCards.map(renderWhaleFlowCompactCard).join("") : emptyState}</div>` +
    `<div class="muted">${
      state.latestWhaleFlowCompactAction
        ? escapeHtml(state.latestWhaleFlowCompactAction)
        : "Preset view is display-only. It does not modify thresholds, runtime config, or monitoring scope."
    }</div>`;
}

function renderActiveTradeToxicity() {
  const url = activeTradeToxicityUrl();
  const statusUrl = activeTradeToxicityStatusUrl();
  const report = getActiveTradeToxicityPayload();
  const statusPayload = getActiveTradeToxicityStatusPayload();
  const error =
    getError(url) ||
    getError(statusUrl) ||
    (!state.activeTradeToxicitySymbol
      ? getError("/api/toxicity/active-trade/recent") ||
        getError("/api/toxicity/active-trade/status")
      : null);
  if (error) {
    setBadge("activeTradeToxicityBadge", "API Error", "error");
    $("activeTradeToxicityContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("activeTradeToxicityBadge", "Loading", "none");
    $("activeTradeToxicityContent").innerHTML =
      `<div class="muted">Active trade toxicity will appear after the read-only flow analysis loads.</div>`;
    return;
  }

  const tone =
    report.status === "high_toxicity_watch"
      ? "alert"
      : report.status === "buy_toxicity_watch" || report.status === "sell_toxicity_watch"
        ? "warning"
        : report.status === "neutral"
          ? "none"
          : "disabled";

  setBadge("activeTradeToxicityBadge", (report.status || "unknown").toUpperCase(), tone);
  const signals = report.signals || [];
  const oneHourSignals = signals.filter((signal) => signal.timeframe === "1h");
  $("activeTradeToxicityContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(statusPayload?.readOnly ?? report.readOnly)) },
      {
        label: "Runtime Modified",
        value: formatBool(Boolean(statusPayload?.runtimeModified ?? report.runtimeModified)),
      },
      { label: "Status", value: statusPayload?.mode || "analysis_only" },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Toxicity Status", value: report.status || "Unavailable" },
      { label: "Score", value: formatNumber(report.score, 1) },
      { label: "Side Bias", value: report.sideBias || "Unavailable" },
      { label: "Signal Count", value: formatInteger(statusPayload?.signalCount ?? signals.length) },
      { label: "Last Signal At", value: formatDateTime(statusPayload?.lastSignalAtMs) },
      {
        label: "1H Delta Toxicity",
        value: oneHourSignals.length
          ? oneHourSignals.map((signal) => signal.signalType).join(", ")
          : "None",
      },
    ]) +
    `<div class="action-row">
      <input id="activeTradeToxicitySymbolInput" placeholder="symbol" value="${escapeHtml(
        state.activeTradeToxicitySymbol || report.selectedSymbol || ""
      )}" />
      <button type="button" class="small-button" id="selectActiveTradeToxicitySymbolButton">Select Symbol</button>
      <button type="button" class="small-button" id="refreshActiveTradeToxicityButton">Refresh Active Trade Toxicity</button>
      <button type="button" class="small-button" id="copyActiveTradeToxicityJsonButton">Copy JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Warnings</div>
        <div class="metric-value">${(report.warnings || []).length ? report.warnings.join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">No Trade Reasons</div>
        <div class="metric-value">${(report.noTradeReasons || []).length ? report.noTradeReasons.join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Recent Signals</div>
        <div class="metric-value">${
          signals.length
            ? signals
                .map(
                  (signal) => `
            <div class="recommendation-card">
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Signal</div><div class="metric-value">${escapeHtml(signal.signalType || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Confidence</div><div class="metric-value">${escapeHtml(signal.confidence || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Score</div><div class="metric-value">${formatInteger(signal.toxicityScore)}</div></div>
                <div class="metric"><div class="metric-label">Side</div><div class="metric-value">${escapeHtml(signal.side || "Unavailable")}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Notional USD</div><div class="metric-value">${formatNumber(signal.notionalUsd, 0)}</div></div>
                <div class="metric"><div class="metric-label">CVD Delta</div><div class="metric-value">${formatNumber(signal.cvdDelta, 0)}</div></div>
                <div class="metric"><div class="metric-label">Imbalance Ratio</div><div class="metric-value">${formatNumber(signal.imbalanceRatio, 2)}</div></div>
                <div class="metric"><div class="metric-label">Price Impact bps</div><div class="metric-value">${formatNumber(signal.priceImpactBps, 2)}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Timeframe</div><div class="metric-value">${escapeHtml(signal.timeframe || "short_window")}</div></div>
                <div class="metric"><div class="metric-label">Delta</div><div class="metric-value">${formatNumber(signal.delta, 2)}</div></div>
                <div class="metric"><div class="metric-label">Threshold</div><div class="metric-value">${formatNumber(signal.threshold, 2)}</div></div>
                <div class="metric"><div class="metric-label">Price Change bps</div><div class="metric-value">${formatNumber(signal.priceChangeBps, 2)}</div></div>
              </div>
              <div class="metric">
                <div class="metric-label">Reasons</div>
                <div class="metric-value">${(signal.reason || []).length ? signal.reason.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
              </div>
            </div>`
                )
                .join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          runtimeModified=false<br/>
          analysis_only=true<br/>
          no_live_execution=true
        </div>
      </div>
    </div>` +
    (state.latestActiveTradeToxicityAction
      ? `<div class="muted">${escapeHtml(state.latestActiveTradeToxicityAction)}</div>`
      : "");
}

function renderLiqHunt() {
  const liqHunt = getData("/api/liq-hunt-state");
  const error = getError("/api/liq-hunt-state");
  if (error) {
    setBadge("liqHuntBadge", "API Error", "error");
    $("liqHuntContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const result = liqHunt?.result;
  setBadge("liqHuntBadge", (result?.level || "none").toUpperCase(), result?.level || "none");
  $("liqHuntContent").innerHTML =
    renderMetrics([
      { label: "Level", value: result?.level || "Unavailable" },
      { label: "Direction", value: result?.direction || "Unavailable" },
      { label: "Score", value: formatNumber(result?.score, 1) },
      { label: "Cluster Side", value: result?.nearestClusterSide || "Unavailable" },
      { label: "Distance bps", value: formatNumber(result?.nearestClusterDistanceBps, 1) },
      { label: "Cluster Notional USD", value: formatInteger(result?.nearestClusterNotionalUsd) },
      { label: "Toward Cluster bps", value: formatNumber(result?.priceMoveTowardClusterBps, 1) },
      { label: "Distance Closing", value: formatBool(Boolean(result?.priceDistanceClosing)) },
    ]) +
    renderReasons(result?.reasonCodes || []);
}

function renderVpin() {
  const vpin = getData("/api/vpin-state");
  const error = getError("/api/vpin-state");
  if (error) {
    setBadge("vpinBadge", "API Error", "error");
    $("vpinContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const metrics = vpin?.metrics;
  const badge = metrics?.vpinExtreme ? "EXTREME" : metrics?.vpinSpike ? "SPIKE" : metrics?.vpinHigh ? "HIGH" : "NORMAL";
  setBadge("vpinBadge", badge, badge.toLowerCase());
  const insufficient = (metrics?.completedBucketCount || 0) < (metrics?.minBuckets || 0);
  $("vpinContent").innerHTML =
    renderMetrics([
      { label: "VPIN", value: insufficient ? "Insufficient buckets" : formatNumber(metrics?.vpin, 2) },
      { label: "VPIN Z-score", value: formatNumber(metrics?.vpinZscore, 2) },
      { label: "Completed Buckets", value: formatInteger(metrics?.completedBucketCount) },
      { label: "Active Progress", value: formatNumber((metrics?.activeBucketProgressRatio || 0) * 100, 1) + "%" },
      { label: "Dominant Direction", value: metrics?.dominantDirection || "Unavailable" },
      { label: "Latest Imbalance", value: formatNumber(metrics?.latestBucketImbalanceRatio, 2) },
    ]) +
    renderReasons(metrics?.reasonCodes || []);
}

function renderLiquidation() {
  const liquidation = getData("/api/liquidation-state");
  const error = getError("/api/liquidation-state");
  if (error) {
    setBadge("liquidationBadge", "API Error", "error");
    $("liquidationContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const metrics = liquidation?.metrics;
  const nearestShort = metrics?.nearestShortLiqClusterAbove;
  const nearestLong = metrics?.nearestLongLiqClusterBelow;
  const badge = metrics?.possibleLiqHuntSetup ? "SETUP" : metrics?.liqClusterNearby ? "NEARBY" : "CLEAR";
  setBadge("liquidationBadge", badge, metrics?.possibleLiqHuntSetup ? "red" : metrics?.liqClusterNearby ? "yellow" : "gray");
  $("liquidationContent").innerHTML =
    renderMetrics([
      { label: "Current Price", value: formatNumber(metrics?.currentMid, 1) },
      { label: "Short Cluster Above", value: nearestShort ? formatNumber(nearestShort.price, 1) : "Unavailable" },
      { label: "Short Distance bps", value: nearestShort ? formatNumber(nearestShort.distanceBps, 1) : "Unavailable" },
      { label: "Short Notional USD", value: nearestShort ? formatInteger(nearestShort.clusterNotionalUsd) : "Unavailable" },
      { label: "Long Cluster Below", value: nearestLong ? formatNumber(nearestLong.price, 1) : "Unavailable" },
      { label: "Long Distance bps", value: nearestLong ? formatNumber(nearestLong.distanceBps, 1) : "Unavailable" },
      { label: "Long Notional USD", value: nearestLong ? formatInteger(nearestLong.clusterNotionalUsd) : "Unavailable" },
      { label: "Liq Hunt Pressure", value: formatNumber(metrics?.liqHuntPressure, 2) },
    ]) +
    renderReasons(metrics?.reasonCodes || []);
}

function renderLiquidationToxicity() {
  const report = getData("/api/toxicity/liquidation/recent");
  const statusPayload = getData("/api/toxicity/liquidation/status");
  const error =
    getError("/api/toxicity/liquidation/recent") ||
    getError("/api/toxicity/liquidation/status");
  if (error) {
    setBadge("liquidationToxicityBadge", "API Error", "error");
    $("liquidationToxicityContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("liquidationToxicityBadge", "Loading", "none");
    $("liquidationToxicityContent").innerHTML =
      `<div class="muted">Liquidation toxicity will appear after the read-only liquidation analysis loads.</div>`;
    return;
  }

  const signals = report.signals || [];
  setBadge(
    "liquidationToxicityBadge",
    (statusPayload?.mode || "analysis_only").toUpperCase(),
    signals.length ? "warning" : "none"
  );
  $("liquidationToxicityContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      {
        label: "Runtime Modified",
        value: formatBool(Boolean(statusPayload?.runtimeModified ?? report.runtimeModified)),
      },
      { label: "Status", value: statusPayload?.mode || "analysis_only" },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Signal Count", value: formatInteger(statusPayload?.signalCount ?? signals.length) },
      { label: "Last Signal At", value: formatDateTime(statusPayload?.lastSignalAtMs) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshLiquidationToxicityButton">Refresh Liquidation Toxicity</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Warnings</div>
        <div class="metric-value">${(report.warnings || []).length ? report.warnings.join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">No Trade Reasons</div>
        <div class="metric-value">${(report.noTradeReasons || []).length ? report.noTradeReasons.join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Recent Signals</div>
        <div class="metric-value">${
          signals.length
            ? signals
                .map(
                  (signal) => `
            <div class="recommendation-card">
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Signal</div><div class="metric-value">${escapeHtml(signal.signalType || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Direction</div><div class="metric-value">${escapeHtml(signal.direction || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Confidence</div><div class="metric-value">${escapeHtml(signal.confidence || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Score</div><div class="metric-value">${formatInteger(signal.toxicityScore)}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Current Price</div><div class="metric-value">${formatNumber(signal.currentPrice, 1)}</div></div>
                <div class="metric"><div class="metric-label">Cluster Price</div><div class="metric-value">${formatNumber(signal.clusterPrice, 1)}</div></div>
                <div class="metric"><div class="metric-label">Distance USD</div><div class="metric-value">${formatNumber(signal.distanceUsd, 1)}</div></div>
                <div class="metric"><div class="metric-label">Distance bps</div><div class="metric-value">${formatNumber(signal.distanceBps, 1)}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Estimated Liq Notional USD</div><div class="metric-value">${formatNumber(signal.estimatedLiquidationNotional, 0)}</div></div>
                <div class="metric"><div class="metric-label">Cluster Density Score</div><div class="metric-value">${formatInteger(signal.clusterDensityScore)}</div></div>
                <div class="metric"><div class="metric-label">Magnet Score</div><div class="metric-value">${formatInteger(signal.magnetScore)}</div></div>
                <div class="metric"><div class="metric-label">Cascade Score</div><div class="metric-value">${formatInteger(signal.cascadeScore)}</div></div>
              </div>
              <div class="metric">
                <div class="metric-label">Reasons</div>
                <div class="metric-value">${(signal.reason || []).length ? signal.reason.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
              </div>
            </div>`
                )
                .join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          runtimeModified=false<br/>
          analysis_only=true<br/>
          no_live_execution=true
        </div>
      </div>
    </div>`;
}

function getOrderbookWallPayload() {
  return (
    state.data[orderbookWallRecentUrl()]?.data ||
    getData("/api/toxicity/orderbook-walls/recent")
  );
}

function getOrderbookWallStatusPayload() {
  return (
    state.data[orderbookWallStatusUrl()]?.data ||
    getData("/api/toxicity/orderbook-walls/status")
  );
}

function renderOrderbookWallLifecycle() {
  const url = orderbookWallRecentUrl();
  const statusUrl = orderbookWallStatusUrl();
  const report = getOrderbookWallPayload();
  const statusPayload = getOrderbookWallStatusPayload();
  const error =
    getError(url) ||
    getError(statusUrl) ||
    (!state.orderbookWallSymbol
      ? getError("/api/toxicity/orderbook-walls/recent") ||
        getError("/api/toxicity/orderbook-walls/status")
      : null);
  if (error) {
    setBadge("orderbookWallLifecycleBadge", "API Error", "error");
    $("orderbookWallLifecycleContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("orderbookWallLifecycleBadge", "Loading", "none");
    $("orderbookWallLifecycleContent").innerHTML =
      `<div class="muted">Orderbook wall lifecycle will appear after read-only book snapshots arrive.</div>`;
    return;
  }

  const trackedWalls = report.trackedWalls || [];
  const recentEvents = report.recentEvents || [];
  const candidates = report.toxicityCandidates || [];

  setBadge(
    "orderbookWallLifecycleBadge",
    (statusPayload?.analysisMode || report.analysisMode || "analysis_only").toUpperCase(),
    candidates.length ? "warning" : trackedWalls.length ? "blue" : "none"
  );
  $("orderbookWallLifecycleContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      {
        label: "Runtime Modified",
        value: formatBool(Boolean(statusPayload?.runtimeModified ?? report.runtimeModified)),
      },
      {
        label: "Analysis Mode",
        value: statusPayload?.analysisMode || report.analysisMode || "analysis_only",
      },
      {
        label: "Selected Symbol",
        value: report.symbol || statusPayload?.selectedSymbol || "Unavailable",
      },
      { label: "Status", value: report.status || statusPayload?.status || "Unavailable" },
      {
        label: "Tracked Walls",
        value: formatInteger(statusPayload?.trackedWallCount ?? trackedWalls.length),
      },
      {
        label: "Recent Events",
        value: formatInteger(statusPayload?.recentEventCount ?? recentEvents.length),
      },
      {
        label: "Candidates",
        value: formatInteger(statusPayload?.candidateCount ?? candidates.length),
      },
      { label: "Last Event At", value: formatDateTime(statusPayload?.lastEventAtMs) },
    ]) +
    `<div class="action-row">
      <input id="orderbookWallSymbolInput" placeholder="symbol" value="${escapeHtml(
        state.orderbookWallSymbol || report.symbol || ""
      )}" />
      <button type="button" class="small-button" id="selectOrderbookWallSymbolButton">Select Symbol</button>
      <button type="button" class="small-button" id="refreshOrderbookWallLifecycleButton">Refresh Orderbook Walls</button>
      <button type="button" class="small-button" id="copyOrderbookWallJsonButton">Copy JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Warnings</div>
        <div class="metric-value">${(report.warnings || []).length ? report.warnings.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">No Trade Reasons</div>
        <div class="metric-value">${(report.noTradeReasons || []).length ? report.noTradeReasons.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Tracked Walls</div>
        <div class="metric-value">${
          trackedWalls.length
            ? trackedWalls
                .map(
                  (wall) => `
            <div class="recommendation-card">
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Wall ID</div><div class="metric-value">${escapeHtml(wall.wallId || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Side</div><div class="metric-value">${escapeHtml(wall.side || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Price</div><div class="metric-value">${formatNumber(wall.price, 1)}</div></div>
                <div class="metric"><div class="metric-label">Status</div><div class="metric-value">${escapeHtml(wall.status || "Unavailable")}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Notional</div><div class="metric-value">${formatNumber(wall.notional, 0)}</div></div>
                <div class="metric"><div class="metric-label">Quantity</div><div class="metric-value">${formatNumber(wall.quantity, 4)}</div></div>
                <div class="metric"><div class="metric-label">Distance bps</div><div class="metric-value">${formatNumber(wall.distanceBps, 2)}</div></div>
                <div class="metric"><div class="metric-label">Touches</div><div class="metric-value">${formatInteger(wall.touches)}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">First Seen</div><div class="metric-value">${formatDateTime(wall.firstSeenMs)}</div></div>
                <div class="metric"><div class="metric-label">Last Seen</div><div class="metric-value">${formatDateTime(wall.lastSeenMs)}</div></div>
                <div class="metric"><div class="metric-label">Updates</div><div class="metric-value">${formatInteger(wall.updates)}</div></div>
              </div>
            </div>`
                )
                .join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Recent Lifecycle Events</div>
        <div class="metric-value">${
          recentEvents.length
            ? recentEvents
                .map(
                  (event) => `
            <div class="recommendation-card">
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Event</div><div class="metric-value">${escapeHtml(event.eventType || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Wall ID</div><div class="metric-value">${escapeHtml(event.wallId || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Side</div><div class="metric-value">${escapeHtml(event.side || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Observed At</div><div class="metric-value">${formatDateTime(event.observedAtMs)}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Price</div><div class="metric-value">${formatNumber(event.price, 1)}</div></div>
                <div class="metric"><div class="metric-label">Notional</div><div class="metric-value">${formatNumber(event.notional, 0)}</div></div>
                <div class="metric"><div class="metric-label">Distance bps</div><div class="metric-value">${formatNumber(event.distanceBps, 2)}</div></div>
              </div>
              <div class="metric">
                <div class="metric-label">Reason</div>
                <div class="metric-value">${escapeHtml(event.reason || "None")}</div>
              </div>
            </div>`
                )
                .join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Toxicity Candidates</div>
        <div class="metric-value">${
          candidates.length
            ? candidates
                .map(
                  (candidate) => `
            <div class="recommendation-card">
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Candidate</div><div class="metric-value">${escapeHtml(candidate.candidateType || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Side</div><div class="metric-value">${escapeHtml(candidate.side || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Score</div><div class="metric-value">${formatNumber(candidate.score, 1)}</div></div>
                <div class="metric"><div class="metric-label">Confidence</div><div class="metric-value">${escapeHtml(candidate.confidence || "Unavailable")}</div></div>
              </div>
              <div class="metric">
                <div class="metric-label">Reasons</div>
                <div class="metric-value">${(candidate.reasons || []).length ? candidate.reasons.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
              </div>
              <div class="metric">
                <div class="metric-label">Confluence</div>
                <div class="metric-value">${(candidate.confluence || []).length ? candidate.confluence.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
              </div>
            </div>`
                )
                .join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          runtimeModified=false<br/>
          analysis_only=true<br/>
          no_live_execution=true
        </div>
      </div>
    </div>` +
    (state.latestOrderbookWallAction
      ? `<div class="muted">${escapeHtml(state.latestOrderbookWallAction)}</div>`
      : "");
}

function getOrderbookWallInterpretationPayload() {
  return (
    state.data[orderbookWallInterpretationRecentUrl()]?.data ||
    getData("/api/toxicity/orderbook-wall-interpretation/recent")
  );
}

function getOrderbookWallInterpretationStatusPayload() {
  return (
    state.data[orderbookWallInterpretationStatusUrl()]?.data ||
    getData("/api/toxicity/orderbook-wall-interpretation/status")
  );
}

function renderOrderbookWallInterpretation() {
  const url = orderbookWallInterpretationRecentUrl();
  const statusUrl = orderbookWallInterpretationStatusUrl();
  const report = getOrderbookWallInterpretationPayload();
  const statusPayload = getOrderbookWallInterpretationStatusPayload();
  const error =
    getError(url) ||
    getError(statusUrl) ||
    (!state.orderbookWallInterpretationSymbol
      ? getError("/api/toxicity/orderbook-wall-interpretation/recent") ||
        getError("/api/toxicity/orderbook-wall-interpretation/status")
      : null);
  if (error) {
    setBadge("orderbookWallInterpretationBadge", "API Error", "error");
    $("orderbookWallInterpretationContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("orderbookWallInterpretationBadge", "Loading", "none");
    $("orderbookWallInterpretationContent").innerHTML =
      `<div class="muted">Orderbook wall interpretation will appear after the read-only wall lifecycle layer loads.</div>`;
    return;
  }

  const signals = report.signals || [];
  setBadge(
    "orderbookWallInterpretationBadge",
    (statusPayload?.mode || report.analysisMode || "analysis_only").toUpperCase(),
    signals.length ? "warning" : "none"
  );
  $("orderbookWallInterpretationContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      {
        label: "Runtime Modified",
        value: formatBool(Boolean(statusPayload?.runtimeModified ?? report.runtimeModified)),
      },
      {
        label: "Mode",
        value: statusPayload?.mode || report.analysisMode || "analysis_only",
      },
      {
        label: "Selected Symbol",
        value: report.selectedSymbol || "Unavailable",
      },
      { label: "Status", value: report.status || "Unavailable" },
      { label: "Signal Count", value: formatInteger(statusPayload?.signalCount ?? signals.length) },
      { label: "Last Signal At", value: formatDateTime(statusPayload?.lastSignalAtMs) },
    ]) +
    `<div class="action-row">
      <input id="orderbookWallInterpretationSymbolInput" placeholder="symbol" value="${escapeHtml(
        state.orderbookWallInterpretationSymbol || report.selectedSymbol || ""
      )}" />
      <button type="button" class="small-button" id="selectOrderbookWallInterpretationSymbolButton">Select Symbol</button>
      <button type="button" class="small-button" id="refreshOrderbookWallInterpretationButton">Refresh Wall Interpretation</button>
      <button type="button" class="small-button" id="copyOrderbookWallInterpretationJsonButton">Copy JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Warnings</div>
        <div class="metric-value">${(report.warnings || []).length ? report.warnings.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">No Trade Reasons</div>
        <div class="metric-value">${(report.noTradeReasons || []).length ? report.noTradeReasons.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Recent Signals</div>
        <div class="metric-value">${
          signals.length
            ? signals
                .map(
                  (signal) => `
            <div class="recommendation-card">
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Signal</div><div class="metric-value">${escapeHtml(signal.signalType || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Side</div><div class="metric-value">${escapeHtml(signal.side || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Confidence</div><div class="metric-value">${escapeHtml(signal.confidence || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Toxicity Score</div><div class="metric-value">${formatInteger(signal.toxicityScore)}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Wall Price</div><div class="metric-value">${formatNumber(signal.wallPrice, 1)}</div></div>
                <div class="metric"><div class="metric-label">Wall Notional USD</div><div class="metric-value">${formatNumber(signal.wallNotionalUsd, 0)}</div></div>
                <div class="metric"><div class="metric-label">Persistence ms</div><div class="metric-value">${formatInteger(signal.persistenceMs)}</div></div>
                <div class="metric"><div class="metric-label">Touch Count</div><div class="metric-value">${formatInteger(signal.touchCount)}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Consumed Ratio</div><div class="metric-value">${formatNumber(signal.consumedRatio, 2)}</div></div>
                <div class="metric"><div class="metric-label">Cancel Ratio</div><div class="metric-value">${formatNumber(signal.cancelRatio, 2)}</div></div>
                <div class="metric"><div class="metric-label">Spoof Score</div><div class="metric-value">${formatInteger(signal.spoofScore)}</div></div>
                <div class="metric"><div class="metric-label">Absorption Score</div><div class="metric-value">${formatInteger(signal.absorptionScore)}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Inducement Score</div><div class="metric-value">${formatInteger(signal.inducementScore)}</div></div>
                <div class="metric"><div class="metric-label">Moved Count</div><div class="metric-value">${formatInteger(signal.movedCount)}</div></div>
                <div class="metric"><div class="metric-label">Distance bps</div><div class="metric-value">${formatNumber(signal.distanceToMidBps, 2)}</div></div>
                <div class="metric"><div class="metric-label">Post-touch Markout bps</div><div class="metric-value">${formatNumber(signal.postTouchMarkoutBps, 2)}</div></div>
              </div>
              <div class="metric">
                <div class="metric-label">Reasons</div>
                <div class="metric-value">${(signal.reason || []).length ? signal.reason.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
              </div>
            </div>`
                )
                .join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          runtimeModified=false<br/>
          analysis_only=true<br/>
          no_live_execution=true
        </div>
      </div>
    </div>` +
    (state.latestOrderbookWallInterpretationAction
      ? `<div class="muted">${escapeHtml(state.latestOrderbookWallInterpretationAction)}</div>`
      : "");
}

function getStructuralToxicityPayload() {
  return (
    state.data[structuralToxicityRecentUrl()]?.data ||
    getData("/api/toxicity/structural/recent")
  );
}

function getStructuralToxicityStatusPayload() {
  return (
    state.data[structuralToxicityStatusUrl()]?.data ||
    getData("/api/toxicity/structural/status")
  );
}

function getToxicSignalFusionPayload() {
  return (
    state.data[toxicSignalFusionRecentUrl()]?.data ||
    getData("/api/toxicity/fusion/recent")
  );
}

function getToxicSignalFusionStatusPayload() {
  return (
    state.data[toxicSignalFusionStatusUrl()]?.data ||
    getData("/api/toxicity/fusion/status")
  );
}

function getToxicSignalInboxPayload() {
  return (
    state.data[toxicSignalInboxRecentUrl()]?.data ||
    getData("/api/toxicity/signal-inbox/recent")
  );
}

function getToxicSignalInboxStatusPayload() {
  return (
    state.data[toxicSignalInboxStatusUrl()]?.data ||
    getData("/api/toxicity/signal-inbox/status")
  );
}

function getToxicSignalGroupsPayload() {
  return (
    state.data[toxicSignalGroupsRecentUrl()]?.data ||
    getData("/api/toxicity/signal-groups/recent")
  );
}

function getToxicSignalGroupsStatusPayload() {
  return (
    state.data[toxicSignalGroupsStatusUrl()]?.data ||
    getData("/api/toxicity/signal-groups/status")
  );
}

function getToxicSignalDetailStatusPayload() {
  return state.data[toxicSignalDetailStatusUrl()]?.data;
}

function getToxicSignalHistoryPayload() {
  return (
    state.data[toxicSignalHistoryRecentUrl()]?.data ||
    getData("/api/toxicity/signal-history/recent")
  );
}

function getToxicSignalHistoryStatusPayload() {
  return (
    state.data[toxicSignalHistoryStatusUrl()]?.data ||
    getData("/api/toxicity/signal-history/status")
  );
}

function getToxicSignalHistoryAlertsPayload() {
  return (
    state.data[toxicSignalHistoryAlertsUrl()]?.data ||
    getData("/api/toxicity/signal-history/alerts/recent")
  );
}

function getToxicSignalHistoryReportsPayload() {
  return (
    state.data[toxicSignalHistoryReportsUrl()]?.data ||
    getData("/api/toxicity/signal-history/reports/recent")
  );
}

function getToxicSignalHealthPayload() {
  return (
    state.data[toxicSignalHealthSummaryUrl()]?.data ||
    getData("/api/toxicity/signal-health/summary")
  );
}

function getToxicSignalHealthStatusPayload() {
  return (
    state.data[toxicSignalHealthStatusUrl()]?.data ||
    getData("/api/toxicity/signal-health/status")
  );
}

function getToxicSignalReportPayload() {
  return (
    state.data[toxicSignalReportDailyUrl()]?.data ||
    getData("/api/toxicity/signal-report/daily")
  );
}

function getToxicSignalReportStatusPayload() {
  return (
    state.data[toxicSignalReportStatusUrl()]?.data ||
    getData("/api/toxicity/signal-report/status")
  );
}

function getToxicSignalRollingPayload() {
  return state.data[toxicSignalReportRollingUrl()]?.data;
}

function getToxicSignalAlertPreviewPayload() {
  return (
    state.data[toxicSignalAlertPreviewRecentUrl()]?.data ||
    getData("/api/toxicity/signal-alert-preview/recent")
  );
}

function getToxicSignalAlertPreviewItems() {
  return getToxicSignalAlertPreviewPayload()?.items || [];
}

function getToxicSignalAlertPreviewStatusPayload() {
  return (
    state.data[toxicSignalAlertPreviewStatusUrl()]?.data ||
    getData("/api/toxicity/signal-alert-preview/status")
  );
}

function getToxicSignalAlertPreviewExplainPayload() {
  return state.toxicSignalAlertExplainPayload;
}

function getDurableArchiveDryRunPayload() {
  return state.data[durableArchiveDryRunUrl()]?.data;
}

function getDurableArchiveDryRunReviewPackPayload() {
  return state.data[durableArchiveDryRunReviewPackLatestUrl()]?.data;
}

function getDurableArchiveWriteGatePayload() {
  return state.data[durableArchiveWriteGateStatusUrl()]?.data;
}

function getDurableArchiveWriteAuditStatusPayload() {
  return state.data[durableArchiveWriteAuditStatusUrl()]?.data;
}

function getDurableArchiveWriteAuditRecentPayload() {
  return state.data[durableArchiveWriteAuditRecentUrl()]?.data;
}

function getDurableArchiveWriteAuditLatestPayload() {
  return state.data[durableArchiveWriteAuditLatestUrl()]?.data;
}

function suspiciousSeverityRank(severity) {
  switch ((severity || "").toString().toLowerCase()) {
    case "extreme":
    case "high":
    case "alert":
      return 4;
    case "warning":
    case "medium":
      return 3;
    case "watch":
    case "low":
      return 2;
    default:
      return 1;
  }
}

function isSuspiciousInboxItem(item) {
  const action = (item?.operatorAction || "").toString().toLowerCase();
  const severityRank = suspiciousSeverityRank(item?.severity);
  return (
    severityRank >= 2 ||
    ["watch_signal_only", "review_evidence", "review_markout", "review_quality", "no_trade_warning"].includes(action)
  );
}

function currentSuspiciousToxicOrderItems() {
  const alertPreviewMap = new Map(
    getToxicSignalAlertPreviewItems().map((item) => [item.signalId, item])
  );
  const inbox = getToxicSignalInboxPayload();
  const inboxItems = (inbox?.items || [])
    .filter(isSuspiciousInboxItem)
    .map((item) => ({
      source: "inbox",
      id: item.signalId,
      signalId: item.signalId,
      symbol: item.symbol,
      kind: item.signalKind,
      direction: item.directionBias,
      severity: item.severity,
      confidence: typeof item.confidence === "number" ? item.confidence : Number(item.confidence) || 0,
      createdAtMs: item.createdAtMs || item.historyRecordedAtMs || 0,
      action: item.operatorAction,
      summary: item.fusion?.summary || "Suspicious toxic order-flow signal.",
      qualityBucket: item.quality?.qualityBucket || item.qualityBucket || "not_enough_data",
      recommendationAction: item.recommendation?.action || item.recommendationAction || "insufficient_data",
      alertDecision: alertPreviewMap.get(item.signalId)?.previewStatus || item.previewStatus || item.alertDecision || item.recommendation?.action || item.latestGovernanceDecision || "unknown",
      status: suspiciousOrderStatusFromItem(item),
      markoutOneMinute: item.markout?.oneMinute || item.markoutOneMinute || "not_enough_data",
      markoutFiveMinute: item.markout?.fiveMinute || item.markoutFiveMinute || "not_enough_data",
      markoutFifteenMinute: item.markout?.fifteenMinute || item.markoutFifteenMinute || "not_enough_data",
      markoutOneHour: item.markout?.oneHour || item.markoutOneHour || "not_enough_data",
      groupedBurstId: item.source?.groupId || item.groupId || null,
    }));

  if (inboxItems.length) {
    return inboxItems.sort((left, right) => {
      const severity = suspiciousSeverityRank(right.severity) - suspiciousSeverityRank(left.severity);
      return severity || (right.createdAtMs || 0) - (left.createdAtMs || 0);
    });
  }

  const fusion = getToxicSignalFusionPayload();
  return (fusion?.signals || [])
    .map((signal) => ({
      source: "fusion",
      id: signal.signalId,
      signalId: signal.signalId,
      symbol: signal.symbol,
      kind: signal.signalType,
      direction: signal.direction,
      severity: signal.chaseRisk || "watch",
      confidence: typeof signal.confidence === "number" ? signal.confidence : Number(signal.confidence) || 0,
      createdAtMs: signal.tsMs || 0,
      action: signal.noTradeReasons?.length ? "no_trade_warning" : "watch_signal_only",
      summary: signal.primaryReason || "Suspicious toxic order-flow signal.",
      qualityBucket: signal.chaseRisk || "not_enough_data",
      recommendationAction: signal.noTradeReasons?.length ? "no_trade_only" : "watch_signal_only",
      alertDecision: signal.noTradeReasons?.length ? "review_candidate" : "notify_candidate",
      status: suspiciousOrderStatusFromFusion(signal),
      markoutOneMinute: signal.markoutOneMinute || "not_enough_data",
      markoutFiveMinute: signal.markoutFiveMinute || "not_enough_data",
      markoutFifteenMinute: signal.markoutFifteenMinute || "not_enough_data",
      markoutOneHour: signal.markoutOneHour || "not_enough_data",
      groupedBurstId: signal.groupId || signal.source?.groupId || null,
    }))
    .sort((left, right) => (right.createdAtMs || 0) - (left.createdAtMs || 0));
}

function suspiciousOrderKey(item) {
  return String(
    item?.signalId ||
      item?.id ||
      `${item?.symbol || "UNKNOWN"}:${item?.kind || "toxic_signal"}:${item?.createdAtMs || 0}`
  );
}

function pruneSuspiciousOrdersLastSeen(nowMs = Date.now()) {
  const next = {};
  Object.entries(state.suspiciousOrdersLastSeen || {}).forEach(([key, entry]) => {
    if (nowMs - Number(entry.lastSeenAtMs || 0) <= SUSPICIOUS_ORDERS_LAST_SEEN_WINDOW_MS) {
      next[key] = entry;
    }
  });
  state.suspiciousOrdersLastSeen = next;
}

function syncSuspiciousOrdersLastSeen(liveItems, nowMs = Date.now()) {
  pruneSuspiciousOrdersLastSeen(nowMs);
  const existing = state.suspiciousOrdersLastSeen || {};
  const next = { ...existing };
  const liveKeys = new Set(liveItems.map((item) => suspiciousOrderKey(item)));

  Object.entries(next).forEach(([key, entry]) => {
    if (!liveKeys.has(key)) {
      next[key] = { ...entry, snapshotState: "stale" };
    }
  });

  liveItems.forEach((item) => {
    const key = suspiciousOrderKey(item);
    const previous = next[key];
    next[key] = {
      ...item,
      firstSeenAtMs: previous?.firstSeenAtMs || nowMs,
      lastSeenAtMs: nowMs,
      snapshotState: "live",
    };
  });

  state.suspiciousOrdersLastSeen = next;
}

function getSuspiciousToxicOrderItems() {
  pruneSuspiciousOrdersLastSeen();
  return Object.values(state.suspiciousOrdersLastSeen || {}).sort((left, right) => {
    const liveRank =
      (right.snapshotState === "live" ? 1 : 0) - (left.snapshotState === "live" ? 1 : 0);
    return (
      liveRank ||
      suspiciousSeverityRank(right.severity) - suspiciousSeverityRank(left.severity) ||
      Number(right.lastSeenAtMs || right.createdAtMs || 0) -
        Number(left.lastSeenAtMs || left.createdAtMs || 0)
    );
  });
}

function suspiciousOrdersSortMode() {
  return state.suspiciousOrdersSortMode || "severity";
}

function suspiciousOrdersFilterSymbol() {
  return (state.suspiciousOrdersFilterSymbol || "").trim().toUpperCase();
}

function suspiciousOrdersFilterAlertDecision() {
  return (state.suspiciousOrdersFilterAlertDecision || "").trim().toLowerCase();
}

function suspiciousOrdersVisibleItems(items = getSuspiciousToxicOrderItems()) {
  const symbolFilter = suspiciousOrdersFilterSymbol();
  const alertDecisionFilter = suspiciousOrdersFilterAlertDecision();
  const hideNotEnoughData = Boolean(state.suspiciousOrdersHideNotEnoughData);
  const highSeverityOnly = Boolean(state.suspiciousOrdersHighSeverityOnly);
  const filteredItems = items
    .filter((item) => {
      const symbol = String(item.symbol || "").toUpperCase();
      const alertDecision = String(item.alertDecision || "").toLowerCase();
      const status = normalizeSuspiciousStatus(item.status);
      if (symbolFilter && !symbol.includes(symbolFilter)) {
        return false;
      }
      if (alertDecisionFilter && !alertDecision.includes(alertDecisionFilter)) {
        return false;
      }
      if (hideNotEnoughData && status === "not_enough_data") {
        return false;
      }
      if (highSeverityOnly && suspiciousSeverityRank(item.severity) < 4) {
        return false;
      }
      return true;
    })
    .sort((left, right) => {
      switch (suspiciousOrdersSortMode()) {
        case "confidence":
          return (
            (right.confidence || 0) - (left.confidence || 0) ||
            suspiciousSeverityRank(right.severity) - suspiciousSeverityRank(left.severity) ||
            (right.createdAtMs || 0) - (left.createdAtMs || 0)
          );
        case "createdAtMs":
          return (
            (right.createdAtMs || 0) - (left.createdAtMs || 0) ||
            suspiciousSeverityRank(right.severity) - suspiciousSeverityRank(left.severity) ||
            (right.confidence || 0) - (left.confidence || 0)
          );
        case "severity":
        default:
          return (
            suspiciousSeverityRank(right.severity) - suspiciousSeverityRank(left.severity) ||
            (right.confidence || 0) - (left.confidence || 0) ||
            (right.createdAtMs || 0) - (left.createdAtMs || 0)
          );
      }
    });
  return filteredItems;
}

function suspiciousOrdersSummaryText() {
  const parts = [
    `readOnly=true`,
    `analysisOnly=true`,
    `executionEnabled=false`,
    `view-only`,
    `persistentWatchlistEnabled=false`,
    `runtimeMonitorModified=false`,
  ];
  return parts.join(" / ");
}

function suspiciousOrderStatusFromItem(item) {
  return normalizeSuspiciousStatus(
    item?.status ||
      item?.markout?.oneMinute ||
      item?.markout?.fiveMinute ||
      item?.markout?.fifteenMinute ||
      item?.markout?.oneHour ||
      item?.markoutOneMinute ||
      item?.markoutFiveMinute ||
      item?.markoutFifteenMinute ||
      item?.markoutOneHour ||
      item?.quality?.qualityBucket ||
      item?.qualityBucket
  );
}

function suspiciousOrderStatusFromFusion(signal) {
  return normalizeSuspiciousStatus(
    signal?.status ||
      signal?.markoutOneMinute ||
      signal?.markoutFiveMinute ||
      signal?.markoutFifteenMinute ||
      signal?.markoutOneHour ||
      signal?.toxicityScore ||
      signal?.chaseRisk
  );
}

function normalizeSuspiciousStatus(value) {
  const status = (value || "").toString().toLowerCase();
  if (["aligned", "adverse", "neutral", "not_enough_data"].includes(status)) {
    return status;
  }
  if (status.includes("adverse")) {
    return "adverse";
  }
  if (status.includes("aligned")) {
    return "aligned";
  }
  if (status.includes("neutral")) {
    return "neutral";
  }
  return "not_enough_data";
}

function suspiciousOrderStatusTone(status) {
  switch (normalizeSuspiciousStatus(status)) {
    case "aligned":
      return "success";
    case "adverse":
      return "danger";
    case "neutral":
      return "warning";
    case "not_enough_data":
    default:
      return "muted";
  }
}

function renderSuspiciousToxicOrderItem(item) {
  const confidence =
    typeof item.confidence === "number"
      ? `${formatNumber(item.confidence * 100, 1)}%`
      : escapeHtml(item.confidence || "Unavailable");
  const status = normalizeSuspiciousStatus(item.status);
  const alertDecision = item.alertDecision || "unknown";
  const snapshotState = item.snapshotState === "stale" ? "STALE" : "LIVE";
  const lastSeenAtMs = Number(item.lastSeenAtMs || item.createdAtMs || 0);
  return `
    <div class="suspicious-order">
      <div class="suspicious-order-header">
        <div class="suspicious-order-title">
          <div class="suspicious-order-symbol">${escapeHtml(item.signalId || item.id || "UNKNOWN")}</div>
          <div class="suspicious-order-meta">
            ${escapeHtml(item.symbol || "UNKNOWN")} · ${escapeHtml(item.kind || "toxic_signal")} · ${formatDateTime(item.createdAtMs)}
          </div>
        </div>
        <span class="badge ${badgeClass(item.severity)}">${escapeHtml(item.severity || "watch")}</span>
      </div>
      <div class="suspicious-order-summary">${escapeHtml(item.summary || "Suspicious toxic order-flow signal.")}</div>
      <div class="signal-chip-row">
        <span class="signal-chip signal-chip-${item.snapshotState === "stale" ? "warning" : "success"}">snapshot ${snapshotState}</span>
        <span class="signal-chip signal-chip-warning">confidence ${confidence}</span>
        <span class="signal-chip signal-chip-muted">alertDecision ${escapeHtml(alertDecision)}</span>
        <span class="signal-chip signal-chip-${suspiciousOrderStatusTone(status)}">status ${escapeHtml(status)}</span>
        <span class="signal-chip signal-chip-muted">createdAt ${formatDateTime(item.createdAtMs)}</span>
        <span class="signal-chip signal-chip-muted">last seen ${escapeHtml(formatAgeFromNow(lastSeenAtMs))}</span>
      </div>
      <div class="signal-chip-row">
        <span class="signal-chip signal-chip-muted">action ${escapeHtml(item.action || "watch_signal_only")}</span>
        <span class="signal-chip signal-chip-muted">markout 1m ${escapeHtml(item.markoutOneMinute || "not_enough_data")}</span>
        <span class="signal-chip signal-chip-muted">markout 5m ${escapeHtml(item.markoutFiveMinute || "not_enough_data")}</span>
        <span class="signal-chip signal-chip-muted">markout 15m ${escapeHtml(item.markoutFifteenMinute || "not_enough_data")}</span>
        <span class="signal-chip signal-chip-muted">markout 1h ${escapeHtml(item.markoutOneHour || "not_enough_data")}</span>
      </div>
      <div class="action-row">
        <button type="button" class="small-button" data-suspicious-replay-signal-id="${escapeHtml(item.signalId || item.id || "")}" data-suspicious-replay-symbol="${escapeHtml(item.symbol || "")}">查看回放</button>
      </div>
    </div>`;
}

function suspiciousReplaySelectedSymbol() {
  const explicit = (state.suspiciousReplaySymbol || "").trim();
  return explicit ? explicit.toUpperCase() : null;
}

function suspiciousReplayStatusUrl() {
  const symbol = suspiciousReplaySelectedSymbol();
  return symbol
    ? `/api/toxicity/signal-history/status?symbol=${encodeURIComponent(symbol)}`
    : "/api/toxicity/signal-history/status";
}

function suspiciousReplayHistoryUrl() {
  const symbol = suspiciousReplaySelectedSymbol();
  return symbol
    ? `${toxicSignalHistorySymbolEndpointTemplate.replace(":symbol", encodeURIComponent(symbol))}`
    : "/api/toxicity/signal-history/recent";
}

function suspiciousReplaySignalUrl(signalId = state.suspiciousReplaySignalId) {
  const normalizedSignalId = (signalId || "").trim();
  if (!normalizedSignalId) {
    return null;
  }
  const symbol = suspiciousReplaySelectedSymbol();
  const base = toxicSignalHistorySignalEndpointTemplate.replace(
    ":signal_id",
    encodeURIComponent(normalizedSignalId)
  );
  return symbol ? `${base}?symbol=${encodeURIComponent(symbol)}` : base;
}

function suspiciousReplayDetailUrl(signalId = state.suspiciousReplaySignalId, symbolOverride = null) {
  const normalizedSignalId = (signalId || "").trim();
  if (!normalizedSignalId) {
    return null;
  }
  const symbol = symbolOverride || suspiciousReplaySelectedSignalSymbol();
  const base = toxicSignalDetailSignalEndpointTemplate.replace(
    ":signal_id",
    encodeURIComponent(normalizedSignalId)
  );
  return symbol ? `${base}?symbol=${encodeURIComponent(symbol)}` : base;
}

function suspiciousReplayExplainUrl(signalId = state.suspiciousReplaySignalId, symbolOverride = null) {
  const normalizedSignalId = (signalId || "").trim();
  if (!normalizedSignalId) {
    return null;
  }
  const symbol = symbolOverride || suspiciousReplaySelectedSignalSymbol();
  const base = toxicSignalAlertPreviewExplainEndpointTemplate.replace(
    ":signal_id",
    encodeURIComponent(normalizedSignalId)
  );
  return symbol ? `${base}?symbol=${encodeURIComponent(symbol)}` : base;
}

function suspiciousReplayOverlaySymbol() {
  return suspiciousReplaySelectedSignalSymbol() || suspiciousReplaySelectedSymbol();
}

function whaleFlowSymbolUrl(symbol) {
  const normalizedSymbol = (symbol || "").trim();
  if (!normalizedSymbol) {
    return null;
  }
  return whaleFlowSymbolEndpointTemplate.replace(":symbol", encodeURIComponent(normalizedSymbol));
}

function toxicMarkoutSymbolUrl(symbol) {
  const normalizedSymbol = (symbol || "").trim();
  if (!normalizedSymbol) {
    return null;
  }
  return toxicMarkoutSymbolEndpointTemplate.replace(":symbol", encodeURIComponent(normalizedSymbol));
}

function toxicQualityScorecardSymbolUrl(symbol) {
  const normalizedSymbol = (symbol || "").trim();
  if (!normalizedSymbol) {
    return null;
  }
  return toxicQualityScorecardSymbolEndpointTemplate.replace(
    ":symbol",
    encodeURIComponent(normalizedSymbol)
  );
}

function toxicWeightRecommendationSymbolUrl(symbol) {
  const normalizedSymbol = (symbol || "").trim();
  if (!normalizedSymbol) {
    return null;
  }
  return toxicWeightRecommendationSymbolEndpointTemplate.replace(
    ":symbol",
    encodeURIComponent(normalizedSymbol)
  );
}

function toxicWeightReviewSymbolUrl(symbol) {
  const normalizedSymbol = (symbol || "").trim();
  if (!normalizedSymbol) {
    return null;
  }
  return toxicWeightReviewSymbolEndpointTemplate.replace(
    ":symbol",
    encodeURIComponent(normalizedSymbol)
  );
}

function toxicGovernanceLedgerSymbolUrl(symbol) {
  const normalizedSymbol = (symbol || "").trim();
  if (!normalizedSymbol) {
    return null;
  }
  return toxicGovernanceLedgerSymbolEndpointTemplate.replace(
    ":symbol",
    encodeURIComponent(normalizedSymbol)
  );
}

function getSuspiciousReplayStatusPayload() {
  return state.suspiciousReplayStatusPayload || getData("/api/toxicity/signal-history/status");
}

function getSuspiciousReplayHistoryPayload() {
  return state.suspiciousReplayHistoryPayload || getData("/api/toxicity/signal-history/recent");
}

function getSuspiciousReplayItems() {
  return sortRecentSignalItems(getSuspiciousReplayHistoryPayload()?.items || [], "severity");
}

function suspiciousReplaySelectedSignalSymbol() {
  const explicit = suspiciousReplaySelectedSymbol();
  if (explicit) {
    return explicit;
  }
  const lookupSymbol = state.suspiciousReplayLookupPayload?.signal?.symbol;
  if (lookupSymbol) {
    return String(lookupSymbol).toUpperCase();
  }
  const currentSignalId = (state.suspiciousReplaySignalId || "").trim();
  if (currentSignalId) {
    const matchedHistory = getSuspiciousReplayItems().find((item) => item.signalId === currentSignalId);
    if (matchedHistory?.symbol) {
      return String(matchedHistory.symbol).toUpperCase();
    }
    const matchedSuspicious = getSuspiciousToxicOrderItems().find((item) => item.id === currentSignalId);
    if (matchedSuspicious?.symbol) {
      return String(matchedSuspicious.symbol).toUpperCase();
    }
  }
  const runtimeSymbol = getData("/api/status")?.symbol;
  return runtimeSymbol ? String(runtimeSymbol).toUpperCase() : null;
}

function buildSuspiciousReplayCopyPayload() {
  const overlaySymbol = suspiciousReplayOverlaySymbol();
  const whaleFlowOverlay = buildWhaleReplayOverlay(overlaySymbol, state.suspiciousReplaySignalId);
  return {
    filter: {
      symbol: suspiciousReplaySelectedSymbol(),
      signalId: state.suspiciousReplaySignalId || null,
      readOnly: true,
      analysisOnly: true,
      executionEnabled: false,
      persistentWatchlistEnabled: false,
      runtimeMonitorModified: false,
    },
    status: getSuspiciousReplayStatusPayload() || null,
    history: getSuspiciousReplayHistoryPayload() || null,
    lookup: state.suspiciousReplayLookupPayload || null,
    detail: state.suspiciousReplayDetailPayload || null,
    alertExplainability: state.suspiciousReplayExplainPayload || null,
    overlay: overlaySymbol
      ? {
          symbol: overlaySymbol,
          whaleFlowOverlay,
          markout: getData(toxicMarkoutSymbolUrl(overlaySymbol)) || null,
          qualityScorecard: getData(toxicQualityScorecardSymbolUrl(overlaySymbol)) || null,
          weightRecommendation: getData(toxicWeightRecommendationSymbolUrl(overlaySymbol)) || null,
          weightReview: getData(toxicWeightReviewSymbolUrl(overlaySymbol)) || null,
          governanceLedger: getData(toxicGovernanceLedgerSymbolUrl(overlaySymbol)) || null,
        }
      : null,
  };
}

function whaleCandidateLinkedSignalIds(candidate) {
  return [
    ...(candidate?.linkedActiveTradeSignalIds || []),
    ...(candidate?.linkedLiquidationSignalIds || []),
    ...(candidate?.linkedWallCandidateIds || []),
    ...(candidate?.linkedWallInterpretationSignalIds || []),
    ...(candidate?.linkedStructuralSignalIds || []),
    ...(candidate?.linkedFusionSignalIds || []),
  ].filter(Boolean);
}

function getWhaleFlowPayloadForSymbol(symbol) {
  const url = whaleFlowSymbolUrl(symbol);
  return url ? getData(url) || null : null;
}

function findWhaleFlowCandidateForSignal(whalePayload, signalId) {
  const normalizedSignalId = (signalId || "").trim();
  if (!whalePayload || !normalizedSignalId) {
    return null;
  }
  return (whalePayload.candidates || []).find((candidate) =>
    whaleCandidateLinkedSignalIds(candidate).includes(normalizedSignalId)
  ) || null;
}

function findWhaleFlowLinkedHistoryItem(candidate, symbol) {
  const linkedSignalIds = whaleCandidateLinkedSignalIds(candidate);
  if (!linkedSignalIds.length) {
    return null;
  }
  return getSuspiciousReplayItems().find((item) => {
    const itemSignalId = item.signalId || item.id || "";
    const itemSymbol = String(item.symbol || "").toUpperCase();
    return linkedSignalIds.includes(itemSignalId) && (!symbol || itemSymbol === String(symbol).toUpperCase());
  }) || null;
}

function buildWhaleOverlayMarkoutFromHistory(item) {
  if (!item) {
    return {
      oneMinute: "not_enough_data",
      fiveMinute: "not_enough_data",
      fifteenMinute: "not_enough_data",
      oneHour: "not_enough_data",
    };
  }
  return {
    oneMinute: item.markoutOneMinute || "not_enough_data",
    fiveMinute: item.markoutFiveMinute || "not_enough_data",
    fifteenMinute: item.markoutFifteenMinute || "not_enough_data",
    oneHour: item.markoutOneHour || "not_enough_data",
  };
}

function buildWhaleReplayOverlay(symbolOverride = null, signalIdOverride = null) {
  const symbol = (symbolOverride || suspiciousReplayOverlaySymbol() || "").trim();
  const signalId = (signalIdOverride || state.suspiciousReplaySignalId || "").trim();
  if (!symbol) {
    return {
      available: false,
      partial: true,
      reason: "Whale flow data unavailable",
      operatorNote: "signal-only, no execution",
    };
  }

  const whalePayload = getWhaleFlowPayloadForSymbol(symbol);
  if (!whalePayload) {
    return {
      available: false,
      partial: true,
      symbol,
      reason: "Whale flow data unavailable",
      operatorNote: "signal-only, no execution",
    };
  }

  let candidate = signalId
    ? findWhaleFlowCandidateForSignal(whalePayload, signalId)
    : (whalePayload.candidates || [])[0] || null;
  let linkedHistoryItem = null;
  if (candidate) {
    linkedHistoryItem = signalId
      ? getSuspiciousReplayItems().find((item) => (item.signalId || item.id || "") === signalId) || null
      : findWhaleFlowLinkedHistoryItem(candidate, symbol);
  }

  if (!candidate) {
    return {
      available: false,
      partial: whalePayload.dataQuality?.status === "partial" || whalePayload.dataQuality?.status === "degraded",
      symbol,
      reason: signalId
        ? "No whale flow candidate for selected signal."
        : whalePayload.status === "no_whale_flow"
          ? "No whale flow candidate"
          : "Whale flow data unavailable",
      baselineSource: whalePayload.baselineQuality?.baselineSource || "insufficient_history",
      dataQuality: whalePayload.dataQuality?.status || "no_data",
      degradationWarnings: whalePayload.degradationWarnings || [],
      operatorNote: "signal-only, no execution",
    };
  }

  const markout = buildWhaleOverlayMarkoutFromHistory(linkedHistoryItem);
  const degradationWarnings = [
    ...(candidate.diagnostics?.degradationReasons || []),
    ...(whalePayload.degradationWarnings || []),
  ].filter(Boolean);
  const partial = Boolean(degradationWarnings.length || whalePayload.dataQuality?.status === "partial" || whalePayload.dataQuality?.status === "degraded");

  return {
    available: true,
    partial,
    symbol,
    signalId: signalId || linkedHistoryItem?.signalId || null,
    classification: candidate.candidateType || "no_candidate",
    window: candidate.window || null,
    windowMs: candidate.windowMs || null,
    volumeBtc: candidate.volumeBtc ?? null,
    directionBias: candidate.direction || "neutral",
    directionRatio: candidate.directionBias ?? null,
    relativeVolumeMultiple: candidate.historicalVolumeRatio ?? null,
    venueConfluence: candidate.sameDirectionVenues ?? 0,
    venueConfluenceSatisfied: Boolean(
      whalePayload.venueCoverage?.venueConfluenceSatisfied ??
      ((candidate.sameDirectionVenues || 0) >= (whalePayload.thresholds?.minVenueConfirmations || 2))
    ),
    priceImpactBps: candidate.priceImpactBps ?? null,
    depthDropRatio: candidate.depthDropRatio ?? null,
    dataQuality: candidate.diagnostics?.dataQuality || whalePayload.dataQuality?.status || "no_data",
    baselineSource: whalePayload.baselineQuality?.baselineSource || "insufficient_history",
    degradationWarnings,
    whyCandidate: candidate.diagnostics?.whyCandidate || [],
    missingInputs: candidate.diagnostics?.missingInputs || [],
    confidenceModifiers: candidate.diagnostics?.confidenceModifiers || [],
    markout,
    operatorNote: "signal-only, no execution",
  };
}

function buildWhaleReplayOverlayMarkdown(overlay) {
  const payload = overlay || {
    available: false,
    partial: true,
    reason: "Whale flow data unavailable",
    operatorNote: "signal-only, no execution",
  };
  const lines = [
    "## Whale Flow Overlay",
    `- Available: ${payload.available ? "true" : "false"}`,
    `- Partial: ${payload.partial ? "true" : "false"}`,
    `- Symbol: ${payload.symbol || "Unavailable"}`,
    `- Classification: ${payload.classification || "No whale flow candidate"}`,
    `- Volume BTC: ${payload.volumeBtc == null ? "Unavailable" : formatNumber(payload.volumeBtc, 1)}`,
    `- Direction bias: ${payload.directionBias || "neutral"}`,
    `- Direction ratio: ${payload.directionRatio == null ? "Unavailable" : formatPercent(payload.directionRatio, 0)}`,
    `- Venue confluence: ${payload.venueConfluence == null ? "Unavailable" : payload.venueConfluence}`,
    `- Baseline source: ${payload.baselineSource || "insufficient_history"}`,
    `- Data quality: ${payload.dataQuality || "no_data"}`,
    `- Markout +1m: ${payload.markout?.oneMinute || "not_enough_data"}`,
    `- Markout +5m: ${payload.markout?.fiveMinute || "not_enough_data"}`,
    `- Markout +15m: ${payload.markout?.fifteenMinute || "not_enough_data"}`,
    `- Markout +1h: ${payload.markout?.oneHour || "not_enough_data"}`,
    `- Operator note: ${payload.operatorNote || "signal-only, no execution"}`,
  ];
  if (payload.reason) {
    lines.push(`- Reason: ${payload.reason}`);
  }
  if ((payload.degradationWarnings || []).length) {
    lines.push(`- Degradation warnings: ${payload.degradationWarnings.join(" | ")}`);
  }
  if ((payload.missingInputs || []).length) {
    lines.push(`- Missing inputs: ${payload.missingInputs.join(" | ")}`);
  }
  return lines.join("\n");
}

function normalizeReplayRecommendation(value) {
  const normalized = String(value || "").trim().toLowerCase();
  if (!normalized) {
    return "insufficient_data";
  }
  return normalized;
}

function renderReplayOverlayMetricRows(payload, symbol) {
  const whaleFlowOverlay = payload?.whaleFlowOverlay || null;
  const markout = payload?.markout || null;
  const quality = payload?.qualityScorecard || null;
  const recommendation = payload?.weightRecommendation || null;
  const review = payload?.weightReview || null;
  const ledger = payload?.governanceLedger || null;
  return renderMetrics([
    { label: "Overlay Symbol", value: escapeHtml(symbol || "Unavailable") },
    { label: "Markout Status", value: escapeHtml(markout?.status || "Unavailable") },
    { label: "Markout Signals", value: formatInteger(markout?.signalCount) },
    { label: "Quality Status", value: escapeHtml(quality?.status || "Unavailable") },
    { label: "Quality Evaluations", value: formatInteger(quality?.totalEvaluations) },
    {
      label: "Whale Overlay",
      value: escapeHtml(
        whaleFlowOverlay?.available
          ? whaleFlowOverlay.classification || "available"
          : whaleFlowOverlay?.reason || "No whale flow candidate"
      ),
    },
    {
      label: "Whale Baseline",
      value: escapeHtml(whaleFlowOverlay?.baselineSource || "insufficient_history"),
    },
    {
      label: "Whale Data Quality",
      value: escapeHtml(whaleFlowOverlay?.dataQuality || "no_data"),
    },
    {
      label: "Recommendation Status",
      value: escapeHtml(recommendation?.status || "Unavailable"),
    },
    {
      label: "Recommendation Count",
      value: formatInteger(recommendation?.totalRecommendations),
    },
    { label: "Review Status", value: escapeHtml(review?.status || "Unavailable") },
    { label: "Ledger Status", value: escapeHtml(ledger?.status || "Unavailable") },
  ]);
}

function renderSuspiciousReplayOverlaySummary(payload, symbol) {
  const whaleFlowOverlay = payload?.whaleFlowOverlay || null;
  const markout = payload?.markout || null;
  const quality = payload?.qualityScorecard || null;
  const recommendation = payload?.weightRecommendation || null;
  const review = payload?.weightReview || null;
  const ledger = payload?.governanceLedger || null;
  const markoutSignals = markout?.signals || [];
  const qualityCandidates = recommendation?.recommendations || review?.reviewItems || [];

  return `
    <div class="signal-details-grid">
      <details class="signal-details" open>
        <summary>Whale Flow Overlay</summary>
        ${
          whaleFlowOverlay?.available
            ? `
              ${renderMetrics([
                { label: "Classification", value: escapeHtml(formatWhaleFlowCandidateType(whaleFlowOverlay.classification)) },
                { label: "Window", value: escapeHtml(whaleFlowOverlay.window || "Unavailable") },
                { label: "Window Ms", value: formatInteger(whaleFlowOverlay.windowMs) },
                { label: "Volume BTC", value: whaleFlowOverlay.volumeBtc == null ? "Unavailable" : `${formatNumber(whaleFlowOverlay.volumeBtc, 1)} BTC` },
                { label: "Direction bias", value: escapeHtml(whaleFlowOverlay.directionBias || "neutral") },
                { label: "Direction ratio", value: whaleFlowOverlay.directionRatio == null ? "Unavailable" : formatPercent(whaleFlowOverlay.directionRatio, 0) },
                { label: "Relative volume", value: whaleFlowOverlay.relativeVolumeMultiple == null ? "Unavailable" : `${formatNumber(whaleFlowOverlay.relativeVolumeMultiple, 2)}x` },
                { label: "Venue confluence", value: `${formatInteger(whaleFlowOverlay.venueConfluence)} / ${whaleFlowOverlay.venueConfluenceSatisfied ? "satisfied" : "not satisfied"}` },
                { label: "Price impact", value: formatNumber(whaleFlowOverlay.priceImpactBps, 2) },
                { label: "Depth drop", value: formatPercent(whaleFlowOverlay.depthDropRatio, 0) },
                { label: "Data Quality", value: escapeHtml(formatWhaleFlowQualityStatus(whaleFlowOverlay.dataQuality)) },
                { label: "Baseline Source", value: escapeHtml(formatWhaleFlowBaselineSource(whaleFlowOverlay.baselineSource)) },
              ])}
              <div class="signal-chip-row">
                ${renderSignalChip(`+1m ${escapeHtml(whaleFlowOverlay.markout?.oneMinute || "not_enough_data")}`, replayHeatmapStatusTone(whaleFlowOverlay.markout?.oneMinute))}
                ${renderSignalChip(`+5m ${escapeHtml(whaleFlowOverlay.markout?.fiveMinute || "not_enough_data")}`, replayHeatmapStatusTone(whaleFlowOverlay.markout?.fiveMinute))}
                ${renderSignalChip(`+15m ${escapeHtml(whaleFlowOverlay.markout?.fifteenMinute || "not_enough_data")}`, replayHeatmapStatusTone(whaleFlowOverlay.markout?.fifteenMinute))}
                ${renderSignalChip(`+1h ${escapeHtml(whaleFlowOverlay.markout?.oneHour || "not_enough_data")}`, replayHeatmapStatusTone(whaleFlowOverlay.markout?.oneHour))}
              </div>
              <div class="muted">${
                whaleFlowOverlay.partial
                  ? "Whale flow overlay partial: venue/depth/baseline inputs are missing."
                  : "Whale flow overlay available for replay review."
              }</div>
              <div class="muted">${
                (whaleFlowOverlay.degradationWarnings || []).join("<br/>") || "No degradation warnings"
              }</div>
            `
            : `<div class="muted">${escapeHtml(whaleFlowOverlay?.reason || "No whale flow candidate for selected signal.")}</div>`
        }
      </details>
      <details class="signal-details" open>
        <summary>Markout Overlay</summary>
        ${
          markout
            ? `
              ${renderMetrics([
                { label: "Selected Symbol", value: escapeHtml(markout.selectedSymbol || symbol || "Unavailable") },
                { label: "Status", value: escapeHtml(markout.status || "Unavailable") },
                { label: "Signal Count", value: formatInteger(markout.signalCount) },
                { label: "Read Only", value: formatBool(Boolean(markout.readOnly)) },
                { label: "Analysis Only", value: formatBool(Boolean(markout.analysisOnly)) },
              ])}
              <div class="signal-chip-row">
                ${renderSignalChip(`1m ${escapeHtml(markoutSignals[0]?.windows?.[0]?.outcome || "not_enough_data")}`, markoutSignals[0]?.windows?.[0]?.outcome === "aligned" ? "success" : markoutSignals[0]?.windows?.[0]?.outcome === "adverse" ? "danger" : markoutSignals[0]?.windows?.[0]?.outcome === "neutral" ? "warning" : "muted")}
                ${renderSignalChip(`5m ${escapeHtml(markoutSignals[0]?.windows?.[1]?.outcome || "not_enough_data")}`, markoutSignals[0]?.windows?.[1]?.outcome === "aligned" ? "success" : markoutSignals[0]?.windows?.[1]?.outcome === "adverse" ? "danger" : markoutSignals[0]?.windows?.[1]?.outcome === "neutral" ? "warning" : "muted")}
                ${renderSignalChip(`15m ${escapeHtml(markoutSignals[0]?.windows?.[2]?.outcome || "not_enough_data")}`, markoutSignals[0]?.windows?.[2]?.outcome === "aligned" ? "success" : markoutSignals[0]?.windows?.[2]?.outcome === "adverse" ? "danger" : markoutSignals[0]?.windows?.[2]?.outcome === "neutral" ? "warning" : "muted")}
                ${renderSignalChip(`1h ${escapeHtml(markoutSignals[0]?.overallOutcome || "not_enough_data")}`, markoutSignals[0]?.overallOutcome === "aligned" ? "success" : markoutSignals[0]?.overallOutcome === "adverse" ? "danger" : markoutSignals[0]?.overallOutcome === "neutral" ? "warning" : "muted")}
              </div>
              <div class="muted">${(markoutSignals.length ? markoutSignals.slice(0, 3) : []).map((signal) => `${escapeHtml(signal.signalKind || "Unavailable")}: ${escapeHtml(signal.overallOutcome || "not_enough_data")}`).join("<br/>") || "not_enough_data"}</div>
            `
            : `<div class="muted">Markout not_enough_data</div>`
        }
      </details>
      <details class="signal-details" open>
        <summary>Quality Overlay</summary>
        ${
          quality
            ? `
              ${renderMetrics([
                { label: "Total Evaluations", value: formatInteger(quality.totalEvaluations) },
                { label: "Aligned Ratio", value: formatNumber((quality.alignedRatio || 0) * 100, 2) + "%" },
                { label: "Adverse Ratio", value: formatNumber((quality.adverseRatio || 0) * 100, 2) + "%" },
                { label: "Neutral Ratio", value: formatNumber((quality.neutralRatio || 0) * 100, 2) + "%" },
                { label: "Not Enough Data", value: formatNumber((quality.notEnoughDataRatio || 0) * 100, 2) + "%" },
              ])}
              <div class="muted">${
                (quality.bySignalType || []).length
                  ? quality.bySignalType
                      .slice(0, 3)
                      .map((item) => `${escapeHtml(item.label || item.key || "Unavailable")}: aligned ${formatNumber((item.alignedRatio || 0) * 100, 1)}%, adverse ${formatNumber((item.adverseRatio || 0) * 100, 1)}%, neutral ${formatNumber((item.neutralRatio || 0) * 100, 1)}%`)
                      .join("<br/>")
                  : "not_enough_data"
              }</div>
            `
            : `<div class="muted">not_enough_data</div>`
        }
      </details>
      <details class="signal-details" open>
        <summary>Recommendation Overlay</summary>
        ${
          recommendation
            ? `
              ${renderMetrics([
                { label: "Total Recommendations", value: formatInteger(recommendation.totalRecommendations) },
                { label: "Keep", value: formatInteger(recommendation.keepCount) },
                { label: "Upgrade", value: formatInteger(recommendation.slightUpgradeCandidateCount) },
                { label: "Downgrade", value: formatInteger((recommendation.slightDowngradeCandidateCount || 0) + (recommendation.downgradeCandidateCount || 0)) },
                { label: "No-trade Only", value: formatInteger(recommendation.noTradeOnlyCandidateCount) },
                { label: "Disable", value: formatInteger(recommendation.disableCandidateCount) },
              ])}
              <div class="signal-chip-row">
                ${renderSignalChip(normalizeReplayRecommendation(recommendation.recommendation || recommendation.recommendations?.[0]?.recommendation || "keep"), "neutral")}
                ${renderSignalChip(`review ${formatBool(Boolean(review?.manualReviewRequired || recommendation.manualReviewRequired))}`, "warning")}
                ${renderSignalChip(`ledger ${ledger?.status || "Unavailable"}`, "muted")}
              </div>
              <div class="muted">${
                (qualityCandidates.length
                  ? qualityCandidates
                      .slice(0, 3)
                      .map((item) => `${escapeHtml(item.signalType || item.symbol || "Unavailable")}: ${escapeHtml(item.recommendation || item.recommendedAction || "Unavailable")}`)
                  : []
                ).join("<br/>") || "not_enough_data"
              }</div>
            `
            : `<div class="muted">not_enough_data</div>`
        }
      </details>
      <details class="signal-details" open>
        <summary>Governance / Evidence Summary</summary>
        ${
          ledger
            ? `
              ${renderMetrics([
                { label: "Selected Symbol", value: escapeHtml(ledger.selectedSymbol || symbol || "Unavailable") },
                { label: "Status", value: escapeHtml(ledger.status || "Unavailable") },
                { label: "Entry Count", value: formatInteger(ledger.entryCount || ledger.entries?.length) },
                { label: "Read Only", value: formatBool(Boolean(ledger.readOnly)) },
                { label: "Analysis Only", value: formatBool(Boolean(ledger.analysisOnly)) },
              ])}
              <div class="muted">${
                (ledger.entries || ledger.recentEntries || []).length
                  ? (ledger.entries || ledger.recentEntries || [])
                      .slice(0, 3)
                      .map((entry) => escapeHtml(entry.summary || entry.reason || entry.action || "Unavailable"))
                      .join("<br/>")
                  : "Governance ledger unavailable"
              }</div>
            `
            : `<div class="muted">Governance ledger unavailable</div>`
        }
      </details>
    </div>`;
}

function renderSuspiciousReplayDrilldown() {
  const content = $("suspiciousReplayDrilldownContent");
  if (!content) {
    return;
  }

  if (state.suspiciousReplayError) {
    setBadge("suspiciousReplayDrilldownBadge", "API Error", "error");
    content.innerHTML = `<div class="error">${escapeHtml(state.suspiciousReplayError)}</div>`;
    return;
  }

  const symbol = suspiciousReplayOverlaySymbol();
  const selectedSignalId = (state.suspiciousReplaySignalId || "").trim();
  const overlayUrls = {
    whaleFlow: whaleFlowSymbolUrl(symbol),
    markout: toxicMarkoutSymbolUrl(symbol),
    quality: toxicQualityScorecardSymbolUrl(symbol),
    recommendation: toxicWeightRecommendationSymbolUrl(symbol),
    review: toxicWeightReviewSymbolUrl(symbol),
    ledger: toxicGovernanceLedgerSymbolUrl(symbol),
  };
  const whaleFlowOverlay = buildWhaleReplayOverlay(symbol, selectedSignalId);
  const overlayPayload = {
    whaleFlowOverlay,
    markout: overlayUrls.markout ? getData(overlayUrls.markout) : null,
    qualityScorecard: overlayUrls.quality ? getData(overlayUrls.quality) : null,
    weightRecommendation: overlayUrls.recommendation ? getData(overlayUrls.recommendation) : null,
    weightReview: overlayUrls.review ? getData(overlayUrls.review) : null,
    governanceLedger: overlayUrls.ledger ? getData(overlayUrls.ledger) : null,
  };
  const hasOverlay =
    overlayPayload.whaleFlowOverlay ||
    overlayPayload.markout ||
    overlayPayload.qualityScorecard ||
    overlayPayload.weightRecommendation ||
    overlayPayload.weightReview ||
    overlayPayload.governanceLedger;
  const selectedSignalSymbol = suspiciousReplaySelectedSignalSymbol();
  const detailPayload = state.suspiciousReplayDetailPayload;
  const detailAvailable = detailPayload?.available !== false && Boolean(detailPayload?.representative);
  const representative = detailPayload?.representative || null;
  const explainPayload = state.suspiciousReplayExplainPayload;
  const lookupPayload = state.suspiciousReplayLookupPayload;
  const groupedBurstId =
    representative?.source?.groupId ||
    detailPayload?.groupId ||
    lookupPayload?.signal?.groupId ||
    null;
  const governanceLedgerAvailable =
    lookupPayload?.signal?.governance?.ledgerAvailable ??
    representative?.governance?.ledgerAvailable ??
    null;

  if (!symbol && !selectedSignalId && !hasOverlay && !detailPayload && !explainPayload) {
    setBadge("suspiciousReplayDrilldownBadge", "Waiting", "gray");
    content.innerHTML = `<div class="replay-empty">Load Replay by Symbol or Signal ID to inspect markout / quality / recommendation overlay.</div>`;
    return;
  }

  setBadge(
    "suspiciousReplayDrilldownBadge",
    selectedSignalId ? "DRILLDOWN" : symbol ? "OVERLAY" : "VIEW_ONLY",
    selectedSignalId ? "warning" : symbol ? "ok" : "gray"
  );

  content.innerHTML = `
    <div class="replay-panel replay-overlay">
      <div class="muted">只读 Drilldown 面板，仅聚合历史快照、markout、quality、recommendation、governance 和 explainability；不触发交易、write、apply、reload、notification、wallet、signing。</div>
      <div class="action-row">
        <button type="button" class="small-button" id="refreshWhaleReplayOverlayButton">Refresh Whale Overlay</button>
        <button type="button" class="small-button" id="loadWhaleReplayOverlayBySymbolButton">Load Whale Overlay by Symbol</button>
        <button type="button" class="small-button" id="loadWhaleReplayOverlayBySignalIdButton">Load Whale Overlay by Signal ID</button>
        <button type="button" class="small-button" id="copyWhaleReplayOverlayJsonButton">Copy Whale Overlay JSON</button>
        <button type="button" class="small-button" id="copyWhaleReplayOverlayMarkdownButton">Copy Whale Overlay Markdown</button>
      </div>
      ${renderReplayOverlayMetricRows(overlayPayload, symbol)}
      <div class="metric">
        <div class="metric-label">Whale Flow / Markout / Quality / Recommendation Overlay</div>
        <div class="metric-value">${
          hasOverlay
            ? renderSuspiciousReplayOverlaySummary(overlayPayload, symbol)
            : "Markout not_enough_data"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Replay Drilldown</div>
        <div class="metric-value">${
          selectedSignalId
            ? `
              ${lookupPayload?.found
                ? renderMetrics([
                    { label: "signalId", value: escapeHtml(lookupPayload.signal?.signalId || selectedSignalId) },
                    { label: "symbol", value: escapeHtml(lookupPayload.signal?.symbol || selectedSignalSymbol || "Unavailable") },
                    { label: "signalKind", value: escapeHtml(lookupPayload.signal?.signalKind || "Unavailable") },
                    { label: "directionBias", value: escapeHtml(lookupPayload.signal?.directionBias || "neutral") },
                    { label: "severity", value: escapeHtml(lookupPayload.signal?.severity || "Unavailable") },
                    { label: "confidence", value: typeof lookupPayload.signal?.confidence === "number" ? `${formatNumber(lookupPayload.signal.confidence * 100, 1)}%` : "Unavailable" },
                    { label: "operatorAction", value: escapeHtml(lookupPayload.signal?.operatorAction || "watch_signal_only") },
                    { label: "markout summary", value: escapeHtml(lookupPayload.signal?.markoutOneMinute || "Markout not_enough_data") },
                    { label: "quality bucket", value: escapeHtml(lookupPayload.signal?.qualityBucket || "not_enough_data") },
                    { label: "recommendation action", value: escapeHtml(lookupPayload.signal?.recommendationAction || "insufficient_data") },
                    { label: "alert decision", value: escapeHtml(explainPayload?.alertDecision || "Alert explanation unavailable") },
                    { label: "grouped burst", value: escapeHtml(groupedBurstId || "grouped burst unavailable") },
                  ])
                : `<div class="muted">Signal not found</div>`}
              <div class="signal-details-grid">
                <details class="signal-details" open>
                  <summary>Signal Detail Timeline</summary>
                  ${
                    detailAvailable
                      ? `
                        ${renderMetrics([
                          { label: "Detail", value: "available" },
                          { label: "Fusion summary", value: escapeHtml(representative?.summary || representative?.noExecutionReason || "Detail unavailable") },
                          { label: "Replay evidence summary", value: escapeHtml((representative?.source?.stageSummary || detailPayload?.reason || "Detail unavailable")) },
                          { label: "Markout 1m", value: escapeHtml(lookupPayload?.signal?.markoutOneMinute || "not_enough_data") },
                          { label: "Markout 5m", value: escapeHtml(lookupPayload?.signal?.markoutFiveMinute || "not_enough_data") },
                          { label: "Markout 15m", value: escapeHtml(lookupPayload?.signal?.markoutFifteenMinute || "not_enough_data") },
                          { label: "Markout 1h", value: escapeHtml(lookupPayload?.signal?.markoutOneHour || "not_enough_data") },
                          { label: "Quality", value: escapeHtml(lookupPayload?.signal?.qualityBucket || "not_enough_data") },
                          { label: "Recommendation", value: escapeHtml(lookupPayload?.signal?.recommendationAction || "insufficient_data") },
                          { label: "Governance", value: governanceLedgerAvailable === true ? "ledgerAvailable=true" : "Governance ledger unavailable" },
                        ])}
                        <div class="muted">${(detailPayload?.timelineStages || []).length ? detailPayload.timelineStages.map((stage) => `${escapeHtml(stage.label || stage.stage || "stage")}: ${escapeHtml(stage.summary || "available")}`).join("<br/>") : "Detail unavailable"}</div>
                      `
                      : `<div class="muted">Detail unavailable</div>`
                  }
                </details>
                <details class="signal-details" open>
                  <summary>Alert Explainability</summary>
                  ${
                    explainPayload?.found
                      ? `
                        ${renderMetrics([
                          { label: "alertDecision", value: escapeHtml(explainPayload.alertDecision || "Unavailable") },
                          { label: "notificationSent", value: formatBool(Boolean(explainPayload.notificationSent)) },
                          { label: "executionTriggered", value: formatBool(Boolean(explainPayload.executionTriggered)) },
                        ])}
                        <div class="muted">Decision Reasons</div>
                        ${renderReasons(explainPayload.decisionReasons || [])}
                        <div class="muted">Suppression Reasons</div>
                        ${renderReasons(explainPayload.suppressionReasons || [])}
                        <div class="muted">Missing Inputs</div>
                        ${renderReasons(explainPayload.missingInputs || [])}
                      `
                      : `<div class="muted">Alert explanation unavailable</div>`
                  }
                </details>
              </div>
            `
            : `<div class="muted">Load Replay by Signal ID to inspect one retained signal. Signal not found / Detail unavailable / Alert explanation unavailable / Governance ledger unavailable / Markout not_enough_data will be shown explicitly when data is missing.</div>`
        }</div>
      </div>
      ${state.latestSuspiciousReplayAction
        ? `<div class="muted">${escapeHtml(state.latestSuspiciousReplayAction)}</div>`
        : ""}
    </div>`;
}

const replayHeatmapWindows = [
  { key: "markoutOneMinute", label: "+1m" },
  { key: "markoutFiveMinute", label: "+5m" },
  { key: "markoutFifteenMinute", label: "+15m" },
  { key: "markoutOneHour", label: "+1h" },
];

function replayHeatmapHistoryUrl() {
  const symbol = (state.replayHeatmapSymbolFilter || "").trim().toUpperCase();
  if (!symbol) {
    return "/api/toxicity/signal-history/recent";
  }
  return toxicSignalHistorySymbolEndpointTemplate.replace(":symbol", encodeURIComponent(symbol));
}

function replayHeatmapRollingUrl() {
  const symbol = (state.replayHeatmapSymbolFilter || "").trim().toUpperCase();
  const params = new URLSearchParams({ window: "7d" });
  if (symbol) {
    params.set("symbol", symbol);
  }
  return `/api/toxicity/signal-report/rolling?${params.toString()}`;
}

function replayHeatmapSourceItems() {
  return (
    state.replayHeatmapHistoryPayload?.items ||
    state.suspiciousReplayHistoryPayload?.items ||
    getData("/api/toxicity/signal-history/recent")?.items ||
    []
  );
}

function replayHeatmapWhaleOverlay(item) {
  const symbol = String(item?.symbol || "").toUpperCase();
  const whalePayload = getWhaleFlowPayloadForSymbol(symbol);
  const candidate = findWhaleFlowCandidateForSignal(whalePayload, item?.signalId || item?.id || "");
  if (!whalePayload) {
    return {
      whaleClassification: "no_candidate",
      baselineSource: "insufficient_history",
      dataQuality: "no_data",
    };
  }
  return {
    whaleClassification: candidate?.candidateType || "no_candidate",
    baselineSource: whalePayload.baselineQuality?.baselineSource || "insufficient_history",
    dataQuality: candidate?.diagnostics?.dataQuality || whalePayload.dataQuality?.status || "no_data",
  };
}

function replayHeatmapNormalizedSymbolFilter() {
  return (state.replayHeatmapSymbolFilter || "").trim().toUpperCase();
}

function replayHeatmapNormalizedSignalKindFilter() {
  return (state.replayHeatmapSignalKindFilter || "").trim().toLowerCase();
}

function replayHeatmapNormalizedDirectionFilter() {
  return (state.replayHeatmapDirectionFilter || "").trim().toLowerCase();
}

function normalizeReplayHeatmapStatus(value) {
  switch (String(value || "").trim().toLowerCase()) {
    case "aligned":
      return "aligned";
    case "adverse":
      return "adverse";
    case "neutral":
      return "neutral";
    default:
      return "not_enough_data";
  }
}

function replayHeatmapStatusTone(status) {
  switch (normalizeReplayHeatmapStatus(status)) {
    case "aligned":
      return "success";
    case "adverse":
      return "danger";
    case "neutral":
      return "warning";
    default:
      return "muted";
  }
}

function replayHeatmapEmptyCounts() {
  return {
    aligned: 0,
    adverse: 0,
    neutral: 0,
    notEnoughData: 0,
  };
}

function replayHeatmapCountsSnapshot(counts) {
  return {
    aligned: counts.aligned ?? counts.alignedCount ?? 0,
    adverse: counts.adverse ?? counts.adverseCount ?? 0,
    neutral: counts.neutral ?? counts.neutralCount ?? 0,
    notEnoughData: counts.notEnoughData ?? counts.notEnoughDataCount ?? 0,
  };
}

function incrementReplayHeatmapCount(counts, status) {
  const normalized = normalizeReplayHeatmapStatus(status);
  const alignedKey = Object.prototype.hasOwnProperty.call(counts, "alignedCount")
    ? "alignedCount"
    : "aligned";
  const adverseKey = Object.prototype.hasOwnProperty.call(counts, "adverseCount")
    ? "adverseCount"
    : "adverse";
  const neutralKey = Object.prototype.hasOwnProperty.call(counts, "neutralCount")
    ? "neutralCount"
    : "neutral";
  const notEnoughDataKey = Object.prototype.hasOwnProperty.call(counts, "notEnoughDataCount")
    ? "notEnoughDataCount"
    : "notEnoughData";
  if (normalized === "aligned") {
    counts[alignedKey] += 1;
    return;
  }
  if (normalized === "adverse") {
    counts[adverseKey] += 1;
    return;
  }
  if (normalized === "neutral") {
    counts[neutralKey] += 1;
    return;
  }
  counts[notEnoughDataKey] += 1;
}

function replayHeatmapDominantStatus(counts) {
  const normalizedCounts = replayHeatmapCountsSnapshot(counts);
  const candidates = [
    { key: "aligned", count: normalizedCounts.aligned },
    { key: "adverse", count: normalizedCounts.adverse },
    { key: "neutral", count: normalizedCounts.neutral },
    { key: "not_enough_data", count: normalizedCounts.notEnoughData },
  ];
  candidates.sort((a, b) => b.count - a.count || a.key.localeCompare(b.key));
  return candidates[0]?.count > 0 ? candidates[0].key : "not_enough_data";
}

function replayHeatmapItemDominantStatus(item) {
  const counts = replayHeatmapEmptyCounts();
  replayHeatmapWindows.forEach((window) => {
    incrementReplayHeatmapCount(counts, item?.[window.key]);
  });
  return replayHeatmapDominantStatus(counts);
}

function replayHeatmapFilteredItems() {
  const symbolFilter = replayHeatmapNormalizedSymbolFilter();
  const signalKindFilter = replayHeatmapNormalizedSignalKindFilter();
  const directionFilter = replayHeatmapNormalizedDirectionFilter();

  return replayHeatmapSourceItems().filter((item) => {
    const symbol = String(item.symbol || "").toUpperCase();
    const signalKind = String(item.signalKind || "").toLowerCase();
    const directionBias = String(item.directionBias || "neutral").toLowerCase();
    if (symbolFilter && symbol !== symbolFilter) {
      return false;
    }
    if (signalKindFilter && !signalKind.includes(signalKindFilter)) {
      return false;
    }
    if (directionFilter && directionFilter !== "all" && directionBias !== directionFilter) {
      return false;
    }
    return true;
  });
}

function buildReplayHeatmapMarkdown(payload) {
  const lines = [
    "# Replay Markout Heatmap",
    "",
    "## Safety",
    "- Read-only",
    "- Analysis only",
    "- Execution disabled",
    "- No order placement",
    "- No wallet/signing",
    "- No live trading",
    "",
    "## Filter",
    `- Symbol: ${payload.filter.symbol || "ALL"}`,
    `- Signal kind: ${payload.filter.signalKind || "ALL"}`,
    `- Direction bias: ${payload.filter.directionBias || "ALL"}`,
    `- Whale classification: ${payload.filter.whaleClassification || "ALL"}`,
    `- Baseline source: ${payload.filter.baselineSource || "ALL"}`,
    `- Data quality: ${payload.filter.dataQuality || "ALL"}`,
    "",
    "## Summary",
    `- Groups: ${payload.summary.groups}`,
    `- Signals included: ${payload.summary.signalsIncluded}`,
    `- Aligned: ${payload.summary.aligned}`,
    `- Adverse: ${payload.summary.adverse}`,
    `- Neutral: ${payload.summary.neutral}`,
    `- Not enough data: ${payload.summary.notEnoughData}`,
    "",
    "## Heatmap by Group",
  ];

  if (!payload.groups.length) {
    lines.push("- No signals matched filter");
  }

  payload.groups.forEach((group) => {
    lines.push("");
    lines.push(`### ${group.symbol} / ${group.signalKind} / ${group.directionBias}`);
    lines.push(`- whaleClassification: ${group.whaleClassification}`);
    lines.push(`- baselineSource: ${group.baselineSource}`);
    lines.push(`- dataQuality: ${group.dataQuality}`);
    lines.push(`- sampleCount: ${group.sampleCount}`);
    lines.push(`- avgConfidence: ${group.avgConfidence == null ? "Unavailable" : formatNumber(group.avgConfidence * 100, 1) + "%"}`);
    lines.push(`- maxSeverity: ${group.maxSeverity}`);
    lines.push(`- dominantMarkout: ${group.dominantMarkout}`);
    lines.push(`- alignedCount: ${group.alignedCount}`);
    lines.push(`- adverseCount: ${group.adverseCount}`);
    lines.push(`- neutralCount: ${group.neutralCount}`);
    lines.push(`- notEnoughDataCount: ${group.notEnoughDataCount}`);
    lines.push("");
    lines.push("| Window | aligned | adverse | neutral | not_enough_data | dominantStatus | sampleCount |");
    lines.push("| --- | --- | --- | --- | --- | --- | --- |");
    group.windows.forEach((window) => {
      lines.push(
        `| ${window.window} | ${window.aligned} | ${window.adverse} | ${window.neutral} | ${window.notEnoughData} | ${window.dominantStatus} | ${window.sampleCount} |`
      );
    });
  });

  lines.push("");
  lines.push("## Operator Notes");
  lines.push("- Direction bias is a signal attribute, not an order instruction.");
  lines.push("- This heatmap is for review only.");
  (payload.operatorNotes || []).forEach((note) => lines.push(`- ${note}`));
  return lines.join("\n");
}

function buildReplayHeatmapPayload() {
  const rawItems = replayHeatmapSourceItems();
  const filteredItems = replayHeatmapFilteredItems();
  const groups = new Map();

  filteredItems.forEach((item) => {
    const symbol = String(item.symbol || "UNKNOWN").toUpperCase();
    const signalKind = String(item.signalKind || "unknown_signal");
    const directionBias = String(item.directionBias || "neutral").toLowerCase();
    const whale = replayHeatmapWhaleOverlay(item);
    const key = `${symbol}__${signalKind}__${directionBias}__${whale.whaleClassification}__${whale.baselineSource}__${whale.dataQuality}`;
    if (!groups.has(key)) {
      groups.set(key, {
        symbol,
        signalKind,
        directionBias,
        whaleClassification: whale.whaleClassification,
        baselineSource: whale.baselineSource,
        dataQuality: whale.dataQuality,
        sampleCount: 0,
        avgConfidence: 0,
        confidenceSum: 0,
        confidenceCount: 0,
        maxSeverity: "low",
        dominantMarkout: "not_enough_data",
        alignedCount: 0,
        adverseCount: 0,
        neutralCount: 0,
        notEnoughDataCount: 0,
        memberSignalIds: [],
        windows: replayHeatmapWindows.map((window) => ({
          window: window.label,
          aligned: 0,
          adverse: 0,
          neutral: 0,
          notEnoughData: 0,
          dominantStatus: "not_enough_data",
          sampleCount: 0,
        })),
      });
    }

    const group = groups.get(key);
    group.sampleCount += 1;
    if (typeof item.confidence === "number") {
      group.confidenceSum += item.confidence;
      group.confidenceCount += 1;
    }
    if (severityRank(item.severity) > severityRank(group.maxSeverity)) {
      group.maxSeverity = item.severity || group.maxSeverity;
    }

    const dominantStatus = replayHeatmapItemDominantStatus(item);
    incrementReplayHeatmapCount(group, dominantStatus);

    replayHeatmapWindows.forEach((window, index) => {
      const status = normalizeReplayHeatmapStatus(item?.[window.key]);
      const bucket = group.windows[index];
      incrementReplayHeatmapCount(bucket, status);
      bucket.sampleCount += 1;
    });

    const signalId = item.signalId || item.id;
    if (signalId) {
      group.memberSignalIds.push(signalId);
    }
  });

  const groupList = [...groups.values()]
    .map((group) => {
      group.avgConfidence =
        group.confidenceCount > 0 ? group.confidenceSum / group.confidenceCount : null;
      delete group.confidenceSum;
      delete group.confidenceCount;
      group.windows = group.windows.map((window) => ({
        ...window,
        dominantStatus: replayHeatmapDominantStatus(window),
      }));
      group.dominantMarkout = replayHeatmapDominantStatus(group);
      return group;
    })
    .sort(
      (a, b) =>
        severityRank(b.maxSeverity) - severityRank(a.maxSeverity) ||
        (b.sampleCount || 0) - (a.sampleCount || 0) ||
        String(a.symbol || "").localeCompare(String(b.symbol || ""))
    );

  const summary = groupList.reduce(
    (acc, group) => {
      acc.groups += 1;
      acc.signalsIncluded += group.sampleCount;
      acc.aligned += group.alignedCount;
      acc.adverse += group.adverseCount;
      acc.neutral += group.neutralCount;
      acc.notEnoughData += group.notEnoughDataCount;
      return acc;
    },
    { groups: 0, signalsIncluded: 0, aligned: 0, adverse: 0, neutral: 0, notEnoughData: 0 }
  );

  const rollingPayload = state.replayHeatmapRollingPayload;
  const operatorNotes = [
    "Direction bias is a signal attribute, not an order instruction.",
    "This heatmap is for review only.",
    ...((rollingPayload?.operatorNotes || []).slice(0, 3)),
  ];

  const payload = {
    readOnly: true,
    analysisOnly: true,
    executionEnabled: false,
    runtimeModified: false,
    viewOnly: true,
    filter: {
      symbol: replayHeatmapNormalizedSymbolFilter() || null,
      signalKind: state.replayHeatmapSignalKindFilter || null,
      directionBias: replayHeatmapNormalizedDirectionFilter() || null,
      whaleClassification: "ALL",
      baselineSource: "ALL",
      dataQuality: "ALL",
      persistentWatchlistEnabled: false,
      runtimeMonitorModified: false,
    },
    summary,
    groups: groupList,
    operatorNotes,
    source: {
      historyItems: rawItems.length,
      filteredItems: filteredItems.length,
      rollingAvailable: Boolean(rollingPayload),
    },
  };
  payload.markdown = buildReplayHeatmapMarkdown(payload);
  return payload;
}

function syncReplayHeatmapFiltersFromControls() {
  state.replayHeatmapSymbolFilter = $("replayHeatmapSymbolInput")?.value?.trim() || "";
  state.replayHeatmapSignalKindFilter = $("replayHeatmapSignalKindInput")?.value?.trim() || "";
  state.replayHeatmapDirectionFilter = $("replayHeatmapDirectionSelect")?.value || "";
}

function renderReplayHeatmapWindow(window) {
  return `
    <div class="heatmap-window">
      <div class="replay-overlay-title">${escapeHtml(window.window)}</div>
      ${renderSignalChipRow([
        renderSignalChip(`dominant ${window.dominantStatus}`, replayHeatmapStatusTone(window.dominantStatus)),
        renderSignalChip(`samples ${formatInteger(window.sampleCount)}`, "neutral"),
      ])}
      ${renderMetrics([
        { label: "aligned", value: formatInteger(window.aligned) },
        { label: "adverse", value: formatInteger(window.adverse) },
        { label: "neutral", value: formatInteger(window.neutral) },
        { label: "not_enough_data", value: formatInteger(window.notEnoughData) },
      ])}
    </div>`;
}

function renderReplayHeatmapGroup(group) {
  return `
    <div class="heatmap-group">
      <div class="suspicious-order-header">
        <div class="suspicious-order-title">
          <div class="suspicious-order-symbol">${escapeHtml(group.symbol)}</div>
          <div class="suspicious-order-meta">${escapeHtml(group.signalKind)} · directionBias=${escapeHtml(group.directionBias)}</div>
          <div class="suspicious-order-meta">whaleClassification=${escapeHtml(group.whaleClassification)} · baselineSource=${escapeHtml(group.baselineSource)} · dataQuality=${escapeHtml(group.dataQuality)}</div>
        </div>
        <span class="badge ${badgeClass(group.maxSeverity || "none")}">${escapeHtml(group.maxSeverity || "low")}</span>
      </div>
      ${renderSignalChipRow([
        renderSignalChip(`sampleCount ${formatInteger(group.sampleCount)}`, "neutral"),
        renderSignalChip(`avgConfidence ${group.avgConfidence == null ? "Unavailable" : formatNumber(group.avgConfidence * 100, 1) + "%"}`, "warning"),
        renderSignalChip(`dominant ${group.dominantMarkout}`, replayHeatmapStatusTone(group.dominantMarkout)),
        renderSignalChip(`whale ${group.whaleClassification}`, "warning"),
        renderSignalChip(`baseline ${group.baselineSource}`, "neutral"),
        renderSignalChip(`dataQuality ${group.dataQuality}`, "neutral"),
      ])}
      ${renderMetrics([
        { label: "alignedCount", value: formatInteger(group.alignedCount) },
        { label: "adverseCount", value: formatInteger(group.adverseCount) },
        { label: "neutralCount", value: formatInteger(group.neutralCount) },
        { label: "notEnoughDataCount", value: formatInteger(group.notEnoughDataCount) },
      ])}
      <div class="heatmap-window-grid">
        ${group.windows.map((window) => renderReplayHeatmapWindow(window)).join("")}
      </div>
      <div class="muted">memberSignalIds: ${group.memberSignalIds.length ? group.memberSignalIds.map((item) => escapeHtml(item)).join(", ") : "Unavailable"}</div>
    </div>`;
}

function renderReplayHeatmap() {
  const content = $("replayHeatmapContent");
  if (!content) {
    return;
  }

  if (state.replayHeatmapError) {
    setBadge("replayHeatmapBadge", "API Error", "error");
    content.innerHTML = `<div class="error">${escapeHtml(state.replayHeatmapError)}</div>`;
    return;
  }

  const payload = state.replayHeatmapBuiltPayload;
  const rawItems = replayHeatmapSourceItems();
  if (!payload) {
    setBadge("replayHeatmapBadge", rawItems.length ? "READY" : "Loading", rawItems.length ? "warning" : "none");
    content.innerHTML = `
      <div class="heatmap-controls">
        <div class="control-grid">
          <div>
            <div class="metric-label">Symbol filter</div>
            <input id="replayHeatmapSymbolInput" placeholder="symbol" value="${escapeHtml(state.replayHeatmapSymbolFilter || "")}" />
          </div>
          <div>
            <div class="metric-label">SignalKind filter</div>
            <input id="replayHeatmapSignalKindInput" placeholder="signalKind" value="${escapeHtml(state.replayHeatmapSignalKindFilter || "")}" />
          </div>
          <div>
            <div class="metric-label">Direction filter</div>
            <select id="replayHeatmapDirectionSelect" class="signal-sort-select">
              <option value="">All</option>
              <option value="long"${state.replayHeatmapDirectionFilter === "long" ? " selected" : ""}>long</option>
              <option value="short"${state.replayHeatmapDirectionFilter === "short" ? " selected" : ""}>short</option>
              <option value="neutral"${state.replayHeatmapDirectionFilter === "neutral" ? " selected" : ""}>neutral</option>
            </select>
          </div>
        </div>
        <div class="action-row">
          <button type="button" class="small-button" id="refreshReplayHeatmapButton">Refresh Heatmap</button>
          <button type="button" class="small-button" id="buildReplayHeatmapButton">Build Heatmap</button>
          <button type="button" class="small-button" id="clearReplayHeatmapFilterButton">Clear Heatmap Filter</button>
          <button type="button" class="small-button" id="copyReplayHeatmapJsonButton">Copy Heatmap JSON</button>
          <button type="button" class="small-button" id="copyReplayHeatmapMarkdownButton">Copy Heatmap Markdown</button>
        </div>
      </div>
      <div class="heatmap-empty">${rawItems.length ? "Build Heatmap" : "No history available"}</div>`;
    return;
  }

  const noRawHistory = !rawItems.length;
  const noFilteredSignals = rawItems.length > 0 && payload.summary.signalsIncluded === 0;
  const insufficientSamples = payload.summary.signalsIncluded > 0 && payload.summary.signalsIncluded < 2;
  setBadge(
    "replayHeatmapBadge",
    payload.summary.groups ? `${payload.summary.groups} GROUPS` : noRawHistory ? "NO HISTORY" : "VIEW_ONLY",
    payload.summary.groups ? "ok" : noRawHistory ? "gray" : "warning"
  );

  content.innerHTML = `
    <div class="replay-panel">
      <div class="muted">Read-only / analysisOnly / executionEnabled=false / view-only / persistentWatchlistEnabled=false / runtimeMonitorModified=false</div>
      <div class="heatmap-controls">
        <div class="control-grid">
          <div>
            <div class="metric-label">Symbol filter</div>
            <input id="replayHeatmapSymbolInput" placeholder="symbol" value="${escapeHtml(state.replayHeatmapSymbolFilter || "")}" />
          </div>
          <div>
            <div class="metric-label">SignalKind filter</div>
            <input id="replayHeatmapSignalKindInput" placeholder="signalKind" value="${escapeHtml(state.replayHeatmapSignalKindFilter || "")}" />
          </div>
          <div>
            <div class="metric-label">Direction filter</div>
            <select id="replayHeatmapDirectionSelect" class="signal-sort-select">
              <option value="">All</option>
              <option value="long"${state.replayHeatmapDirectionFilter === "long" ? " selected" : ""}>long</option>
              <option value="short"${state.replayHeatmapDirectionFilter === "short" ? " selected" : ""}>short</option>
              <option value="neutral"${state.replayHeatmapDirectionFilter === "neutral" ? " selected" : ""}>neutral</option>
            </select>
          </div>
        </div>
        <div class="action-row">
          <button type="button" class="small-button" id="refreshReplayHeatmapButton">Refresh Heatmap</button>
          <button type="button" class="small-button" id="buildReplayHeatmapButton">Build Heatmap</button>
          <button type="button" class="small-button" id="clearReplayHeatmapFilterButton">Clear Heatmap Filter</button>
          <button type="button" class="small-button" id="copyReplayHeatmapJsonButton">Copy Heatmap JSON</button>
          <button type="button" class="small-button" id="copyReplayHeatmapMarkdownButton">Copy Heatmap Markdown</button>
        </div>
      </div>
      ${renderMetrics([
        { label: "Groups", value: formatInteger(payload.summary.groups) },
        { label: "Signals included", value: formatInteger(payload.summary.signalsIncluded) },
        { label: "Aligned", value: formatInteger(payload.summary.aligned) },
        { label: "Adverse", value: formatInteger(payload.summary.adverse) },
        { label: "Neutral", value: formatInteger(payload.summary.neutral) },
        { label: "Not enough data", value: formatInteger(payload.summary.notEnoughData) },
      ])}
      <div class="metric">
        <div class="metric-label">Heatmap by Group</div>
        <div class="metric-value">${
          noRawHistory
            ? "No history available"
            : noFilteredSignals
              ? "No signals matched filter"
              : insufficientSamples
                ? "Insufficient samples for heatmap"
                : `<div class="heatmap-group-list">${payload.groups.map((group) => renderReplayHeatmapGroup(group)).join("")}</div>`
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Window Coverage</div>
        <div class="metric-value">${renderSignalChipRow(
          replayHeatmapWindows.map((window) => renderSignalChip(`${window.label}`, "neutral"))
        )}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Operator Notes</div>
        <div class="metric-value">${payload.operatorNotes.map((note) => escapeHtml(note)).join("<br/>")}</div>
      </div>
      ${state.latestReplayHeatmapAction ? `<div class="muted">${escapeHtml(state.latestReplayHeatmapAction)}</div>` : ""}
    </div>`;
}

function renderSuspiciousReplayHistoryItem(item) {
  const selected = item.signalId && item.signalId === state.suspiciousReplaySignalId;
  const confidence = typeof item.confidence === "number"
    ? `${formatNumber(item.confidence * 100, 1)}%`
    : "Unavailable";
  const tone = severityTone(item.severity);
  return `
    <div class="replay-history-item">
      <div class="replay-history-header">
        <div class="replay-history-title">
          <div class="signal-summary-title">${escapeHtml(item.signalId || "Unavailable")}</div>
          <div class="replay-history-meta">
            ${escapeHtml(item.symbol || "Unavailable")} · ${escapeHtml(item.signalKind || "Unavailable")} · ${escapeHtml(item.directionBias || "neutral")} · ${formatDateTime(item.createdAtMs)}
          </div>
        </div>
        <span class="badge ${badgeClass(item.severity || "none")}">${escapeHtml(item.severity || "watch")}</span>
      </div>
      <div class="signal-chip-row">
        ${renderSignalChip(item.operatorAction || "watch_signal_only", "neutral")}
        ${renderSignalChip(`confidence ${confidence}`, "warning")}
        ${renderSignalChip(`markout 1m ${item.markoutOneMinute || "not_enough_data"}`, item.markoutOneMinute === "aligned" ? "success" : item.markoutOneMinute === "adverse" ? "danger" : item.markoutOneMinute === "neutral" ? "warning" : "muted")}
        ${renderSignalChip(`quality ${item.qualityBucket || "not_enough_data"}`, item.qualityBucket === "good" ? "success" : item.qualityBucket === "not_enough_data" ? "muted" : "warning")}
        ${renderSignalChip(`recommendation ${item.recommendationAction || "insufficient_data"}`, tone)}
      </div>
      <div class="muted">${escapeHtml(item.source || "signal_history")}</div>
      <div class="action-row">
        <button type="button" class="small-button" data-suspicious-replay-signal-id="${escapeHtml(item.signalId || "")}" data-suspicious-replay-symbol="${escapeHtml(item.symbol || "")}">${selected ? "Replay Loaded" : "Load Replay by Signal ID"}</button>
      </div>
    </div>`;
}

function renderSuspiciousReplay() {
  const content = $("suspiciousReplayContent");
  if (!content) {
    return;
  }

  if (state.suspiciousReplayError) {
    setBadge("suspiciousReplayBadge", "API Error", "error");
    content.innerHTML = `<div class="error">${escapeHtml(state.suspiciousReplayError)}</div>`;
    return;
  }

  const statusPayload = getSuspiciousReplayStatusPayload();
  const historyPayload = getSuspiciousReplayHistoryPayload();
  const items = getSuspiciousReplayItems();
  const selectedSignalId = (state.suspiciousReplaySignalId || "").trim();

  if (!statusPayload && !historyPayload) {
    setBadge("suspiciousReplayBadge", "Loading", "none");
    content.innerHTML = `<div class="replay-empty">No history available</div>`;
    return;
  }

  setBadge(
    "suspiciousReplayBadge",
    selectedSignalId ? "REPLAY READY" : items.length ? `${items.length} HISTORY` : "NO HISTORY",
    selectedSignalId ? "warning" : items.length ? "ok" : "gray"
  );

  content.innerHTML =
    `<div class="replay-panel">
      <div class="muted">只读回放面板，只消费历史快照与 explainability 结果；不改 watchlist，不改 runtime monitor scope，不写 DB / JSONL / SQLite / archive。</div>
      ${renderMetrics([
        { label: "readOnly", value: formatBool(Boolean(statusPayload?.readOnly ?? true)) },
        { label: "analysisOnly", value: formatBool(Boolean(statusPayload?.analysisOnly ?? true)) },
        { label: "executionEnabled", value: formatBool(Boolean(statusPayload?.executionEnabled ?? false)) },
        { label: "Filter mode", value: "view-only" },
        { label: "persistentWatchlistEnabled", value: "false" },
        { label: "runtimeMonitorModified", value: "false" },
        { label: "Selected Symbol", value: escapeHtml(suspiciousReplaySelectedSymbol() || "ALL") },
        { label: "Selected Signal ID", value: escapeHtml(selectedSignalId || "none") },
        { label: "Retention Mode", value: escapeHtml(statusPayload?.retentionMode || "in_memory_bounded") },
        { label: "Current Signals", value: formatInteger(statusPayload?.currentSignals || items.length) },
        { label: "Current Groups", value: formatInteger(statusPayload?.currentGroups || 0) },
        { label: "Current Alerts", value: formatInteger(statusPayload?.currentAlerts || 0) },
      ])}
      <div class="action-row">
        <input id="suspiciousReplaySymbolInput" placeholder="symbol" value="${escapeHtml(state.suspiciousReplaySymbol || "")}" />
        <button type="button" class="small-button" id="loadSuspiciousReplayBySymbolButton">Load Replay by Symbol</button>
        <input id="suspiciousReplaySignalIdInput" placeholder="signal_id" value="${escapeHtml(selectedSignalId)}" />
        <button type="button" class="small-button" id="loadSuspiciousReplayBySignalIdButton">Load Replay by Signal ID</button>
        <button type="button" class="small-button" id="clearSuspiciousReplayFilterButton">Clear Replay Filter</button>
        <button type="button" class="small-button" id="refreshSuspiciousReplayButton">Refresh Replay</button>
        <button type="button" class="small-button" id="copySuspiciousReplayJsonButton">Copy Replay JSON</button>
      </div>
      <div class="muted">Filter mode: view-only</div>
      <div class="muted">Persistent watchlist: disabled</div>
      <div class="muted">Runtime monitor modified: false</div>
      <div class="metric">
        <div class="metric-label">History List</div>
        <div class="metric-value">${
          items.length
            ? `<div class="replay-history-list">${items.slice(0, 10).map((item) => renderSuspiciousReplayHistoryItem(item)).join("")}</div>`
            : "No history available"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Drilldown Hint</div>
        <div class="metric-value">Use the dedicated Replay Drilldown card below for markout / quality / recommendation overlay and single-signal evidence.</div>
      </div>
      ${state.latestSuspiciousReplayAction
        ? `<div class="muted">${escapeHtml(state.latestSuspiciousReplayAction)}</div>`
        : ""}
    </div>`;
}

function renderSignalSymbolFilter() {
  const content = $("signalSymbolFilterContent");
  if (!content) {
    return;
  }

  const currentFilter = signalSymbolFilterValue();
  setBadge(
    "signalSymbolFilterBadge",
    currentFilter ? "FILTERED" : "VIEW_ONLY",
    currentFilter ? "warning" : "none"
  );

  content.innerHTML =
    renderMetrics([
      { label: "Current Filter", value: escapeHtml(currentFilter || "runtime default") },
      { label: "Filter Mode", value: "view-only" },
      { label: "Persistent watchlist", value: "disabled" },
      { label: "Runtime Monitor Modified", value: "false" },
      { label: "Read Only", value: "true" },
      { label: "Analysis Only", value: "true" },
    ]) +
    `<div class="action-row">
      <input id="signalSymbolFilterInput" placeholder="symbol" value="${escapeHtml(
        currentFilter
      )}" />
      <button type="button" class="small-button" id="filterSignalsButton">Filter Signals</button>
      <button type="button" class="small-button" id="clearSignalFilterButton">Clear Filter</button>
      <button type="button" class="small-button" id="copyFilteredSignalJsonButton">Copy Filtered Signal JSON</button>
    </div>` +
    `<div class="muted">Current filter affects view-only signal inbox, signal groups, and signal detail queries. Persistent watchlist disabled. It does not save a watchlist, reload runtime, or change the monitoring scope.</div>` +
    `<div class="muted">Read-only. Analysis only. No order placement. No wallet/signing. No live trading.</div>` +
    (state.latestSignalSymbolFilterAction
      ? `<div class="muted">${escapeHtml(state.latestSignalSymbolFilterAction)}</div>`
      : "");
}

function renderToxicSignalHealth() {
  const summaryUrl = toxicSignalHealthSummaryUrl();
  const statusUrl = toxicSignalHealthStatusUrl();
  const payload = getToxicSignalHealthPayload();
  const statusPayload = getToxicSignalHealthStatusPayload();
  const error =
    getError(summaryUrl) ||
    getError(statusUrl) ||
    (!state.toxicSignalHealthSymbol
      ? getError("/api/toxicity/signal-health/summary") ||
        getError("/api/toxicity/signal-health/status")
      : null);

  if (error) {
    setBadge("toxicSignalHealthBadge", "API Error", "error");
    $("toxicSignalHealthContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!payload || !statusPayload) {
    setBadge("toxicSignalHealthBadge", "Loading", "none");
    $("toxicSignalHealthContent").innerHTML =
      `<div class="muted">Signal Health / Completeness will appear after inbox, groups, report, alert preview, and history coverage load.</div>`;
    return;
  }

  const summary = payload.summary || {};
  const issues = payload.issues || [];

  setBadge(
    "toxicSignalHealthBadge",
    (payload.healthBucket || statusPayload.healthBucket || "diagnostic_only").toUpperCase(),
    payload.healthBucket === "excellent"
      ? "ok"
      : payload.healthBucket === "good"
        ? "blue"
        : payload.healthBucket === "thin_data"
          ? "warning"
          : payload.healthBucket === "degraded"
            ? "orange"
            : "none"
  );

  $("toxicSignalHealthContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(payload.readOnly)) },
      { label: "Runtime Modified", value: formatBool(Boolean(payload.runtimeModified)) },
      { label: "Analysis Only", value: formatBool(Boolean(payload.analysisOnly)) },
      { label: "Execution Enabled", value: formatBool(Boolean(payload.executionEnabled)) },
      { label: "healthMode", value: escapeHtml(payload.healthMode || "diagnostic_only") },
      { label: "repairEnabled", value: formatBool(Boolean(payload.repairEnabled)) },
      { label: "backfillEnabled", value: formatBool(Boolean(payload.backfillEnabled)) },
      {
        label: "runtimeMutationEnabled",
        value: formatBool(Boolean(payload.runtimeMutationEnabled)),
      },
      { label: "Selected Symbol", value: escapeHtml(payload.selectedSymbol || "ALL") },
      { label: "Health Bucket", value: escapeHtml(payload.healthBucket || "Unavailable") },
      { label: "Total Signals", value: formatInteger(summary.totalSignals) },
      { label: "Not Enough Data", value: formatInteger(summary.notEnoughDataCount) },
    ]) +
    `<div class="action-row">
      <input id="toxicSignalHealthSymbolInput" placeholder="symbol" value="${escapeHtml(
        state.toxicSignalHealthSymbol || toxicSignalHealthSelectedSymbol() || ""
      )}" />
      <button type="button" class="small-button" id="loadToxicSignalHealthBySymbolButton">Load Signal Health by Symbol</button>
      <button type="button" class="small-button" id="refreshToxicSignalHealthButton">Refresh Signal Health</button>
      <button type="button" class="small-button" id="copyToxicSignalHealthJsonButton">Copy Signal Health JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Availability</div>
        <div class="metric-value">${renderMetrics([
          { label: "Inbox", value: formatBool(Boolean(summary.inboxAvailable)) },
          { label: "Groups", value: formatBool(Boolean(summary.groupsAvailable)) },
          { label: "Detail", value: formatBool(Boolean(summary.detailAvailable)) },
          {
            label: "Daily Report",
            value: formatBool(Boolean(summary.dailyReportAvailable)),
          },
          {
            label: "Alert Preview",
            value: formatBool(Boolean(summary.alertPreviewAvailable)),
          },
          { label: "History", value: formatBool(Boolean(summary.historyAvailable)) },
        ])}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Coverage</div>
        <div class="metric-value">${renderMetrics([
          { label: "signalsWithMarkout", value: formatInteger(summary.signalsWithMarkout) },
          {
            label: "signalsMissingMarkout",
            value: formatInteger(summary.signalsMissingMarkout),
          },
          { label: "signalsWithQuality", value: formatInteger(summary.signalsWithQuality) },
          {
            label: "signalsMissingQuality",
            value: formatInteger(summary.signalsMissingQuality),
          },
          {
            label: "signalsWithRecommendation",
            value: formatInteger(summary.signalsWithRecommendation),
          },
          {
            label: "signalsMissingRecommendation",
            value: formatInteger(summary.signalsMissingRecommendation),
          },
          {
            label: "signalsWithGovernance",
            value: formatInteger(summary.signalsWithGovernance),
          },
          {
            label: "signalsMissingGovernance",
            value: formatInteger(summary.signalsMissingGovernance),
          },
        ])}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Issues</div>
        <div class="metric-value">${
          issues.length
            ? renderTable(
                ["Kind", "Severity", "Count", "Operator Note"],
                issues.map((issue) => [
                  escapeHtml(issue.kind || "Unavailable"),
                  escapeHtml(issue.severity || "Unavailable"),
                  formatInteger(issue.count),
                  escapeHtml(issue.operatorNote || "Unavailable"),
                ])
              )
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Operator Notes</div>
        <div class="metric-value">${(payload.operatorNotes || [])
          .map((note) => escapeHtml(note))
          .join("<br/>") || "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          Read-only<br/>
          Analysis only<br/>
          No live trading<br/>
          No order placement<br/>
          repairEnabled=false<br/>
          backfillEnabled=false<br/>
          runtimeMutationEnabled=false
        </div>
      </div>
    </div>` +
    (state.latestToxicSignalHealthAction
      ? `<div class="muted">${escapeHtml(state.latestToxicSignalHealthAction)}</div>`
      : "");
}

function renderToxicSignalInbox() {
  const url = toxicSignalInboxRecentUrl();
  const statusUrl = toxicSignalInboxStatusUrl();
  const report = getToxicSignalInboxPayload();
  const statusPayload = getToxicSignalInboxStatusPayload();
  const error =
    getError(url) ||
    getError(statusUrl) ||
    (!state.toxicSignalInboxSymbol
      ? getError("/api/toxicity/signal-inbox/recent") ||
        getError("/api/toxicity/signal-inbox/status")
      : null);
  if (error) {
    setBadge("toxicSignalInboxBadge", "API Error", "error");
    $("toxicSignalInboxContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicSignalInboxBadge", "Loading", "none");
    $("toxicSignalInboxContent").innerHTML =
      `<div class="muted">Signal inbox will appear after fused toxicity and evidence summaries load.</div>`;
    return;
  }

  const items = report.items || [];
  setBadge(
    "toxicSignalInboxBadge",
    (statusPayload?.mode || report.mode || "analysis_only").toUpperCase(),
    items.length ? "warning" : "none"
  );

  $("toxicSignalInboxContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      { label: "Runtime Modified", value: formatBool(Boolean(report.runtimeModified)) },
      { label: "Analysis Only", value: formatBool(Boolean(report.analysisOnly)) },
      { label: "Execution Enabled", value: formatBool(Boolean(report.executionEnabled)) },
      { label: "Manual Review Required", value: formatBool(Boolean(report.manualReviewRequired)) },
      { label: "Items", value: formatInteger(items.length) },
      { label: "Selected Symbol", value: escapeHtml(report.selectedSymbol || "ALL") },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicSignalInboxButton">Refresh Signal Inbox</button>
      <button type="button" class="small-button" id="copyToxicSignalInboxJsonButton">Copy Signal Inbox JSON</button>
    </div>` +
    `<div class="stack">
      ${
        items.length
          ? items
              .slice(0, 6)
              .map(
                (item) => `
                <div class="metric">
                  <div class="metric-label">${escapeHtml(item.symbol)} · ${escapeHtml(item.signalKind)}</div>
                  <div class="metric-value">${escapeHtml(item.operatorAction || "watch_signal_only")}</div>
                  <div class="muted">
                    Direction: ${escapeHtml(item.directionBias || "neutral")} ·
                    Severity: ${escapeHtml(item.severity || "unknown")} ·
                    Confidence: ${formatNumber((item.confidence || 0) * 100, 1)}%
                  </div>
                  <div class="muted">${escapeHtml(item.fusion?.summary || "No fusion summary")}</div>
                  ${renderMetrics([
                    { label: "Replay Evidence", value: formatInteger(item.replay?.evidenceCount) },
                    { label: "1m Markout", value: escapeHtml(item.markout?.oneMinute || "not_enough_data") },
                    { label: "5m Markout", value: escapeHtml(item.markout?.fiveMinute || "not_enough_data") },
                    { label: "15m Markout", value: escapeHtml(item.markout?.fifteenMinute || "not_enough_data") },
                    { label: "1h Markout", value: escapeHtml(item.markout?.oneHour || "not_enough_data") },
                    { label: "Quality", value: escapeHtml(item.quality?.qualityBucket || "not_enough_data") },
                    { label: "Recommendation", value: escapeHtml(item.recommendation?.action || "insufficient_data") },
                    { label: "Ledger Available", value: formatBool(Boolean(item.governance?.ledgerAvailable)) },
                  ])}
                </div>`
              )
              .join("")
          : `<div class="muted">No unified signal inbox items for the selected symbol.</div>`
      }
    </div>` +
    (state.latestToxicSignalInboxAction
      ? `<div class="muted">${escapeHtml(state.latestToxicSignalInboxAction)}</div>`
      : "");
}

function renderToxicSignalGroups() {
  const url = toxicSignalGroupsRecentUrl();
  const statusUrl = toxicSignalGroupsStatusUrl();
  const report = getToxicSignalGroupsPayload();
  const statusPayload = getToxicSignalGroupsStatusPayload();
  const error =
    getError(url) ||
    getError(statusUrl) ||
    (!state.toxicSignalGroupSymbol
      ? getError("/api/toxicity/signal-groups/recent") ||
        getError("/api/toxicity/signal-groups/status")
      : null);
  if (error) {
    setBadge("toxicSignalGroupsBadge", "API Error", "error");
    $("toxicSignalGroupsContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicSignalGroupsBadge", "Loading", "none");
    $("toxicSignalGroupsContent").innerHTML =
      `<div class="muted">Signal groups will appear after signal inbox items are available.</div>`;
    return;
  }

  const groups = report.groups || [];
  setBadge(
    "toxicSignalGroupsBadge",
    (statusPayload?.mode || report.mode || "analysis_only").toUpperCase(),
    groups.length ? "warning" : "none"
  );

  $("toxicSignalGroupsContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      { label: "Runtime Modified", value: formatBool(Boolean(report.runtimeModified)) },
      { label: "Analysis Only", value: formatBool(Boolean(report.analysisOnly)) },
      { label: "Execution Enabled", value: formatBool(Boolean(report.executionEnabled)) },
      { label: "Group Count", value: formatInteger(groups.length) },
      { label: "Cooldown Window ms", value: formatInteger(report.cooldownWindowMs) },
      { label: "Selected Symbol", value: escapeHtml(report.selectedSymbol || "ALL") },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicSignalGroupsButton">Refresh Signal Groups</button>
      <button type="button" class="small-button" id="copyToxicSignalGroupsJsonButton">Copy Signal Groups JSON</button>
    </div>` +
    `<div class="stack">
      ${
        groups.length
          ? groups
              .slice(0, 6)
              .map(
                (group) => `
                <div class="metric">
                  <div class="metric-label">${escapeHtml(group.symbol)} · ${escapeHtml(group.signalKind)}</div>
                  <div class="metric-value">${escapeHtml(group.operatorAction || "review_grouped_signal")}</div>
                  ${renderMetrics([
                    { label: "Direction", value: escapeHtml(group.directionBias || "neutral") },
                    { label: "Count", value: formatInteger(group.count) },
                    { label: "Max Severity", value: escapeHtml(group.maxSeverity || "unknown") },
                    { label: "Avg Confidence", value: formatNumber((group.avgConfidence || 0) * 100, 1) + "%" },
                    { label: "Representative", value: escapeHtml(group.representativeSignalId || "Unavailable") },
                    { label: "First Seen", value: formatDateTime(group.firstSeenAtMs) },
                    { label: "Last Seen", value: formatDateTime(group.lastSeenAtMs) },
                    { label: "Original Signals Preserved", value: formatBool(Boolean(group.originalSignalsPreserved)) },
                  ])}
                  <div class="muted">${escapeHtml(group.suppressionHint || "Grouped for display only.")}</div>
                </div>`
              )
              .join("")
          : `<div class="muted">No signal groups for the selected symbol.</div>`
      }
    </div>` +
    (state.latestToxicSignalGroupAction
      ? `<div class="muted">${escapeHtml(state.latestToxicSignalGroupAction)}</div>`
      : "");
}

function renderToxicSignalDetail() {
  const statusUrl = toxicSignalDetailStatusUrl();
  const statusPayload = getToxicSignalDetailStatusPayload();
  const payload = state.toxicSignalDetailPayload;
  const error = getError(statusUrl);
  if (error) {
    setBadge("toxicSignalDetailBadge", "API Error", "error");
    $("toxicSignalDetailContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!statusPayload) {
    setBadge("toxicSignalDetailBadge", "Loading", "none");
    $("toxicSignalDetailContent").innerHTML =
      `<div class="muted">Signal detail will appear after signal inbox and grouped summaries are available.</div>`;
    return;
  }

  const activeDetail = payload?.detail || null;
  const representative = activeDetail?.representativeSignal || activeDetail;
  const timeline = representative?.timeline || [];
  setBadge(
    "toxicSignalDetailBadge",
    payload?.available ? "DETAIL READY" : "ANALYSIS_ONLY",
    payload?.available ? "warning" : "none"
  );

  $("toxicSignalDetailContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(statusPayload.readOnly)) },
      { label: "Runtime Modified", value: formatBool(Boolean(statusPayload.runtimeModified)) },
      { label: "Analysis Only", value: formatBool(Boolean(statusPayload.analysisOnly)) },
      { label: "Execution Enabled", value: formatBool(Boolean(statusPayload.executionEnabled)) },
      {
        label: "Manual Review Required",
        value: formatBool(Boolean(statusPayload.manualReviewRequired)),
      },
      { label: "Signal Count", value: formatInteger(statusPayload.signalCount) },
      { label: "Group Count", value: formatInteger(statusPayload.groupCount) },
      { label: "Selected Symbol", value: escapeHtml(statusPayload.selectedSymbol || "ALL") },
    ]) +
    `<div class="action-row">
      <input id="toxicSignalDetailSignalIdInput" placeholder="signal id" value="${escapeHtml(
        state.toxicSignalDetailSignalId || ""
      )}" />
      <button type="button" class="small-button" id="loadToxicSignalDetailButton">Load Signal Detail</button>
      <input id="toxicSignalDetailGroupIdInput" placeholder="group id" value="${escapeHtml(
        state.toxicSignalDetailGroupId || ""
      )}" />
      <button type="button" class="small-button" id="loadToxicSignalGroupDetailButton">Load Group Detail</button>
      <button type="button" class="small-button" id="copyToxicSignalDetailJsonButton">Copy Signal Detail JSON</button>
    </div>` +
    (payload?.available && representative
      ? `
      <div class="metric">
        <div class="metric-label">${escapeHtml(representative.symbol)} · ${escapeHtml(representative.signalKind)}</div>
        <div class="metric-value">${escapeHtml(representative.operatorAction || "review_evidence")}</div>
        ${renderMetrics([
          { label: "Direction", value: escapeHtml(representative.directionBias || "neutral") },
          { label: "Severity", value: escapeHtml(representative.severity || "unknown") },
          {
            label: "Confidence",
            value: formatNumber((representative.confidence || 0) * 100, 1) + "%",
          },
          { label: "Signal ID", value: escapeHtml(representative.signalId || "Unavailable") },
          {
            label: "Group ID",
            value: escapeHtml(representative.source?.groupId || "Unavailable"),
          },
        ])}
        <details class="signal-details" open>
          <summary>Evidence Timeline</summary>
          <div class="stack">
            ${timeline
              .map(
                (stage) => `
                  <div class="metric">
                    <div class="metric-label">${escapeHtml(stage.label || stage.stage)}</div>
                    <div class="metric-value">${escapeHtml(stage.available ? "available" : "unavailable")}</div>
                    <div class="muted">${escapeHtml(stage.summary || "Unavailable")}</div>
                    <div class="muted">Timestamp: ${formatDateTime(stage.timestampMs)}</div>
                  </div>`
              )
              .join("")}
          </div>
        </details>
        <details class="signal-details">
          <summary>Operator Narrative</summary>
          ${renderReasons(representative.operatorNarrative?.whySignalFired || [])}
          ${renderReasons(representative.operatorNarrative?.whatConfirmedIt || [])}
          ${renderReasons(representative.operatorNarrative?.whatConflicted || [])}
          ${renderReasons(representative.operatorNarrative?.whyNoExecution || [])}
          <div class="muted">${escapeHtml(representative.noExecutionReason || "Signal-only analysis.")}</div>
        </details>
      </div>
      ${
        activeDetail?.group && activeDetail?.members
          ? `<div class="stack">
              <div class="section-label">Grouped Members</div>
              ${activeDetail.members
                .map(
                  (member) => `
                    <article class="signal-card signal-card-${severityTone(member.severity)}">
                      <details class="signal-details">
                        <summary>
                          <div class="signal-summary">
                            <div class="signal-summary-title">${escapeHtml(member.signalId)}</div>
                            ${renderSignalChipRow([
                              renderSignalChip((member.severity || "unknown").toString().toUpperCase(), severityTone(member.severity)),
                              renderSignalChip(member.operatorAction || "watch_signal_only", "neutral"),
                            ])}
                          </div>
                          <div class="signal-summary-meta">${escapeHtml(member.signalId || "Unavailable")} · ${escapeHtml(member.severity || "unknown")}</div>
                        </summary>
                        <div class="signal-card-body">
                          ${renderMetrics([
                            { label: "Confidence", value: `${formatNumber((member.confidence || 0) * 100, 1)}%` },
                            { label: "Action", value: escapeHtml(member.operatorAction || "watch_signal_only") },
                          ])}
                        </div>
                      </details>
                    </article>`
                )
                .join("")}
            </div>`
          : ""
      }`
      : `<div class="muted">Load a signal or group detail to inspect the evidence timeline. Read-only. Analysis only. No order placement. No live trading.</div>`) +
    (payload && !payload.available && payload.reason
      ? `<div class="muted">${escapeHtml(payload.reason)}</div>`
      : "") +
    (state.latestToxicSignalDetailAction
      ? `<div class="muted">${escapeHtml(state.latestToxicSignalDetailAction)}</div>`
      : "");
}

function renderToxicSignalHistorySignalCard(item) {
  const tone = severityTone(item.severity);
  const alertExplanationFound =
    item.alertExplanationFound === undefined
      ? "unknown"
      : item.alertExplanationFound
        ? "true"
        : "false";
  const alertExplanationTone =
    item.alertExplanationFound === undefined
      ? "neutral"
      : item.alertExplanationFound
        ? "success"
        : "muted";
  return `
    <article class="signal-card signal-card-${tone}">
      <details class="signal-details" open>
        <summary>
          <div class="signal-summary">
            <div class="signal-summary-title">${escapeHtml(item.signalId || "Unavailable")}</div>
            ${renderSignalChipRow([
              renderSignalChip((item.severity || "unknown").toString().toUpperCase(), tone),
              renderSignalChip(item.operatorAction || "watch_signal_only", "neutral"),
              renderSignalChip(item.noTradeOnly ? "no-trade-only" : "trade-view", item.noTradeOnly ? "muted" : "success"),
              renderSignalChip(`alert_explanation_found=${alertExplanationFound}`, alertExplanationTone),
              renderSignalChip(
                item.markoutOneMinute || "not_enough_data",
                item.markoutOneMinute === "aligned"
                  ? "success"
                  : item.markoutOneMinute === "adverse"
                    ? "danger"
                    : item.markoutOneMinute === "neutral"
                      ? "warning"
                      : "muted"
              ),
            ])}
          </div>
          <div class="signal-summary-meta">${escapeHtml(item.symbol || "Unavailable")} · ${escapeHtml(item.signalKind || "Unavailable")}</div>
        </summary>
        <div class="signal-card-body">
          ${renderMetrics([
            { label: "Direction Bias", value: escapeHtml(item.directionBias || "neutral") },
            { label: "Confidence", value: `${formatNumber((item.confidence || 0) * 100, 1)}%` },
            { label: "Quality Bucket", value: escapeHtml(item.qualityBucket || "not_enough_data") },
            { label: "Recommendation", value: escapeHtml(item.recommendationAction || "insufficient_data") },
            { label: "Created", value: formatDateTime(item.createdAtMs) },
            { label: "Recorded", value: formatDateTime(item.historyRecordedAtMs) },
          ])}
          <div class="signal-chip-row">
            ${renderSignalChip(`1m ${item.markoutOneMinute || "not_enough_data"}`, item.markoutOneMinute === "adverse" ? "danger" : item.markoutOneMinute === "aligned" ? "success" : item.markoutOneMinute === "neutral" ? "warning" : "muted")}
            ${renderSignalChip(`5m ${item.markoutFiveMinute || "not_enough_data"}`, item.markoutFiveMinute === "adverse" ? "danger" : item.markoutFiveMinute === "aligned" ? "success" : item.markoutFiveMinute === "neutral" ? "warning" : "muted")}
            ${renderSignalChip(`15m ${item.markoutFifteenMinute || "not_enough_data"}`, item.markoutFifteenMinute === "adverse" ? "danger" : item.markoutFifteenMinute === "aligned" ? "success" : item.markoutFifteenMinute === "neutral" ? "warning" : "muted")}
            ${renderSignalChip(`1h ${item.markoutOneHour || "not_enough_data"}`, item.markoutOneHour === "adverse" ? "danger" : item.markoutOneHour === "aligned" ? "success" : item.markoutOneHour === "neutral" ? "warning" : "muted")}
          </div>
          <div class="muted">Source: ${escapeHtml(item.source || "signal_inbox")}</div>
        </div>
      </details>
    </article>`;
}

function renderToxicSignalHistoryGroupCard(item) {
  const tone = severityTone(item.maxSeverity);
  const memberIds = item.memberSignalIds || [];
  return `
    <article class="signal-card signal-card-${tone}">
      <details class="signal-details">
        <summary>
          <div class="signal-summary">
            <div class="signal-summary-title">${escapeHtml(item.groupId || "Unavailable")}</div>
            ${renderSignalChipRow([
              renderSignalChip(`count ${formatInteger(item.count)}`, "neutral"),
              renderSignalChip((item.maxSeverity || "unknown").toString().toUpperCase(), tone),
              renderSignalChip(
                item.originalSignalsPreserved ? "original signals preserved" : "original signals not preserved",
                item.originalSignalsPreserved ? "success" : "danger"
              ),
            ])}
          </div>
          <div class="signal-summary-meta">${escapeHtml(item.symbol || "Unavailable")} · ${escapeHtml(item.signalKind || "Unavailable")} · rep ${escapeHtml(item.representativeSignalId || "Unavailable")}</div>
        </summary>
        <div class="signal-card-body">
          ${renderMetrics([
            { label: "Direction Bias", value: escapeHtml(item.directionBias || "neutral") },
            { label: "Avg Confidence", value: `${formatNumber((item.avgConfidence || 0) * 100, 1)}%` },
            { label: "First Seen", value: formatDateTime(item.firstSeenAtMs) },
            { label: "Last Seen", value: formatDateTime(item.lastSeenAtMs) },
            { label: "Source", value: escapeHtml(item.source || "signal_history") },
            { label: "Recorded", value: formatDateTime(item.historyRecordedAtMs) },
          ])}
          <div class="signal-chip-row">
            ${(memberIds.length ? memberIds : ["No members"])
              .map((id) => renderSignalChip(id, memberIds.length ? "neutral" : "muted"))
              .join("")}
          </div>
        </div>
      </details>
    </article>`;
}

function renderToxicSignalHistoryAlertCard(item) {
  const tone = previewStatusTone(item.previewStatus);
  return `
    <article class="signal-card signal-card-${tone}">
      <details class="signal-details">
        <summary>
          <div class="signal-summary">
            <div class="signal-summary-title">${escapeHtml(item.signalId || "Unavailable")}</div>
            ${renderSignalChipRow([
              renderSignalChip(item.previewStatus || "Unavailable", tone),
              renderSignalChip(
                item.wouldNotifyIfEnabled ? "would notify if enabled" : "no notification",
                item.wouldNotifyIfEnabled ? "success" : "muted"
              ),
              renderSignalChip(item.noTradeOnly ? "no-trade-only" : "trade-view", item.noTradeOnly ? "muted" : "success"),
              renderSignalChip(item.markoutReadiness || "Unavailable", "neutral"),
            ])}
          </div>
          <div class="signal-summary-meta">${escapeHtml(item.symbol || "Unavailable")} · ${escapeHtml(item.signalKind || "Unavailable")}</div>
        </summary>
        <div class="signal-card-body">
          ${renderMetrics([
            { label: "Notification Sent", value: formatBool(Boolean(item.notificationSent)) },
            { label: "Execution Triggered", value: formatBool(Boolean(item.executionTriggered)) },
            { label: "Source", value: escapeHtml(item.source || "signal_history") },
            { label: "Recorded", value: formatDateTime(item.historyRecordedAtMs) },
          ])}
          <div class="muted">${item.wouldNotifyIfEnabled ? "Notification preview candidate." : "Preview only. No notification was sent."}</div>
        </div>
      </details>
    </article>`;
}

function renderToxicSignalHistoryReportCard(item) {
  return `
    <article class="signal-card signal-card-neutral">
      <details class="signal-details">
        <summary>
          <div class="signal-summary">
            <div class="signal-summary-title">${escapeHtml(item.date || "Unavailable")} · ${escapeHtml(item.symbol || "Unavailable")}</div>
            ${renderSignalChipRow([
              renderSignalChip(`total ${formatInteger(item.totalSignals)}`, "neutral"),
              renderSignalChip(`grouped ${formatInteger(item.groupedSignals)}`, "neutral"),
              renderSignalChip(`high ${formatInteger(item.highSeveritySignals)}`, "warning"),
              renderSignalChip(`no-trade ${formatInteger(item.noTradeOnlyCandidates)}`, "muted"),
              renderSignalChip(`downgrade ${formatInteger(item.downgradeCandidates)}`, "warning"),
            ])}
          </div>
          <div class="signal-summary-meta">history report · ${escapeHtml(item.reportType || "daily")}</div>
        </summary>
        <div class="signal-card-body">
          ${renderMetrics([
            { label: "Not Enough Data", value: formatInteger(item.notEnoughDataSignals) },
            { label: "Source", value: escapeHtml(item.source || "signal_history") },
            { label: "Recorded", value: formatDateTime(item.historyRecordedAtMs) },
          ])}
        </div>
      </details>
    </article>`;
}

function renderToxicSignalHistory() {
  const recentUrl = toxicSignalHistoryRecentUrl();
  const statusUrl = toxicSignalHistoryStatusUrl();
  const alertsUrl = toxicSignalHistoryAlertsUrl();
  const reportsUrl = toxicSignalHistoryReportsUrl();
  const report = getToxicSignalHistoryPayload();
  const statusPayload = getToxicSignalHistoryStatusPayload();
  const alertsPayload = getToxicSignalHistoryAlertsPayload();
  const reportsPayload = getToxicSignalHistoryReportsPayload();
  const error =
    getError(recentUrl) ||
    getError(statusUrl) ||
    getError(alertsUrl) ||
    getError(reportsUrl) ||
    (!state.toxicSignalHistorySymbol
      ? getError("/api/toxicity/signal-history/recent") ||
        getError("/api/toxicity/signal-history/status")
      : null);
  if (error) {
    setBadge("toxicSignalHistoryBadge", "API Error", "error");
    $("toxicSignalHistoryContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report || !statusPayload || !alertsPayload || !reportsPayload) {
    setBadge("toxicSignalHistoryBadge", "Loading", "none");
    $("toxicSignalHistoryContent").innerHTML =
      `<div class="muted">Signal history will appear after read-only signal snapshots are captured in memory.</div>`;
    return;
  }

  const items = report.items || [];
  const groupItems = report.groupItems || [];
  const alertItems = alertsPayload.items || [];
  const reportItems = reportsPayload.items || [];
  const lookupPayload = state.toxicSignalHistoryLookupPayload;
  const historySortMode = state.toxicSignalHistorySortMode || "severity";
  const sortedItems = sortRecentSignalItems(items, historySortMode);
  const sortedGroupItems = sortGroupHistoryItems(groupItems, historySortMode);
  const sortedAlertItems = sortAlertHistoryItems(alertItems, historySortMode);
  const sortedReportItems = [...reportItems].sort(
    (a, b) =>
      String(b.date || "").localeCompare(String(a.date || "")) ||
      (b.totalSignals || 0) - (a.totalSignals || 0)
  );

  setBadge(
    "toxicSignalHistoryBadge",
    (statusPayload.retentionMode || "in_memory_bounded").toUpperCase(),
    items.length ? "warning" : "none"
  );

  $("toxicSignalHistoryContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(statusPayload.readOnly)) },
      { label: "Runtime Modified", value: formatBool(Boolean(statusPayload.runtimeModified)) },
      { label: "Analysis Only", value: formatBool(Boolean(statusPayload.analysisOnly)) },
      { label: "Execution Enabled", value: formatBool(Boolean(statusPayload.executionEnabled)) },
      { label: "Retention Mode", value: escapeHtml(statusPayload.retentionMode || "Unavailable") },
      {
        label: "durableStorageEnabled",
        value: formatBool(Boolean(statusPayload.durableStorageEnabled)),
      },
      {
        label: "databaseWriteEnabled",
        value: formatBool(Boolean(statusPayload.databaseWriteEnabled)),
      },
      { label: "Current Signals", value: formatInteger(statusPayload.currentSignals) },
      { label: "Current Groups", value: formatInteger(statusPayload.currentGroups) },
      { label: "Current Alerts", value: formatInteger(statusPayload.currentAlerts) },
      { label: "Current Reports", value: formatInteger(statusPayload.currentReports) },
      { label: "Selected Symbol", value: escapeHtml(report.selectedSymbol || "ALL") },
    ]) +
    `<div class="action-row signal-history-toolbar">
      <div class="signal-toolbar-label">Signal History Sort</div>
      <select id="toxicSignalHistorySortSelect" class="signal-sort-select">
        <option value="severity"${historySortMode === "severity" ? " selected" : ""}>Sort by Severity</option>
        <option value="newest"${historySortMode === "newest" ? " selected" : ""}>Sort by Newest</option>
        <option value="symbol"${historySortMode === "symbol" ? " selected" : ""}>Sort by Symbol</option>
        <option value="count"${historySortMode === "count" ? " selected" : ""}>Sort by Group Count</option>
      </select>
      <input id="toxicSignalHistorySymbolInput" placeholder="symbol" value="${escapeHtml(
        state.toxicSignalHistorySymbol || toxicSignalHistorySelectedSymbol() || ""
      )}" />
      <button type="button" class="small-button" id="loadToxicSignalHistoryBySymbolButton">Load Signal History by Symbol</button>
      <input id="toxicSignalHistorySignalIdInput" placeholder="signal_id" value="${escapeHtml(
        state.toxicSignalHistorySignalId || ""
      )}" />
      <button type="button" class="small-button" id="loadToxicSignalHistorySignalButton">Load Signal by ID</button>
      <button type="button" class="small-button" id="refreshToxicSignalHistoryButton">Refresh Signal History</button>
      <button type="button" class="small-button" id="copyToxicSignalHistoryJsonButton">Copy Signal History JSON</button>
      <button type="button" class="small-button" id="exportToxicSignalHistoryJsonButton">Export Signal History JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Recent Signals</div>
        <div class="metric-value">${
          sortedItems.length
            ? `<div class="signal-card-grid">${sortedItems.slice(0, 8).map((item) => renderToxicSignalHistorySignalCard(item)).join("")}</div>`
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Group History</div>
        <div class="metric-value">${
          sortedGroupItems.length
            ? `<div class="signal-card-grid">${sortedGroupItems.slice(0, 6).map((item) => renderToxicSignalHistoryGroupCard(item)).join("")}</div>`
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Alert Preview History</div>
        <div class="metric-value">${
          sortedAlertItems.length
            ? `<div class="signal-card-grid">${sortedAlertItems.slice(0, 6).map((item) => renderToxicSignalHistoryAlertCard(item)).join("")}</div>`
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Report History</div>
        <div class="metric-value">${
          sortedReportItems.length
            ? `<div class="signal-card-grid">${sortedReportItems.slice(0, 6).map((item) => renderToxicSignalHistoryReportCard(item)).join("")}</div>`
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Signal Lookup</div>
        <div class="metric-value">${
          lookupPayload
            ? lookupPayload.found
              ? `
                <details class="signal-details" open>
                  <summary>
                    <div class="signal-summary">
                      <div class="signal-summary-title">${escapeHtml(lookupPayload.signal?.signalId || "Unavailable")}</div>
                      ${renderSignalChipRow([
                        renderSignalChip(lookupPayload.signal?.severity || "unknown", severityTone(lookupPayload.signal?.severity)),
                        renderSignalChip(lookupPayload.signal?.qualityBucket || "unknown", lookupPayload.signal?.qualityBucket === "good" ? "success" : lookupPayload.signal?.qualityBucket === "not_enough_data" ? "muted" : "warning"),
                        renderSignalChip(lookupPayload.signal?.recommendationAction || "unknown", "neutral"),
                      ])}
                    </div>
                    <div class="signal-summary-meta">${escapeHtml(lookupPayload.signal?.symbol || "Unavailable")} · ${escapeHtml(lookupPayload.signal?.signalKind || "Unavailable")}</div>
                  </summary>
                  <div class="signal-card-body">
                    ${renderMetrics([
                      { label: "Direction Bias", value: escapeHtml(lookupPayload.signal?.directionBias || "neutral") },
                      { label: "Confidence", value: `${formatNumber((lookupPayload.signal?.confidence || 0) * 100, 1)}%` },
                      { label: "Markout 1m", value: escapeHtml(lookupPayload.signal?.markoutOneMinute || "not_enough_data") },
                      { label: "Markout 5m", value: escapeHtml(lookupPayload.signal?.markoutFiveMinute || "not_enough_data") },
                      { label: "Markout 15m", value: escapeHtml(lookupPayload.signal?.markoutFifteenMinute || "not_enough_data") },
                      { label: "Markout 1h", value: escapeHtml(lookupPayload.signal?.markoutOneHour || "not_enough_data") },
                      { label: "Operator Action", value: escapeHtml(lookupPayload.signal?.operatorAction || "watch_signal_only") },
                    ])}
                  </div>
                </details>`
              : `<div class="signal-card signal-card-muted"><div class="muted">not_found</div></div>`
            : "Load a signal ID to inspect one retained signal."
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Operator Notes</div>
        <div class="metric-value">${(report.operatorNotes || [])
          .map((note) => escapeHtml(note))
          .join("<br/>") || "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          Read-only<br/>
          Analysis only<br/>
          retentionMode=in_memory_bounded<br/>
          durableStorageEnabled=false<br/>
          databaseWriteEnabled=false<br/>
          No live trading<br/>
          No order placement
        </div>
      </div>
    </div>` +
    (state.latestToxicSignalHistoryAction
      ? `<div class="muted">${escapeHtml(state.latestToxicSignalHistoryAction)}</div>`
      : "");
}

function renderToxicSignalReport() {
  const url = toxicSignalReportDailyUrl();
  const statusUrl = toxicSignalReportStatusUrl();
  const report = getToxicSignalReportPayload();
  const statusPayload = getToxicSignalReportStatusPayload();
  const error = getError(url) || getError(statusUrl);
  if (error) {
    setBadge("toxicSignalReportBadge", "API Error", "error");
    $("toxicSignalReportContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report || !statusPayload) {
    setBadge("toxicSignalReportBadge", "Loading", "none");
    $("toxicSignalReportContent").innerHTML =
      `<div class="muted">Daily report will appear after signal inbox, grouped summaries, and quality views load.</div>`;
    return;
  }

  const summary = report.summary || {};
  const markoutSummary = report.markoutSummary || {};
  const bySymbol = report.bySymbol || [];
  const bySignalKind = report.bySignalKind || [];
  const topGroups = report.topGroups || [];

  setBadge(
    "toxicSignalReportBadge",
    (statusPayload.reportType || report.reportType || "daily").toUpperCase(),
    summary.totalSignals ? "warning" : "none"
  );

  $("toxicSignalReportContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      { label: "Runtime Modified", value: formatBool(Boolean(report.runtimeModified)) },
      { label: "Analysis Only", value: formatBool(Boolean(report.analysisOnly)) },
      { label: "Execution Enabled", value: formatBool(Boolean(report.executionEnabled)) },
      {
        label: "Manual Review Required",
        value: formatBool(Boolean(report.manualReviewRequired)),
      },
      { label: "Report Type", value: escapeHtml(report.reportType || "daily") },
      { label: "Date", value: escapeHtml(report.date || "Unavailable") },
      { label: "Filter Symbol", value: escapeHtml(report.filter?.symbol || "ALL") },
      { label: "Total Signals", value: formatInteger(summary.totalSignals) },
      { label: "Grouped Signals", value: formatInteger(summary.groupedSignals) },
      {
        label: "High Severity Signals",
        value: formatInteger(summary.highSeveritySignals),
      },
      {
        label: "Not Enough Data Signals",
        value: formatInteger(summary.notEnoughDataSignals),
      },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicSignalReportButton">Refresh Daily Report</button>
      <button type="button" class="small-button" id="copyToxicSignalReportJsonButton">Copy Daily Report JSON</button>
      <button type="button" class="small-button" id="copyToxicSignalReportMarkdownButton">Copy Daily Report Markdown</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Summary</div>
        <div class="metric-value">${renderMetrics([
          { label: "No-trade Only", value: formatInteger(summary.noTradeOnlyCandidates) },
          { label: "Downgrade Candidates", value: formatInteger(summary.downgradeCandidates) },
          { label: "View Only", value: formatBool(Boolean(report.filter?.viewOnly)) },
          {
            label: "Persistent watchlist disabled",
            value: formatBool(!Boolean(report.filter?.persistentWatchlistEnabled)),
          },
        ])}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Markout</div>
        <div class="metric-value">${renderMetrics([
          { label: "Aligned", value: formatInteger(markoutSummary.aligned) },
          { label: "Adverse", value: formatInteger(markoutSummary.adverse) },
          { label: "Neutral", value: formatInteger(markoutSummary.neutral) },
          {
            label: "Not Enough Data",
            value: formatInteger(markoutSummary.notEnoughData),
          },
        ])}</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Symbol</div>
        <div class="metric-value">${
          bySymbol.length
            ? renderTable(
                ["Symbol", "Signals", "High Severity", "No-trade Only", "Downgrade", "Not Enough Data", "Avg Confidence"],
                bySymbol.map((bucket) => [
                  escapeHtml(bucket.label || bucket.key || "Unavailable"),
                  formatInteger(bucket.signalCount),
                  formatInteger(bucket.highSeveritySignals),
                  formatInteger(bucket.noTradeOnlyCandidates),
                  formatInteger(bucket.downgradeCandidates),
                  formatInteger(bucket.notEnoughDataSignals),
                  `${formatNumber((bucket.avgConfidence || 0) * 100, 1)}%`,
                ])
              )
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Signal Kind</div>
        <div class="metric-value">${
          bySignalKind.length
            ? renderTable(
                ["Signal Kind", "Signals", "High Severity", "No-trade Only", "Downgrade", "Not Enough Data", "Avg Confidence"],
                bySignalKind.map((bucket) => [
                  escapeHtml(bucket.label || bucket.key || "Unavailable"),
                  formatInteger(bucket.signalCount),
                  formatInteger(bucket.highSeveritySignals),
                  formatInteger(bucket.noTradeOnlyCandidates),
                  formatInteger(bucket.downgradeCandidates),
                  formatInteger(bucket.notEnoughDataSignals),
                  `${formatNumber((bucket.avgConfidence || 0) * 100, 1)}%`,
                ])
              )
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Top Groups</div>
        <div class="metric-value">${
          topGroups.length
            ? renderTable(
                ["Symbol", "Signal Kind", "Count", "Severity", "Representative"],
                topGroups.map((group) => [
                  escapeHtml(group.symbol || "Unavailable"),
                  escapeHtml(group.signalKind || "Unavailable"),
                  formatInteger(group.count),
                  escapeHtml(group.maxSeverity || "Unavailable"),
                  escapeHtml(group.representativeSignalId || "Unavailable"),
                ])
              )
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Operator Notes</div>
        <div class="metric-value">${(report.operatorNotes || [])
          .map((note) => escapeHtml(note))
          .join("<br/>") || "None"}</div>
      </div>
    </div>` +
    (state.latestToxicSignalReportAction
      ? `<div class="muted">${escapeHtml(state.latestToxicSignalReportAction)}</div>`
      : "");
}

function renderToxicSignalRolling() {
  const url = toxicSignalReportRollingUrl();
  const report = getToxicSignalRollingPayload();
  const error = getError(url);
  if (error) {
    setBadge("toxicSignalRollingBadge", "API Error", "error");
    $("toxicSignalRollingContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicSignalRollingBadge", "Loading", "none");
    $("toxicSignalRollingContent").innerHTML =
      `<div class="muted">Rolling digest will appear after bounded signal history and alert preview history are available.</div>`;
    return;
  }

  const summary = report.summary || {};
  const topSymbols = summary.topSymbols || [];
  const topSignalKinds = summary.topSignalKinds || [];
  const mixedMarkout =
    [summary.aligned, summary.adverse, summary.neutral, summary.notEnoughData].filter(
      (value) => Number(value || 0) > 0
    ).length > 1;

  setBadge(
    "toxicSignalRollingBadge",
    (report.reportType || "rolling").toUpperCase(),
    summary.totalSignals ? "warning" : "none"
  );

  $("toxicSignalRollingContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      { label: "Runtime Modified", value: formatBool(Boolean(report.runtimeModified)) },
      { label: "Analysis Only", value: formatBool(Boolean(report.analysisOnly)) },
      { label: "Execution Enabled", value: formatBool(Boolean(report.executionEnabled)) },
      {
        label: "Manual Review Required",
        value: formatBool(Boolean(report.manualReviewRequired)),
      },
      { label: "Report Type", value: escapeHtml(report.reportType || "rolling") },
      { label: "Window", value: escapeHtml(report.window || "7d") },
      { label: "Filter Symbol", value: escapeHtml(report.filter?.symbol || "ALL") },
      { label: "retentionMode", value: escapeHtml(report.retentionMode || "Unavailable") },
      {
        label: "durableStorageEnabled",
        value: formatBool(Boolean(report.durableStorageEnabled)),
      },
      {
        label: "databaseWriteEnabled",
        value: formatBool(Boolean(report.databaseWriteEnabled)),
      },
      { label: "Total Signals", value: formatInteger(summary.totalSignals) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicSignalRollingButton">Refresh Rolling Digest</button>
      <button type="button" class="small-button" id="copyToxicSignalRollingJsonButton">Copy Rolling Digest JSON</button>
      <button type="button" class="small-button" id="copyToxicSignalRollingMarkdownButton">Copy Rolling Digest Markdown</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Markout Summary</div>
        <div class="metric-value">
          ${renderMetrics([
            { label: "Aligned", value: formatInteger(summary.aligned) },
            { label: "Adverse", value: formatInteger(summary.adverse) },
            { label: "Neutral", value: formatInteger(summary.neutral) },
            {
              label: "Not Enough Data",
              value: formatInteger(summary.notEnoughData),
            },
          ])}
          ${renderSignalChipRow([
            renderSignalChip(`aligned ${formatInteger(summary.aligned)}`, "success"),
            renderSignalChip(`adverse ${formatInteger(summary.adverse)}`, "danger"),
            renderSignalChip(`neutral ${formatInteger(summary.neutral)}`, "warning"),
            renderSignalChip(
              `not enough data ${formatInteger(summary.notEnoughData)}`,
              "muted"
            ),
            renderSignalChip(mixedMarkout ? "mixed markout" : "single markout band", mixedMarkout ? "warning" : "neutral"),
          ])}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Signal Quality</div>
        <div class="metric-value">${renderMetrics([
          { label: "No-trade Only", value: formatInteger(summary.noTradeOnlyCandidates) },
          { label: "Downgrade Candidates", value: formatInteger(summary.downgradeCandidates) },
          { label: "Notify Candidates", value: formatInteger(summary.notifyCandidates) },
          { label: "Review Candidates", value: formatInteger(summary.reviewCandidates) },
          { label: "View Only", value: formatBool(Boolean(report.filter?.viewOnly)) },
          {
            label: "Runtime Monitor Modified",
            value: formatBool(Boolean(report.filter?.runtimeMonitorModified)),
          },
        ])}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Top Symbols</div>
        <div class="metric-value">${
          topSymbols.length
            ? `<div class="signal-chip-row">${topSymbols
                .map((item) => renderSignalChip(item, "neutral"))
                .join("")}</div>`
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Top Signal Kinds</div>
        <div class="metric-value">${
          topSignalKinds.length
            ? `<div class="signal-chip-row">${topSignalKinds
                .map((item) => renderSignalChip(item, "neutral"))
                .join("")}</div>`
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Operator Notes</div>
        <div class="metric-value">${(report.operatorNotes || [])
          .map((note) => escapeHtml(note))
          .join("<br/>") || "None"}</div>
      </div>
      <details class="signal-details" open>
        <summary>Rolling Digest Details</summary>
        <div class="signal-card-body">
          ${renderMetrics([
            { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
            { label: "Analysis Only", value: formatBool(Boolean(report.analysisOnly)) },
            { label: "retentionMode", value: escapeHtml(report.retentionMode || "Unavailable") },
            {
              label: "durableStorageEnabled",
              value: formatBool(Boolean(report.durableStorageEnabled)),
            },
            {
              label: "databaseWriteEnabled",
              value: formatBool(Boolean(report.databaseWriteEnabled)),
            },
          ])}
        </div>
      </details>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          Read-only<br/>
          Analysis only<br/>
          retentionMode=in_memory_bounded<br/>
          durableStorageEnabled=false<br/>
          databaseWriteEnabled=false<br/>
          No order placement<br/>
          No live trading
        </div>
      </div>
    </div>` +
    (state.latestToxicSignalRollingAction
      ? `<div class="muted">${escapeHtml(state.latestToxicSignalRollingAction)}</div>`
      : "");
}

function renderToxicSignalAlertPreview() {
  const url = toxicSignalAlertPreviewRecentUrl();
  const statusUrl = toxicSignalAlertPreviewStatusUrl();
  const report = getToxicSignalAlertPreviewPayload();
  const statusPayload = getToxicSignalAlertPreviewStatusPayload();
  const error = getError(url) || getError(statusUrl);
  if (error) {
    setBadge("toxicSignalAlertPreviewBadge", "API Error", "error");
    $("toxicSignalAlertPreviewContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report || !statusPayload) {
    setBadge("toxicSignalAlertPreviewBadge", "Loading", "none");
    $("toxicSignalAlertPreviewContent").innerHTML =
      `<div class="muted">Alert preview will appear after signal inbox data and preview rules load.</div>`;
    return;
  }

  const summary = report.summary || {};
  const gate = report.gate || {};
  const bySymbol = report.bySymbol || [];
  const bySignalKind = report.bySignalKind || [];
  const items = report.items || [];
  const explainPayload = getToxicSignalAlertPreviewExplainPayload();

  setBadge(
    "toxicSignalAlertPreviewBadge",
    (statusPayload.mode || report.mode || "notification_preview_only").toUpperCase(),
    summary.notifyCandidates ? "warning" : "none"
  );

  $("toxicSignalAlertPreviewContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      { label: "Analysis Only", value: formatBool(Boolean(report.analysisOnly)) },
      { label: "Execution Enabled", value: formatBool(Boolean(report.executionEnabled)) },
      { label: "Notification Sent", value: formatBool(Boolean(report.notificationSent)) },
      {
        label: "Execution Triggered",
        value: formatBool(Boolean(report.executionTriggered)),
      },
      { label: "Preview Only", value: formatBool(Boolean(report.previewOnly)) },
      { label: "Selected Symbol", value: escapeHtml(report.selectedSymbol || "Unavailable") },
      { label: "Status", value: escapeHtml(report.status || "Unavailable") },
      { label: "Total Signals", value: formatInteger(summary.totalSignals) },
      { label: "Notify Candidates", value: formatInteger(summary.notifyCandidates) },
      { label: "Review Candidates", value: formatInteger(summary.reviewCandidates) },
      { label: "Suppressed Signals", value: formatInteger(summary.suppressedSignals) },
    ]) +
    `<div class="action-row">
      <input id="toxicSignalAlertExplainSignalIdInput" placeholder="signal_id" value="${escapeHtml(
        state.toxicSignalAlertExplainSignalId || ""
      )}" />
      <button type="button" class="small-button" id="loadToxicSignalAlertExplainButton">Load Alert Explanation</button>
      <button type="button" class="small-button" id="copyToxicSignalAlertExplainJsonButton">Copy Alert Explanation JSON</button>
      <button type="button" class="small-button" id="refreshToxicSignalAlertPreviewButton">Refresh Alert Preview</button>
      <button type="button" class="small-button" id="copyToxicSignalAlertPreviewJsonButton">Copy Alert Preview JSON</button>
      <button type="button" class="small-button" id="copyToxicSignalAlertPreviewMarkdownButton">Copy Alert Preview Markdown</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Preview Gate</div>
        <div class="metric-value">${renderMetrics([
          { label: "Min Severity", value: escapeHtml(gate.minSeverity || "Unavailable") },
          {
            label: "Require Cross Venue",
            value: formatBool(Boolean(gate.requireCrossVenue)),
          },
          {
            label: "Require Markout",
            value: formatBool(Boolean(gate.requireMarkout)),
          },
          {
            label: "Require Liquidity Drain",
            value: formatBool(Boolean(gate.requireLiquidityDrain)),
          },
          { label: "Telegram Enabled", value: formatBool(Boolean(gate.telegramEnabled)) },
          { label: "Dedup Window ms", value: formatInteger(gate.dedupWindowMs) },
        ])}</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Symbol</div>
        <div class="metric-value">${
          bySymbol.length
            ? renderTable(
                ["Symbol", "Total", "Notify", "Review", "Suppressed", "No-trade", "Not Enough Data"],
                bySymbol.map((bucket) => [
                  escapeHtml(bucket.label || bucket.key || "Unavailable"),
                  formatInteger(bucket.totalSignals),
                  formatInteger(bucket.notifyCandidates),
                  formatInteger(bucket.reviewCandidates),
                  formatInteger(bucket.suppressedSignals),
                  formatInteger(bucket.noTradeOnlySignals),
                  formatInteger(bucket.notEnoughDataSignals),
                ])
              )
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Signal Kind</div>
        <div class="metric-value">${
          bySignalKind.length
            ? renderTable(
                ["Signal Kind", "Total", "Notify", "Review", "Suppressed", "No-trade", "Not Enough Data"],
                bySignalKind.map((bucket) => [
                  escapeHtml(bucket.label || bucket.key || "Unavailable"),
                  formatInteger(bucket.totalSignals),
                  formatInteger(bucket.notifyCandidates),
                  formatInteger(bucket.reviewCandidates),
                  formatInteger(bucket.suppressedSignals),
                  formatInteger(bucket.noTradeOnlySignals),
                  formatInteger(bucket.notEnoughDataSignals),
                ])
              )
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Preview Items</div>
        <div class="metric-value">${
          items.length
            ? `<div class="signal-card-grid">${items.map((item) => `
                <article class="signal-card signal-card-${previewStatusTone(item.previewStatus)}">
                  <details class="signal-details" open>
                    <summary>
                      <div class="signal-summary">
                        <div class="signal-summary-title">${escapeHtml(item.signalId || "Unavailable")}</div>
                        ${renderSignalChipRow([
                          renderSignalChip(item.previewStatus || "Unavailable", previewStatusTone(item.previewStatus)),
                          renderSignalChip(item.wouldNotifyIfEnabled ? "would notify" : "no notify", yesNoTone(item.wouldNotifyIfEnabled)),
                          renderSignalChip(item.noTradeOnly ? "no-trade-only" : "trade-view", item.noTradeOnly ? "muted" : "success"),
                        ])}
                      </div>
                      <div class="signal-summary-meta">${escapeHtml(item.symbol || "Unavailable")} · ${escapeHtml(item.signalKind || "Unavailable")}</div>
                    </summary>
                    <div class="signal-card-body">
                      ${renderMetrics([
                        { label: "Quality", value: escapeHtml(item.qualityBucket || "Unavailable") },
                        { label: "Governance", value: escapeHtml(item.latestGovernanceDecision || "Unavailable") },
                        { label: "Markout", value: escapeHtml(item.markoutReadiness || "Unavailable") },
                        { label: "Notification Sent", value: formatBool(Boolean(item.notificationSent)) },
                        { label: "Execution Triggered", value: formatBool(Boolean(item.executionTriggered)) },
                      ])}
                      <div class="muted">${item.wouldNotifyIfEnabled ? "Preview candidate only." : "Preview suppressed or review-only."}</div>
                    </div>
                  </details>
                </article>`).join("")}</div>`
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Signal Alert Explainability</div>
        <div class="metric-value">${
          explainPayload
            ? explainPayload.found
              ? `
                <details class="signal-details" open>
                  <summary>
                    <div class="signal-summary">
                      <div class="signal-summary-title">${escapeHtml(explainPayload.signalId || "Unavailable")}</div>
                      ${renderSignalChipRow([
                        renderSignalChip(explainPayload.alertDecision || "Unavailable", previewStatusTone(explainPayload.alertDecision)),
                        renderSignalChip(explainPayload.found ? "found" : "missing", explainPayload.found ? "success" : "muted"),
                      ])}
                    </div>
                    <div class="signal-summary-meta">${escapeHtml(explainPayload.symbol || "Unavailable")} · explanation</div>
                  </summary>
                  <div class="signal-card-body">
                    ${renderMetrics([
                      { label: "Notification Sent", value: formatBool(Boolean(explainPayload.notificationSent)) },
                      { label: "Execution Triggered", value: formatBool(Boolean(explainPayload.executionTriggered)) },
                    ])}
                    <div class="signal-details-grid">
                      <details class="signal-details" open>
                        <summary>Decision Reasons</summary>
                        ${renderReasons(explainPayload.decisionReasons || [])}
                      </details>
                      <details class="signal-details">
                        <summary>Suppression Reasons</summary>
                        ${renderReasons(explainPayload.suppressionReasons || [])}
                      </details>
                      <details class="signal-details">
                        <summary>Missing Inputs</summary>
                        ${renderReasons(explainPayload.missingInputs || [])}
                      </details>
                    </div>
                    <div class="muted">${escapeHtml(explainPayload.operatorNote || "Unavailable")}</div>
                    ${explainPayload.reason ? `<div class="muted">${escapeHtml(explainPayload.reason)}</div>` : ""}
                  </div>
                </details>`
              : `<div class="signal-card signal-card-muted"><div class="signal-chip-row">${renderSignalChip("alert_explanation_found=false", "muted")}</div><div class="muted">${escapeHtml(explainPayload.reason || "Preview only. No notification was sent.")}</div></div>`
            : "Load one signal ID to inspect alert preview reasons."
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Operator Notes</div>
        <div class="metric-value">${(report.operatorNotes || [])
          .map((note) => escapeHtml(note))
          .join("<br/>") || "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          analysisOnly=true<br/>
          executionEnabled=false<br/>
          notificationSent=false<br/>
          executionTriggered=false<br/>
          previewOnly=true<br/>
          No webhook<br/>
          No order placement<br/>
          No wallet/signing<br/>
          No live trading
        </div>
      </div>
    </div>` +
    (state.latestToxicSignalAlertPreviewAction
      ? `<div class="muted">${escapeHtml(state.latestToxicSignalAlertPreviewAction)}</div>`
      : "");
}

function renderDurableArchiveDryRun() {
  const url = durableArchiveDryRunUrl();
  const payload = getDurableArchiveDryRunPayload();
  const error = getError(url);
  if (error) {
    setBadge("durableArchiveDryRunBadge", "API Error", "error");
    $("durableArchiveDryRunContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!payload) {
    setBadge("durableArchiveDryRunBadge", "Loading", "none");
    $("durableArchiveDryRunContent").innerHTML =
      `<div class="muted">Dry-run archive payload will appear after the schema contract preview is prepared.</div>`;
    return;
  }

  const records = payload.records || [];
  const fieldContract = payload.fieldContract || {};
  const validation = payload.validation || {};
  const safetyBoundary = payload.safetyBoundary || [];
  const operatorNotes = payload.operatorNotes || [];

  setBadge(
    "durableArchiveDryRunBadge",
    payload.ok ? "DRY_RUN_ONLY" : "UNAVAILABLE",
    records.length ? "warning" : "none"
  );

  $("durableArchiveDryRunContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(payload.readOnly)) },
      { label: "Runtime Modified", value: formatBool(Boolean(payload.runtimeModified)) },
      { label: "Analysis Only", value: formatBool(Boolean(payload.analysisOnly)) },
      { label: "Execution Enabled", value: formatBool(Boolean(payload.executionEnabled)) },
      { label: "Action", value: escapeHtml(payload.action || "dry_run_write") },
      { label: "Write Mode", value: escapeHtml(payload.writeMode || "dry_run_only") },
      { label: "Schema Version", value: formatInteger(payload.schemaVersion) },
      { label: "Selected Symbol", value: escapeHtml(payload.selectedSymbol || "ALL") },
      { label: "Records Prepared", value: formatInteger(payload.recordsPrepared) },
      {
        label: "archiveWriteEnabled",
        value: formatBool(Boolean(payload.archiveWriteEnabled)),
      },
      {
        label: "durableStorageEnabled",
        value: formatBool(Boolean(payload.durableStorageEnabled)),
      },
      {
        label: "databaseWriteEnabled",
        value: formatBool(Boolean(payload.databaseWriteEnabled)),
      },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshDurableArchiveDryRunButton">Refresh Dry-run</button>
      <button type="button" class="small-button" id="copyDurableArchiveDryRunJsonButton">Copy Dry-run JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Validation</div>
        <div class="metric-value">${renderMetrics([
          { label: "Valid", value: formatBool(Boolean(validation.valid)) },
          { label: "Field Types Valid", value: formatBool(Boolean(validation.fieldTypesValid)) },
          {
            label: "Source Snapshot Fields Valid",
            value: formatBool(Boolean(validation.sourceSnapshotFieldsValid)),
          },
          {
            label: "Derived Fields Valid",
            value: formatBool(Boolean(validation.derivedFieldsValid)),
          },
          {
            label: "Evidence Refs Valid",
            value: formatBool(Boolean(validation.evidenceRefsValid)),
          },
          {
            label: "Persistence Attempted",
            value: formatBool(Boolean(validation.persistenceAttempted)),
          },
          {
            label: "notificationSent",
            value: formatBool(Boolean(payload.notificationSent)),
          },
        ])}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Validation Errors</div>
        <div class="metric-value">${
          validation.errors?.length
            ? validation.errors.map((item) => renderSignalChip(item, "danger")).join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Validation Warnings</div>
        <div class="metric-value">${
          validation.warnings?.length
            ? validation.warnings.map((item) => renderSignalChip(item, "warning")).join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Unsafe Fields</div>
        <div class="metric-value">${
          validation.unsafeFieldsDetected?.length
            ? validation.unsafeFieldsDetected
                .map((item) => renderSignalChip(item, "danger"))
                .join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Field Contract</div>
        <div class="metric-value">${renderTable(
          ["Group", "Fields"],
          [
            [
              "Source Snapshot",
              escapeHtml((fieldContract.sourceSnapshotFields || []).join(", ") || "None"),
            ],
            [
              "Derived",
              escapeHtml((fieldContract.derivedFields || []).join(", ") || "None"),
            ],
            [
              "Evidence Refs",
              escapeHtml(
                (fieldContract.evidenceReferenceFields || []).join(", ") || "None"
              ),
            ],
          ]
        )}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Prepared Records</div>
        <div class="metric-value">${
          records.length
            ? `<div class="signal-card-grid">${records.slice(0, 8).map((record) => `
                <article class="signal-card signal-card-${severityTone(record.sourceSignalType?.includes("short") ? "high" : "medium")}">
                  <details class="signal-details" open>
                    <summary>
                      <div class="signal-summary">
                        <div class="signal-summary-title">${escapeHtml(record.archiveRecordId || "Unavailable")}</div>
                        ${renderSignalChipRow([
                          renderSignalChip(record.symbol || "Unavailable", "neutral"),
                          renderSignalChip(record.direction || "neutral", "warning"),
                          renderSignalChip(record.writeMode || "dry_run_only", "muted"),
                        ])}
                      </div>
                      <div class="signal-summary-meta">${escapeHtml(record.sourceSignalId || "Unavailable")} · ${escapeHtml(record.sourceSignalType || "Unavailable")}</div>
                    </summary>
                    <div class="signal-card-body">
                      ${renderMetrics([
                        { label: "Signal Layer", value: escapeHtml(record.signalLayer || "Unavailable") },
                        { label: "Confidence", value: `${formatNumber((record.confidence || 0) * 100, 1)}%` },
                        { label: "Toxicity Score", value: formatNumber(record.toxicityScore, 2) },
                        { label: "Created At ms", value: formatInteger(record.createdAtMs) },
                        { label: "Signal ts ms", value: formatInteger(record.signalTsMs) },
                        {
                          label: "archiveWriteEnabled",
                          value: formatBool(Boolean(record.archiveWriteEnabled)),
                        },
                      ])}
                      ${renderTable(
                        ["Evidence Ref", "Value"],
                        [
                          ["signalHistoryRef", escapeHtml(record.evidenceRefs?.signalHistoryRef || "Unavailable")],
                          ["replayRef", escapeHtml(record.replayRef || "Unavailable")],
                          ["markoutRef", escapeHtml(record.markoutRef || "Unavailable")],
                          ["governanceRef", escapeHtml(record.governanceRef || "Unavailable")],
                          [
                            "alertPreviewRef",
                            escapeHtml(record.evidenceRefs?.alertPreviewRef || "Unavailable"),
                          ],
                          ["reportRef", escapeHtml(record.evidenceRefs?.reportRef || "Unavailable")],
                        ]
                      )}
                    </div>
                  </details>
                </article>`)
                .join("")}</div>`
            : "No dry-run records prepared."
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">${safetyBoundary.length
          ? safetyBoundary.map((item) => escapeHtml(item)).join("<br/>")
          : "Read-only<br/>No order placement<br/>No wallet/signing<br/>No live trading"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Operator Notes</div>
        <div class="metric-value">${
          operatorNotes.length
            ? operatorNotes.map((note) => escapeHtml(note)).join("<br/>")
            : "Dry-run payload only. No persistence."
        }</div>
      </div>
    </div>` +
    (state.latestDurableArchiveDryRunAction
      ? `<div class="muted">${escapeHtml(state.latestDurableArchiveDryRunAction)}</div>`
      : "");
}

function renderDurableArchiveDryRunReviewPack() {
  const url = durableArchiveDryRunReviewPackLatestUrl();
  const payload = getDurableArchiveDryRunReviewPackPayload();
  const error = getError(url);
  if (error) {
    setBadge("durableArchiveDryRunReviewPackBadge", "API Error", "error");
    $("durableArchiveDryRunReviewPackContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!payload) {
    setBadge("durableArchiveDryRunReviewPackBadge", "Loading", "none");
    $("durableArchiveDryRunReviewPackContent").innerHTML =
      `<div class="muted">Dry-run review pack will appear after the latest review snapshot is prepared.</div>`;
    return;
  }

  const summary = payload.summary || {};
  const validation = payload.validation || {};
  const fieldContract = payload.fieldContract || {};
  const safetyBoundary = payload.safetyBoundary || [];

  setBadge(
    "durableArchiveDryRunReviewPackBadge",
    payload.found ? "REVIEW_PACK" : "NOT_FOUND",
    payload.found ? "warning" : "none"
  );

  $("durableArchiveDryRunReviewPackContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(payload.readOnly)) },
      { label: "Analysis Only", value: formatBool(Boolean(payload.analysisOnly)) },
      {
        label: "Manual Review Required",
        value: formatBool(Boolean(payload.manualReviewRequired)),
      },
      { label: "Execution Enabled", value: formatBool(Boolean(payload.executionEnabled)) },
      { label: "Found", value: formatBool(Boolean(payload.found)) },
      { label: "Dry Run ID", value: escapeHtml(payload.dryRunId || "Unavailable") },
      { label: "Selected Symbol", value: escapeHtml(payload.selectedSymbol || "ALL") },
      { label: "Records Prepared", value: formatInteger(summary.recordsPrepared) },
      {
        label: "Validation Errors",
        value: formatInteger(summary.validationErrorCount),
      },
      {
        label: "Validation Warnings",
        value: formatInteger(summary.validationWarningCount),
      },
      { label: "Unsafe Fields", value: formatInteger(summary.unsafeFieldCount) },
      {
        label: "archiveWriteEnabled",
        value: formatBool(Boolean(payload.archiveWriteEnabled)),
      },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshDurableArchiveDryRunReviewPackButton">Refresh Review Pack</button>
      <button type="button" class="small-button" id="copyDurableArchiveDryRunReviewPackJsonButton">Copy Review Pack JSON</button>
      <button type="button" class="small-button" id="copyDurableArchiveDryRunReviewPackMarkdownButton">Copy Review Pack Markdown</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Validation Errors</div>
        <div class="metric-value">${
          validation.errors?.length
            ? validation.errors.map((item) => renderSignalChip(item, "danger")).join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Validation Warnings</div>
        <div class="metric-value">${
          validation.warnings?.length
            ? validation.warnings.map((item) => renderSignalChip(item, "warning")).join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Unsafe Fields</div>
        <div class="metric-value">${
          validation.unsafeFieldsDetected?.length
            ? validation.unsafeFieldsDetected
                .map((item) => renderSignalChip(item, "danger"))
                .join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Field Contract</div>
        <div class="metric-value">${renderTable(
          ["Group", "Fields"],
          [
            [
              "Source Snapshot",
              escapeHtml((fieldContract.sourceSnapshotFields || []).join(", ") || "None"),
            ],
            [
              "Derived",
              escapeHtml((fieldContract.derivedFields || []).join(", ") || "None"),
            ],
            [
              "Evidence Refs",
              escapeHtml(
                (fieldContract.evidenceReferenceFields || []).join(", ") || "None"
              ),
            ],
          ]
        )}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">${
          safetyBoundary.length
            ? safetyBoundary.map((item) => escapeHtml(item)).join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Markdown Review Pack</div>
        <div class="metric-value"><pre>${escapeHtml(payload.markdown || "Unavailable")}</pre></div>
      </div>
    </div>` +
    (state.latestDurableArchiveDryRunReviewPackAction
      ? `<div class="muted">${escapeHtml(state.latestDurableArchiveDryRunReviewPackAction)}</div>`
      : "");
}

function renderDurableArchiveWriteGate() {
  const url = durableArchiveWriteGateStatusUrl();
  const payload = getDurableArchiveWriteGatePayload();
  const error = getError(url);
  if (error) {
    setBadge("durableArchiveWriteGateBadge", "API Error", "error");
    $("durableArchiveWriteGateContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!payload) {
    setBadge("durableArchiveWriteGateBadge", "Loading", "none");
    $("durableArchiveWriteGateContent").innerHTML =
      `<div class="muted">Write gate status will appear after the disabled-by-default contract is loaded.</div>`;
    return;
  }

  const safetyBoundary = payload.safetyBoundary || [];
  const operatorNotes = payload.operatorNotes || [];

  setBadge(
    "durableArchiveWriteGateBadge",
    payload.archiveWriteEnabled ? "ENABLED" : "DISABLED_BY_DEFAULT",
    payload.archiveWriteEnabled ? "error" : "warning"
  );

  $("durableArchiveWriteGateContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(payload.readOnly)) },
      { label: "Analysis Only", value: formatBool(Boolean(payload.analysisOnly)) },
      {
        label: "Manual Review Required",
        value: formatBool(Boolean(payload.manualReviewRequired)),
      },
      {
        label: "Archive Write Enabled",
        value: formatBool(Boolean(payload.archiveWriteEnabled)),
      },
      { label: "Write Status", value: escapeHtml(payload.writeStatus || "disabled_by_default") },
      { label: "Records Written", value: formatInteger(payload.recordsWritten) },
      { label: "Bytes Written", value: formatInteger(payload.bytesWritten) },
      { label: "DB Write", value: formatBool(Boolean(payload.databaseWriteEnabled)) },
      { label: "JSONL Write", value: formatBool(Boolean(payload.jsonlWriteEnabled)) },
      { label: "SQLite Write", value: formatBool(Boolean(payload.sqliteWriteEnabled)) },
      {
        label: "File Archive Write",
        value: formatBool(Boolean(payload.fileArchiveWriteEnabled)),
      },
      {
        label: "Rejection Reason",
        value: escapeHtml(payload.rejectionReason || "archive_write_disabled_by_default"),
      },
      {
        label: "Dry-run Contract Preserved",
        value: formatBool(Boolean(payload.dryRunContractPreserved)),
      },
      {
        label: "Review Pack Contract Preserved",
        value: formatBool(Boolean(payload.reviewPackContractPreserved)),
      },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshDurableArchiveWriteGateButton">Refresh Write Gate</button>
      <button type="button" class="small-button" id="copyDurableArchiveWriteGateJsonButton">Copy Write Gate JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">${
          safetyBoundary.length
            ? safetyBoundary.map((item) => escapeHtml(item)).join("<br/>")
            : "archiveWriteEnabled=false<br/>recordsWritten=0<br/>bytesWritten=0"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Operator Notes</div>
        <div class="metric-value">${
          operatorNotes.length
            ? operatorNotes.map((note) => escapeHtml(note)).join("<br/>")
            : "This is a disabled-by-default write gate."
        }</div>
      </div>
    </div>` +
    (state.latestDurableArchiveWriteGateAction
      ? `<div class="muted">${escapeHtml(state.latestDurableArchiveWriteGateAction)}</div>`
      : "");
}

function renderDurableArchiveWriteAudit() {
  const statusUrl = durableArchiveWriteAuditStatusUrl();
  const recentUrl = durableArchiveWriteAuditRecentUrl();
  const latestUrl = durableArchiveWriteAuditLatestUrl();
  const statusPayload = getDurableArchiveWriteAuditStatusPayload();
  const recentPayload = getDurableArchiveWriteAuditRecentPayload();
  const latestPayload = getDurableArchiveWriteAuditLatestPayload();
  const error = getError(statusUrl) || getError(recentUrl) || getError(latestUrl);
  if (error) {
    setBadge("durableArchiveWriteAuditBadge", "API Error", "error");
    $("durableArchiveWriteAuditContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!statusPayload && !recentPayload && !latestPayload) {
    setBadge("durableArchiveWriteAuditBadge", "Loading", "none");
    $("durableArchiveWriteAuditContent").innerHTML =
      `<div class="muted">Write audit preview will appear after the preview-only audit contract is loaded.</div>`;
    return;
  }

  const latestAttempt = latestPayload?.attempt || null;
  const attempts = recentPayload?.attempts || [];
  const latestAttemptAvailable =
    Boolean(latestPayload?.latestAttemptAvailable) ||
    Boolean(statusPayload?.latestAttemptAvailable);
  const recentAttemptCount = statusPayload?.recentAttemptCount ?? attempts.length;
  const operatorNote =
    latestPayload?.operatorNote ||
    recentPayload?.operatorNote ||
    "No rejected archive write attempts are currently available in preview memory.";

  setBadge(
    "durableArchiveWriteAuditBadge",
    latestAttemptAvailable ? "PREVIEW_READY" : "PREVIEW_ONLY",
    latestAttemptAvailable ? "warning" : "none"
  );

  $("durableArchiveWriteAuditContent").innerHTML =
    renderMetrics([
      { label: "Audit Mode", value: escapeHtml(statusPayload?.auditMode || "preview_only") },
      {
        label: "Attempt Log Persistence",
        value: formatBool(Boolean(statusPayload?.attemptLogPersistenceEnabled)),
      },
      {
        label: "Attempt Log File Write",
        value: formatBool(Boolean(statusPayload?.attemptLogFileWriteEnabled)),
      },
      {
        label: "Archive Write Enabled",
        value: formatBool(Boolean(statusPayload?.archiveWriteEnabled)),
      },
      {
        label: "Latest Attempt Available",
        value: formatBool(Boolean(latestAttemptAvailable)),
      },
      {
        label: "Recent Attempt Count",
        value: formatInteger(recentAttemptCount),
      },
      {
        label: "Last Rejection Reason",
        value: escapeHtml(
          latestAttempt?.rejectionReason || "archive_write_disabled_by_default"
        ),
      },
      {
        label: "Records Requested",
        value: latestAttempt
          ? formatInteger(latestAttempt.recordsRequested)
          : "Unavailable",
      },
      {
        label: "Records Written",
        value: formatInteger(latestAttempt?.recordsWritten || 0),
      },
      {
        label: "Bytes Written",
        value: formatInteger(latestAttempt?.bytesWritten || 0),
      },
      {
        label: "Notification Sent",
        value: formatBool(Boolean(latestAttempt?.notificationSent)),
      },
      {
        label: "Execution Triggered",
        value: formatBool(Boolean(latestAttempt?.executionTriggered)),
      },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshDurableArchiveWriteAuditButton">Refresh Write Audit</button>
      <button type="button" class="small-button" id="loadLatestDurableArchiveWriteAttemptButton">Load Latest Write Attempt</button>
      <button type="button" class="small-button" id="copyDurableArchiveWriteAuditJsonButton">Copy Write Audit JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Latest Attempt Preview</div>
        <div class="metric-value">${
          latestAttempt
            ? [
                `endpoint=${escapeHtml(latestAttempt.endpoint || "POST /api/archive/write")}`,
                `writeRejected=${escapeHtml(String(Boolean(latestAttempt.writeRejected)))}`,
                `recordsRequested=${escapeHtml(String(latestAttempt.recordsRequested || 0))}`,
                `recordsWritten=${escapeHtml(String(latestAttempt.recordsWritten || 0))}`,
                `bytesWritten=${escapeHtml(String(latestAttempt.bytesWritten || 0))}`,
              ].join("<br/>")
            : escapeHtml(operatorNote)
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Recent Attempts</div>
        <div class="metric-value">${
          attempts.length
            ? attempts
                .map(
                  (attempt) =>
                    `${escapeHtml(attempt.attemptId || "attempt")}<br/>${escapeHtml(
                      attempt.rejectionReason || "archive_write_disabled_by_default"
                    )}`
                )
                .join("<br/><br/>")
            : escapeHtml(operatorNote)
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Summary</div>
        <div class="metric-value">${
          latestAttempt?.safetySummary?.length
            ? latestAttempt.safetySummary.map((item) => escapeHtml(item)).join("<br/>")
            : "No DB/JSONL/SQLite/file write occurred.<br/>No runtime mutation occurred.<br/>No notification or execution was triggered."
        }</div>
      </div>
    </div>` +
    (state.latestDurableArchiveWriteAuditAction
      ? `<div class="muted">${escapeHtml(state.latestDurableArchiveWriteAuditAction)}</div>`
      : "");
}

function getToxicReplayPayload() {
  return getData("/api/toxicity/replay/recent");
}

function getToxicReplayStatusPayload() {
  return getData("/api/toxicity/replay/status");
}

function getToxicMarkoutPayload() {
  return getData("/api/toxicity/markout/recent");
}

function getToxicMarkoutStatusPayload() {
  return getData("/api/toxicity/markout/status");
}

function renderStructuralToxicity() {
  const url = structuralToxicityRecentUrl();
  const statusUrl = structuralToxicityStatusUrl();
  const report = getStructuralToxicityPayload();
  const statusPayload = getStructuralToxicityStatusPayload();
  const error =
    getError(url) ||
    getError(statusUrl) ||
    (!state.structuralToxicitySymbol
      ? getError("/api/toxicity/structural/recent") ||
        getError("/api/toxicity/structural/status")
      : null);
  if (error) {
    setBadge("structuralToxicityBadge", "API Error", "error");
    $("structuralToxicityContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("structuralToxicityBadge", "Loading", "none");
    $("structuralToxicityContent").innerHTML =
      `<div class="muted">Structural toxicity will appear after active trade, liquidation, and wall interpretation evidence loads.</div>`;
    return;
  }

  const signals = report.signals || [];
  setBadge(
    "structuralToxicityBadge",
    (statusPayload?.mode || report.mode || "analysis_only").toUpperCase(),
    signals.length ? "warning" : "none"
  );
  $("structuralToxicityContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      {
        label: "Runtime Modified",
        value: formatBool(Boolean(statusPayload?.runtimeModified ?? report.runtimeModified)),
      },
      { label: "Mode", value: statusPayload?.mode || report.mode || "analysis_only" },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
      { label: "Signal Count", value: formatInteger(statusPayload?.signalCount ?? signals.length) },
      { label: "Last Signal At", value: formatDateTime(statusPayload?.lastSignalAtMs) },
    ]) +
    `<div class="action-row">
      <input id="structuralToxicitySymbolInput" placeholder="symbol" value="${escapeHtml(
        state.structuralToxicitySymbol || report.selectedSymbol || ""
      )}" />
      <button type="button" class="small-button" id="selectStructuralToxicitySymbolButton">Select Symbol</button>
      <button type="button" class="small-button" id="refreshStructuralToxicityButton">Refresh Structural Toxicity</button>
      <button type="button" class="small-button" id="copyStructuralToxicityJsonButton">Copy JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Warnings</div>
        <div class="metric-value">${(report.warnings || []).length ? report.warnings.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">No Trade Reasons</div>
        <div class="metric-value">${(report.noTradeReasons || []).length ? report.noTradeReasons.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Recent Signals</div>
        <div class="metric-value">${
          signals.length
            ? signals
                .map(
                  (signal) => `
            <div class="recommendation-card">
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Signal</div><div class="metric-value">${escapeHtml(signal.signalType || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Direction</div><div class="metric-value">${escapeHtml(signal.direction || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Level Type</div><div class="metric-value">${escapeHtml(signal.levelType || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Confidence</div><div class="metric-value">${escapeHtml(signal.confidence || "Unavailable")}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Level Price</div><div class="metric-value">${formatNumber(signal.levelPrice, 1)}</div></div>
                <div class="metric"><div class="metric-label">Current Price</div><div class="metric-value">${formatNumber(signal.currentPrice, 1)}</div></div>
                <div class="metric"><div class="metric-label">Sweep Distance USD</div><div class="metric-value">${formatNumber(signal.sweepDistanceUsd, 1)}</div></div>
                <div class="metric"><div class="metric-label">Sweep Distance bps</div><div class="metric-value">${formatNumber(signal.sweepDistanceBps, 2)}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Reclaim / Reject</div><div class="metric-value">${formatBool(Boolean(signal.reclaimOrReject))}</div></div>
                <div class="metric"><div class="metric-label">Time Outside ms</div><div class="metric-value">${formatInteger(signal.timeOutsideLevelMs)}</div></div>
                <div class="metric"><div class="metric-label">Toxicity Score</div><div class="metric-value">${formatInteger(signal.toxicityScore)}</div></div>
              </div>
              <div class="metric">
                <div class="metric-label">Linked Evidence</div>
                <div class="metric-value">
                  Active: ${(signal.linkedActiveTradeSignalIds || []).length ? signal.linkedActiveTradeSignalIds.map((item) => escapeHtml(item)).join("<br/>") : "None"}<br/>
                  Liquidation: ${(signal.linkedLiquidationSignalIds || []).length ? signal.linkedLiquidationSignalIds.map((item) => escapeHtml(item)).join("<br/>") : "None"}<br/>
                  Walls: ${(signal.linkedWallSignalIds || []).length ? signal.linkedWallSignalIds.map((item) => escapeHtml(item)).join("<br/>") : "None"}<br/>
                  Interpretation: ${(signal.linkedWallInterpretationSignalIds || []).length ? signal.linkedWallInterpretationSignalIds.map((item) => escapeHtml(item)).join("<br/>") : "None"}
                </div>
              </div>
              <div class="metric">
                <div class="metric-label">Reasons</div>
                <div class="metric-value">${(signal.reason || []).length ? signal.reason.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
              </div>
            </div>`
                )
                .join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          runtimeModified=false<br/>
          analysis_only=true<br/>
          no_live_execution=true
        </div>
      </div>
    </div>` +
    (state.latestStructuralToxicityAction
      ? `<div class="muted">${escapeHtml(state.latestStructuralToxicityAction)}</div>`
      : "");
}

function formatWhaleFlowStatus(status) {
  switch (status) {
    case "candidate_active":
      return "Candidate Active";
    case "data_insufficient":
      return "Data insufficient";
    case "no_whale_flow":
      return "No Whale Flow";
    default:
      return status || "Unavailable";
  }
}

function formatWhaleFlowCandidateType(candidateType) {
  switch (candidateType) {
    case "aggressive_buy":
      return "主动买入";
    case "aggressive_sell":
      return "主动卖出";
    case "absorption":
      return "吸收型";
    case "liquidation_sweep":
      return "清算扫单";
    case "trap":
      return "诱导/陷阱";
    default:
      return candidateType || "Unavailable";
  }
}

function formatWhaleFlowBaselineSource(source) {
  switch (source) {
    case "one_hour_normalized":
      return "one_hour_normalized";
    case "sixty_second_fallback":
      return "sixty_second_fallback";
    case "longer_window_fallback":
      return "longer_window_fallback";
    case "insufficient_history":
      return "insufficient_history";
    default:
      return source || "Unavailable";
  }
}

function formatWhaleFlowQualityStatus(status) {
  switch ((status || "").toLowerCase()) {
    case "healthy":
      return "healthy";
    case "partial":
      return "partial";
    case "degraded":
      return "degraded";
    case "no_data":
      return "no_data";
    default:
      return status || "Unavailable";
  }
}

function whaleFlowQualityTone(status) {
  switch ((status || "").toLowerCase()) {
    case "healthy":
      return "green";
    case "partial":
      return "yellow";
    case "degraded":
      return "orange";
    case "no_data":
      return "gray";
    default:
      return "blue";
  }
}

function renderWhaleFlowMonitor() {
  const url = whaleFlowRecentUrl();
  const statusUrl = whaleFlowStatusUrl();
  const report = getWhaleFlowPayload();
  const statusPayload = getWhaleFlowStatusPayload();
  const error =
    getError(url) ||
    getError(statusUrl) ||
    (!signalSymbolFilterValue()
      ? getError("/api/toxicity/whale-flow/recent") || getError("/api/toxicity/whale-flow/status")
      : null);
  if (error) {
    setBadge("whaleFlowMonitorBadge", "API Error", "error");
    $("whaleFlowMonitorContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("whaleFlowMonitorBadge", "Loading", "none");
    $("whaleFlowMonitorContent").innerHTML =
      `<div class="muted">Whale-flow candidates will appear after the read-only market monitor loads.</div>`;
    return;
  }

  const candidates = report.candidates || [];
  const dataQuality = report.dataQuality || statusPayload?.dataQuality || {};
  const venueCoverage = report.venueCoverage || statusPayload?.venueCoverage || {};
  const baselineQuality = report.baselineQuality || statusPayload?.baselineQuality || {};
  const thresholds = report.thresholds || statusPayload?.thresholds || {};
  const tone =
    report.status === "candidate_active"
      ? "warning"
      : report.status === "data_insufficient"
        ? "error"
        : "none";
  setBadge("whaleFlowMonitorBadge", formatWhaleFlowStatus(report.status), tone);

  const candidateTable = candidates.length
    ? `<div class="table-scroll">
        <table class="whale-flow-table">
          <thead>
            <tr>
              <th>timestamp</th>
              <th>symbol</th>
              <th>window</th>
              <th>volume</th>
              <th>directionBias</th>
              <th>price impact</th>
              <th>depth drop</th>
              <th>candidate type</th>
            </tr>
          </thead>
          <tbody>
            ${candidates
              .map(
                (candidate) => `
              <tr>
                <td>${escapeHtml(formatDateTime(candidate.tsMs))}</td>
                <td>${escapeHtml(candidate.symbol || "Unavailable")}</td>
                <td>${escapeHtml(candidate.window || "Unavailable")}</td>
                <td>${escapeHtml(formatNumber(candidate.volumeBtc, 1))} BTC</td>
                <td>${escapeHtml(formatPercent(candidate.directionBias, 0))}</td>
                <td>${escapeHtml(formatNumber(candidate.priceImpactBps, 2))}</td>
                <td>${escapeHtml(formatPercent(candidate.depthDropRatio, 0))}</td>
                <td>${escapeHtml(formatWhaleFlowCandidateType(candidate.candidateType))}</td>
              </tr>
              <tr>
                <td colspan="8" class="whale-flow-note">${escapeHtml(
                  candidate.primaryReason || "No primary reason"
                )}<br/>
                Data Quality: ${escapeHtml(formatWhaleFlowQualityStatus(candidate.diagnostics?.dataQuality))}<br/>
                Why Candidate: ${escapeHtml((candidate.diagnostics?.whyCandidate || []).join(" | ") || "None")}<br/>
                Missing Inputs: ${escapeHtml((candidate.diagnostics?.missingInputs || []).join(" | ") || "None")}<br/>
                Degradation Reasons: ${escapeHtml((candidate.diagnostics?.degradationReasons || []).join(" | ") || "None")}<br/>
                Confidence Modifiers: ${escapeHtml((candidate.diagnostics?.confidenceModifiers || []).join(" | ") || "None")}
                </td>
              </tr>`
              )
              .join("")}
          </tbody>
        </table>
      </div>`
    : `<div class="whale-flow-empty">${
        report.status === "data_insufficient"
          ? "Data insufficient"
          : "No Whale Flow"
      }</div>`;

  const venueCoverageSummary = `${formatInteger(venueCoverage.activeTradeVenues)} / ${formatInteger(
    venueCoverage.enabledVenues
  )} active`;
  const confluenceSummary = venueCoverage.venueConfluenceSatisfied ? "satisfied" : "not satisfied";
  const dataQualityStrip = `
    <div class="monitor-quality-strip">
      ${renderSignalChip(`Data Quality: ${formatWhaleFlowQualityStatus(dataQuality.status)}`, whaleFlowQualityTone(dataQuality.status))}
      ${renderSignalChip(`Venue Coverage: ${venueCoverageSummary}`, whaleFlowQualityTone(dataQuality.venueCoverageStatus))}
      ${renderSignalChip(`Trade Venues: ${(venueCoverage.venuesWithRecentTrades || []).join(", ") || "Unavailable"}`, "blue")}
      ${renderSignalChip(`Book Venues: ${(venueCoverage.venuesWithRecentBooks || []).join(", ") || "Unavailable"}`, "blue")}
      ${renderSignalChip(`Baseline: ${formatWhaleFlowBaselineSource(baselineQuality.baselineSource)}`, whaleFlowQualityTone(dataQuality.baselineStatus))}
      ${renderSignalChip(`Confluence: ${confluenceSummary}`, venueCoverage.venueConfluenceSatisfied ? "green" : "orange")}
    </div>`;

  $("whaleFlowMonitorContent").innerHTML =
    dataQualityStrip +
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      { label: "Analysis Only", value: formatBool(Boolean(report.analysisOnly)) },
      {
        label: "Execution Enabled",
        value: formatBool(Boolean(statusPayload?.executionEnabled ?? report.executionEnabled)),
      },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: formatWhaleFlowStatus(report.status) },
      {
        label: "Candidate Count",
        value: formatInteger(statusPayload?.candidateCount ?? candidates.length),
      },
      { label: "Lagged Events", value: formatInteger(report.laggedEvents) },
      { label: "Dropped Events", value: formatInteger(report.droppedEvents) },
      { label: "Connected Venues", value: formatInteger(report.connectedVenues) },
      { label: "Baseline Mode", value: formatWhaleFlowBaselineSource(report.historyBaselineMode) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshWhaleFlowMonitorButton">Refresh</button>
      <button type="button" class="small-button" id="copyWhaleFlowMonitorJsonButton">Copy JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Data Quality</div>
        <div class="metric-value">
          Status: ${escapeHtml(formatWhaleFlowQualityStatus(dataQuality.status))}<br/>
          Venue Coverage Status: ${escapeHtml(formatWhaleFlowQualityStatus(dataQuality.venueCoverageStatus))}<br/>
          Baseline Status: ${escapeHtml(dataQuality.baselineStatus || "Unavailable")}<br/>
          Latest Trade Available: ${escapeHtml(formatBool(Boolean(dataQuality.latestTradeAvailable)))}<br/>
          Latest Book Available: ${escapeHtml(formatBool(Boolean(dataQuality.latestBookAvailable)))}<br/>
          Operator Warning: ${escapeHtml(dataQuality.operatorWarning || "None")}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Venue Coverage</div>
        <div class="metric-value">
          configuredVenues=${escapeHtml(formatInteger(venueCoverage.configuredVenues))}<br/>
          enabledVenues=${escapeHtml(formatInteger(venueCoverage.enabledVenues))}<br/>
          connectedVenues=${escapeHtml(formatInteger(venueCoverage.connectedVenues))}<br/>
          activeTradeVenues=${escapeHtml(formatInteger(venueCoverage.activeTradeVenues))}<br/>
          activeBookVenues=${escapeHtml(formatInteger(venueCoverage.activeBookVenues))}<br/>
          venuesWithRecentTrades=${escapeHtml((venueCoverage.venuesWithRecentTrades || []).join(", ") || "None")}<br/>
          venuesWithRecentBooks=${escapeHtml((venueCoverage.venuesWithRecentBooks || []).join(", ") || "None")}<br/>
          venuesMissingTrades=${escapeHtml((venueCoverage.venuesMissingTrades || []).join(", ") || "None")}<br/>
          venuesMissingBooks=${escapeHtml((venueCoverage.venuesMissingBooks || []).join(", ") || "None")}<br/>
          minVenueConfluenceRequired=${escapeHtml(formatInteger(venueCoverage.minVenueConfluenceRequired))}<br/>
          venueConfluenceSatisfied=${escapeHtml(formatBool(Boolean(venueCoverage.venueConfluenceSatisfied)))}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Baseline Source</div>
        <div class="metric-value">
          baselineSource=${escapeHtml(formatWhaleFlowBaselineSource(baselineQuality.baselineSource))}<br/>
          baselineWindowMs=${escapeHtml(formatInteger(baselineQuality.baselineWindowMs))}<br/>
          relativeVolumeMultiple=${escapeHtml(formatNumber(baselineQuality.relativeVolumeMultiple, 2))}<br/>
          fallbackUsed=${escapeHtml(formatBool(Boolean(baselineQuality.fallbackUsed)))}<br/>
          insufficientHistory=${escapeHtml(formatBool(Boolean(baselineQuality.insufficientHistory)))}<br/>
          operatorWarning=${escapeHtml(baselineQuality.operatorWarning || "None")}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Warnings</div>
        <div class="metric-value">${(report.warnings || []).length ? report.warnings.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">No-candidate Reasons</div>
        <div class="metric-value">${(report.noCandidateReasons || []).length ? report.noCandidateReasons.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Degradation Warnings</div>
        <div class="metric-value">${(report.degradationWarnings || []).length ? report.degradationWarnings.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Thresholds</div>
        <div class="metric-value">
          1s=${escapeHtml(formatNumber(thresholds.oneSecondBtc, 0))} BTC<br/>
          5s=${escapeHtml(formatNumber(thresholds.fiveSecondBtc, 0))} BTC<br/>
          15s=${escapeHtml(formatNumber(thresholds.fifteenSecondBtc, 0))} BTC<br/>
          60s=${escapeHtml(formatNumber(thresholds.sixtySecondBtc, 0))} BTC<br/>
          directionRatioMin=${escapeHtml(formatPercent(thresholds.directionRatioMin, 0))}<br/>
          relativeVolumeMultipleMin=${escapeHtml(formatNumber(thresholds.relativeVolumeMultipleMin, 0))}x<br/>
          minVenueConfirmations=${escapeHtml(formatInteger(thresholds.minVenueConfirmations))}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Candidate Windows</div>
        <div class="metric-value">${candidateTable}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          analysisOnly=true<br/>
          executionEnabled=false<br/>
          no order / wallet / signing / live trading<br/>
          no apply / patch / reload / runtime mutation<br/>
          no DB / JSONL / SQLite / archive write
        </div>
      </div>
    </div>` +
    (state.latestWhaleFlowAction
      ? `<div class="muted">${escapeHtml(state.latestWhaleFlowAction)}</div>`
      : "");
}

function whaleFlowCalibrationStatusTone(status) {
  switch ((status || "").toString().toLowerCase()) {
    case "calibration_ready":
      return "warning";
    case "markout_not_enough_data":
    case "not_enough_samples":
    case "current_snapshot_only":
    case "resolved_markout_evidence_too_thin":
    case "not_enough_data_rate_too_high":
      return "orange";
    case "insufficient_history":
    case "no_whale_flow_candidates":
      return "none";
    default:
      return "blue";
  }
}

function whaleFlowCandidateHistoryTone(statusPayload = {}) {
  if (statusPayload.calibrationReady) {
    return "green";
  }
  if (Number(statusPayload.currentCandidates || 0) === 0) {
    return "none";
  }
  return "orange";
}

function formatCalibrationAction(action) {
  const normalized = (action || "").toString().toLowerCase();
  if (!normalized) {
    return "Unavailable";
  }
  return normalized.replaceAll("_", " ");
}

function formatEvidenceGatedCalibrationAction(action, enoughData) {
  if (!enoughData) {
    return "needs more data";
  }
  return formatCalibrationAction(action);
}

function renderWhaleFlowCalibration() {
  const url = whaleFlowCalibrationReportUrl();
  const statusUrl = whaleFlowCalibrationStatusUrl();
  const report = getWhaleFlowCalibrationPayload();
  const statusPayload = getWhaleFlowCalibrationStatusPayload();
  const error =
    getError(url) ||
    getError(statusUrl) ||
    (!whaleFlowCalibrationSelectedSymbol()
      ? getError("/api/toxicity/whale-flow/calibration/report") ||
        getError("/api/toxicity/whale-flow/calibration/status")
      : null);
  if (error) {
    setBadge("whaleFlowCalibrationBadge", "API Error", "error");
    $("whaleFlowCalibrationContent").innerHTML = `<div class="error">${escapeHtml(error)}</div>`;
    return;
  }

  if (!report) {
    setBadge("whaleFlowCalibrationBadge", "Loading", "none");
    $("whaleFlowCalibrationContent").innerHTML =
      `<div class="muted">Whale Flow Threshold Calibration will appear after read-only whale-flow and markout history are available.</div>`;
    return;
  }

  const sampleStatus = report.sampleStatus || {};
  const evidenceSource = report.evidenceSource || {};
  const outcomeLinkage = report.outcomeLinkage || {};
  const enoughCalibrationEvidence = Boolean(sampleStatus.enoughData);
  const thresholds = report.currentThresholds || statusPayload?.currentThresholds || {};
  const thresholdPerformance = report.thresholdPerformance || {};
  const byClassification = report.byClassification || [];
  const venueConfluence = report.venueConfluence || [];
  const baselineSourceQuality = report.baselineSourceQuality || [];
  const manualTuningNotes = report.manualTuningNotes || [];

  setBadge(
    "whaleFlowCalibrationBadge",
    escapeHtml((statusPayload?.status || report.status || "loading").toUpperCase()),
    whaleFlowCalibrationStatusTone(statusPayload?.status || report.status)
  );

  const thresholdCards = [
    ["1s", thresholdPerformance.oneSecondBtc],
    ["5s", thresholdPerformance.fiveSecondBtc],
    ["15s", thresholdPerformance.fifteenSecondBtc],
    ["60s", thresholdPerformance.sixtySecondBtc],
  ]
    .map(
      ([label, item]) => `
        <div class="recommendation-card">
          <div class="metric-grid">
            <div class="metric"><div class="metric-label">Window</div><div class="metric-value">${escapeHtml(label)}</div></div>
            <div class="metric"><div class="metric-label">Threshold</div><div class="metric-value">${formatNumber(item?.threshold, 0)} BTC</div></div>
            <div class="metric"><div class="metric-label">Candidates</div><div class="metric-value">${formatInteger(item?.candidateCount)}</div></div>
            <div class="metric"><div class="metric-label">Verdict</div><div class="metric-value">${escapeHtml(formatEvidenceGatedCalibrationAction(item?.verdict, enoughCalibrationEvidence))}</div></div>
          </div>
          <div class="signal-chip-row">
            ${renderSignalChip(`aligned ${formatPercent(item?.alignedRate, 0)}`, replayHeatmapStatusTone("aligned"))}
            ${renderSignalChip(`adverse ${formatPercent(item?.adverseRate, 0)}`, replayHeatmapStatusTone("adverse"))}
            ${renderSignalChip(`neutral ${formatPercent(item?.neutralRate, 0)}`, replayHeatmapStatusTone("neutral"))}
            ${renderSignalChip(`not_enough_data ${formatPercent(item?.notEnoughDataRate, 0)}`, replayHeatmapStatusTone("not_enough_data"))}
          </div>
        </div>`
    )
    .join("");

  $("whaleFlowCalibrationContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      { label: "Analysis Only", value: formatBool(Boolean(report.analysisOnly)) },
      {
        label: "Execution Enabled",
        value: formatBool(Boolean(statusPayload?.executionEnabled ?? report.executionEnabled)),
      },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: escapeHtml(report.status || "Unavailable") },
      { label: "Total Candidates", value: formatInteger(sampleStatus.totalCandidates) },
      {
        label: "Linked Markout Samples",
        value: formatInteger(sampleStatus.linkedMarkoutSamples),
      },
      {
        label: "Enough Data",
        value: formatBool(Boolean(sampleStatus.enoughData)),
      },
      {
        label: "Resolved Evidence",
        value: formatInteger(sampleStatus.resolvedMarkoutEvidenceCount),
      },
      {
        label: "Evidence Source",
        value: evidenceSource.mode || "Unavailable",
      },
      { label: "Retention Mode", value: sampleStatus.retentionMode || "Unavailable" },
    ]) +
    `<div class="action-row">
      <input id="whaleFlowCalibrationSymbolInput" placeholder="symbol" value="${escapeHtml(
        state.whaleFlowCalibrationSymbol || report.selectedSymbol || signalSymbolFilterValue() || ""
      )}" />
      <button type="button" class="small-button" id="loadWhaleFlowCalibrationBySymbolButton">Load Calibration by Symbol</button>
      <button type="button" class="small-button" id="refreshWhaleFlowCalibrationButton">Refresh Calibration Report</button>
      <button type="button" class="small-button" id="copyWhaleFlowCalibrationJsonButton">Copy Calibration JSON</button>
      <button type="button" class="small-button" id="copyWhaleFlowCalibrationMarkdownButton">Copy Calibration Markdown</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Sample Status</div>
        <div class="metric-value">
          totalCandidates=${escapeHtml(formatInteger(sampleStatus.totalCandidates))}<br/>
          linkedMarkoutSamples=${escapeHtml(formatInteger(sampleStatus.linkedMarkoutSamples))}<br/>
          resolvedMarkoutEvidenceCount=${escapeHtml(formatInteger(sampleStatus.resolvedMarkoutEvidenceCount))}<br/>
          unresolvedMarkoutCount=${escapeHtml(formatInteger(sampleStatus.unresolvedMarkoutCount))}<br/>
          notEnoughDataRate=${escapeHtml(formatPercent(sampleStatus.notEnoughDataRate, 0))}<br/>
          minSamplesRequired=${escapeHtml(formatInteger(sampleStatus.minSamplesRequired))}<br/>
          minResolvedEvidenceRequired=${escapeHtml(formatInteger(sampleStatus.minResolvedEvidenceRequired))}<br/>
          maxNotEnoughDataRateForTuning=${escapeHtml(formatPercent(sampleStatus.maxNotEnoughDataRateForTuning, 0))}<br/>
          enoughData=${escapeHtml(formatBool(Boolean(sampleStatus.enoughData)))}<br/>
          blockedReason=${escapeHtml(sampleStatus.blockedReason || "None")}<br/>
          retentionMode=${escapeHtml(sampleStatus.retentionMode || "Unavailable")}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Evidence Source</div>
        <div class="metric-value">
          mode=${escapeHtml(evidenceSource.mode || "Unavailable")}<br/>
          usesCurrentSnapshotOnly=${escapeHtml(formatBool(Boolean(evidenceSource.usesCurrentSnapshotOnly)))}<br/>
          currentSnapshotFallbackUsed=${escapeHtml(formatBool(Boolean(evidenceSource.currentSnapshotFallbackUsed)))}<br/>
          historySignalsAvailable=${escapeHtml(formatInteger(evidenceSource.historySignalsAvailable))}<br/>
          whaleCandidatesEvaluated=${escapeHtml(formatInteger(evidenceSource.whaleCandidatesEvaluated))}<br/>
          resolvedMarkoutEvidenceCount=${escapeHtml(formatInteger(evidenceSource.resolvedMarkoutEvidenceCount))}<br/>
          unresolvedMarkoutCount=${escapeHtml(formatInteger(evidenceSource.unresolvedMarkoutCount))}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Outcome Linkage</div>
        <div class="metric-value">
          linkedSignalIdMatches=${escapeHtml(formatInteger(outcomeLinkage.linkedSignalIdMatches))}<br/>
          fallbackMatches=${escapeHtml(formatInteger(outcomeLinkage.fallbackMatches))}<br/>
          noOutcomeLinkageCount=${escapeHtml(formatInteger(outcomeLinkage.noOutcomeLinkageCount))}<br/>
          fallbackUsed=${escapeHtml(formatBool(Boolean(outcomeLinkage.fallbackUsed)))}<br/>
          ${(outcomeLinkage.operatorWarnings || []).map((item) => escapeHtml(item)).join("<br/>") || "No linkage warnings"}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Current Thresholds</div>
        <div class="metric-value">
          oneSecondBtc=${escapeHtml(formatNumber(thresholds.oneSecondBtc, 0))}<br/>
          fiveSecondBtc=${escapeHtml(formatNumber(thresholds.fiveSecondBtc, 0))}<br/>
          fifteenSecondBtc=${escapeHtml(formatNumber(thresholds.fifteenSecondBtc, 0))}<br/>
          sixtySecondBtc=${escapeHtml(formatNumber(thresholds.sixtySecondBtc, 0))}<br/>
          directionRatioMin=${escapeHtml(formatPercent(thresholds.directionRatioMin, 0))}<br/>
          relativeVolumeMultipleMin=${escapeHtml(formatNumber(thresholds.relativeVolumeMultipleMin, 1))}x<br/>
          minVenueConfirmations=${escapeHtml(formatInteger(thresholds.minVenueConfirmations))}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Threshold Performance Summary</div>
        <div class="metric-value">${thresholdCards || "No threshold performance samples yet."}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Classification Quality</div>
        <div class="metric-value">${
          byClassification.length
            ? byClassification
                .map(
                  (item) => `
                  <div class="recommendation-card">
                    <div class="metric-grid">
                      <div class="metric"><div class="metric-label">classification</div><div class="metric-value">${escapeHtml(item.classification || "Unavailable")}</div></div>
                      <div class="metric"><div class="metric-label">sampleCount</div><div class="metric-value">${formatInteger(item.sampleCount)}</div></div>
                      <div class="metric"><div class="metric-label">qualityBucket</div><div class="metric-value">${escapeHtml(item.qualityBucket || "Unavailable")}</div></div>
                    </div>
                    <div class="signal-chip-row">
                      ${renderSignalChip(`aligned ${formatPercent(item.alignedRate, 0)}`, replayHeatmapStatusTone("aligned"))}
                      ${renderSignalChip(`adverse ${formatPercent(item.adverseRate, 0)}`, replayHeatmapStatusTone("adverse"))}
                      ${renderSignalChip(`neutral ${formatPercent(item.neutralRate, 0)}`, replayHeatmapStatusTone("neutral"))}
                      ${renderSignalChip(`not_enough_data ${formatPercent(item.notEnoughDataRate, 0)}`, replayHeatmapStatusTone("not_enough_data"))}
                    </div>
                    <div class="muted">${escapeHtml(item.manualTuningNote || "No note")}</div>
                  </div>`
                )
                .join("")
            : "No whale flow candidates available"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Venue Confluence Effect</div>
        <div class="metric-value">${
          venueConfluence.length
            ? venueConfluence
                .map(
                  (item) => `
                  <div class="recommendation-card">
                    <div class="metric-grid">
                      <div class="metric"><div class="metric-label">venueCount</div><div class="metric-value">${formatInteger(item.venueCount)}</div></div>
                      <div class="metric"><div class="metric-label">sampleCount</div><div class="metric-value">${formatInteger(item.sampleCount)}</div></div>
                      <div class="metric"><div class="metric-label">verdict</div><div class="metric-value">${escapeHtml(item.verdict || "Unavailable")}</div></div>
                    </div>
                    <div class="signal-chip-row">
                      ${renderSignalChip(`aligned ${formatPercent(item.alignedRate, 0)}`, replayHeatmapStatusTone("aligned"))}
                      ${renderSignalChip(`adverse ${formatPercent(item.adverseRate, 0)}`, replayHeatmapStatusTone("adverse"))}
                      ${renderSignalChip(`neutral ${formatPercent(item.neutralRate, 0)}`, replayHeatmapStatusTone("neutral"))}
                      ${renderSignalChip(`not_enough_data ${formatPercent(item.notEnoughDataRate, 0)}`, replayHeatmapStatusTone("not_enough_data"))}
                    </div>
                  </div>`
                )
                .join("")
            : "No venue confluence samples"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Baseline Source Effect</div>
        <div class="metric-value">${
          baselineSourceQuality.length
            ? baselineSourceQuality
                .map(
                  (item) => `
                  <div class="recommendation-card">
                    <div class="metric-grid">
                      <div class="metric"><div class="metric-label">baselineSource</div><div class="metric-value">${escapeHtml(item.baselineSource || "Unavailable")}</div></div>
                      <div class="metric"><div class="metric-label">sampleCount</div><div class="metric-value">${formatInteger(item.sampleCount)}</div></div>
                      <div class="metric"><div class="metric-label">qualityBucket</div><div class="metric-value">${escapeHtml(item.qualityBucket || "Unavailable")}</div></div>
                    </div>
                    <div class="signal-chip-row">
                      ${renderSignalChip(`aligned ${formatPercent(item.alignedRate, 0)}`, replayHeatmapStatusTone("aligned"))}
                      ${renderSignalChip(`adverse ${formatPercent(item.adverseRate, 0)}`, replayHeatmapStatusTone("adverse"))}
                      ${renderSignalChip(`neutral ${formatPercent(item.neutralRate, 0)}`, replayHeatmapStatusTone("neutral"))}
                      ${renderSignalChip(`not_enough_data ${formatPercent(item.notEnoughDataRate, 0)}`, replayHeatmapStatusTone("not_enough_data"))}
                    </div>
                    <div class="muted">${escapeHtml(item.manualTuningNote || "No note")}</div>
                  </div>`
                )
                .join("")
            : "Baseline insufficient"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Suggested Manual Tuning Notes</div>
        <div class="metric-value">${
          manualTuningNotes.length
            ? manualTuningNotes
                .map(
                  (note) => `
                  <div class="recommendation-card">
                    <div class="metric-grid">
                      <div class="metric"><div class="metric-label">target</div><div class="metric-value">${escapeHtml(note.target || "Unavailable")}</div></div>
                      <div class="metric"><div class="metric-label">currentValue</div><div class="metric-value">${formatNumber(note.currentValue, 2)}</div></div>
                      <div class="metric"><div class="metric-label">suggestedAction</div><div class="metric-value">${escapeHtml(note.suggestedAction || "Unavailable")}</div></div>
                    </div>
                    <div class="muted">${escapeHtml(note.reason || "No reason")}</div>
                    <div class="signal-chip-row">
                      ${renderSignalChip(`manualReviewRequired ${formatBool(Boolean(note.manualReviewRequired))}`, "warning")}
                      ${renderSignalChip(`autoApplied ${formatBool(Boolean(note.autoApplied))}`, "muted")}
                      ${renderSignalChip(`configModified ${formatBool(Boolean(note.configModified))}`, "muted")}
                    </div>
                  </div>`
                )
                .join("")
            : "Calibration evidence too thin"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Warnings</div>
        <div class="metric-value">${(report.warnings || []).length ? report.warnings.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">No Candidate Reasons</div>
        <div class="metric-value">${(report.noCandidateReasons || []).length ? report.noCandidateReasons.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          analysisOnly=true<br/>
          executionEnabled=false<br/>
          runtimeModified=false<br/>
          manualReviewRequired=true<br/>
          thresholdModified=false<br/>
          configModified=false<br/>
          runtimeThresholdModified=false<br/>
          autoApplyEnabled=false<br/>
          no threshold mutation / config write / apply / reload / execution<br/>
          no DB / JSONL / SQLite / archive write
        </div>
      </div>
    </div>` +
    (state.latestWhaleFlowCalibrationAction
      ? `<div class="muted">${escapeHtml(state.latestWhaleFlowCalibrationAction)}</div>`
      : "");
}

function renderWhaleFlowCandidateHistory() {
  const url = whaleFlowCandidateHistoryRecentUrl();
  const statusUrl = whaleFlowCandidateHistoryStatusUrl();
  const report = getWhaleFlowCandidateHistoryPayload();
  const statusPayload = getWhaleFlowCandidateHistoryStatusPayload();
  const error =
    getError(url) ||
    getError(statusUrl) ||
    (!whaleFlowCandidateHistorySelectedSymbol()
      ? getError("/api/toxicity/whale-flow/history/recent") ||
        getError("/api/toxicity/whale-flow/history/status")
      : null);
  if (error) {
    setBadge("whaleFlowCandidateHistoryBadge", "API Error", "error");
    $("whaleFlowCandidateHistoryContent").innerHTML = `<div class="error">${escapeHtml(error)}</div>`;
    return;
  }

  if (!report || !statusPayload) {
    setBadge("whaleFlowCandidateHistoryBadge", "Loading", "none");
    $("whaleFlowCandidateHistoryContent").innerHTML =
      `<div class="muted">Whale candidate history will appear after bounded in-memory whale candidates are recorded.</div>`;
    return;
  }

  setBadge(
    "whaleFlowCandidateHistoryBadge",
    statusPayload.calibrationReady ? "READY" : "NOT READY",
    whaleFlowCandidateHistoryTone(statusPayload)
  );

  const items = report.items || [];
  const blockedReasons = statusPayload.calibrationBlockedReasons || [];
  const itemTable = items.length
    ? `<div class="table-scroll">
        <table class="whale-flow-table">
          <thead>
            <tr>
              <th>Time</th>
              <th>Symbol</th>
              <th>Classification</th>
              <th>Window</th>
              <th>Volume</th>
              <th>Direction</th>
              <th>Relative</th>
              <th>Markout</th>
            </tr>
          </thead>
          <tbody>
            ${items
              .map(
                (item) => `
                <tr>
                  <td>${escapeHtml(formatDateTime(item.createdAtMs))}</td>
                  <td>${escapeHtml(item.symbol || "Unavailable")}</td>
                  <td>${escapeHtml(formatWhaleFlowCandidateType(item.classification))}</td>
                  <td>${formatInteger(item.windowMs)} ms</td>
                  <td>${formatNumber(item.volumeBtc, 1)} BTC</td>
                  <td>${escapeHtml(item.directionBias || "neutral")}</td>
                  <td>${item.relativeVolumeMultiple == null ? "Unavailable" : `${formatNumber(item.relativeVolumeMultiple, 2)}x`}</td>
                  <td>${escapeHtml(item.markoutStatus || "not_enough_data")}</td>
                </tr>`
              )
              .join("")}
          </tbody>
        </table>
      </div>`
    : `<div class="muted">No whale candidates have been recorded in bounded memory yet.</div>`;

  $("whaleFlowCandidateHistoryContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(statusPayload.readOnly)) },
      { label: "Analysis Only", value: formatBool(Boolean(statusPayload.analysisOnly)) },
      { label: "Execution Enabled", value: formatBool(Boolean(statusPayload.executionEnabled)) },
      { label: "Selected Symbol", value: statusPayload.selectedSymbol || "Unavailable" },
      { label: "Current Candidates", value: formatInteger(statusPayload.currentCandidates) },
      { label: "Max Candidates", value: formatInteger(statusPayload.maxCandidates) },
      { label: "Resolved Evidence", value: formatInteger(statusPayload.resolvedMarkoutEvidenceCount) },
      { label: "Calibration Ready", value: formatBool(Boolean(statusPayload.calibrationReady)) },
    ]) +
    `<div class="action-row">
      <input id="whaleFlowCandidateHistorySymbolInput" placeholder="symbol" value="${escapeHtml(
        state.whaleFlowCandidateHistorySymbol || statusPayload.selectedSymbol || signalSymbolFilterValue() || ""
      )}" />
      <button type="button" class="small-button" id="loadWhaleFlowCandidateHistoryBySymbolButton">Load Whale History by Symbol</button>
      <button type="button" class="small-button" id="refreshWhaleFlowCandidateHistoryButton">Refresh Whale History</button>
      <button type="button" class="small-button" id="copyWhaleFlowCandidateHistoryJsonButton">Copy Whale History JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">History Status</div>
        <div class="metric-value">
          retentionMode=${escapeHtml(statusPayload.retentionMode || "Unavailable")}<br/>
          durableStorageEnabled=${escapeHtml(formatBool(Boolean(statusPayload.durableStorageEnabled)))}<br/>
          databaseWriteEnabled=${escapeHtml(formatBool(Boolean(statusPayload.databaseWriteEnabled)))}<br/>
          jsonlWriteEnabled=${escapeHtml(formatBool(Boolean(statusPayload.jsonlWriteEnabled)))}<br/>
          sqliteWriteEnabled=${escapeHtml(formatBool(Boolean(statusPayload.sqliteWriteEnabled)))}<br/>
          archiveWriteEnabled=${escapeHtml(formatBool(Boolean(statusPayload.archiveWriteEnabled)))}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Capacity / Timing</div>
        <div class="metric-value">
          currentCandidates=${escapeHtml(formatInteger(statusPayload.currentCandidates))}<br/>
          maxCandidates=${escapeHtml(formatInteger(statusPayload.maxCandidates))}<br/>
          oldestCandidate=${escapeHtml(formatDateTime(statusPayload.oldestCandidateAtMs))}<br/>
          latestCandidate=${escapeHtml(formatDateTime(statusPayload.latestCandidateAtMs))}<br/>
          recordedCount=${escapeHtml(formatInteger(statusPayload.recordedCount))}<br/>
          deduplicatedCount=${escapeHtml(formatInteger(statusPayload.deduplicatedCount))}<br/>
          evictedCount=${escapeHtml(formatInteger(statusPayload.evictedCount))}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Calibration Readiness</div>
        <div class="metric-value">
          resolvedMarkoutEvidenceCount=${escapeHtml(formatInteger(statusPayload.resolvedMarkoutEvidenceCount))}<br/>
          unresolvedCandidateCount=${escapeHtml(formatInteger(statusPayload.unresolvedCandidateCount))}<br/>
          notEnoughDataCount=${escapeHtml(formatInteger(statusPayload.notEnoughDataCount))}<br/>
          minCandidatesRequired=${escapeHtml(formatInteger(statusPayload.minCandidatesRequired))}<br/>
          minResolvedEvidenceRequired=${escapeHtml(formatInteger(statusPayload.minResolvedEvidenceRequired))}<br/>
          maxNotEnoughDataRateForTuning=${escapeHtml(formatPercent(statusPayload.maxNotEnoughDataRateForTuning, 0))}<br/>
          calibrationReady=${escapeHtml(formatBool(Boolean(statusPayload.calibrationReady)))}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Blocked Reasons</div>
        <div class="metric-value">${
          blockedReasons.length
            ? `Calibration not ready<br/>Reason: ${blockedReasons.map((item) => escapeHtml(item)).join("<br/>Reason: ")}`
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Operator Notes</div>
        <div class="metric-value">${
          (statusPayload.operatorNotes || []).length
            ? statusPayload.operatorNotes.map((item) => escapeHtml(item)).join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Recent Candidate History</div>
        <div class="metric-value">${itemTable}</div>
      </div>
    </div>` +
    (state.latestWhaleFlowCandidateHistoryAction
      ? `<div class="muted">${escapeHtml(state.latestWhaleFlowCandidateHistoryAction)}</div>`
      : "");
}

function renderToxicSignalFusion() {
  const url = toxicSignalFusionRecentUrl();
  const statusUrl = toxicSignalFusionStatusUrl();
  const report = getToxicSignalFusionPayload();
  const statusPayload = getToxicSignalFusionStatusPayload();
  const error =
    getError(url) ||
    getError(statusUrl) ||
    (!state.toxicSignalFusionSymbol
      ? getError("/api/toxicity/fusion/recent") || getError("/api/toxicity/fusion/status")
      : null);
  if (error) {
    setBadge("toxicSignalFusionBadge", "API Error", "error");
    $("toxicSignalFusionContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicSignalFusionBadge", "Loading", "none");
    $("toxicSignalFusionContent").innerHTML =
      `<div class="muted">Toxic signal fusion will appear after active trade, liquidation, wall, and structural evidence loads.</div>`;
    return;
  }

  const signals = report.signals || [];
  setBadge(
    "toxicSignalFusionBadge",
    (statusPayload?.mode || report.mode || "analysis_only").toUpperCase(),
    signals.length ? "warning" : "none"
  );
  $("toxicSignalFusionContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      {
        label: "Runtime Modified",
        value: formatBool(Boolean(statusPayload?.runtimeModified ?? report.runtimeModified)),
      },
      { label: "Mode", value: statusPayload?.mode || report.mode || "analysis_only" },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
      { label: "Signal Count", value: formatInteger(statusPayload?.signalCount ?? signals.length) },
      { label: "Last Signal At", value: formatDateTime(statusPayload?.lastSignalAtMs) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicSignalFusionButton">Refresh Toxic Fusion</button>
      <button type="button" class="small-button" id="copyToxicSignalFusionJsonButton">Copy Fusion JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Warnings</div>
        <div class="metric-value">${(report.warnings || []).length ? report.warnings.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">No Trade Reasons</div>
        <div class="metric-value">${(report.noTradeReasons || []).length ? report.noTradeReasons.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Recent Fusion Signals</div>
        <div class="metric-value">${
          signals.length
            ? signals
                .map(
                  (signal) => `
            <div class="recommendation-card">
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Signal</div><div class="metric-value">${escapeHtml(signal.signalType || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Direction</div><div class="metric-value">${escapeHtml(signal.direction || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Confidence</div><div class="metric-value">${escapeHtml(signal.confidence || "Unavailable")}</div></div>
                <div class="metric"><div class="metric-label">Chase Risk</div><div class="metric-value">${escapeHtml(signal.chaseRisk || "Unavailable")}</div></div>
              </div>
              <div class="metric-grid">
                <div class="metric"><div class="metric-label">Toxicity Score</div><div class="metric-value">${formatInteger(signal.toxicityScore)}</div></div>
                <div class="metric"><div class="metric-label">Invalidation Price</div><div class="metric-value">${formatNumber(signal.invalidationPrice, 1)}</div></div>
                <div class="metric"><div class="metric-label">Suggested Stop Distance USD</div><div class="metric-value">${formatNumber(signal.suggestedStopDistanceUsd, 1)}</div></div>
              </div>
              <div class="metric">
                <div class="metric-label">Primary Reason</div>
                <div class="metric-value">${escapeHtml(signal.primaryReason || "Unavailable")}</div>
              </div>
              <div class="metric">
                <div class="metric-label">Supporting Evidence</div>
                <div class="metric-value">${
                  (signal.supportingEvidence || []).length
                    ? signal.supportingEvidence
                        .map(
                          (item) =>
                            `${escapeHtml(item.source || "source")} / ${escapeHtml(item.signalType || "signal")} (${formatInteger(item.contributionScore)}): ${escapeHtml(item.summary || "Unavailable")}`
                        )
                        .join("<br/>")
                    : "None"
                }</div>
              </div>
              <div class="metric">
                <div class="metric-label">No Trade Reasons</div>
                <div class="metric-value">${(signal.noTradeReasons || []).length ? signal.noTradeReasons.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
              </div>
              <div class="metric">
                <div class="metric-label">Reasons</div>
                <div class="metric-value">${(signal.reason || []).length ? signal.reason.map((item) => escapeHtml(item)).join("<br/>") : "None"}</div>
              </div>
            </div>`
                )
                .join("")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          runtimeModified=false<br/>
          analysis_only=true<br/>
          no_live_execution=true
        </div>
      </div>
    </div>` +
    (state.latestToxicSignalFusionAction
      ? `<div class="muted">${escapeHtml(state.latestToxicSignalFusionAction)}</div>`
      : "");
}

function renderToxicReplay() {
  const report = getToxicReplayPayload();
  const statusPayload = getToxicReplayStatusPayload();
  const error =
    getError("/api/toxicity/replay/recent") || getError("/api/toxicity/replay/status");
  if (error) {
    setBadge("toxicReplayBadge", "API Error", "error");
    $("toxicReplayContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicReplayBadge", "Loading", "none");
    $("toxicReplayContent").innerHTML =
      `<div class="muted">Toxic replay evidence will appear after the fused toxicity layer loads.</div>`;
    return;
  }

  const signals = report.signals || [];
  const detail = state.toxicReplayDetail?.replay || null;
  setBadge(
    "toxicReplayBadge",
    (statusPayload?.mode || report.mode || "analysis_only").toUpperCase(),
    signals.length ? "warning" : "none"
  );
  $("toxicReplayContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      {
        label: "Runtime Modified",
        value: formatBool(Boolean(statusPayload?.runtimeModified ?? report.runtimeModified)),
      },
      { label: "Mode", value: statusPayload?.mode || report.mode || "analysis_only" },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
      { label: "Signal Count", value: formatInteger(statusPayload?.signalCount ?? signals.length) },
      { label: "Last Signal At", value: formatDateTime(statusPayload?.lastSignalAtMs) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicReplayButton">Refresh Replay</button>
      <button type="button" class="small-button" id="loadLatestToxicReplayButton">Load Latest Signal</button>
      <button type="button" class="small-button" id="copyToxicReplayJsonButton">Copy Replay JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Recent Signals</div>
        <div class="metric-value">${
          signals.length
            ? signals
                .map(
                  (signal) =>
                    `${escapeHtml(signal.signalKind || "Unavailable")} score context ${escapeHtml(signal.severity || "Unavailable")} confidence ${formatNumber(signal.confidence, 2)}<br/>${escapeHtml(signal.primaryReason || "Unavailable")}`
                )
                .join("<br/><br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Replay Detail</div>
        <div class="metric-value">${
          detail
            ? `
              Signal: ${escapeHtml(detail.signalKind || "Unavailable")}<br/>
              Confidence: ${formatNumber(detail.confidence, 2)}<br/>
              Severity: ${escapeHtml(detail.severity || "Unavailable")}<br/>
              Source Signal: ${escapeHtml(detail.sourceSignal?.signalType || "Unavailable")}<br/>
              Why Signal Fired:<br/>${(detail.operatorNarrative?.whySignalFired || []).map((item) => escapeHtml(item)).join("<br/>") || "None"}<br/><br/>
              Supporting Evidence:<br/>${(detail.operatorNarrative?.supportingEvidence || []).map((item) => escapeHtml(item)).join("<br/>") || "None"}<br/><br/>
              Conflicting Evidence:<br/>${(detail.operatorNarrative?.conflictingEvidence || []).map((item) => escapeHtml(item)).join("<br/>") || "None"}<br/><br/>
              Why Not Entry Signal:<br/>${(detail.operatorNarrative?.whyNotEntrySignal || []).map((item) => escapeHtml(item)).join("<br/>") || "None"}<br/><br/>
              Risk Warnings:<br/>${(detail.operatorNarrative?.riskWarnings || []).map((item) => escapeHtml(item)).join("<br/>") || "None"}<br/><br/>
              Evidence Counts: activeTrade=${formatInteger(detail.evidenceBreakdown?.activeTrade?.length || 0)}, liquidation=${formatInteger(detail.evidenceBreakdown?.liquidation?.length || 0)}, orderbook=${formatInteger(detail.evidenceBreakdown?.orderbook?.length || 0)}, wallInterpretation=${formatInteger(detail.evidenceBreakdown?.wallInterpretation?.length || 0)}, structural=${formatInteger(detail.evidenceBreakdown?.structural?.length || 0)}<br/><br/>
              Reference Levels: invalidation=${formatNumber(detail.referenceLevels?.invalidationPrice, 1)}, stopDistanceUsd=${formatNumber(detail.referenceLevels?.suggestedStopDistanceUsd, 1)}<br/>
              ${escapeHtml(detail.referenceLevels?.wording || "Reference only. No order instruction.")}
            `
            : escapeHtml(state.toxicReplayDetail?.reason || "Load the latest fused signal to inspect evidence breakdown.")
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          runtimeModified=false<br/>
          analysis_only=true<br/>
          no_live_execution=true
        </div>
      </div>
    </div>` +
    (state.latestToxicReplayAction
      ? `<div class="muted">${escapeHtml(state.latestToxicReplayAction)}</div>`
      : "");
}

function renderToxicMarkout() {
  const report = getToxicMarkoutPayload();
  const statusPayload = getToxicMarkoutStatusPayload();
  const error =
    getError("/api/toxicity/markout/recent") || getError("/api/toxicity/markout/status");
  if (error) {
    setBadge("toxicMarkoutBadge", "API Error", "error");
    $("toxicMarkoutContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicMarkoutBadge", "Loading", "none");
    $("toxicMarkoutContent").innerHTML =
      `<div class="muted">Toxic markout evaluation will appear after fused toxicity signals are available.</div>`;
    return;
  }

  const signals = report.signals || [];
  setBadge(
    "toxicMarkoutBadge",
    (statusPayload?.mode || report.mode || "analysis_only").toUpperCase(),
    signals.length ? "warning" : "none"
  );
  $("toxicMarkoutContent").innerHTML =
    renderMetrics([
      { label: "Read Only", value: formatBool(Boolean(report.readOnly)) },
      {
        label: "Runtime Modified",
        value: formatBool(Boolean(statusPayload?.runtimeModified ?? report.runtimeModified)),
      },
      { label: "Mode", value: statusPayload?.mode || report.mode || "analysis_only" },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
      { label: "Signal Count", value: formatInteger(statusPayload?.signalCount ?? signals.length) },
      { label: "Last Signal At", value: formatDateTime(statusPayload?.lastSignalAtMs) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicMarkoutButton">Refresh Markout</button>
      <button type="button" class="small-button" id="copyToxicMarkoutJsonButton">Copy Markout JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Recent Markout Signals</div>
        <div class="metric-value">${
          signals.length
            ? signals
                .map(
                  (signal) => `
                    ${escapeHtml(signal.signalKind || "Unavailable")} outcome ${escapeHtml(signal.overallOutcome || "Unavailable")} confidence ${escapeHtml(signal.confidence || "Unavailable")}<br/>
                    Windows: ${(signal.windows || [])
                      .map(
                        (window) =>
                          `${escapeHtml(window.label || "Window")}=${escapeHtml(window.outcome || "Unavailable")} (${formatNumber(window.markoutBps, 2)} bps)`
                      )
                      .join(", ")}<br/>
                    No-trade reasons: ${(signal.noTradeReasons || []).length ? signal.noTradeReasons.map((item) => escapeHtml(item)).join("; ") : "None"}`
                )
                .join("<br/><br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          runtimeModified=false<br/>
          analysis_only=true<br/>
          no_live_execution=true
        </div>
      </div>
    </div>` +
    (state.latestToxicMarkoutAction
      ? `<div class="muted">${escapeHtml(state.latestToxicMarkoutAction)}</div>`
      : "");
}

function getToxicQualityScorecardPayload() {
  return getData("/api/toxicity/quality-scorecard/summary");
}

function getToxicQualityScorecardStatusPayload() {
  return getData("/api/toxicity/quality-scorecard/status");
}

function renderToxicQualityScorecard() {
  const report = getToxicQualityScorecardPayload();
  const statusPayload = getToxicQualityScorecardStatusPayload();
  const error =
    getError("/api/toxicity/quality-scorecard/summary") ||
    getError("/api/toxicity/quality-scorecard/status");
  if (error) {
    setBadge("toxicQualityScorecardBadge", "API Error", "error");
    $("toxicQualityScorecardContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicQualityScorecardBadge", "Loading", "none");
    $("toxicQualityScorecardContent").innerHTML =
      `<div class="muted">Toxic quality scorecard will appear after markout evaluation data is available.</div>`;
    return;
  }

  setBadge(
    "toxicQualityScorecardBadge",
    (statusPayload?.mode || report.mode || "analysis_only").toUpperCase(),
    report.totalEvaluations ? "warning" : "none"
  );

  const bySignalType = report.bySignalType || [];
  const byWindow = report.byWindow || [];
  const downgradeCandidates = report.downgradeCandidates || [];
  const noTradeCandidates = report.noTradeCandidates || [];

  $("toxicQualityScorecardContent").innerHTML =
    renderMetrics([
      { label: "Total Evaluations", value: formatInteger(report.totalEvaluations) },
      { label: "Aligned Ratio", value: formatNumber((report.alignedRatio || 0) * 100, 2) + "%" },
      { label: "Adverse Ratio", value: formatNumber((report.adverseRatio || 0) * 100, 2) + "%" },
      { label: "Neutral Ratio", value: formatNumber((report.neutralRatio || 0) * 100, 2) + "%" },
      {
        label: "Not Enough Data Ratio",
        value: formatNumber((report.notEnoughDataRatio || 0) * 100, 2) + "%",
      },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicQualityScorecardButton">Refresh Quality Scorecard</button>
      <button type="button" class="small-button" id="copyToxicQualityScorecardJsonButton">Copy Scorecard JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">By Signal Type</div>
        <div class="metric-value">${
          bySignalType.length
            ? bySignalType
                .map(
                  (item) =>
                    `${escapeHtml(item.label || item.key || "Unavailable")}: aligned ${formatNumber((item.alignedRatio || 0) * 100, 2)}%, adverse ${formatNumber((item.adverseRatio || 0) * 100, 2)}%, neutral ${formatNumber((item.neutralRatio || 0) * 100, 2)}%, not_enough_data ${formatNumber((item.notEnoughDataRatio || 0) * 100, 2)}%`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Window</div>
        <div class="metric-value">${
          byWindow.length
            ? byWindow
                .map(
                  (item) =>
                    `${escapeHtml(item.label || item.key || "Unavailable")}: aligned ${formatNumber((item.alignedRatio || 0) * 100, 2)}%, adverse ${formatNumber((item.adverseRatio || 0) * 100, 2)}%, neutral ${formatNumber((item.neutralRatio || 0) * 100, 2)}%`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Downgrade Candidates</div>
        <div class="metric-value">${
          downgradeCandidates.length
            ? downgradeCandidates
                .map(
                  (item) =>
                    `${escapeHtml(item.label || item.key || "Unavailable")}: ${escapeHtml(item.reason || "Unavailable")}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">No-trade Candidates</div>
        <div class="metric-value">${
          noTradeCandidates.length
            ? noTradeCandidates
                .map(
                  (item) =>
                    `${escapeHtml(item.label || item.key || "Unavailable")}: ${escapeHtml(item.reason || "Unavailable")}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          runtimeModified=false<br/>
          analysis_only=true<br/>
          no_live_execution=true
        </div>
      </div>
    </div>` +
    (state.latestToxicQualityScorecardAction
      ? `<div class="muted">${escapeHtml(state.latestToxicQualityScorecardAction)}</div>`
      : "");
}

function getToxicWeightRecommendationPayload() {
  return getData("/api/toxicity/weight-recommendation/summary");
}

function getToxicWeightRecommendationStatusPayload() {
  return getData("/api/toxicity/weight-recommendation/status");
}

function renderToxicWeightRecommendation() {
  const report = getToxicWeightRecommendationPayload();
  const statusPayload = getToxicWeightRecommendationStatusPayload();
  const error =
    getError("/api/toxicity/weight-recommendation/summary") ||
    getError("/api/toxicity/weight-recommendation/status");
  if (error) {
    setBadge("toxicWeightRecommendationBadge", "API Error", "error");
    $("toxicWeightRecommendationContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicWeightRecommendationBadge", "Loading", "none");
    $("toxicWeightRecommendationContent").innerHTML =
      `<div class="muted">Toxic weight recommendations will appear after quality scorecard data is available.</div>`;
    return;
  }

  setBadge(
    "toxicWeightRecommendationBadge",
    (statusPayload?.mode || report.mode || "analysis_only").toUpperCase(),
    report.recommendations?.length ? "warning" : "none"
  );

  const recommendations = report.recommendations || [];
  const bySignalType = report.bySignalType || [];
  const bySymbol = report.bySymbol || [];
  const reviewFlags = report.reviewFlags || [];

  $("toxicWeightRecommendationContent").innerHTML =
    renderMetrics([
      { label: "Total Recommendations", value: formatInteger(report.totalRecommendations) },
      { label: "Keep", value: formatInteger(report.keepCount) },
      {
        label: "Upgrade Candidates",
        value: formatInteger(report.slightUpgradeCandidateCount),
      },
      {
        label: "Downgrade Candidates",
        value: formatInteger(
          (report.slightDowngradeCandidateCount || 0) + (report.downgradeCandidateCount || 0)
        ),
      },
      {
        label: "No-trade Only",
        value: formatInteger(report.noTradeOnlyCandidateCount),
      },
      {
        label: "Disable Candidates",
        value: formatInteger(report.disableCandidateCount),
      },
      {
        label: "Insufficient Data",
        value: formatInteger(report.insufficientDataCount),
      },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicWeightRecommendationButton">Refresh Weight Recommendations</button>
      <button type="button" class="small-button" id="copyToxicWeightRecommendationJsonButton">Copy Recommendation JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">By Signal Type</div>
        <div class="metric-value">${
          bySignalType.length
            ? bySignalType
                .map(
                  (item) =>
                    `${escapeHtml(item.signalType || "Unavailable")}: ${escapeHtml(item.recommendation || "Unavailable")}, aligned ${formatNumber((item.alignedRatio || 0) * 100, 2)}%, adverse ${formatNumber((item.adverseRatio || 0) * 100, 2)}%, samples ${formatInteger(item.sampleCount)}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Symbol</div>
        <div class="metric-value">${
          bySymbol.length
            ? bySymbol
                .map(
                  (item) =>
                    `${escapeHtml(item.symbol || "Unavailable")}: total ${formatInteger(item.totalRecommendations)}, keep ${formatInteger(item.keepCount)}, downgrade ${formatInteger(item.downgradeCandidateCount)}, no-trade ${formatInteger(item.noTradeOnlyCandidateCount)}, disable ${formatInteger(item.disableCandidateCount)}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Recommendations</div>
        <div class="metric-value">${
          recommendations.length
            ? recommendations
                .map(
                  (item) =>
                    `${escapeHtml(item.signalType || "Unavailable")}: ${escapeHtml(item.recommendation || "Unavailable")}, confidence ${escapeHtml(item.confidence || "Unavailable")}, aligned ${formatNumber((item.alignedRatio || 0) * 100, 2)}%, adverse ${formatNumber((item.adverseRatio || 0) * 100, 2)}%, samples ${formatInteger(item.sampleCount)}, review ${formatBool(Boolean(item.manualReviewRequired))}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Review Flags</div>
        <div class="metric-value">${
          reviewFlags.length
            ? reviewFlags
                .map(
                  (item) =>
                    `${escapeHtml(item.reviewFlag || "Unavailable")}: count ${formatInteger(item.count)}, severity ${escapeHtml(item.severity || "Unavailable")}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          analysisOnly=true<br/>
          runtimeModified=false<br/>
          runtimeWeightModified=false<br/>
          configModified=false<br/>
          no_auto_weight_update=true
        </div>
      </div>
    </div>` +
    (state.latestToxicWeightRecommendationAction
      ? `<div class="muted">${escapeHtml(state.latestToxicWeightRecommendationAction)}</div>`
      : "");
}

function getToxicWeightReviewPayload() {
  return getData("/api/toxicity/weight-review/latest");
}

function getToxicWeightReviewStatusPayload() {
  return getData("/api/toxicity/weight-review/status");
}

function renderToxicWeightReview() {
  const report = getToxicWeightReviewPayload();
  const statusPayload = getToxicWeightReviewStatusPayload();
  const error =
    getError("/api/toxicity/weight-review/latest") ||
    getError("/api/toxicity/weight-review/status");
  if (error) {
    setBadge("toxicWeightReviewBadge", "API Error", "error");
    $("toxicWeightReviewContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicWeightReviewBadge", "Loading", "none");
    $("toxicWeightReviewContent").innerHTML =
      `<div class="muted">Manual weight review export will appear after weight recommendations are available.</div>`;
    return;
  }

  setBadge(
    "toxicWeightReviewBadge",
    (statusPayload?.mode || report.mode || "review_export_only").toUpperCase(),
    report.reviewItems?.length ? "warning" : "none"
  );

  const reviewItems = report.reviewItems || [];
  const bySymbol = report.bySymbol || [];
  const governanceNotes = report.governanceNotes || [];

  $("toxicWeightReviewContent").innerHTML =
    renderMetrics([
      { label: "Total Review Items", value: formatInteger(report.totalItems) },
      {
        label: "Manual Review Required",
        value: formatInteger(report.manualReviewRequiredCount),
      },
      { label: "Keep", value: formatInteger(report.keepCount) },
      { label: "Upgrade Candidates", value: formatInteger(report.upgradeCandidateCount) },
      { label: "Downgrade Candidates", value: formatInteger(report.downgradeCandidateCount) },
      { label: "No-trade Only", value: formatInteger(report.noTradeOnlyCount) },
      { label: "Disable Candidates", value: formatInteger(report.disableCandidateCount) },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicWeightReviewButton">Refresh Weight Review</button>
      <button type="button" class="small-button" id="copyToxicWeightReviewJsonButton">Copy Review JSON</button>
      <button type="button" class="small-button" id="copyToxicWeightReviewMarkdownButton">Copy Markdown Report</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">By Symbol</div>
        <div class="metric-value">${
          bySymbol.length
            ? bySymbol
                .map(
                  (item) =>
                    `${escapeHtml(item.symbol || "Unavailable")}: total ${formatInteger(item.totalItems)}, keep ${formatInteger(item.keepCount)}, downgrade ${formatInteger(item.downgradeCandidateCount)}, no-trade ${formatInteger(item.noTradeOnlyCount)}, disable ${formatInteger(item.disableCandidateCount)}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Governance Notes</div>
        <div class="metric-value">${
          governanceNotes.length
            ? governanceNotes.map((note) => escapeHtml(note)).join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Review Items</div>
        <div class="metric-value">${
          reviewItems.length
            ? reviewItems
                .map(
                  (item) =>
                    `${escapeHtml(item.signalType || "Unavailable")}: Recommended Action ${escapeHtml(item.recommendedAction || "Unavailable")}, Confidence ${escapeHtml(item.confidence || "Unavailable")}, aligned ${formatNumber((item.alignedRatio || 0) * 100, 2)}%, adverse ${formatNumber((item.adverseRatio || 0) * 100, 2)}%, samples ${formatInteger(item.sampleCount)}, review ${formatBool(Boolean(item.manualReviewRequired))}, exportOnly ${formatBool(Boolean(item.exportOnly))}<br/>Reason Codes: ${escapeHtml((item.reasonCodes || []).join(", ") || "None")}<br/>Evidence Summary: ${escapeHtml((item.evidenceSummary || []).join("; ") || "None")}`
                )
                .join("<br/><br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          analysisOnly=true<br/>
          exportOnly=true<br/>
          runtimeModified=false<br/>
          runtimeWeightModified=false<br/>
          configModified=false<br/>
          autoApplyEnabled=false
        </div>
      </div>
    </div>` +
    (state.latestToxicWeightReviewAction
      ? `<div class="muted">${escapeHtml(state.latestToxicWeightReviewAction)}</div>`
      : "");
}

function getToxicGovernanceLedgerPayload() {
  return getData("/api/toxicity/governance-ledger/recent");
}

function getToxicGovernanceLedgerStatusPayload() {
  return getData("/api/toxicity/governance-ledger/status");
}

function renderToxicGovernanceLedger() {
  const report = getToxicGovernanceLedgerPayload();
  const statusPayload = getToxicGovernanceLedgerStatusPayload();
  const error =
    getError("/api/toxicity/governance-ledger/recent") ||
    getError("/api/toxicity/governance-ledger/status");
  if (error) {
    setBadge("toxicGovernanceLedgerBadge", "API Error", "error");
    $("toxicGovernanceLedgerContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicGovernanceLedgerBadge", "Loading", "none");
    $("toxicGovernanceLedgerContent").innerHTML =
      `<div class="muted">Governance ledger will appear after manual review decisions are recorded.</div>`;
    return;
  }

  setBadge(
    "toxicGovernanceLedgerBadge",
    (statusPayload?.mode || report.mode || "governance_ledger_only").toUpperCase(),
    report.totalDecisions ? "warning" : "none"
  );

  const bySymbol = report.bySymbol || [];
  const bySignalType = report.bySignalType || [];
  const decisions = report.decisions || [];
  const recentNotes = report.recentGovernanceNotes || [];

  $("toxicGovernanceLedgerContent").innerHTML =
    renderMetrics([
      { label: "Total Decisions", value: formatInteger(report.totalDecisions) },
      { label: "Accepted Recommendations", value: formatInteger(report.acceptCount) },
      { label: "Rejected Recommendations", value: formatInteger(report.rejectCount) },
      { label: "Watch More", value: formatInteger(report.watchMoreCount) },
      { label: "Needs More Samples", value: formatInteger(report.needsMoreSamplesCount) },
      { label: "Suppressed For Now", value: formatInteger(report.suppressForNowCount) },
      { label: "Escalated Review", value: formatInteger(report.escalateReviewCount) },
      { label: "Consensus", value: report.consensusStatus || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicGovernanceLedgerButton">Refresh Governance Ledger</button>
      <button type="button" class="small-button" id="copyToxicGovernanceLedgerJsonButton">Copy Ledger JSON</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">By Signal Type</div>
        <div class="metric-value">${
          bySignalType.length
            ? bySignalType
                .map(
                  (item) =>
                    `${escapeHtml(item.signalType || "Unavailable")}: total ${formatInteger(item.totalDecisions)}, accept ${formatInteger(item.acceptCount)}, reject ${formatInteger(item.rejectCount)}, watch ${formatInteger(item.watchMoreCount)}, suppress ${formatInteger(item.suppressForNowCount)}, consensus ${escapeHtml(item.consensusStatus || "Unavailable")}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Symbol</div>
        <div class="metric-value">${
          bySymbol.length
            ? bySymbol
                .map(
                  (item) =>
                    `${escapeHtml(item.symbol || "Unavailable")}: total ${formatInteger(item.totalDecisions)}, accept ${formatInteger(item.acceptCount)}, reject ${formatInteger(item.rejectCount)}, watch ${formatInteger(item.watchMoreCount)}, suppress ${formatInteger(item.suppressForNowCount)}, consensus ${escapeHtml(item.consensusStatus || "Unavailable")}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Recent Governance Notes</div>
        <div class="metric-value">${
          recentNotes.length
            ? recentNotes.map((note) => escapeHtml(note)).join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Decisions</div>
        <div class="metric-value">${
          decisions.length
            ? decisions
                .map(
                  (item) =>
                    `${escapeHtml(item.symbol || "Unavailable")} / ${escapeHtml(item.signalType || "Unavailable")}: recommendation ${escapeHtml(item.recommendation || "Unavailable")}, decision ${escapeHtml(item.decision || "Unavailable")}, reviewer ${escapeHtml(item.reviewer || "Unavailable")}, confidence ${formatNumber(item.confidence || 0, 2)}, manual ledger only ${formatBool(Boolean(item.governanceLedgerOnly))}<br/>Reason: ${escapeHtml(item.reason || "None")}<br/>Notes: ${escapeHtml(item.notes || "None")}<br/>Evidence Summary: ${escapeHtml((item.evidenceSummary || []).join("; ") || "None")}`
                )
                .join("<br/><br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          analysisOnly=true<br/>
          governanceLedgerOnly=true<br/>
          runtimeWeightModified=false<br/>
          configModified=false<br/>
          runtimeModified=false<br/>
          autoApplyEnabled=false<br/>
          strategyReloaded=false<br/>
          No automatic weight update<br/>
          No runtime config mutation
        </div>
      </div>
    </div>` +
    (state.latestToxicGovernanceLedgerAction
      ? `<div class="muted">${escapeHtml(state.latestToxicGovernanceLedgerAction)}</div>`
      : "");
}

function getToxicGovernanceProposalPayload() {
  return getData("/api/toxicity/governance-proposal/summary");
}

function getToxicGovernanceProposalStatusPayload() {
  return getData("/api/toxicity/governance-proposal/status");
}

function getToxicGovernanceReviewPackPayload() {
  return getData("/api/toxicity/governance-review-pack/summary");
}

function getToxicGovernanceReviewPackStatusPayload() {
  return getData("/api/toxicity/governance-review-pack/status");
}

function getToxicGovernanceSignoffPackPayload() {
  return getData("/api/toxicity/governance-signoff-pack/summary");
}

function getToxicGovernanceSignoffPackStatusPayload() {
  return getData("/api/toxicity/governance-signoff-pack/status");
}

function renderToxicGovernanceProposal() {
  const report = getToxicGovernanceProposalPayload();
  const statusPayload = getToxicGovernanceProposalStatusPayload();
  const error =
    getError("/api/toxicity/governance-proposal/summary") ||
    getError("/api/toxicity/governance-proposal/status");
  if (error) {
    setBadge("toxicGovernanceProposalBadge", "API Error", "error");
    $("toxicGovernanceProposalContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicGovernanceProposalBadge", "Loading", "none");
    $("toxicGovernanceProposalContent").innerHTML =
      `<div class="muted">Governance proposals will appear after review and ledger data are available.</div>`;
    return;
  }

  setBadge(
    "toxicGovernanceProposalBadge",
    (statusPayload?.mode || report.mode || "proposal_draft_only").toUpperCase(),
    report.totalProposals ? "warning" : "none"
  );

  const bySignalType = report.bySignalType || [];
  const bySymbol = report.bySymbol || [];
  const items = report.items || [];
  const byAction = report.byAction || {};

  $("toxicGovernanceProposalContent").innerHTML =
    renderMetrics([
      { label: "Total Proposals", value: formatInteger(report.totalProposals) },
      { label: "Keep", value: formatInteger(byAction.keep) },
      {
        label: "Upgrade Candidates",
        value: formatInteger(byAction.slightUpgradeCandidate),
      },
      {
        label: "Downgrade Candidates",
        value: formatInteger(
          Number(byAction.slightDowngradeCandidate || 0) +
            Number(byAction.downgradeCandidate || 0)
        ),
      },
      { label: "No-trade Only", value: formatInteger(byAction.noTradeOnlyCandidate) },
      { label: "Disable Candidates", value: formatInteger(byAction.disableCandidate) },
      { label: "Needs More Samples", value: formatInteger(byAction.needsMoreSamples) },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicGovernanceProposalButton">Refresh Governance Proposals</button>
      <button type="button" class="small-button" id="copyToxicGovernanceProposalJsonButton">Copy Proposal JSON</button>
      <button type="button" class="small-button" id="copyToxicGovernanceProposalMarkdownButton">Copy Markdown Proposal</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">By Signal Type</div>
        <div class="metric-value">${
          bySignalType.length
            ? bySignalType
                .map(
                  (item) =>
                    `${escapeHtml(item.signalType || "Unavailable")}: total ${formatInteger(item.totalProposals)}, keep ${formatInteger(item.byAction?.keep || 0)}, upgrade ${formatInteger(item.byAction?.slightUpgradeCandidate || 0)}, downgrade ${formatInteger(Number(item.byAction?.slightDowngradeCandidate || 0) + Number(item.byAction?.downgradeCandidate || 0))}, no-trade ${formatInteger(item.byAction?.noTradeOnlyCandidate || 0)}, needs samples ${formatInteger(item.byAction?.needsMoreSamples || 0)}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Symbol</div>
        <div class="metric-value">${
          bySymbol.length
            ? bySymbol
                .map(
                  (item) =>
                    `${escapeHtml(item.symbol || "Unavailable")}: total ${formatInteger(item.totalProposals)}, keep ${formatInteger(item.byAction?.keep || 0)}, upgrade ${formatInteger(item.byAction?.slightUpgradeCandidate || 0)}, downgrade ${formatInteger(Number(item.byAction?.slightDowngradeCandidate || 0) + Number(item.byAction?.downgradeCandidate || 0))}, no-trade ${formatInteger(item.byAction?.noTradeOnlyCandidate || 0)}, needs samples ${formatInteger(item.byAction?.needsMoreSamples || 0)}`
                )
                .join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Proposals</div>
        <div class="metric-value">${
          items.length
            ? items
                .map(
                  (item) =>
                    `${escapeHtml(item.symbol || "Unavailable")} / ${escapeHtml(item.signalType || "Unavailable")}: proposed ${escapeHtml(item.proposedAction || "Unavailable")}, recommendation ${escapeHtml(item.recommendedAction || "Unavailable")}, status ${escapeHtml(item.proposalStatus || "Unavailable")}, confidence ${escapeHtml(item.confidence || "Unavailable")}<br/>Reason Codes: ${escapeHtml((item.reasonCodes || []).join(", ") || "None")}<br/>Evidence Summary: ${escapeHtml((item.evidenceSummary || []).join("; ") || "None")}`
                )
                .join("<br/><br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          readOnly=true<br/>
          proposalOnly=true<br/>
          runtimeWeightModified=false<br/>
          configModified=false<br/>
          runtimeModified=false<br/>
          autoApplyEnabled=false<br/>
          strategyReloaded=false<br/>
          No automatic weight update<br/>
          No runtime config mutation
        </div>
      </div>
    </div>` +
    (state.latestToxicGovernanceProposalAction
      ? `<div class="muted">${escapeHtml(state.latestToxicGovernanceProposalAction)}</div>`
      : "");
}

function renderToxicGovernanceReviewPack() {
  const report = getToxicGovernanceReviewPackPayload();
  const statusPayload = getToxicGovernanceReviewPackStatusPayload();
  const error =
    getError("/api/toxicity/governance-review-pack/summary") ||
    getError("/api/toxicity/governance-review-pack/status");
  if (error) {
    setBadge("toxicGovernanceReviewPackBadge", "API Error", "error");
    $("toxicGovernanceReviewPackContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicGovernanceReviewPackBadge", "Loading", "gray");
    $("toxicGovernanceReviewPackContent").innerHTML =
      `<div class="muted">Loading governance review pack...</div>`;
    return;
  }

  const bySignalType = report.bySignalType || [];
  const bySymbol = report.bySymbol || [];
  const byDecision = report.byDecision || {};
  const items = report.items || [];
  setBadge(
    "toxicGovernanceReviewPackBadge",
    report.readyForManualReview ? "Ready" : "Idle",
    report.readyForManualReview ? "green" : "gray"
  );
  $("toxicGovernanceReviewPackContent").innerHTML =
    renderKeyValueGrid([
      { label: "Total Review Items", value: formatInteger(report.totalItems || 0) },
      { label: "Ready For Manual Review", value: formatBool(report.readyForManualReview) },
      { label: "Accepted", value: formatInteger(byDecision.acceptedCount || 0) },
      { label: "Rejected", value: formatInteger(byDecision.rejectedCount || 0) },
      { label: "Watch More", value: formatInteger(byDecision.watchMoreCount || 0) },
      {
        label: "Needs More Samples",
        value: formatInteger(byDecision.needsMoreSamplesCount || 0),
      },
      {
        label: "Suppressed For Now",
        value: formatInteger(byDecision.suppressForNowCount || 0),
      },
      {
        label: "Escalated Review",
        value: formatInteger(byDecision.escalateReviewCount || 0),
      },
      {
        label: "Pending Governance Review",
        value: formatInteger(byDecision.pendingGovernanceReviewCount || 0),
      },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicGovernanceReviewPackButton">Refresh Governance Review Pack</button>
      <button type="button" class="small-button" id="copyToxicGovernanceReviewPackJsonButton">Copy Review Pack JSON</button>
      <button type="button" class="small-button" id="copyToxicGovernanceReviewPackMarkdownButton">Copy Markdown Review Pack</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">By Signal Type</div>
        <div class="metric-value">${
          bySignalType.length
            ? bySignalType
                .map(
                  (item) =>
                    `${escapeHtml(item.signalType || "Unavailable")}: total ${formatInteger(item.totalItems || 0)}, accepted ${formatInteger(item.byDecision?.acceptedCount || 0)}, rejected ${formatInteger(item.byDecision?.rejectedCount || 0)}, watch ${formatInteger(item.byDecision?.watchMoreCount || 0)}, pending ${formatInteger(item.byDecision?.pendingGovernanceReviewCount || 0)}`
                )
                .join("<br/>")
            : "Unavailable"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Symbol</div>
        <div class="metric-value">${
          bySymbol.length
            ? bySymbol
                .map(
                  (item) =>
                    `${escapeHtml(item.symbol || "Unavailable")}: total ${formatInteger(item.totalItems || 0)}, accepted ${formatInteger(item.byDecision?.acceptedCount || 0)}, rejected ${formatInteger(item.byDecision?.rejectedCount || 0)}, pending ${formatInteger(item.byDecision?.pendingGovernanceReviewCount || 0)}`
                )
                .join("<br/>")
            : "Unavailable"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Review Pack Items</div>
        <div class="metric-value">${
          items.length
            ? items
                .map(
                  (item) =>
                    `${escapeHtml(item.symbol || "Unavailable")} / ${escapeHtml(item.signalType || "Unavailable")}: ${escapeHtml(item.proposalStatus || "Unavailable")} (${escapeHtml(item.confidence || "Unavailable")})`
                )
                .join("<br/>")
            : "Unavailable"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">${
          (statusPayload?.safetyBoundary || [])
            .map((entry) => escapeHtml(entry))
            .join("<br/>") || "Unavailable"
        }<br/>reviewPackOnly=true</div>
      </div>
    </div>` +
    (state.latestToxicGovernanceReviewPackAction
      ? `<div class="muted">${escapeHtml(state.latestToxicGovernanceReviewPackAction)}</div>`
      : "");
}

function renderToxicGovernanceSignoffPack() {
  const report = getToxicGovernanceSignoffPackPayload();
  const statusPayload = getToxicGovernanceSignoffPackStatusPayload();
  const error =
    getError("/api/toxicity/governance-signoff-pack/summary") ||
    getError("/api/toxicity/governance-signoff-pack/status");
  if (error) {
    setBadge("toxicGovernanceSignoffPackBadge", "API Error", "error");
    $("toxicGovernanceSignoffPackContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  if (!report) {
    setBadge("toxicGovernanceSignoffPackBadge", "Loading", "gray");
    $("toxicGovernanceSignoffPackContent").innerHTML =
      `<div class="muted">Loading governance signoff pack...</div>`;
    return;
  }

  const bySignalType = report.bySignalType || [];
  const bySymbol = report.bySymbol || [];
  const blockedReasons = report.blockedReasons || [];
  const items = report.items || [];
  setBadge(
    "toxicGovernanceSignoffPackBadge",
    report.readyForManualSignoff ? "Ready" : "Blocked",
    report.readyForManualSignoff ? "green" : "warning"
  );
  $("toxicGovernanceSignoffPackContent").innerHTML =
    renderKeyValueGrid([
      { label: "Total Items", value: formatInteger(report.totalItems || 0) },
      {
        label: "Ready For Manual Signoff",
        value: formatBool(report.readyForManualSignoff),
      },
      {
        label: "Ready For Signoff",
        value: formatInteger(report.readyForSignoffCount || 0),
      },
      { label: "Hold For Review", value: formatInteger(report.holdCount || 0) },
      { label: "Selected Symbol", value: report.selectedSymbol || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshToxicGovernanceSignoffPackButton">Refresh Governance Signoff Pack</button>
      <button type="button" class="small-button" id="copyToxicGovernanceSignoffPackJsonButton">Copy Signoff Pack JSON</button>
      <button type="button" class="small-button" id="copyToxicGovernanceSignoffPackMarkdownButton">Copy Markdown Signoff Pack</button>
    </div>` +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Blocked Reasons</div>
        <div class="metric-value">${
          blockedReasons.length
            ? blockedReasons.map((entry) => escapeHtml(entry)).join("<br/>")
            : "None"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Signal Type</div>
        <div class="metric-value">${
          bySignalType.length
            ? bySignalType
                .map(
                  (item) =>
                    `${escapeHtml(item.signalType || "Unavailable")}: total ${formatInteger(item.totalItems || 0)}, ready ${formatInteger(item.readyForSignoffCount || 0)}, hold ${formatInteger(item.holdCount || 0)}`
                )
                .join("<br/>")
            : "Unavailable"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">By Symbol</div>
        <div class="metric-value">${
          bySymbol.length
            ? bySymbol
                .map(
                  (item) =>
                    `${escapeHtml(item.symbol || "Unavailable")}: total ${formatInteger(item.totalItems || 0)}, ready ${formatInteger(item.readyForSignoffCount || 0)}, hold ${formatInteger(item.holdCount || 0)}`
                )
                .join("<br/>")
            : "Unavailable"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Signoff Items</div>
        <div class="metric-value">${
          items.length
            ? items
                .map(
                  (item) =>
                    `${escapeHtml(item.symbol || "Unavailable")} / ${escapeHtml(item.signalType || "Unavailable")}: ${escapeHtml(item.signoffRecommendation || "Unavailable")} (${escapeHtml(item.blockedReason || "none")})`
                )
                .join("<br/>")
            : "Unavailable"
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">${
          (statusPayload?.safetyBoundary || [])
            .map((entry) => escapeHtml(entry))
            .join("<br/>") || "Unavailable"
        }<br/>signoffPackOnly=true</div>
      </div>
    </div>` +
    (state.latestToxicGovernanceSignoffPackAction
      ? `<div class="muted">${escapeHtml(state.latestToxicGovernanceSignoffPackAction)}</div>`
      : "");
}

function renderVenueHealth() {
  const status = getData("/api/status");
  const error = getError("/api/status");
  if (error) {
    setBadge("venueHealthBadge", "API Error", "error");
    $("venueHealthContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const venues = Object.entries(status?.venues || {});
  const connectedCount = venues.filter(([, venue]) => venue.status === "connected").length;
  setBadge("venueHealthBadge", `${connectedCount}/${venues.length} Connected`, connectedCount > 0 ? "ok" : "gray");
  $("venueHealthContent").innerHTML = renderTable(
    ["Venue", "Status", "Last Trade", "Last Book", "Reconnects"],
    venues.map(([name, venue]) => [
      name,
      `<span class="badge ${badgeClass(venue.status)}">${venue.status}</span>`,
      formatTime(venue.lastTradeTs),
      formatTime(venue.lastBookTs),
      formatInteger(venue.reconnectCount),
    ])
  );
}

function renderWindows() {
  const flow = getData("/api/flow-state");
  const toxic = getData("/api/toxic-state");
  const sweep = getData("/api/sweep-state");
  const markout = getData("/api/markout-state");

  $("flowWindowsTable").innerHTML = getError("/api/flow-state")
    ? `<div class="error">${getError("/api/flow-state")}</div>`
    : renderTable(
        ["Window", "Buy BTC", "Sell BTC", "Net BTC", "Abs BTC", "Price Move bps", "Active Venues"],
        Object.values(flow?.windows || {}).map((window) => [
          `${window.windowMs / 1000}s`,
          formatNumber(window.aggressiveBuyBtc),
          formatNumber(window.aggressiveSellBtc),
          formatNumber(window.netAggressiveBtc),
          formatNumber(window.absAggressiveBtc),
          formatNumber(window.priceMoveBps, 1),
          (window.dataQuality?.activeVenues || []).join(", ") || "None",
        ])
      );

  $("toxicWindowsTable").innerHTML = getError("/api/toxic-state")
    ? `<div class="error">${getError("/api/toxic-state")}</div>`
    : renderTable(
        ["Window", "Direction", "Severity", "Toxic BTC", "Ratio", "Alert", "Reasons"],
        Object.values(toxic?.results || {}).map((result) => [
          `${result.windowMs / 1000}s`,
          result.direction,
          `<span class="badge ${badgeClass(result.severity)}">${result.severity}</span>`,
          formatNumber(result.toxicVolumeBtc),
          formatNumber(result.toxicRatio, 2),
          formatBool(Boolean(result.alertTriggered)),
          (result.reasonCodes || []).slice(0, 4).join(", "),
        ])
      );

  $("sweepWindowsTable").innerHTML = getError("/api/sweep-state")
    ? `<div class="error">${getError("/api/sweep-state")}</div>`
    : renderTable(
        ["Window", "Direction", "Sweep", "Swept BTC", "Price Impact bps", "Ask Thin", "Bid Thin", "Spread Widened"],
        Object.values(sweep?.results || {}).map((result) => [
          `${result.windowMs / 1000}s`,
          result.direction,
          formatBool(Boolean(result.sweepDetected)),
          formatNumber(result.sweptVolumeBtc),
          formatNumber(result.priceImpactBps, 1),
          formatBool(Boolean(result.liquidity?.askThin)),
          formatBool(Boolean(result.liquidity?.bidThin)),
          formatBool(Boolean(result.liquidity?.spreadWidened)),
        ])
      );

  $("markoutTable").innerHTML = getError("/api/markout-state")
    ? `<div class="error">${getError("/api/markout-state")}</div>`
    : renderTable(
        ["Horizon", "Buy VW Markout", "Sell VW Markout", "Buy Positive BTC", "Sell Positive BTC"],
        Object.values(markout?.summaries || {}).map((summary) => [
          `${summary.horizonMs / 1000}s`,
          formatNumber(summary.buy?.volumeWeightedMarkoutBps, 2),
          formatNumber(summary.sell?.volumeWeightedMarkoutBps, 2),
          formatNumber(summary.buy?.positiveVolumeBtc),
          formatNumber(summary.sell?.positiveVolumeBtc),
        ])
      );
}

function renderRecentEvents() {
  const payload = getData("/api/toxic-events?limit=50");
  const error = getError("/api/toxic-events?limit=50");
  if (error) {
    $("recentEventsContent").innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const events = payload?.events || [];
  $("recentEventsContent").innerHTML = renderTable(
    ["Time", "Direction", "Severity", "Toxic BTC", "Window", "Leader", "Liq Cluster", "Possible Hunt", "Reasons", "JSON"],
    events.map((event) => [
      formatTime(event.ts),
      event.direction,
      `<span class="badge ${badgeClass(event.severity)}">${event.severity}</span>`,
      formatNumber(event.toxicVolumeBtc),
      `${(event.windowMs || 0) / 1000}s`,
      event.leaderVenue || "Unavailable",
      formatBool(Boolean(event.liqClusterNearby)),
      formatBool(Boolean(event.possibleLiqHuntSetup)),
      (event.reasonCodes || []).slice(0, 4).join(", "),
      `<details><summary>View JSON</summary><pre>${escapeHtml(
        JSON.stringify(event, null, 2)
      )}</pre></details>`,
    ])
  );
}

function renderReplayReports() {
  const payload = getData("/api/replay-reports");
  const error = getError("/api/replay-reports");
  const listEl = $("replayReportsList");
  const viewerEl = $("replayReportViewer");
  if (error) {
    listEl.innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const reports = payload?.reports || [];
  listEl.innerHTML = reports.length
    ? reports
        .map(
          (report) => `
          <button
            type="button"
            class="report-link ${state.selectedReport === report.fileName ? "active" : ""}"
            data-file-name="${report.fileName}"
          >
            <div>${report.fileName}</div>
            <div class="muted">${formatTime(report.modifiedAt)}</div>
          </button>`
        )
        .join("")
    : `<div class="muted">No replay reports yet.</div>`;

  if (state.selectedReportContent) {
    viewerEl.className = "report-viewer";
    viewerEl.textContent = state.selectedReportContent;
  } else {
    viewerEl.className = "report-viewer muted";
    viewerEl.textContent = "Select a report to view its markdown.";
  }
}

function reportSummaryRows(items) {
  return renderTable(
    ["Parameter Set", "Hits", "False Positives", "Hit Rate", "False Positive Rate", "Max Toxic BTC"],
    items.map((item) => [
      item.label || "Unavailable",
      formatInteger(item.hitCount),
      formatInteger(item.falsePositiveCount),
      formatNumber((item.hitRate || 0) * 100, 1) + "%",
      formatNumber((item.falsePositiveRate || 0) * 100, 1) + "%",
      formatNumber(item.maxToxicVolumeBtc),
    ])
  );
}

function renderCalibrationReports() {
  const latestPayload = getData("/api/calibration/reports/latest");
  const listPayload = getData("/api/calibration/reports");
  const error =
    getError("/api/calibration/reports/latest") || getError("/api/calibration/reports");

  const summaryEl = $("calibrationSummaryContent");
  const governanceEl = $("manualGovernanceIndexContent");
  const gateEl = $("manualApplyGateContent");
  const remediationEl = $("manualApplyGateRemediationContent");
  const handoffEl = $("manualApplyGateHandoffContent");
  const auditStoryEl = $("manualAuditStoryContent");
  const startupCheckEl = $("manualStartupCheckContent");
  const listEl = $("calibrationReportsList");
  const comparisonsEl = $("calibrationComparisonsContent");
  const reasonStatsEl = $("calibrationReasonStatsContent");
  const outcomesEl = $("calibrationOutcomesContent");
  const recommendationsEl = $("calibrationRecommendationsContent");
  const parameterReviewEl = $("parameterReviewContent");
  const manualExportEl = $("manualExportContent");
  const patchDiffEl = $("parameterPatchDiffContent");
  const runbookEl = $("manualApplyRunbookContent");
  const dryRunEl = $("manualApplyDryRunContent");
  const evidencePackEl = $("manualApplyEvidencePackContent");
  const signoffEl = $("manualSignoffContent");
  const freshnessEl = $("manualEvidenceFreshnessContent");
  const viewerEl = $("calibrationReportViewer");

  if (error) {
    summaryEl.innerHTML = `<div class="error">${error}</div>`;
    governanceEl.innerHTML = renderManualGovernanceIndexSection();
    gateEl.innerHTML = `<div class="error">${error}</div>`;
    remediationEl.innerHTML = `<div class="error">${error}</div>`;
    handoffEl.innerHTML = `<div class="error">${error}</div>`;
    auditStoryEl.innerHTML = `<div class="error">${error}</div>`;
    startupCheckEl.innerHTML = "";
    listEl.innerHTML = `<div class="error">${error}</div>`;
    return;
  }

  const latest = latestPayload?.report || null;
  const reports = listPayload?.reports || [];

  if (!latest) {
    summaryEl.innerHTML = `<div class="muted">No calibration reports yet.</div>`;
    governanceEl.innerHTML = renderManualGovernanceIndexSection();
    gateEl.innerHTML = renderManualApplyGateSection();
    remediationEl.innerHTML = renderManualGateRemediationSection();
    handoffEl.innerHTML = renderOperatorHandoffNoteSection();
    auditStoryEl.innerHTML = renderManualAuditStorySection();
    startupCheckEl.innerHTML = renderManualStartupCheckSection();
    listEl.innerHTML = `<div class="muted">No calibration reports yet.</div>`;
    comparisonsEl.innerHTML = "";
    reasonStatsEl.innerHTML = "";
    outcomesEl.innerHTML = "";
    recommendationsEl.innerHTML = "";
    parameterReviewEl.innerHTML = "";
    manualExportEl.innerHTML = "";
    patchDiffEl.innerHTML = "";
    runbookEl.innerHTML = "";
    dryRunEl.innerHTML = "";
    evidencePackEl.innerHTML = "";
    signoffEl.innerHTML = renderManualSignoffSection();
    freshnessEl.innerHTML = renderManualEvidenceFreshnessSection();
    viewerEl.className = "report-viewer muted";
    viewerEl.textContent = "Run calibrate to generate a report.";
    return;
  }

  const baseline = latest.report?.baseline;
  summaryEl.innerHTML =
    `<h3>Latest Report Summary</h3>` +
    renderMetrics([
      { label: "Report", value: latest.summary?.id || "Unavailable" },
      { label: "Created", value: formatDateTime(latest.summary?.createdAtMs) },
      { label: "Events", value: formatInteger(latest.summary?.eventCount) },
      { label: "Hits", value: formatInteger(latest.summary?.hitCount) },
      { label: "False Positives", value: formatInteger(latest.summary?.falsePositiveCount) },
      { label: "Unknown", value: formatInteger(latest.summary?.unknownCount) },
      { label: "Best Threshold", value: formatNumber(latest.summary?.bestThreshold) },
      { label: "Best Liq Hunt Score", value: formatNumber(latest.summary?.bestLiqHuntScore) },
      { label: "Baseline Threshold", value: formatNumber(baseline?.toxicThresholdBtc) },
      { label: "Baseline Hit Rate", value: formatNumber((baseline?.hitRate || 0) * 100, 1) + "%" },
    ]);
  governanceEl.innerHTML = renderManualGovernanceIndexSection();
  gateEl.innerHTML = renderManualApplyGateSection();
  remediationEl.innerHTML = renderManualGateRemediationSection();
  handoffEl.innerHTML = renderOperatorHandoffNoteSection();
  auditStoryEl.innerHTML = renderManualAuditStorySection();
  startupCheckEl.innerHTML = renderManualStartupCheckSection();

  listEl.innerHTML = reports.length
    ? reports
        .map(
          (report) => `
          <button
            type="button"
            class="report-link ${state.selectedCalibrationReport === report.id ? "active" : ""}"
            data-calibration-report-id="${report.id}"
          >
            <div>${report.id}</div>
            <div class="muted">${formatDateTime(report.createdAtMs)}</div>
          </button>`
        )
        .join("")
    : `<div class="muted">No calibration reports yet.</div>`;

  const current = state.selectedCalibrationReportContent?.report || latest;
  const report = current.report;
  const topReasonStats = (report?.reasonCodeStats || []).slice(0, 12);
  const falsePositives = (report?.topFalsePositives || []).slice(0, 10);
  const hits = (report?.topHits || []).slice(0, 10);

  comparisonsEl.innerHTML =
    `<h3>Threshold Comparison</h3>` +
    reportSummaryRows(report?.thresholdComparison || []) +
    `<h3>Toxic Ratio Comparison</h3>` +
    reportSummaryRows(report?.toxicRatioComparison || []) +
    `<h3>VPIN Parameter Comparison</h3>` +
    reportSummaryRows(report?.vpinParameterComparison || []) +
    `<h3>Liq Hunt Score Comparison</h3>` +
    reportSummaryRows(report?.liqHuntScoreComparison || []);

  reasonStatsEl.innerHTML =
    `<h3>Reason Code Stats</h3>` +
    renderTable(
      ["Reason Code", "Count", "Hit Rate", "False Positive Rate"],
      topReasonStats.map((item) => [
        item.reasonCode,
        formatInteger(item.totalCount),
        formatNumber((item.hitRate || 0) * 100, 1) + "%",
        formatNumber((item.falsePositiveRate || 0) * 100, 1) + "%",
      ])
    );

  outcomesEl.innerHTML =
    `<h3>Top False Positives</h3>` +
    renderTable(
      ["Event", "Time", "Score", "Move bps", "Reasons"],
      falsePositives.map((item) => [
        item.event?.id || "Unavailable",
        formatTime(item.event?.ts),
        formatNumber(item.event?.toxicVolumeBtc),
        formatNumber(item.primaryMoveBps, 1),
        (item.event?.reasonCodes || []).slice(0, 4).join(", "),
      ])
    ) +
    `<h3>Top Hits</h3>` +
    renderTable(
      ["Event", "Time", "Score", "Move bps", "Reasons"],
      hits.map((item) => [
        item.event?.id || "Unavailable",
        formatTime(item.event?.ts),
        formatNumber(item.event?.toxicVolumeBtc),
        formatNumber(item.primaryMoveBps, 1),
        (item.event?.reasonCodes || []).slice(0, 4).join(", "),
      ])
    );

  recommendationsEl.innerHTML =
    `<h3>Recommendations</h3>` +
    ((report?.recommendations || []).length
      ? `<div class="stack">${report.recommendations
          .map(
            (item) => `
            <div class="metric">
              <div class="metric-label">${item.title}</div>
              <div class="metric-value">${item.detail}</div>
            </div>`
          )
          .join("")}</div>`
      : `<div class="muted">No recommendations available.</div>`);

  parameterReviewEl.innerHTML = renderParameterReviewSection(current.summary?.id || latest.summary?.id);
  manualExportEl.innerHTML = renderManualExportSection();
  patchDiffEl.innerHTML = renderParameterPatchDiffSection();
  runbookEl.innerHTML = renderManualApplyRunbookSection();
  dryRunEl.innerHTML = renderManualApplyDryRunSection();
  evidencePackEl.innerHTML = renderManualApplyEvidencePackSection();
  signoffEl.innerHTML = renderManualSignoffSection();
  freshnessEl.innerHTML = renderManualEvidenceFreshnessSection();

  if (state.selectedCalibrationReportContent?.markdownContent) {
    viewerEl.className = "report-viewer";
    viewerEl.textContent = state.selectedCalibrationReportContent.markdownContent;
  } else if (latest.markdownContent) {
    viewerEl.className = "report-viewer";
    viewerEl.textContent = latest.markdownContent;
  } else {
    viewerEl.className = "report-viewer muted";
    viewerEl.textContent = "Markdown report unavailable.";
  }
}

function formatReviewStatus(status) {
  switch (status) {
    case "approved_for_manual_apply":
      return "Approved For Manual Apply";
    case "needs_more_data":
      return "Needs More Data";
    default:
      return (status || "pending").replaceAll("_", " ");
  }
}

function reviewBadgeTone(status) {
  switch (status) {
    case "approved_for_manual_apply":
      return "green";
    case "watch":
      return "yellow";
    case "needs_more_data":
      return "orange";
    case "rejected":
    case "archived":
      return "gray";
    default:
      return "blue";
  }
}

function renderParameterReviewSection(reportId) {
  const recommendationsPayload = getData("/api/parameter-review/recommendations");
  const reviewsPayload = getData("/api/parameter-review/reviews");
  const error =
    getError("/api/parameter-review/recommendations") || getError("/api/parameter-review/reviews");
  if (error) {
    return `<h3>Parameter Review</h3><div class="error">${error}</div>`;
  }

  const recommendations = (recommendationsPayload?.recommendations || []).filter(
    (item) => item.reportId === reportId
  );
  const reviews = reviewsPayload?.reviews || [];
  const recentReviews = reviews.slice(0, 8);

  return (
    `<h3>Parameter Review</h3>` +
    (recommendations.length
      ? `<div class="recommendation-grid">${recommendations
          .map((card) => {
            const review = card.currentReview;
            return `
            <article class="recommendation-card">
              <div class="card-header">
                <h3>${card.parameterKey}</h3>
                <span class="badge ${badgeClass(reviewBadgeTone(review?.status))}">${formatReviewStatus(
              review?.status
            )}</span>
              </div>
              ${renderMetrics([
                { label: "Current", value: card.currentConfigSummary || "Unavailable" },
                { label: "Recommended", value: card.recommendedConfigSummary || "Unavailable" },
                { label: "Direction", value: card.direction || "Unavailable" },
                { label: "Confidence", value: card.confidence || "Unavailable" },
                {
                  label: "Hit Rate",
                  value: formatNumber((card.sourceMetrics?.hitRate || 0) * 100, 1) + "%",
                },
                {
                  label: "False Positive Rate",
                  value: formatNumber((card.sourceMetrics?.falsePositiveRate || 0) * 100, 1) + "%",
                },
              ])}
              <div class="metric">
                <div class="metric-label">Reason</div>
                <div class="metric-value">${card.reason || "Unavailable"}</div>
              </div>
              <div class="metric">
                <div class="metric-label">Expected Effect</div>
                <div class="metric-value">${card.expectedEffect || "Unavailable"}</div>
              </div>
              <div class="metric">
                <div class="metric-label">Risk</div>
                <div class="metric-value">${card.riskNote || "Unavailable"}</div>
              </div>
              <label class="form-field">
                <span>Reviewer</span>
                <input id="reviewer-${card.recommendationId}" data-reviewer-input="${card.recommendationId}" placeholder="optional name" />
              </label>
              <label class="form-field">
                <span>Reviewer Note</span>
                <textarea id="review-note-${card.recommendationId}" data-review-note="${card.recommendationId}" rows="3" placeholder="why this stays watch / reject / manual apply"></textarea>
              </label>
              <div class="action-row">
                ${[
                  ["watch", "Watch"],
                  ["needs_more_data", "Needs More Data"],
                  ["rejected", "Reject"],
                  ["approved_for_manual_apply", "Approve For Manual Apply"],
                  ["archived", "Archive"],
                ]
                  .map(
                    ([status, label]) => `
                    <button
                      type="button"
                      class="small-button"
                      data-review-action="${status}"
                      data-recommendation-id="${card.recommendationId}"
                      data-report-id="${card.reportId}"
                    >${label}</button>`
                  )
                  .join("")}
              </div>
              ${
                review
                  ? `<div class="muted">Last review: ${formatReviewStatus(
                      review.status
                    )} at ${formatDateTime(review.updatedAt)}${
                      review.reviewer ? ` by ${review.reviewer}` : ""
                    }</div>`
                  : `<div class="muted">No review recorded yet.</div>`
              }
            </article>`
          })
          .join("")}</div>`
      : `<div class="muted">No recommendation cards available for this report.</div>`) +
    `<h3>Recent Review Decisions</h3>` +
    renderTable(
      ["Recommendation", "Status", "Reviewer", "Updated", "Note"],
      recentReviews.map((review) => [
        review.recommendationId,
        `<span class="badge ${badgeClass(reviewBadgeTone(review.status))}">${formatReviewStatus(
          review.status
        )}</span>`,
        review.reviewer || "Unavailable",
        formatDateTime(review.updatedAt),
        review.reviewerNote || "Unavailable",
      ])
    )
  );
}

function renderManualExportSection() {
  const exportsPayload = getData("/api/parameter-review/exports");
  const latestPayload = getData("/api/parameter-review/exports/latest");
  const reviewsPayload = getData("/api/parameter-review/reviews");
  const error =
    getError("/api/parameter-review/exports") ||
    getError("/api/parameter-review/exports/latest") ||
    getError("/api/parameter-review/reviews");
  if (error) {
    return `<div class="error">${error}</div>`;
  }

  const latest = latestPayload?.export || null;
  const exports = exportsPayload?.exports || [];
  const approvedCount = (reviewsPayload?.reviews || []).filter(
    (review) => review.status === "approved_for_manual_apply"
  ).length;

  return (
    renderMetrics([
      { label: "Approved Count", value: formatInteger(approvedCount) },
      { label: "Latest Export", value: latest?.summary?.exportId || "Unavailable" },
      { label: "Latest Export Time", value: formatDateTime(latest?.summary?.createdAtMs) },
      { label: "Latest Item Count", value: formatInteger(latest?.summary?.recommendationCount) },
      { label: "Apply Mode", value: latest?.summary?.applyMode || "manual_only" },
      { label: "Runtime Modified", value: formatBool(Boolean(latest?.summary?.runtimeModified)) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="generateManualPatchButton">Generate Manual Patch</button>
      <button type="button" class="small-button" id="refreshExportsButton">Refresh Exports</button>
    </div>` +
    (latest
      ? `<div class="metric">
          <div class="metric-label">Latest Export Paths</div>
          <div class="metric-value">${latest.summary?.jsonPath || "Unavailable"}<br/>${
            latest.summary?.markdownPath || "Unavailable"
          }</div>
        </div>`
      : `<div class="muted">No manual exports generated yet.</div>`) +
    renderTable(
      ["Export ID", "Created", "Items", "Mode", "Runtime Modified", "Runbook", "Dry-run"],
      exports.slice(0, 8).map((entry) => [
        entry.exportId,
        formatDateTime(entry.createdAtMs),
        formatInteger(entry.recommendationCount),
        entry.applyMode,
        formatBool(Boolean(entry.runtimeModified)),
        `<button type="button" class="small-button" data-runbook-export-id="${entry.exportId}">Load Runbook</button>`,
        `<button type="button" class="small-button" data-dryrun-export-id="${entry.exportId}">Load Dry-run</button>`,
        `<button type="button" class="small-button" data-evidence-export-id="${entry.exportId}">Load Evidence Pack</button>`,
      ])
    )
  );
}

function renderParameterPatchDiffSection() {
  const diffPayload = getData("/api/parameter-review/exports/latest/diff");
  const auditPayload = getData("/api/parameter-review/exports/latest/audit");
  const diffError = getError("/api/parameter-review/exports/latest/diff");
  const auditError = getError("/api/parameter-review/exports/latest/audit");

  if (diffError && !diffError.includes("404")) {
    return `<div class="error">${diffError}</div>`;
  }
  if (auditError && !auditError.includes("404")) {
    return `<div class="error">${auditError}</div>`;
  }

  if (!diffPayload?.diff) {
    return `<div class="muted">No manual exports available for diff.</div>`;
  }

  const diff = diffPayload.diff;
  const audit = auditPayload?.audit || null;

  return (
    renderMetrics([
      { label: "Export ID", value: diff.exportId || "Unavailable" },
      { label: "Generated At", value: diff.generatedAt || "Unavailable" },
      {
        label: "Approved Recommendations",
        value: formatInteger(diff.summary?.approvedRecommendationsCount),
      },
      { label: "Changed Fields", value: formatInteger(diff.summary?.changedFieldsCount) },
      { label: "Warnings", value: formatInteger(diff.summary?.warningCount) },
      { label: "Apply Mode", value: diff.applyMode || "manual_only" },
      { label: "Runtime Modified", value: formatBool(Boolean(diff.runtimeModified)) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshPatchDiffButton">Refresh Patch Diff</button>
    </div>` +
    (audit?.manualApplyChecklist?.length
      ? `<div class="metric">
          <div class="metric-label">Manual Apply Checklist</div>
          <div class="metric-value">${audit.manualApplyChecklist
            .map((item) => `- ${item}`)
            .join("<br/>")}</div>
        </div>`
      : "") +
    (diff.warnings?.length
      ? `<div class="metric">
          <div class="metric-label">Warnings</div>
          <div class="metric-value">${diff.warnings.join("<br/>")}</div>
        </div>`
      : `<div class="muted">No audit warnings.</div>`) +
    renderTable(
      ["Parameter", "Current", "Recommended", "Delta", "Change Type", "Severity", "Source", "Notes"],
      (diff.entries || []).map((entry) => [
        entry.key,
        entry.currentDisplay || "Unavailable",
        entry.recommendedDisplay || "Unavailable",
        renderDelta(entry.numericDelta, entry.percentDelta),
        entry.changeType,
        `<span class="badge ${badgeClass(entry.severity)}">${entry.severity}</span>`,
        entry.sourceRecommendationId,
        (entry.notes || []).slice(0, 3).join(" | "),
      ])
    )
  );
}

function renderManualApplyRunbookSection() {
  const latestPayload = getData("/api/parameter-review/exports/latest/runbook");
  const latestMarkdown = state.selectedRunbookExportId ? null : state.data["/api/parameter-review/exports/latest/runbook.md"];

  const runbook = state.selectedRunbook || latestPayload;
  const markdown =
    state.selectedRunbookMarkdown ||
    (typeof latestMarkdown === "string" ? latestMarkdown : null);

  if (!runbook) {
    const latestError = getError("/api/parameter-review/exports/latest/runbook");
    if (latestError && !latestError.includes("404")) {
      return `<div class="error">${latestError}</div>`;
    }
    return `<div class="muted">No runbook available yet.</div>`;
  }

  return (
    renderMetrics([
      { label: "Export ID", value: runbook.exportId || "Unavailable" },
      { label: "Generated At", value: runbook.generatedAt || "Unavailable" },
      {
        label: "Patch Fields",
        value: formatInteger(runbook.summary?.totalPatchFields),
      },
      {
        label: "Changed Fields",
        value: formatInteger(runbook.summary?.changedFields),
      },
      {
        label: "Missing Fields",
        value: formatInteger(runbook.summary?.missingInCurrentConfig),
      },
      { label: "Risk Level", value: runbook.summary?.riskLevel || "Unavailable" },
      { label: "Apply Mode", value: runbook.applyMode || "manual_only" },
      { label: "Runtime Modified", value: formatBool(Boolean(runbook.runtimeModified)) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="loadLatestRunbookButton">Load Latest Runbook</button>
      <button type="button" class="small-button" id="refreshRunbookButton">Refresh Runbook</button>
    </div>` +
    renderTable(
      ["Field", "Current", "Recommended", "Status", "Action"],
      (runbook.fieldChanges || []).map((entry) => [
        entry.path,
        entry.currentValue,
        entry.recommendedValue,
        entry.status,
        entry.action,
      ])
    ) +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Pre-apply Checklist</div>
        <div class="metric-value">${(runbook.preApplyChecklist || [])
          .map((item) => `- ${item.title}${item.note ? `: ${item.note}` : ""}`)
          .join("<br/>")}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Manual Steps</div>
        <div class="metric-value">${(runbook.manualSteps || [])
          .map((item) => `${item.step}. ${item.title}: ${item.instruction}`)
          .join("<br/>")}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Post-apply Verification</div>
        <div class="metric-value">${(runbook.postApplyVerification || [])
          .map((item) => `- ${item.title}${item.command ? ` (${item.command})` : ""}${item.instruction ? `: ${item.instruction}` : ""}`)
          .join("<br/>")}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Rollback Plan</div>
        <div class="metric-value">${(runbook.rollbackPlan || [])
          .map((item) => `${item.step}. ${item.title}: ${item.instruction}`)
          .join("<br/>")}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Guards</div>
        <div class="metric-value">
          auto_apply_enabled=${formatBool(Boolean(runbook.safetyGuards?.autoApplyEnabled))}<br/>
          runtime_reload_enabled=${formatBool(Boolean(runbook.safetyGuards?.runtimeReloadEnabled))}<br/>
          calibration_runner_triggered=${formatBool(Boolean(runbook.safetyGuards?.calibrationRunnerTriggered))}<br/>
          trading_path_touched=${formatBool(Boolean(runbook.safetyGuards?.tradingPathTouched))}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Markdown Runbook</div>
        <div class="metric-value"><pre>${escapeHtml(markdown || "Unavailable")}</pre></div>
      </div>
    </div>`
  );
}

function renderManualApplyDryRunSection() {
  const latestPayload = getData("/api/parameter-review/exports/latest/dry-run");
  const latestMarkdown = state.selectedDryRunExportId
    ? null
    : state.data["/api/parameter-review/exports/latest/dry-run.md"];
  const report = state.selectedDryRun || latestPayload;
  const markdown =
    state.selectedDryRunMarkdown || (typeof latestMarkdown === "string" ? latestMarkdown : null);

  if (!report) {
    const latestError = getError("/api/parameter-review/exports/latest/dry-run");
    if (latestError && !latestError.includes("404")) {
      return `<div class="error">${latestError}</div>`;
    }
    return `<div class="muted">No dry-run validation available yet.</div>`;
  }

  return (
    renderMetrics([
      { label: "Export ID", value: report.exportId || "Unavailable" },
      { label: "Status", value: report.status || "Unavailable" },
      { label: "Apply Mode", value: report.applyMode || "dry_run_only" },
      { label: "Runtime Modified", value: formatBool(Boolean(report.runtimeModified)) },
      { label: "Can Apply Manually", value: formatBool(Boolean(report.canApplyManually)) },
      {
        label: "Blocking Issues",
        value: formatInteger((report.blockingIssues || []).length),
      },
      { label: "Warnings", value: formatInteger((report.warnings || []).length) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="loadLatestDryRunButton">View Dry-run Report</button>
      <button type="button" class="small-button" id="refreshDryRunButton">Refresh Dry-run</button>
    </div>` +
    renderTable(
      ["Check", "Status", "Passed", "Issue Count"],
      (report.checks || []).map((check) => [
        check.name,
        check.status,
        formatBool(Boolean(check.passed)),
        formatInteger(check.issueCount),
      ])
    ) +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Blocking Issues</div>
        <div class="metric-value">${renderIssueList(report.blockingIssues)}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Warnings</div>
        <div class="metric-value">${renderIssueList(report.warnings)}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Markdown Dry-run Report</div>
        <div class="metric-value"><pre>${escapeHtml(markdown || "Unavailable")}</pre></div>
      </div>
    </div>`
  );
}

function renderManualApplyEvidencePackSection() {
  const latestPayload = getData("/api/parameter-review/exports/latest/evidence-pack");
  const latestMarkdown = state.selectedEvidencePackExportId
    ? null
    : state.data["/api/parameter-review/exports/latest/evidence-pack.md"];
  const pack = state.selectedEvidencePack || latestPayload;
  const markdown =
    state.selectedEvidencePackMarkdown ||
    (typeof latestMarkdown === "string" ? latestMarkdown : null);

  if (!pack) {
    const latestError = getError("/api/parameter-review/exports/latest/evidence-pack");
    if (latestError && !latestError.includes("404")) {
      return `<div class="error">${latestError}</div>`;
    }
    return `<div class="muted">No evidence pack available yet.</div>`;
  }

  const signoffMessage = !pack.signoffAllowed
    ? "Operator sign-off is blocked because dry-run failed."
    : pack.dryRunSummary?.status === "passed_with_warnings"
      ? "Operator sign-off is allowed, but warnings must be reviewed."
      : "Operator sign-off can proceed after manual review.";

  return (
    renderMetrics([
      { label: "Evidence Pack ID", value: pack.evidencePackId || "Unavailable" },
      { label: "Export ID", value: pack.exportId || "Unavailable" },
      { label: "Dry-run Status", value: pack.dryRunSummary?.status || "Unavailable" },
      { label: "Warnings", value: formatInteger((pack.warnings || []).length) },
      { label: "Blockers", value: formatInteger((pack.blockers || []).length) },
      { label: "Signoff Required", value: formatBool(Boolean(pack.signoffRequired)) },
      { label: "Signoff Allowed", value: formatBool(Boolean(pack.signoffAllowed)) },
      { label: "Apply Mode", value: pack.applyMode || "manual_signoff_only" },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="loadLatestEvidencePackButton">Load Latest Evidence Pack</button>
      <button type="button" class="small-button" id="refreshEvidencePackButton">Refresh Evidence Pack</button>
    </div>` +
    `<div class="${pack.signoffAllowed ? "muted" : "error"}">${signoffMessage}</div>` +
    renderTable(
      ["Section", "Summary"],
      [
        [
          "Recommendation Review",
          `total=${formatInteger(pack.recommendationSummary?.totalRecommendations)}, approved=${formatInteger(
            pack.recommendationSummary?.approvedForManualApply
          )}`,
        ],
        [
          "Manual Export",
          `items=${formatInteger(pack.exportSummary?.recommendationCount)}, export=${pack.exportSummary?.exportId || "Unavailable"}`,
        ],
        [
          "Patch Diff / Audit",
          `changed=${formatInteger(pack.diffSummary?.changedFields)}, warnings=${formatInteger(pack.diffSummary?.warningsCount)}`,
        ],
        [
          "Manual Apply Runbook",
          `risk=${pack.runbookSummary?.riskLevel || "Unavailable"}, rollback=${formatInteger(pack.runbookSummary?.rollbackSteps)}`,
        ],
        [
          "Dry-run Validation",
          `status=${pack.dryRunSummary?.status || "Unavailable"}, blockers=${formatInteger(pack.dryRunSummary?.blockerCount)}`,
        ],
      ]
    ) +
    `<div class="stack">
      <div class="metric">
        <div class="metric-label">Warnings</div>
        <div class="metric-value">${(pack.warnings || []).length ? pack.warnings.join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Blockers</div>
        <div class="metric-value">${(pack.blockers || []).length ? pack.blockers.join("<br/>") : "None"}</div>
      </div>
      <div class="metric">
        <div class="metric-label">Operator Sign-off Template</div>
        <div class="metric-value">
          signoff_required=${formatBool(Boolean(pack.operatorSignoffTemplate?.signoffRequired))}<br/>
          signoff_allowed=${formatBool(Boolean(pack.operatorSignoffTemplate?.signoffAllowed))}<br/>
          options=${(pack.operatorSignoffTemplate?.decisionOptions || []).join(", ")}<br/>
          ${pack.operatorSignoffTemplate?.blockingMessage || ""}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Safety Boundary</div>
        <div class="metric-value">
          apply_mode=${pack.safetyBoundary?.applyMode || "manual_signoff_only"}<br/>
          runtime_modified=${formatBool(Boolean(pack.safetyBoundary?.runtimeModified))}<br/>
          no_runtime_config_changed=${formatBool(Boolean(pack.safetyBoundary?.noRuntimeConfigChanged))}<br/>
          no_calibration_runner_triggered=${formatBool(Boolean(pack.safetyBoundary?.noCalibrationRunnerTriggered))}<br/>
          no_runtime_reload_triggered=${formatBool(Boolean(pack.safetyBoundary?.noRuntimeReloadTriggered))}
        </div>
      </div>
      <div class="metric">
        <div class="metric-label">Markdown Evidence Pack</div>
        <div class="metric-value"><pre>${escapeHtml(markdown || "Unavailable")}</pre></div>
      </div>
    </div>`
  );
}

function renderManualApplyGateSection() {
  const readiness = getData("/api/calibration/manual-startup/check");
  const signoff = getData("/api/calibration/manual-signoff/status");
  const freshness = getData("/api/calibration/manual-evidence/freshness");
  const auditStory = getData("/api/calibration/manual-audit-story");
  const error =
    getError("/api/calibration/manual-startup/check") ||
    getError("/api/calibration/manual-signoff/status") ||
    getError("/api/calibration/manual-evidence/freshness");

  if (error) {
    return `<div class="error">${error}</div>`;
  }

  if (!readiness && !signoff && !freshness) {
    return `<div class="muted">Manual apply gate status will appear after the read-only checks load.</div>`;
  }

  const readinessStatus = readiness?.status || "Unavailable";
  const signoffStatus = signoff?.status || "Unavailable";
  const freshnessStatus = freshness?.status || "Unavailable";
  const finalGateReady =
    readinessStatus === "READY_FOR_MANUAL_APPLY" &&
    signoffStatus === "SIGNED_OFF" &&
    freshnessStatus === "FRESH";
  const finalGateStatus = auditStory?.finalGate || (finalGateReady ? "READY" : "BLOCKED");

  const blockingReasons = auditStory?.blockers?.length
    ? auditStory.blockers
    : [];
  if (!blockingReasons.length && !finalGateReady) {
    if (readinessStatus !== "READY_FOR_MANUAL_APPLY") {
      blockingReasons.push(`Startup readiness is ${readinessStatus}.`);
    }
    if (signoffStatus !== "SIGNED_OFF") {
      if (signoffStatus === "NO_SIGNOFF") {
        blockingReasons.push("Operator sign-off is missing.");
      } else if (signoffStatus === "SIGNOFF_STALE") {
        blockingReasons.push("Existing sign-off no longer matches current evidence.");
      } else if (signoffStatus === "SIGNOFF_EXPIRED") {
        blockingReasons.push("Latest operator sign-off has expired.");
      } else if (signoffStatus !== "Unavailable") {
        blockingReasons.push(`Operator sign-off status is ${signoffStatus}.`);
      }
    }
    if (freshnessStatus !== "FRESH") {
      if (freshnessStatus === "STALE") {
        blockingReasons.push("Existing sign-off no longer matches current evidence.");
      } else if (freshnessStatus === "EXPIRED") {
        blockingReasons.push("Latest sign-off expired and must be refreshed.");
      } else if (freshnessStatus === "MISSING_EVIDENCE") {
        blockingReasons.push("Required evidence is missing.");
      } else if (freshnessStatus !== "Unavailable") {
        blockingReasons.push(`Evidence freshness status is ${freshnessStatus}.`);
      }
    }
  }

  const changedEvidence = auditStory?.changedEvidence || freshness?.changedEvidence || [];
  const nextAction =
    auditStory?.nextAction ||
    freshness?.nextAction ||
    signoff?.nextAction ||
    readiness?.nextAction ||
    "Follow the manual runbook outside this system.";

  return (
    renderMetrics([
      {
        label: "Final Gate",
        value: `<span class="badge ${badgeClass(finalGateReady ? "ok" : "error")}">${finalGateStatus}</span>`,
      },
      {
        label: "Startup Readiness",
        value: `<span class="badge ${badgeClass(
          readinessStatus === "READY_FOR_MANUAL_APPLY"
            ? "ok"
            : readinessStatus === "BLOCKED"
              ? "error"
              : readinessStatus === "MISSING_REPORT"
                ? "none"
                : "warning"
        )}">${readinessStatus}</span>`,
      },
      {
        label: "Operator Sign-off",
        value: `<span class="badge ${badgeClass(
          signoffStatus === "SIGNED_OFF"
            ? "ok"
            : signoffStatus === "SIGNOFF_STALE" || signoffStatus === "SIGNOFF_EXPIRED"
              ? "warning"
              : signoffStatus === "READINESS_NOT_READY"
                ? "error"
                : "none"
        )}">${signoffStatus}</span>`,
      },
      {
        label: "Evidence Freshness",
        value: `<span class="badge ${badgeClass(
          freshnessStatus === "FRESH"
            ? "ok"
            : freshnessStatus === "STALE"
              ? "warning"
              : freshnessStatus === "EXPIRED" || freshnessStatus === "READINESS_NOT_READY"
                ? "error"
                : "none"
        )}">${freshnessStatus}</span>`,
      },
    ]) +
    `<div class="metric">
      <div class="metric-label">Reason</div>
      <div class="metric-value">${
        finalGateReady
          ? "All checks passed: startup readiness is ready, operator sign-off is valid, and evidence is fresh."
          : blockingReasons.length
            ? blockingReasons.join("<br/>")
            : "Manual gate is blocked until the upstream checks converge."
      }</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Changed Evidence</div>
      <div class="metric-value">${changedEvidence.length ? changedEvidence.join(", ") : "None"}</div>
    </div>` +
    `<div class="metric-grid">
      <div class="metric">
        <div class="metric-label">Evidence Age</div>
        <div class="metric-value">${
          freshness?.ageMs == null ? "Unavailable" : `${formatInteger(freshness.ageMs)} ms`
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">Expires In</div>
        <div class="metric-value">${
          freshness?.expiresInMs == null
            ? "Unavailable"
            : `${formatInteger(freshness.expiresInMs)} ms`
        }</div>
      </div>
      <div class="metric">
        <div class="metric-label">TTL</div>
        <div class="metric-value">${
          freshness?.ttlMs == null ? "Unavailable" : `${formatInteger(freshness.ttlMs)} ms`
        }</div>
      </div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Next</div>
      <div class="metric-value">${nextAction}</div>
    </div>`
  );
}

function buildManualGateRemediation(readinessStatus, signoffStatus, freshnessStatus, changedEvidence) {
  const items = [];

  if (readinessStatus !== "READY_FOR_MANUAL_APPLY") {
    items.push({
      level: "error",
      reason: `Startup readiness is ${readinessStatus || "UNKNOWN"}`,
      actions: [
        "Run Manual Startup Check.",
        "Review failed readiness checks.",
        "Fix missing report, review, export, diff, runbook, or dry-run evidence before proceeding.",
      ],
    });
  }

  if (signoffStatus !== "SIGNED_OFF") {
    const actions = [
      "Review the evidence pack.",
      "Approve or reject the manual gate explicitly.",
      "Do not proceed without a valid operator sign-off.",
    ];
    if (signoffStatus === "SIGNOFF_EXPIRED") {
      actions.unshift("Refresh evidence and sign off again because the latest sign-off expired.");
    } else if (signoffStatus === "SIGNOFF_STALE") {
      actions.unshift("Create a new sign-off after reviewing the changed evidence.");
    }
    items.push({
      level: signoffStatus === "REJECTED" ? "error" : "warning",
      reason: `Operator sign-off is ${signoffStatus || "UNKNOWN"}`,
      actions,
    });
  }

  if (freshnessStatus !== "FRESH") {
    const actions = [
      "Refresh evidence freshness.",
      "Review changed evidence and TTL details.",
      "Create a new sign-off if evidence changed or expired.",
    ];
    if (freshnessStatus === "STALE" && changedEvidence.length) {
      actions.unshift(`Changed evidence: ${changedEvidence.join(", ")}.`);
    }
    if (freshnessStatus === "EXPIRED") {
      actions.unshift("Refresh evidence, re-run startup check, and sign off again.");
    }
    items.push({
      level: freshnessStatus === "MISSING_EVIDENCE" ? "error" : "warning",
      reason: `Evidence freshness is ${freshnessStatus || "UNKNOWN"}`,
      actions,
    });
  }

  if (items.length === 0) {
    items.push({
      level: "info",
      reason: "Manual gate is ready",
      actions: ["Follow the manual runbook outside this system."],
    });
  }

  return items;
}

function buildOperatorHandoffNote(gateState) {
  const { readinessStatus, signoffStatus, freshnessStatus, changedEvidence, finalGateReady } = gateState;

  if (finalGateReady) {
    return {
      title: "READY",
      severity: "ok",
      summary: "All required evidence is aligned and the manual gate is ready for external execution.",
      nextOwner: "External Manual Executor",
      steps: [
        "Evidence pack reviewed.",
        "Valid sign-off is present.",
        "Startup check is fresh.",
        "Dry-run passed.",
        "Follow the Manual Apply Runbook outside this system.",
      ],
      boundary: [
        "This dashboard will not apply or reload runtime parameters.",
      ],
    };
  }

  if (signoffStatus === "NO_SIGNOFF") {
    return {
      title: "BLOCKED / NO_SIGNOFF",
      severity: "warning",
      summary: "Evidence exists but has not been signed off.",
      nextOwner: "Reviewer / Approver",
      steps: [
        "Review evidence pack first.",
        "Approve or reject through the manual sign-off flow.",
        "Do not perform external manual apply until sign-off is valid.",
      ],
      boundary: [
        "This dashboard will not apply or reload runtime parameters.",
      ],
    };
  }

  if (freshnessStatus === "STALE" || signoffStatus === "SIGNOFF_STALE") {
    return {
      title: "BLOCKED / STALE",
      severity: "warning",
      summary: "Existing sign-off no longer matches current evidence.",
      nextOwner: "Reviewer",
      steps: [
        changedEvidence.length
          ? `Review changedEvidence: ${changedEvidence.join(", ")}.`
          : "Review changed evidence.",
        "Regenerate or refresh the required evidence.",
        "Re-sign after confirming the updated evidence.",
      ],
      boundary: [
        "This dashboard will not apply or reload runtime parameters.",
      ],
    };
  }

  if (freshnessStatus === "EXPIRED" || signoffStatus === "SIGNOFF_EXPIRED") {
    return {
      title: "BLOCKED / EXPIRED",
      severity: "warning",
      summary: "Startup check or sign-off freshness window has expired.",
      nextOwner: "Operator",
      steps: [
        "Refresh evidence.",
        "Re-run Manual Startup Check.",
        "Re-sign before external manual execution.",
      ],
      boundary: [
        "This dashboard will not apply or reload runtime parameters.",
      ],
    };
  }

  if (
    readinessStatus === "BLOCKED" ||
    readinessStatus === "MISSING_REPORT" ||
    readinessStatus === "NEEDS_REVIEW" ||
    signoffStatus === "READINESS_NOT_READY"
  ) {
    return {
      title: "BLOCKED",
      severity: "error",
      summary: "Required manual-apply evidence is incomplete or readiness failed.",
      nextOwner: "Operator",
      steps: [
        "Run Manual Startup Check.",
        "Fix failed readiness items.",
        "Confirm report, export, diff, runbook, and dry-run are all present.",
      ],
      boundary: [
        "This dashboard will not apply or reload runtime parameters.",
      ],
    };
  }

  return {
    title: "BLOCKED",
    severity: "warning",
    summary: "Manual apply remains blocked until the current evidence and sign-off converge.",
    nextOwner: "Operator",
    steps: [
      "Review startup readiness, sign-off, and evidence freshness together.",
      "Refresh the stale layer before proceeding.",
      "Do not perform external manual apply while the gate is blocked.",
    ],
    boundary: [
      "This dashboard will not apply or reload runtime parameters.",
    ],
  };
}

function renderManualGateRemediationSection() {
  const readiness = getData("/api/calibration/manual-startup/check");
  const signoff = getData("/api/calibration/manual-signoff/status");
  const freshness = getData("/api/calibration/manual-evidence/freshness");
  const auditStory = getData("/api/calibration/manual-audit-story");
  const error =
    getError("/api/calibration/manual-startup/check") ||
    getError("/api/calibration/manual-signoff/status") ||
    getError("/api/calibration/manual-evidence/freshness");

  if (error) {
    return `<div class="error">${error}</div>`;
  }

  const readinessStatus = readiness?.status || "UNKNOWN";
  const signoffStatus = signoff?.status || "UNKNOWN";
  const freshnessStatus = freshness?.status || "UNKNOWN";
  const changedEvidence = auditStory?.changedEvidence || freshness?.changedEvidence || [];
  const items = auditStory?.remediationChecklist?.length
    ? [
        {
          level: auditStory.finalGate === "READY" ? "info" : "warning",
          reason: auditStory.currentBlocker || "READY",
          actions: auditStory.remediationChecklist,
        },
      ]
    : buildManualGateRemediation(
        readinessStatus,
        signoffStatus,
        freshnessStatus,
        changedEvidence
      );

  return items
    .map(
      (item) => `<div class="recommendation-card">
        <div class="metric">
          <div class="metric-label">Reason</div>
          <div class="metric-value">
            <span class="badge ${badgeClass(item.level)}">${item.reason}</span>
          </div>
        </div>
        <div class="metric">
          <div class="metric-label">Fix This First</div>
          <div class="metric-value">${item.actions
            .map((action) => `- ${escapeHtml(action)}`)
            .join("<br/>")}</div>
        </div>
      </div>`
    )
    .join("");
}

function renderOperatorHandoffNoteSection() {
  const readiness = getData("/api/calibration/manual-startup/check");
  const signoff = getData("/api/calibration/manual-signoff/status");
  const freshness = getData("/api/calibration/manual-evidence/freshness");
  const auditStory = getData("/api/calibration/manual-audit-story");
  const error =
    getError("/api/calibration/manual-startup/check") ||
    getError("/api/calibration/manual-signoff/status") ||
    getError("/api/calibration/manual-evidence/freshness");

  if (error) {
    return `<div class="error">${error}</div>`;
  }

  const readinessStatus = readiness?.status || "UNKNOWN";
  const signoffStatus = signoff?.status || "UNKNOWN";
  const freshnessStatus = freshness?.status || "UNKNOWN";
  const changedEvidence = auditStory?.changedEvidence || freshness?.changedEvidence || [];
  const finalGateReady =
    readinessStatus === "READY_FOR_MANUAL_APPLY" &&
    signoffStatus === "SIGNED_OFF" &&
    freshnessStatus === "FRESH";

  const note = auditStory
    ? {
        title: auditStory.finalGate === "READY" ? "READY" : auditStory.currentBlocker || "BLOCKED",
        severity: auditStory.finalGate === "READY" ? "ok" : "warning",
        summary: auditStory.handoffSummary || "Unavailable",
        nextOwner: auditStory.nextOwner || "Operator",
        steps: auditStory.remediationChecklist?.length
          ? auditStory.remediationChecklist
          : [auditStory.nextAction || "Unavailable"],
        boundary: auditStory.safetyBoundary || [
          "This dashboard will not apply or reload runtime parameters.",
        ],
      }
    : buildOperatorHandoffNote({
        readinessStatus,
        signoffStatus,
        freshnessStatus,
        changedEvidence,
        finalGateReady,
      });

  return (
    renderMetrics([
      {
        label: "Status",
        value: `<span class="badge ${badgeClass(note.severity)}">${note.title}</span>`,
      },
      { label: "Next Owner", value: note.nextOwner },
    ]) +
    `<div class="metric">
      <div class="metric-label">Summary</div>
      <div class="metric-value">${note.summary}</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Handoff Steps</div>
      <div class="metric-value">${note.steps.map((step) => `- ${escapeHtml(step)}`).join("<br/>")}</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Boundary</div>
      <div class="metric-value">${note.boundary
        .map((item) => `- ${escapeHtml(item)}`)
        .join("<br/>")}</div>
    </div>`
  );
}

function buildManualApplySessionTimeline(auditStory) {
  if (!auditStory) {
    return [];
  }

  if (Array.isArray(auditStory.timeline) && auditStory.timeline.length) {
    return auditStory.timeline.map((item) => ({
      key: item.key,
      title: item.title,
      status: item.status || "unknown",
      summary: item.summary || "Unavailable",
      owner:
        item.key === "review"
          ? "Reviewer / Approver"
          : item.key === "handoff"
            ? auditStory.nextOwner || "Operator"
            : item.key === "runbook"
              ? "External Manual Executor"
              : auditStory.nextOwner || "Operator",
      evidenceRef: item.evidenceRef || null,
      missingReason: item.missingReason || null,
      blockingReason: item.blockingReason || null,
      remediationHint: item.remediationHint || null,
      observedAt: item.observedAt || null,
    }));
  }

  const finalGateReady = auditStory.finalGate === "READY";
  const blocker = auditStory.currentBlocker || "None";
  return [
    {
      key: "review",
      title: "Recommendation Review",
      status: auditStory.readinessStatus === "MISSING_REPORT" ? "missing" : "passed",
      summary: "Recommendation review evidence is available for this manual apply story.",
      owner: "Reviewer / Approver",
    },
    {
      key: "export",
      title: "Manual Parameter Export",
      status: auditStory.readinessStatus === "MISSING_REPORT" ? "missing" : "passed",
      summary: "Manual export artifact is available for read-only review.",
      owner: "Operator",
    },
    {
      key: "diff",
      title: "Patch Diff / Audit",
      status: blocker === "MISSING_EVIDENCE" ? "missing" : "passed",
      summary: "Diff and audit layers are ready for inspection.",
      owner: "Operator",
    },
    {
      key: "runbook",
      title: "Manual Apply Runbook",
      status: blocker === "MISSING_EVIDENCE" ? "missing" : "passed",
      summary: "Manual runbook exists for external execution guidance.",
      owner: "External Manual Executor",
    },
    {
      key: "dryrun",
      title: "Dry-run Validation",
      status:
        auditStory.readinessStatus === "BLOCKED"
          ? "blocked"
          : auditStory.readinessStatus === "NEEDS_REVIEW"
            ? "warning"
            : "passed",
      summary: auditStory.readinessStatus === "BLOCKED"
        ? "Dry-run validation or prerequisite checks are blocking manual apply."
        : "Dry-run evidence is available for review.",
      owner: "Operator",
    },
    {
      key: "evidence",
      title: "Evidence Pack",
      status:
        auditStory.freshnessStatus === "MISSING_EVIDENCE"
          ? "missing"
          : auditStory.freshnessStatus === "STALE"
            ? "stale"
            : auditStory.freshnessStatus === "EXPIRED"
              ? "expired"
              : "passed",
      summary: auditStory.changedEvidence?.length
        ? `Changed evidence: ${auditStory.changedEvidence.join(", ")}.`
        : "Evidence pack is available.",
      owner: "Reviewer",
    },
    {
      key: "signoff",
      title: "Operator Sign-off",
      status:
        auditStory.signoffStatus === "SIGNED_OFF"
          ? "passed"
          : auditStory.signoffStatus === "NO_SIGNOFF"
            ? "blocked"
            : auditStory.signoffStatus === "SIGNOFF_STALE"
              ? "stale"
              : auditStory.signoffStatus === "SIGNOFF_EXPIRED"
                ? "expired"
                : "blocked",
      summary: `Current sign-off status is ${auditStory.signoffStatus}.`,
      owner: auditStory.nextOwner || "Operator",
    },
    {
      key: "startup",
      title: "Manual Startup Check",
      status:
        auditStory.readinessStatus === "READY_FOR_MANUAL_APPLY"
          ? "passed"
          : auditStory.readinessStatus === "NEEDS_REVIEW"
            ? "warning"
            : auditStory.readinessStatus === "MISSING_REPORT"
              ? "missing"
              : "blocked",
      summary: `Startup readiness is ${auditStory.readinessStatus}.`,
      owner: "Operator",
    },
    {
      key: "gate",
      title: "Manual Apply Gate",
      status: finalGateReady ? "ready" : "blocked",
      summary: finalGateReady
        ? "Ready for external manual execution by runbook."
        : `Current blocker: ${blocker}.`,
      owner: auditStory.nextOwner || "Operator",
    },
    {
      key: "remediation",
      title: "Remediation Checklist",
      status: finalGateReady ? "ready" : "blocked",
      summary: finalGateReady
        ? "No remediation required."
        : auditStory.remediationChecklist?.[0] || "Review remediation steps before proceeding.",
      owner: auditStory.nextOwner || "Operator",
    },
    {
      key: "handoff",
      title: "Operator Handoff Note",
      status: finalGateReady ? "ready" : "blocked",
      summary: auditStory.handoffSummary || "Operator handoff summary is unavailable.",
      owner: auditStory.nextOwner || "Operator",
    },
  ];
}

function formatEvidenceRef(evidenceRef) {
  if (!evidenceRef) {
    return "Unavailable";
  }
  const parts = [
    evidenceRef.label || "Unavailable",
    evidenceRef.kind ? `(${evidenceRef.kind})` : null,
    evidenceRef.sourceEndpoint ? `source: ${evidenceRef.sourceEndpoint}` : null,
    evidenceRef.markdownEndpoint ? `markdown: ${evidenceRef.markdownEndpoint}` : null,
  ].filter(Boolean);
  return parts.join(" | ");
}

function renderManualApplySessionTimeline(items) {
  if (!items.length) {
    return `<div class="muted">Manual apply session timeline will appear after the audit story loads.</div>`;
  }

  return items
    .map(
      (item) => `<div class="recommendation-card">
        <div class="metric-grid">
          <div class="metric">
            <div class="metric-label">Step</div>
            <div class="metric-value">${item.title}</div>
          </div>
          <div class="metric">
            <div class="metric-label">Status</div>
            <div class="metric-value"><span class="badge ${badgeClass(item.status)}">${item.status}</span></div>
          </div>
          <div class="metric">
            <div class="metric-label">Owner</div>
            <div class="metric-value">${item.owner || "Unavailable"}</div>
          </div>
        </div>
        <div class="metric">
          <div class="metric-label">Summary</div>
          <div class="metric-value">${escapeHtml(item.summary || "Unavailable")}</div>
        </div>
        <div class="metric">
          <div class="metric-label">Evidence Ref</div>
          <div class="metric-value">${escapeHtml(
            item.evidenceRef ? formatEvidenceRef(item.evidenceRef) : item.missingReason || "Unavailable"
          )}</div>
        </div>
        <div class="metric">
          <div class="metric-label">Blocking Reason</div>
          <div class="metric-value">${escapeHtml(item.blockingReason || "None")}</div>
        </div>
        <div class="metric">
          <div class="metric-label">Remediation Hint</div>
          <div class="metric-value">${escapeHtml(item.remediationHint || "None")}</div>
        </div>
        <div class="metric">
          <div class="metric-label">Observed At</div>
          <div class="metric-value">${escapeHtml(item.observedAt || "Unavailable")}</div>
        </div>
      </div>`
    )
    .join("");
}

function renderGovernanceIssues(issues = []) {
  if (!issues.length) {
    return `<div class="muted">None</div>`;
  }
  return issues
    .map(
      (issue) => `<div class="recommendation-card">
        <div class="metric-grid">
          <div class="metric">
            <div class="metric-label">Code</div>
            <div class="metric-value">${escapeHtml(issue.code || "Unavailable")}</div>
          </div>
          <div class="metric">
            <div class="metric-label">Severity</div>
            <div class="metric-value"><span class="badge ${badgeClass(issue.severity || "warning")}">${escapeHtml(issue.severity || "warning")}</span></div>
          </div>
        </div>
        <div class="metric">
          <div class="metric-label">Message</div>
          <div class="metric-value">${escapeHtml(issue.message || "Unavailable")}</div>
        </div>
        <div class="metric">
          <div class="metric-label">Next</div>
          <div class="metric-value">${escapeHtml(issue.nextAction || "Unavailable")}</div>
        </div>
      </div>`
    )
    .join("");
}

function renderGovernanceLinks(links) {
  if (!links) {
    return `<div class="muted">Unavailable</div>`;
  }
  return renderTable(
    ["Artifact", "Endpoint"],
    [
      ["Startup Check", links.startupCheck],
      ["Sign-off Status", links.signoffStatus],
      ["Evidence Freshness", links.evidenceFreshness],
      ["Audit Story", links.auditStory],
      ["Evidence Pack", links.evidencePack],
      ["Runbook", links.runbook],
      ["Dry-run", links.dryRun],
      ["Patch Diff / Audit", links.patchDiffAudit],
    ].map(([label, value]) => [label, value || "Unavailable"])
  );
}

function renderManualGovernanceIndexSection() {
  const payload = getData("/api/calibration/manual-governance/index");
  const error = getError("/api/calibration/manual-governance/index");

  if (error) {
    return `<div class="error">${error}</div>`;
  }

  if (!payload) {
    return `<div class="muted">Governance index will appear after the read-only manual apply chain loads.</div>`;
  }

  return (
    renderMetrics([
      {
        label: "Governance Status",
        value: `<span class="badge ${badgeClass(payload.governanceStatus === "READY_FOR_EXTERNAL_MANUAL_APPLY" ? "ok" : "warning")}">${payload.governanceStatus || "Unavailable"}</span>`,
      },
      {
        label: "Final Gate",
        value: `<span class="badge ${badgeClass(payload.finalGate === "READY" ? "ok" : "error")}">${payload.finalGate || "Unavailable"}</span>`,
      },
      { label: "Read Only", value: formatBool(Boolean(payload.readOnly)) },
      { label: "Runtime Modified", value: formatBool(Boolean(payload.runtimeModified)) },
      { label: "Apply Mode", value: payload.applyMode || "governance_index_only" },
      { label: "Latest Export ID", value: payload.latestExportId || "Unavailable" },
      { label: "Next Owner", value: payload.nextOwner || "Unavailable" },
      {
        label: "Next Action",
        value: payload.nextAction || payload.nextRequiredAction || "Unavailable",
      },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshGovernanceIndexButton">Refresh Governance Index</button>
      <button type="button" class="small-button" id="copyGovernanceMarkdownButton">Copy Governance Markdown</button>
    </div>` +
    `<div class="muted">This dashboard is read-only. It does not apply parameters, reload runtime config, or trigger calibration_runner.</div>` +
    `<div class="metric-grid">
      <div class="metric"><div class="metric-label">Startup Readiness</div><div class="metric-value">${escapeHtml(payload.readinessStatus || "Unavailable")}</div></div>
      <div class="metric"><div class="metric-label">Operator Sign-off</div><div class="metric-value">${escapeHtml(payload.signoffStatus || "Unavailable")}</div></div>
      <div class="metric"><div class="metric-label">Evidence Freshness</div><div class="metric-value">${escapeHtml(payload.freshnessStatus || "Unavailable")}</div></div>
      <div class="metric"><div class="metric-label">Audit Story</div><div class="metric-value">${escapeHtml(payload.auditStoryStatus || "Unavailable")}</div></div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Blocking Issues</div>
      <div class="metric-value">${renderGovernanceIssues(payload.blockingReasons || payload.blockingIssues || [])}</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Warnings</div>
      <div class="metric-value">${renderGovernanceIssues(payload.warnings || [])}</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Artifact Links</div>
      <div class="metric-value">${renderGovernanceLinks(payload.links)}</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Safety Boundary</div>
      <div class="metric-value">${(payload.safetyBoundary || []).map((item) => `- ${escapeHtml(item)}`).join("<br/>")}</div>
    </div>` +
    (state.latestGovernanceAction
      ? `<div class="muted">${escapeHtml(state.latestGovernanceAction)}</div>`
      : "")
  );
}

function renderManualAuditStorySection() {
  const payload = getData("/api/calibration/manual-audit-story");
  const error = getError("/api/calibration/manual-audit-story");

  if (error) {
    return `<div class="error">${error}</div>`;
  }

  if (!payload) {
    return `<div class="muted">Manual audit story will appear after the read-only evidence layers load.</div>`;
  }

  const timeline = buildManualApplySessionTimeline(payload);
  return (
    renderMetrics([
      {
        label: "Final Gate",
        value: `<span class="badge ${badgeClass(payload.finalGate === "READY" ? "ok" : "error")}">${payload.finalGate || "Unavailable"}</span>`,
      },
      { label: "Generated At", value: payload.generatedAt || "Unavailable" },
      { label: "Read Only", value: formatBool(Boolean(payload.readOnly)) },
      { label: "Apply Mode", value: payload.applyMode || "read_only_audit_story" },
      { label: "Next Owner", value: payload.nextOwner || "Unavailable" },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshManualAuditStoryButton">Refresh Audit Story</button>
      <button type="button" class="small-button" id="copyManualAuditStoryButton">Copy Markdown Audit Story</button>
    </div>` +
    `<div class="muted">Handoff Artifact: Markdown is suitable for issue / ops log handoff. Review Summary, Timeline, Blockers, Warnings, Evidence References, Remediation Checklist, Operator Handoff and Safety Boundary before external manual execution.</div>` +
    `<div class="muted">Ops Log Snippet is included in the markdown export for copy/paste handoff.</div>` +
    `<div class="metric">
      <div class="metric-label">Summary</div>
      <div class="metric-value">${escapeHtml(payload.handoffSummary || "Unavailable")}</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Changed Evidence</div>
      <div class="metric-value">${(payload.changedEvidence || []).length ? payload.changedEvidence.join(", ") : "None"}</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Safety Boundary</div>
      <div class="metric-value">${(payload.safetyBoundary || [])
        .map((item) => `- ${escapeHtml(item)}`)
        .join("<br/>")}</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Manual Apply Session Timeline</div>
      <div class="metric-value">read_only_audit_story</div>
    </div>` +
    renderManualApplySessionTimeline(timeline) +
    `<div class="metric">
      <div class="metric-label">Markdown Audit Story</div>
      <div class="metric-value"><pre>${escapeHtml(payload.markdown || "Unavailable")}</pre></div>
    </div>` +
    (state.latestManualAuditStoryAction
      ? `<div class="muted">${escapeHtml(state.latestManualAuditStoryAction)}</div>`
      : "")
  );
}

function renderManualStartupCheckSection() {
  const payload = getData("/api/calibration/manual-startup/check");
  const error = getError("/api/calibration/manual-startup/check");
  if (error) {
    return `<div class="action-row">
      <button type="button" class="small-button" id="manualStartupCheckButton">Manual Startup Check</button>
    </div><div class="error">${error}</div>`;
  }

  if (!payload) {
    return `<div class="action-row">
      <button type="button" class="small-button" id="manualStartupCheckButton">Manual Startup Check</button>
    </div><div class="muted">Click the button to run the read-only startup checklist.</div>`;
  }

  const tone =
    payload.status === "READY_FOR_MANUAL_APPLY"
      ? "ok"
      : payload.status === "BLOCKED"
        ? "error"
        : payload.status === "NEEDS_REVIEW"
          ? "warning"
          : "none";

  return (
    renderMetrics([
      { label: "Status", value: `<span class="badge ${badgeClass(tone)}">${payload.status}</span>` },
      { label: "Read Only", value: formatBool(Boolean(payload.readOnly)) },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="manualStartupCheckButton">Manual Startup Check</button>
    </div>` +
    renderTable(
      ["Check", "OK", "Severity", "Message"],
      (payload.checks || []).map((check) => [
        check.name,
        formatBool(Boolean(check.ok)),
        `<span class="badge ${badgeClass(check.severity)}">${check.severity}</span>`,
        check.message,
      ])
    ) +
    `<div class="metric">
      <div class="metric-label">Next</div>
      <div class="metric-value">${payload.nextAction || "Unavailable"}</div>
    </div>`
  );
}

function renderManualSignoffSection() {
  const statusPayload = getData("/api/calibration/manual-signoff/status");
  const historyPayload = getData("/api/calibration/manual-signoff/history");
  const error =
    getError("/api/calibration/manual-signoff/status") ||
    getError("/api/calibration/manual-signoff/history");

  if (error) {
    return `<div class="error">${error}</div>`;
  }

  if (!statusPayload) {
    return `<div class="muted">No operator sign-off status yet.</div>`;
  }

  const tone =
    statusPayload.status === "SIGNED_OFF"
      ? "ok"
      : statusPayload.status === "REJECTED"
        ? "none"
        : statusPayload.status === "SIGNOFF_STALE" || statusPayload.status === "SIGNOFF_EXPIRED"
          ? "warning"
          : statusPayload.status === "READINESS_NOT_READY"
            ? "error"
            : "blue";
  const latest = statusPayload.latestSignoff;
  const records = (historyPayload?.records || []).slice(0, 5);
  const helperText =
    "This approval only records operator sign-off. It does not apply parameters, reload runtime config, or trigger trading.";

  return (
    renderMetrics([
      { label: "Current Gate", value: `<span class="badge ${badgeClass(tone)}">${statusPayload.status}</span>` },
      { label: "Read Only", value: formatBool(Boolean(statusPayload.readOnly)) },
      { label: "Readiness", value: statusPayload.latestReadinessStatus || "Unavailable" },
      { label: "Signoff Allowed", value: formatBool(Boolean(statusPayload.signoffAllowed)) },
      { label: "Latest Operator", value: latest?.operator || "Unavailable" },
      { label: "Latest Decision", value: latest?.decision || "Unavailable" },
    ]) +
    `<div class="action-row">
      <button type="button" class="small-button" id="refreshManualSignoffButton">Refresh Sign-off Status</button>
      <button type="button" class="small-button" id="approveManualSignoffButton">Approve Manual Gate</button>
      <button type="button" class="small-button" id="rejectManualSignoffButton">Reject Manual Gate</button>
    </div>
    <label class="form-field">
      <span>Operator</span>
      <input id="manualSignoffOperatorInput" placeholder="operator name" />
    </label>
    <label class="form-field">
      <span>Operator Note</span>
      <textarea id="manualSignoffNoteInput" rows="3" placeholder="review note"></textarea>
    </label>
    <div class="muted">${helperText}</div>
    <div class="metric">
      <div class="metric-label">Evidence Fingerprint</div>
      <div class="metric-value">${statusPayload.currentEvidenceFingerprint || "Unavailable"}</div>
    </div>
    <div class="metric">
      <div class="metric-label">Next</div>
      <div class="metric-value">${statusPayload.nextAction || "Unavailable"}</div>
    </div>` +
    (state.latestManualSignoffAction
      ? `<div class="muted">${escapeHtml(state.latestManualSignoffAction)}</div>`
      : "") +
    renderTable(
      ["Signoff ID", "Operator", "Decision", "Readiness", "Created", "Fingerprint"],
      records.map((record) => [
        record.signoffId || "Unavailable",
        record.operator || "Unavailable",
        record.decision || "Unavailable",
        record.readinessStatus || "Unavailable",
        formatDateTime(record.createdAtMs),
        record.evidenceFingerprint || "Unavailable",
      ])
    )
  );
}

function renderManualEvidenceFreshnessSection() {
  const payload = getData("/api/calibration/manual-evidence/freshness");
  const error = getError("/api/calibration/manual-evidence/freshness");

  if (error) {
    return `<div class="error">${error}</div>`;
  }

  if (!payload) {
    return `<div class="muted">No evidence freshness report available yet.</div>`;
  }

  const tone =
    payload.status === "FRESH"
      ? "ok"
      : payload.status === "STALE"
        ? "warning"
        : payload.status === "EXPIRED" || payload.status === "READINESS_NOT_READY"
          ? "error"
          : "none";

  return (
    renderMetrics([
      { label: "Status", value: `<span class="badge ${badgeClass(tone)}">${payload.status}</span>` },
      { label: "Read Only", value: formatBool(Boolean(payload.readOnly)) },
      { label: "Age", value: payload.ageMs == null ? "Unavailable" : `${formatInteger(payload.ageMs)} ms` },
      {
        label: "Expires In",
        value: payload.expiresInMs == null ? "Unavailable" : `${formatInteger(payload.expiresInMs)} ms`,
      },
      { label: "TTL", value: payload.ttlMs == null ? "Unavailable" : `${formatInteger(payload.ttlMs)} ms` },
    ]) +
    `<div class="metric">
      <div class="metric-label">Fingerprint</div>
      <div class="metric-value">${payload.currentEvidenceFingerprint || "Unavailable"}</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Changed Evidence</div>
      <div class="metric-value">${(payload.changedEvidence || []).length ? payload.changedEvidence.join(", ") : "None"}</div>
    </div>` +
    `<div class="metric">
      <div class="metric-label">Next</div>
      <div class="metric-value">${payload.nextAction || "Unavailable"}</div>
    </div>` +
    renderTable(
      ["Evidence", "Present", "Fresh", "Changed", "Message"],
      (payload.checks || []).map((check) => [
        check.name,
        formatBool(Boolean(check.present)),
        formatBool(Boolean(check.fresh)),
        formatBool(Boolean(check.changedSinceSignoff)),
        check.message,
      ])
    )
  );
}

function renderIssueList(issues = []) {
  if (!issues.length) {
    return "None";
  }
  return issues
    .map(
      (issue) =>
        `- ${issue.code}${issue.fieldPath ? ` [${issue.fieldPath}]` : ""}: ${issue.message}`
    )
    .join("<br/>");
}

function renderDelta(numericDelta, percentDelta) {
  if (numericDelta === null || numericDelta === undefined) {
    return "Unavailable";
  }
  const numeric = formatSignedNumber(numericDelta);
  if (percentDelta === null || percentDelta === undefined) {
    return numeric;
  }
  return `${numeric} (${formatSignedPercent(percentDelta)})`;
}

function formatSignedNumber(value, digits = 2) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "Unavailable";
  }
  const sign = Number(value) > 0 ? "+" : "";
  return `${sign}${formatNumber(value, digits)}`;
}

function formatSignedPercent(value) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "Unavailable";
  }
  const sign = Number(value) > 0 ? "+" : "";
  return `${sign}${formatNumber(value * 100, 1)}%`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function renderAll() {
  renderOperatorHomeSummary();
  renderOperatorConsole();
  renderSuspiciousToxicOrders();
  renderMonitorFlow();
  renderVenueStreamDiagnostics();
  renderSuspiciousReplay();
  renderSuspiciousReplayDrilldown();
  renderReplayHeatmap();
  renderWhaleFlowCompactMode();
  renderWhaleFlowCalibration();
  renderWhaleFlowCandidateHistory();
  renderSystemHealth();
  renderSignalSymbolFilter();
  renderToxicSignalHealth();
  renderToxicSignalInbox();
  renderToxicSignalGroups();
  renderToxicSignalDetail();
  renderToxicSignalHistory();
  renderToxicSignalReport();
  renderToxicSignalRolling();
  renderToxicSignalAlertPreview();
  renderDurableArchiveDryRun();
  renderDurableArchiveDryRunReviewPack();
  renderDurableArchiveWriteGate();
  renderDurableArchiveWriteAudit();
  renderWhaleFlowMonitor();
  renderToxicSignalFusion();
  renderToxicReplay();
  renderToxicMarkout();
  renderToxicQualityScorecard();
  renderToxicWeightRecommendation();
  renderToxicWeightReview();
  renderToxicGovernanceLedger();
  renderToxicGovernanceProposal();
  renderToxicGovernanceReviewPack();
  renderToxicGovernanceSignoffPack();
  renderActiveTradeToxicity();
  renderLiquidationToxicity();
  renderOrderbookWallLifecycle();
  renderOrderbookWallInterpretation();
  renderStructuralToxicity();
  renderToxicFlow();
  renderLiqHunt();
  renderVpin();
  renderLiquidation();
  renderVenueHealth();
  renderWindows();
  renderRecentEvents();
  renderReplayReports();
  renderCalibrationReports();
  const lastUpdated = $("lastUpdated");
  if (lastUpdated) {
    lastUpdated.textContent = `Last updated: ${new Date().toLocaleTimeString()}`;
  }
}

let refreshFastInFlight = null;
let refreshSlowInFlight = null;

async function refreshFast() {
  if (refreshFastInFlight) {
    return refreshFastInFlight;
  }
  refreshFastInFlight = (async () => {
    try {
      await refreshGroup(fastEndpoints);
      renderAll();
    } finally {
      refreshFastInFlight = null;
    }
  })();
  return refreshFastInFlight;
}

async function refreshOperatorHome() {
  await refreshGroup([
    "/api/status",
    "/api/toxicity/fusion/status",
    "/api/toxicity/fusion/recent",
    "/api/toxicity/replay/status",
    "/api/toxicity/markout/status",
    "/api/toxicity/quality-scorecard/summary",
    "/api/toxicity/weight-recommendation/summary",
    "/api/toxicity/weight-review/latest",
    "/api/toxicity/governance-ledger/status",
    "/api/calibration/manual-governance/index",
  ]);
  renderOperatorHomeSummary();
}

async function refreshSlow() {
  if (refreshSlowInFlight) {
    return refreshSlowInFlight;
  }
  refreshSlowInFlight = (async () => {
    try {
      await refreshGroup(slowEndpoints);
      renderAll();
    } finally {
      refreshSlowInFlight = null;
    }
  })();
  return refreshSlowInFlight;
}

async function refreshReplayReports() {
  await refreshGroup(replayEndpoints);
  renderAll();
}

async function refreshCalibrationReports() {
  await refreshGroup(calibrationEndpoints);
  renderAll();
}

async function refreshParameterReviews() {
  await refreshGroup(parameterReviewEndpoints);
  renderAll();
}

async function refreshPatchDiffs() {
  await refreshGroup(patchDiffEndpoints);
  renderAll();
}

async function refreshRunbooks() {
  await refreshGroup(runbookEndpoints);
  try {
    const response = await fetch("/api/parameter-review/exports/latest/runbook.md", {
      cache: "no-store",
    });
    if (!response.ok) {
      throw new Error(`/api/parameter-review/exports/latest/runbook.md -> ${response.status}`);
    }
    state.data["/api/parameter-review/exports/latest/runbook.md"] = await response.text();
  } catch (error) {
    state.data["/api/parameter-review/exports/latest/runbook.md"] = `API error: ${error.message}`;
  }
  renderAll();
}

async function refreshDryRuns() {
  await refreshGroup(dryRunEndpoints);
  try {
    const response = await fetch("/api/parameter-review/exports/latest/dry-run.md", {
      cache: "no-store",
    });
    if (!response.ok) {
      throw new Error(`/api/parameter-review/exports/latest/dry-run.md -> ${response.status}`);
    }
    state.data["/api/parameter-review/exports/latest/dry-run.md"] = await response.text();
  } catch (error) {
    state.data["/api/parameter-review/exports/latest/dry-run.md"] = `API error: ${error.message}`;
  }
  renderAll();
}

async function refreshEvidencePacks() {
  await refreshGroup(evidencePackEndpoints);
  try {
    const response = await fetch("/api/parameter-review/exports/latest/evidence-pack.md", {
      cache: "no-store",
    });
    if (!response.ok) {
      throw new Error(`/api/parameter-review/exports/latest/evidence-pack.md -> ${response.status}`);
    }
    state.data["/api/parameter-review/exports/latest/evidence-pack.md"] = await response.text();
  } catch (error) {
    state.data["/api/parameter-review/exports/latest/evidence-pack.md"] = `API error: ${error.message}`;
  }
  renderAll();
}

async function refreshManualStartupCheck() {
  await refreshGroup(startupCheckEndpoints);
  renderAll();
}

async function refreshManualSignoffs() {
  await refreshGroup(signoffEndpoints);
  renderAll();
}

async function refreshManualEvidenceFreshness() {
  await refreshGroup(evidenceFreshnessEndpoints);
  renderAll();
}

async function refreshManualAuditStory() {
  await refreshGroup(auditStoryEndpoints);
  renderAll();
}

async function refreshGovernanceIndex() {
  await refreshGroup(governanceEndpoints);
  renderAll();
}

async function loadReport(fileName) {
  state.selectedReport = fileName;
  state.selectedReportContent = "Loading report...";
  renderReplayReports();
  try {
    const payload = await fetchJson(`/api/replay-reports/${encodeURIComponent(fileName)}`);
    state.selectedReportContent = payload.content || "Unavailable";
  } catch (error) {
    state.selectedReportContent = `API error: ${error.message}`;
  }
  renderReplayReports();
}

async function loadCalibrationReport(reportId) {
  state.selectedCalibrationReport = reportId;
  state.selectedCalibrationReportContent = { markdownContent: "Loading report..." };
  renderCalibrationReports();
  try {
    const payload = await fetchJson(`/api/calibration/reports/${encodeURIComponent(reportId)}`);
    state.selectedCalibrationReportContent = payload.report || null;
  } catch (error) {
    state.selectedCalibrationReportContent = { markdownContent: `API error: ${error.message}` };
  }
  renderCalibrationReports();
}

async function loadRunbook(exportId) {
  state.selectedRunbookExportId = exportId;
  state.selectedRunbook = null;
  state.selectedRunbookMarkdown = "Loading runbook...";
  renderCalibrationReports();
  try {
    const [jsonResponse, markdownResponse] = await Promise.all([
      fetch(`/api/parameter-review/exports/${encodeURIComponent(exportId)}/runbook`, {
        cache: "no-store",
      }),
      fetch(`/api/parameter-review/exports/${encodeURIComponent(exportId)}/runbook.md`, {
        cache: "no-store",
      }),
    ]);
    if (!jsonResponse.ok) {
      throw new Error(`/api/parameter-review/exports/${exportId}/runbook -> ${jsonResponse.status}`);
    }
    if (!markdownResponse.ok) {
      throw new Error(`/api/parameter-review/exports/${exportId}/runbook.md -> ${markdownResponse.status}`);
    }
    state.selectedRunbook = await jsonResponse.json();
    state.selectedRunbookMarkdown = await markdownResponse.text();
  } catch (error) {
    state.selectedRunbook = null;
    state.selectedRunbookMarkdown = `API error: ${error.message}`;
  }
  renderCalibrationReports();
}

async function loadLatestRunbook() {
  state.selectedRunbookExportId = null;
  state.selectedRunbook = null;
  state.selectedRunbookMarkdown = null;
  await refreshRunbooks();
}

async function loadDryRun(exportId) {
  state.selectedDryRunExportId = exportId;
  state.selectedDryRun = null;
  state.selectedDryRunMarkdown = "Loading dry-run...";
  renderCalibrationReports();
  try {
    const [jsonResponse, markdownResponse] = await Promise.all([
      fetch(`/api/parameter-review/exports/${encodeURIComponent(exportId)}/dry-run`, {
        cache: "no-store",
      }),
      fetch(`/api/parameter-review/exports/${encodeURIComponent(exportId)}/dry-run.md`, {
        cache: "no-store",
      }),
    ]);
    if (!jsonResponse.ok) {
      throw new Error(`/api/parameter-review/exports/${exportId}/dry-run -> ${jsonResponse.status}`);
    }
    if (!markdownResponse.ok) {
      throw new Error(`/api/parameter-review/exports/${exportId}/dry-run.md -> ${markdownResponse.status}`);
    }
    state.selectedDryRun = await jsonResponse.json();
    state.selectedDryRunMarkdown = await markdownResponse.text();
  } catch (error) {
    state.selectedDryRun = null;
    state.selectedDryRunMarkdown = `API error: ${error.message}`;
  }
  renderCalibrationReports();
}

async function loadLatestDryRun() {
  state.selectedDryRunExportId = null;
  state.selectedDryRun = null;
  state.selectedDryRunMarkdown = null;
  await refreshDryRuns();
}

async function loadEvidencePack(exportId) {
  state.selectedEvidencePackExportId = exportId;
  state.selectedEvidencePack = null;
  state.selectedEvidencePackMarkdown = "Loading evidence pack...";
  renderCalibrationReports();
  try {
    const [jsonResponse, markdownResponse] = await Promise.all([
      fetch(`/api/parameter-review/exports/${encodeURIComponent(exportId)}/evidence-pack`, {
        cache: "no-store",
      }),
      fetch(`/api/parameter-review/exports/${encodeURIComponent(exportId)}/evidence-pack.md`, {
        cache: "no-store",
      }),
    ]);
    if (!jsonResponse.ok) {
      throw new Error(`/api/parameter-review/exports/${exportId}/evidence-pack -> ${jsonResponse.status}`);
    }
    if (!markdownResponse.ok) {
      throw new Error(`/api/parameter-review/exports/${exportId}/evidence-pack.md -> ${markdownResponse.status}`);
    }
    state.selectedEvidencePack = await jsonResponse.json();
    state.selectedEvidencePackMarkdown = await markdownResponse.text();
  } catch (error) {
    state.selectedEvidencePack = null;
    state.selectedEvidencePackMarkdown = `API error: ${error.message}`;
  }
  renderCalibrationReports();
}

async function loadLatestEvidencePack() {
  state.selectedEvidencePackExportId = null;
  state.selectedEvidencePack = null;
  state.selectedEvidencePackMarkdown = null;
  await refreshEvidencePacks();
}

async function submitParameterReview(recommendationId, reportId, status) {
  const note = document.querySelector(`[data-review-note="${CSS.escape(recommendationId)}"]`)?.value || "";
  const reviewer =
    document.querySelector(`[data-reviewer-input="${CSS.escape(recommendationId)}"]`)?.value || "";
  try {
    const response = await fetch("/api/parameter-review/reviews", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        recommendation_id: recommendationId,
        report_id: reportId,
        status,
        reviewer_note: note || null,
        reviewer: reviewer || null,
      }),
    });
    if (!response.ok) {
      throw new Error(`/api/parameter-review/reviews -> ${response.status}`);
    }
    await refreshParameterReviews();
  } catch (error) {
    alert(`Review write failed: ${error.message}`);
  }
}

async function generateManualPatch() {
  try {
    const response = await fetch("/api/parameter-review/exports", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        include_statuses: ["ApprovedForManualApply"],
        operator: "manual",
        note: "R14 manual export from dashboard",
      }),
    });
    if (!response.ok) {
      throw new Error(`/api/parameter-review/exports -> ${response.status}`);
    }
    await refreshParameterReviews();
    await refreshPatchDiffs();
  } catch (error) {
    alert(`Manual export failed: ${error.message}`);
  }
}

async function submitManualSignoff(decision) {
  const operator = $("manualSignoffOperatorInput")?.value?.trim() || "";
  const note = $("manualSignoffNoteInput")?.value?.trim() || "";
  try {
    const response = await fetch("/api/calibration/manual-signoff", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        operator: operator || "manual-operator",
        decision,
        note: note || null,
      }),
    });
    if (!response.ok) {
      const payload = await response.json().catch(() => ({}));
      throw new Error(payload.reason || `/api/calibration/manual-signoff -> ${response.status}`);
    }
    const payload = await response.json();
    state.latestManualSignoffAction = `Latest sign-off recorded: ${payload.status}`;
    await refreshManualSignoffs();
    await refreshManualStartupCheck();
  } catch (error) {
    state.latestManualSignoffAction = `Sign-off write failed: ${error.message}`;
    renderAll();
  }
}

async function copyManualAuditStoryMarkdown() {
  const payload = getData("/api/calibration/manual-audit-story");
  if (!payload?.markdown) {
    state.latestManualAuditStoryAction = "Audit story markdown is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(payload.markdown);
    state.latestManualAuditStoryAction = "Markdown audit story copied to clipboard.";
  } catch (error) {
    state.latestManualAuditStoryAction = `Audit story copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyGovernanceMarkdown() {
  const payload = getData("/api/calibration/manual-governance/index");
  if (!payload?.markdown) {
    state.latestGovernanceAction = "Governance markdown is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(payload.markdown);
    state.latestGovernanceAction = "Governance markdown copied to clipboard.";
  } catch (error) {
    state.latestGovernanceAction = `Governance markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function refreshSuspiciousReplay() {
  const statusUrl = suspiciousReplayStatusUrl();
  const historyUrl = suspiciousReplayHistoryUrl();
  state.suspiciousReplayError = null;
  try {
    state.suspiciousReplayStatusPayload = await fetchJson(statusUrl);
    state.suspiciousReplayHistoryPayload = await fetchJson(historyUrl);
    await refreshSuspiciousReplayOverlay(suspiciousReplayOverlaySymbol() || suspiciousReplaySelectedSymbol());
    state.latestSuspiciousReplayAction = suspiciousReplaySelectedSymbol()
      ? `Loaded replay history for ${suspiciousReplaySelectedSymbol()}.`
      : "Loaded replay history for the current read-only view.";
    if (state.suspiciousReplaySignalId) {
      await loadSuspiciousReplayBySignalId(true);
      return;
    }
  } catch (error) {
    state.suspiciousReplayError = `Replay refresh failed: ${error.message}`;
    state.latestSuspiciousReplayAction = state.suspiciousReplayError;
  }
  renderAll();
}

async function refreshReplayHeatmap(skipSync = false) {
  if (!skipSync) {
    syncReplayHeatmapFiltersFromControls();
  }

  const historyUrl = replayHeatmapHistoryUrl();
  const rollingUrl = replayHeatmapRollingUrl();
  state.replayHeatmapError = null;

  try {
    state.replayHeatmapHistoryPayload = await fetchJson(historyUrl);
    state.replayHeatmapLastHistoryUrl = historyUrl;
  } catch (error) {
    state.replayHeatmapHistoryPayload = null;
    state.replayHeatmapBuiltPayload = null;
    state.replayHeatmapError = `Replay heatmap history failed: ${error.message}`;
    state.latestReplayHeatmapAction = state.replayHeatmapError;
    renderAll();
    return;
  }

  const replayHeatmapSymbols = [
    ...new Set(
      (state.replayHeatmapHistoryPayload?.items || [])
        .map((item) => String(item.symbol || "").trim().toUpperCase())
        .filter(Boolean)
    ),
  ];
  if (replayHeatmapSymbols.length) {
    await refreshGroup(
      replayHeatmapSymbols
        .map((symbol) => whaleFlowSymbolUrl(symbol))
        .filter(Boolean)
    );
  }

  try {
    state.replayHeatmapRollingPayload = await fetchJson(rollingUrl);
    state.replayHeatmapLastRollingUrl = rollingUrl;
  } catch (error) {
    state.replayHeatmapRollingPayload = null;
    state.replayHeatmapLastRollingUrl = rollingUrl;
  }

  state.replayHeatmapBuiltPayload = buildReplayHeatmapPayload();
  state.latestReplayHeatmapAction = replayHeatmapNormalizedSymbolFilter()
    ? `Replay heatmap refreshed for ${replayHeatmapNormalizedSymbolFilter()}.`
    : "Replay heatmap refreshed for the current read-only view.";
  renderAll();
}

async function buildReplayHeatmap() {
  syncReplayHeatmapFiltersFromControls();
  const historyUrl = replayHeatmapHistoryUrl();
  if (!state.replayHeatmapHistoryPayload || state.replayHeatmapLastHistoryUrl !== historyUrl) {
    await refreshReplayHeatmap(true);
    return;
  }

  state.replayHeatmapError = null;
  state.replayHeatmapBuiltPayload = buildReplayHeatmapPayload();
  state.latestReplayHeatmapAction = replayHeatmapNormalizedSymbolFilter()
    ? `Replay heatmap built for ${replayHeatmapNormalizedSymbolFilter()}.`
    : "Replay heatmap built for the current read-only view.";
  renderAll();
}

function clearReplayHeatmapFilter() {
  state.replayHeatmapSymbolFilter = "";
  state.replayHeatmapSignalKindFilter = "";
  state.replayHeatmapDirectionFilter = "";
  state.replayHeatmapHistoryPayload = null;
  state.replayHeatmapRollingPayload = null;
  state.replayHeatmapLastHistoryUrl = null;
  state.replayHeatmapLastRollingUrl = null;
  state.replayHeatmapError = null;
  state.replayHeatmapBuiltPayload = buildReplayHeatmapPayload();
  state.latestReplayHeatmapAction = "Replay heatmap filter cleared.";
  renderAll();
}

async function copyReplayHeatmapJson() {
  if (!state.replayHeatmapBuiltPayload) {
    await buildReplayHeatmap();
    if (!state.replayHeatmapBuiltPayload) {
      return;
    }
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(state.replayHeatmapBuiltPayload, null, 2));
    state.latestReplayHeatmapAction = "Replay heatmap JSON copied to clipboard.";
  } catch (error) {
    state.latestReplayHeatmapAction = `Replay heatmap JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyReplayHeatmapMarkdown() {
  if (!state.replayHeatmapBuiltPayload) {
    await buildReplayHeatmap();
    if (!state.replayHeatmapBuiltPayload) {
      return;
    }
  }

  try {
    await navigator.clipboard.writeText(state.replayHeatmapBuiltPayload.markdown || "");
    state.latestReplayHeatmapAction = "Replay heatmap Markdown copied to clipboard.";
  } catch (error) {
    state.latestReplayHeatmapAction = `Replay heatmap Markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function refreshSuspiciousReplayOverlay(symbolOverride = null) {
  const symbol = (symbolOverride || suspiciousReplayOverlaySymbol() || "").trim();
  if (!symbol) {
    return;
  }

  await refreshGroup([
    whaleFlowSymbolUrl(symbol),
    toxicMarkoutSymbolUrl(symbol),
    toxicQualityScorecardSymbolUrl(symbol),
    toxicWeightRecommendationSymbolUrl(symbol),
    toxicWeightReviewSymbolUrl(symbol),
    toxicGovernanceLedgerSymbolUrl(symbol),
  ].filter(Boolean));
}

async function refreshWhaleReplayOverlay() {
  await refreshSuspiciousReplayOverlay();
  state.latestSuspiciousReplayAction = suspiciousReplayOverlaySymbol()
    ? `Whale overlay refreshed for ${suspiciousReplayOverlaySymbol()}.`
    : "Whale overlay refresh skipped: symbol unavailable.";
  renderAll();
}

async function loadWhaleReplayOverlayBySymbol() {
  const input = $("suspiciousReplaySymbolInput");
  state.suspiciousReplaySymbol = input?.value?.trim() || state.suspiciousReplaySymbol || null;
  await refreshSuspiciousReplayOverlay(state.suspiciousReplaySymbol || suspiciousReplayOverlaySymbol());
  state.latestSuspiciousReplayAction = state.suspiciousReplaySymbol
    ? `Loaded whale overlay for ${state.suspiciousReplaySymbol}.`
    : "Whale overlay symbol is unavailable.";
  renderAll();
}

async function loadWhaleReplayOverlayBySignalId() {
  await loadSuspiciousReplayBySignalId(true);
  state.latestSuspiciousReplayAction = state.suspiciousReplaySignalId
    ? `Loaded whale overlay for ${state.suspiciousReplaySignalId}.`
    : "Whale overlay signal is unavailable.";
  renderAll();
}

async function loadSuspiciousReplayBySymbol() {
  const input = $("suspiciousReplaySymbolInput");
  state.suspiciousReplaySymbol = input?.value?.trim() || null;
  state.suspiciousReplaySignalId = null;
  state.suspiciousReplayLookupPayload = null;
  state.suspiciousReplayDetailPayload = null;
  state.suspiciousReplayExplainPayload = null;
  await refreshSuspiciousReplay();
}

async function loadSuspiciousReplayBySignalId(silent = false) {
  const input = $("suspiciousReplaySignalIdInput");
  state.suspiciousReplaySignalId = input?.value?.trim() || state.suspiciousReplaySignalId || null;
  if (!state.suspiciousReplaySignalId) {
    state.suspiciousReplayLookupPayload = null;
    state.suspiciousReplayDetailPayload = null;
    state.suspiciousReplayExplainPayload = null;
    state.latestSuspiciousReplayAction = "Signal replay lookup cleared.";
    renderAll();
    return;
  }

  const signalId = state.suspiciousReplaySignalId;
  const lookupUrl = suspiciousReplaySignalUrl(signalId);
  const symbol = suspiciousReplaySelectedSignalSymbol();
  const detailUrl = suspiciousReplayDetailUrl(signalId, symbol);
  const explainUrl = suspiciousReplayExplainUrl(signalId, symbol);
  state.suspiciousReplayError = null;

  try {
    state.suspiciousReplayLookupPayload = lookupUrl
      ? await fetchJson(lookupUrl)
      : { found: false, signalId, reason: "Signal not found" };
  } catch (error) {
    state.suspiciousReplayLookupPayload = {
      found: false,
      signalId,
      reason: `Signal not found: ${error.message}`,
    };
  }

  try {
    state.suspiciousReplayDetailPayload = detailUrl
      ? await fetchJson(detailUrl)
      : { available: false, reason: "Detail unavailable" };
  } catch (error) {
    state.suspiciousReplayDetailPayload = {
      available: false,
      reason: `Detail unavailable: ${error.message}`,
    };
  }

  try {
    state.suspiciousReplayExplainPayload = explainUrl
      ? await fetchJson(explainUrl)
      : { found: false, signalId, reason: "Alert explanation unavailable" };
  } catch (error) {
    state.suspiciousReplayExplainPayload = {
      found: false,
      signalId,
      alertDecision: "not_found",
      reason: `Alert explanation unavailable: ${error.message}`,
    };
  }

  await refreshSuspiciousReplayOverlay(suspiciousReplaySelectedSignalSymbol() || suspiciousReplaySelectedSymbol());
  state.latestSuspiciousReplayAction = silent
    ? `Replay refreshed for ${signalId}.`
    : `Loaded replay drilldown for ${signalId}.`;
  renderAll();
}

async function clearSuspiciousReplayFilter() {
  state.suspiciousReplaySymbol = null;
  state.suspiciousReplaySignalId = null;
  state.suspiciousReplayStatusPayload = null;
  state.suspiciousReplayHistoryPayload = null;
  state.suspiciousReplayLookupPayload = null;
  state.suspiciousReplayDetailPayload = null;
  state.suspiciousReplayExplainPayload = null;
  state.suspiciousReplayError = null;
  state.latestSuspiciousReplayAction = "Replay filter cleared.";
  renderAll();
}

async function copySuspiciousReplayJson() {
  const payload = buildSuspiciousReplayCopyPayload();
  if (!payload.status && !payload.history && !payload.lookup && !payload.detail && !payload.alertExplainability) {
    state.latestSuspiciousReplayAction = "Replay JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestSuspiciousReplayAction = "Replay JSON copied to clipboard.";
  } catch (error) {
    state.latestSuspiciousReplayAction = `Replay JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyWhaleReplayOverlayJson() {
  const overlay = buildWhaleReplayOverlay();
  if (!overlay.available && !overlay.reason) {
    state.latestSuspiciousReplayAction = "Whale overlay JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify({ whaleFlowOverlay: overlay }, null, 2));
    state.latestSuspiciousReplayAction = "Whale overlay JSON copied to clipboard.";
  } catch (error) {
    state.latestSuspiciousReplayAction = `Whale overlay JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyWhaleReplayOverlayMarkdown() {
  const overlay = buildWhaleReplayOverlay();
  try {
    await navigator.clipboard.writeText(buildWhaleReplayOverlayMarkdown(overlay));
    state.latestSuspiciousReplayAction = "Whale overlay Markdown copied to clipboard.";
  } catch (error) {
    state.latestSuspiciousReplayAction = `Whale overlay Markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyActiveTradeToxicityJson() {
  const payload = getActiveTradeToxicityPayload();
  if (!payload) {
    state.latestActiveTradeToxicityAction = "Active trade toxicity JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestActiveTradeToxicityAction = "Active trade toxicity JSON copied to clipboard.";
  } catch (error) {
    state.latestActiveTradeToxicityAction = `Active trade toxicity copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyOrderbookWallJson() {
  const payload = getOrderbookWallPayload();
  if (!payload) {
    state.latestOrderbookWallAction = "Orderbook wall lifecycle JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestOrderbookWallAction = "Orderbook wall lifecycle JSON copied to clipboard.";
  } catch (error) {
    state.latestOrderbookWallAction = `Orderbook wall lifecycle copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyOrderbookWallInterpretationJson() {
  const payload = getOrderbookWallInterpretationPayload();
  if (!payload) {
    state.latestOrderbookWallInterpretationAction =
      "Orderbook wall interpretation JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestOrderbookWallInterpretationAction =
      "Orderbook wall interpretation JSON copied to clipboard.";
  } catch (error) {
    state.latestOrderbookWallInterpretationAction =
      `Orderbook wall interpretation copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyStructuralToxicityJson() {
  const payload = getStructuralToxicityPayload();
  if (!payload) {
    state.latestStructuralToxicityAction = "Structural toxicity JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestStructuralToxicityAction = "Structural toxicity JSON copied to clipboard.";
  } catch (error) {
    state.latestStructuralToxicityAction = `Structural toxicity copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyWhaleFlowMonitorJson() {
  const payload = getWhaleFlowPayload();
  if (!payload) {
    state.latestWhaleFlowAction = "Whale-flow JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestWhaleFlowAction = "Whale-flow JSON copied to clipboard.";
  } catch (error) {
    state.latestWhaleFlowAction = `Whale-flow copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyVenueDiagnosticsJson() {
  const payload = getData("/api/venues/diagnostics");
  if (!payload) {
    state.latestVenueDiagnosticsAction = "Venue diagnostics JSON is unavailable.";
    renderVenueStreamDiagnostics();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestVenueDiagnosticsAction = "Venue diagnostics JSON copied to clipboard.";
  } catch (error) {
    state.latestVenueDiagnosticsAction = `Venue diagnostics copy failed: ${error.message}`;
  }
  renderVenueStreamDiagnostics();
}

function setWhaleFlowCompactPreset(preset) {
  state.whaleFlowCompactPreset = preset || "all";
  state.latestWhaleFlowCompactAction =
    `Current preset switched to ${whaleFlowCompactPresetLabel(state.whaleFlowCompactPreset)}.`;
  renderAll();
}

function resetWhaleFlowCompactPreset() {
  state.whaleFlowCompactPreset = "all";
  state.latestWhaleFlowCompactAction = "Whale Flow Compact View reset to All.";
  renderAll();
}

async function copyWhaleFlowCompactPresetJson() {
  const payload = buildWhaleFlowCompactCopyPayload();
  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestWhaleFlowCompactAction = "Whale Flow Compact View JSON copied to clipboard.";
  } catch (error) {
    state.latestWhaleFlowCompactAction = `Whale Flow Compact View copy failed: ${error.message}`;
  }
  renderAll();
}

async function loadWhaleFlowCalibrationBySymbol() {
  const input = $("whaleFlowCalibrationSymbolInput");
  state.whaleFlowCalibrationSymbol = input?.value?.trim()?.toUpperCase() || "";
  state.latestWhaleFlowCalibrationAction = state.whaleFlowCalibrationSymbol
    ? `Loading whale-flow calibration for ${state.whaleFlowCalibrationSymbol}.`
    : "Loading whale-flow calibration for the default symbol.";
  await refreshWhaleFlowCalibration();
}

async function copyWhaleFlowCalibrationJson() {
  const payload = getWhaleFlowCalibrationPayload();
  if (!payload) {
    state.latestWhaleFlowCalibrationAction = "Whale-flow calibration JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestWhaleFlowCalibrationAction =
      "Whale-flow calibration JSON copied to clipboard.";
  } catch (error) {
    state.latestWhaleFlowCalibrationAction =
      `Whale-flow calibration copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyWhaleFlowCalibrationMarkdown() {
  const payload = getWhaleFlowCalibrationPayload();
  if (!payload) {
    state.latestWhaleFlowCalibrationAction =
      "Whale-flow calibration Markdown is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(payload.markdown || "");
    state.latestWhaleFlowCalibrationAction =
      "Whale-flow calibration Markdown copied to clipboard.";
  } catch (error) {
    state.latestWhaleFlowCalibrationAction =
      `Whale-flow calibration Markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function loadWhaleFlowCandidateHistoryBySymbol() {
  const input = $("whaleFlowCandidateHistorySymbolInput");
  state.whaleFlowCandidateHistorySymbol = input?.value?.trim()?.toUpperCase() || "";
  state.latestWhaleFlowCandidateHistoryAction = state.whaleFlowCandidateHistorySymbol
    ? `Loading whale candidate history for ${state.whaleFlowCandidateHistorySymbol}.`
    : "Loading whale candidate history for the default symbol.";
  await refreshWhaleFlowCandidateHistory();
}

async function copyWhaleFlowCandidateHistoryJson() {
  const payload = {
    status: getWhaleFlowCandidateHistoryStatusPayload(),
    recent: getWhaleFlowCandidateHistoryPayload(),
  };
  if (!payload.status && !payload.recent) {
    state.latestWhaleFlowCandidateHistoryAction =
      "Whale candidate history JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestWhaleFlowCandidateHistoryAction =
      "Whale candidate history JSON copied to clipboard.";
  } catch (error) {
    state.latestWhaleFlowCandidateHistoryAction =
      `Whale candidate history copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalFusionJson() {
  const payload = getToxicSignalFusionPayload();
  if (!payload) {
    state.latestToxicSignalFusionAction = "Toxic signal fusion JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicSignalFusionAction = "Toxic signal fusion JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalFusionAction = `Toxic signal fusion copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalInboxJson() {
  const payload = getToxicSignalInboxPayload();
  if (!payload) {
    state.latestToxicSignalInboxAction = "Signal inbox JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicSignalInboxAction = "Signal inbox JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalInboxAction = `Signal inbox copy failed: ${error.message}`;
  }
  renderAll();
}

async function applySignalSymbolFilter() {
  const input = $("signalSymbolFilterInput");
  const nextValue = input?.value?.trim().toUpperCase() || "";
  state.signalSymbolFilter = nextValue || null;
  state.latestSignalSymbolFilterAction = state.signalSymbolFilter
    ? `Filter applied: ${state.signalSymbolFilter}`
    : "Filter cleared to runtime default view.";
  await Promise.all([
    refreshToxicSignalHealth(),
    refreshToxicSignalInbox(),
    refreshToxicSignalGroups(),
    refreshToxicSignalDetail(),
    refreshToxicSignalHistory(),
    refreshToxicSignalReport(),
    refreshToxicSignalRolling(),
  ]);
  renderAll();
}

async function clearSignalSymbolFilter() {
  state.signalSymbolFilter = null;
  state.latestSignalSymbolFilterAction = "Filter cleared to runtime default view.";
  await Promise.all([
    refreshToxicSignalHealth(),
    refreshToxicSignalInbox(),
    refreshToxicSignalGroups(),
    refreshToxicSignalDetail(),
    refreshToxicSignalHistory(),
    refreshToxicSignalReport(),
    refreshToxicSignalRolling(),
  ]);
  renderAll();
}

async function copyFilteredSignalJson() {
  const payload = {
    filter: {
      symbol: signalSymbolFilterValue() || null,
      viewOnly: true,
      persistentWatchlistEnabled: false,
      runtimeMonitorModified: false,
    },
    inbox: getToxicSignalInboxPayload(),
    groups: getToxicSignalGroupsPayload(),
    detail: state.toxicSignalDetailPayload,
    dailyReport: getToxicSignalReportPayload(),
  };

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestSignalSymbolFilterAction = "Filtered signal JSON copied to clipboard.";
  } catch (error) {
    state.latestSignalSymbolFilterAction = `Filtered signal JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalGroupsJson() {
  const payload = getToxicSignalGroupsPayload();
  if (!payload) {
    state.latestToxicSignalGroupAction = "Signal groups JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicSignalGroupAction = "Signal groups JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalGroupAction = `Signal groups copy failed: ${error.message}`;
  }
  renderAll();
}

async function loadToxicSignalDetail() {
  const input = $("toxicSignalDetailSignalIdInput");
  const signalId = input?.value?.trim() || state.toxicSignalDetailSignalId || "";
  if (!signalId) {
    state.latestToxicSignalDetailAction = "Signal ID is required.";
    renderAll();
    return;
  }

  state.toxicSignalDetailSignalId = signalId;
  const symbol = toxicSignalDetailSelectedSymbol();
  const url =
    toxicSignalDetailSignalEndpointTemplate.replace(":signal_id", encodeURIComponent(signalId)) +
    `?symbol=${encodeURIComponent(symbol)}`;
  try {
    state.toxicSignalDetailPayload = await fetchJson(url);
    state.latestToxicSignalDetailAction = `Loaded signal detail: ${signalId}`;
  } catch (error) {
    state.toxicSignalDetailPayload = { available: false, reason: error.message };
    state.latestToxicSignalDetailAction = `Signal detail load failed: ${error.message}`;
  }
  renderAll();
}

async function loadToxicSignalGroupDetail() {
  const input = $("toxicSignalDetailGroupIdInput");
  const groupId = input?.value?.trim() || state.toxicSignalDetailGroupId || "";
  if (!groupId) {
    state.latestToxicSignalDetailAction = "Group ID is required.";
    renderAll();
    return;
  }

  state.toxicSignalDetailGroupId = groupId;
  const symbol = toxicSignalDetailSelectedSymbol();
  const url =
    toxicSignalDetailGroupEndpointTemplate.replace(":group_id", encodeURIComponent(groupId)) +
    `?symbol=${encodeURIComponent(symbol)}`;
  try {
    state.toxicSignalDetailPayload = await fetchJson(url);
    state.latestToxicSignalDetailAction = `Loaded group detail: ${groupId}`;
  } catch (error) {
    state.toxicSignalDetailPayload = { available: false, reason: error.message };
    state.latestToxicSignalDetailAction = `Group detail load failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalDetailJson() {
  const payload = state.toxicSignalDetailPayload;
  if (!payload) {
    state.latestToxicSignalDetailAction = "Signal detail JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicSignalDetailAction = "Signal detail JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalDetailAction = `Signal detail copy failed: ${error.message}`;
  }
  renderAll();
}

async function loadToxicSignalHealthBySymbol() {
  const input = $("toxicSignalHealthSymbolInput");
  state.toxicSignalHealthSymbol = input?.value?.trim() || null;
  state.latestToxicSignalHealthAction = state.toxicSignalHealthSymbol
    ? `Loaded signal health for ${state.toxicSignalHealthSymbol.toUpperCase()}.`
    : "Loaded signal health for the current view.";
  await refreshToxicSignalHealth();
  renderAll();
}

async function loadToxicSignalHistoryBySymbol() {
  const input = $("toxicSignalHistorySymbolInput");
  state.toxicSignalHistorySymbol = input?.value?.trim() || null;
  state.latestToxicSignalHistoryAction = state.toxicSignalHistorySymbol
    ? `Loaded signal history for ${state.toxicSignalHistorySymbol.toUpperCase()}.`
    : "Loaded signal history for the current view.";
  await refreshToxicSignalHistory();
  renderAll();
}

async function loadToxicSignalHistorySignalById() {
  const input = $("toxicSignalHistorySignalIdInput");
  state.toxicSignalHistorySignalId = input?.value?.trim() || null;
  if (!state.toxicSignalHistorySignalId) {
    state.toxicSignalHistoryLookupPayload = null;
    state.latestToxicSignalHistoryAction = "Signal history lookup cleared.";
    renderAll();
    return;
  }

  await refreshToxicSignalHistory();
  state.latestToxicSignalHistoryAction = `Loaded retained signal ${state.toxicSignalHistorySignalId}.`;
  renderAll();
}

async function copyToxicSignalHistoryJson() {
  const payload = {
    status: getToxicSignalHistoryStatusPayload(),
    recent: getToxicSignalHistoryPayload(),
    alerts: getToxicSignalHistoryAlertsPayload(),
    reports: getToxicSignalHistoryReportsPayload(),
    lookup: state.toxicSignalHistoryLookupPayload,
  };
  if (!payload.status || !payload.recent) {
    state.latestToxicSignalHistoryAction = "Signal history JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicSignalHistoryAction = "Signal history JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalHistoryAction = `Signal history JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function exportToxicSignalHistoryJson() {
  const payload = {
    status: getToxicSignalHistoryStatusPayload(),
    recent: getToxicSignalHistoryPayload(),
    alerts: getToxicSignalHistoryAlertsPayload(),
    reports: getToxicSignalHistoryReportsPayload(),
    lookup: state.toxicSignalHistoryLookupPayload,
  };
  if (!payload.status || !payload.recent) {
    state.latestToxicSignalHistoryAction = "Signal history export is unavailable.";
    renderAll();
    return;
  }

  try {
    downloadJsonFile("toxic-signal-history.json", payload);
    state.latestToxicSignalHistoryAction = "Signal history JSON exported.";
  } catch (error) {
    state.latestToxicSignalHistoryAction = `Signal history export failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalHealthJson() {
  const payload = getToxicSignalHealthPayload();
  if (!payload) {
    state.latestToxicSignalHealthAction = "Signal health JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicSignalHealthAction = "Signal health JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalHealthAction = `Signal health JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalReportJson() {
  const payload = getToxicSignalReportPayload();
  if (!payload) {
    state.latestToxicSignalReportAction = "Daily report JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicSignalReportAction = "Daily report JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalReportAction = `Daily report JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalReportMarkdown() {
  const payload = getToxicSignalReportPayload();
  if (!payload?.markdown) {
    state.latestToxicSignalReportAction = "Daily report markdown is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(payload.markdown);
    state.latestToxicSignalReportAction = "Daily report markdown copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalReportAction = `Daily report markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalRollingJson() {
  const payload = getToxicSignalRollingPayload();
  if (!payload) {
    state.latestToxicSignalRollingAction = "Rolling digest JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicSignalRollingAction = "Rolling digest JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalRollingAction = `Rolling digest JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalRollingMarkdown() {
  const payload = getToxicSignalRollingPayload();
  if (!payload?.markdown) {
    state.latestToxicSignalRollingAction = "Rolling digest markdown is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(payload.markdown);
    state.latestToxicSignalRollingAction = "Rolling digest markdown copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalRollingAction =
      `Rolling digest markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalAlertPreviewJson() {
  const payload = getToxicSignalAlertPreviewPayload();
  if (!payload) {
    state.latestToxicSignalAlertPreviewAction = "Alert preview JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicSignalAlertPreviewAction =
      "Alert preview JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalAlertPreviewAction =
      `Alert preview JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function loadToxicSignalAlertExplain() {
  const input = $("toxicSignalAlertExplainSignalIdInput");
  state.toxicSignalAlertExplainSignalId = input?.value?.trim() || null;
  if (!state.toxicSignalAlertExplainSignalId) {
    state.toxicSignalAlertExplainPayload = null;
    state.latestToxicSignalAlertPreviewAction = "Alert explanation lookup cleared.";
    renderAll();
    return;
  }

  const url = toxicSignalAlertPreviewExplainUrl();
  try {
    state.toxicSignalAlertExplainPayload = await fetchJson(url);
    state.latestToxicSignalAlertPreviewAction =
      `Loaded alert explanation for ${state.toxicSignalAlertExplainSignalId}.`;
  } catch (error) {
    state.toxicSignalAlertExplainPayload = {
      found: false,
      signalId: state.toxicSignalAlertExplainSignalId,
      alertDecision: "not_found",
      reason: error.message,
    };
    state.latestToxicSignalAlertPreviewAction =
      `Alert explanation load failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalAlertExplainJson() {
  const payload = getToxicSignalAlertPreviewExplainPayload();
  if (!payload) {
    state.latestToxicSignalAlertPreviewAction =
      "Alert explanation JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicSignalAlertPreviewAction =
      "Alert explanation JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalAlertPreviewAction =
      `Alert explanation JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyDurableArchiveDryRunJson() {
  const payload = getDurableArchiveDryRunPayload();
  if (!payload) {
    state.latestDurableArchiveDryRunAction = "Dry-run JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestDurableArchiveDryRunAction = "Dry-run JSON copied to clipboard.";
  } catch (error) {
    state.latestDurableArchiveDryRunAction = `Dry-run JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyDurableArchiveDryRunReviewPackJson() {
  const payload = getDurableArchiveDryRunReviewPackPayload();
  if (!payload) {
    state.latestDurableArchiveDryRunReviewPackAction = "Review pack JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestDurableArchiveDryRunReviewPackAction =
      "Review pack JSON copied to clipboard.";
  } catch (error) {
    state.latestDurableArchiveDryRunReviewPackAction =
      `Review pack JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyDurableArchiveDryRunReviewPackMarkdown() {
  const payload = getDurableArchiveDryRunReviewPackPayload();
  if (!payload?.markdown) {
    state.latestDurableArchiveDryRunReviewPackAction =
      "Review pack markdown is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(payload.markdown);
    state.latestDurableArchiveDryRunReviewPackAction =
      "Review pack markdown copied to clipboard.";
  } catch (error) {
    state.latestDurableArchiveDryRunReviewPackAction =
      `Review pack markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyDurableArchiveWriteGateJson() {
  const payload = getDurableArchiveWriteGatePayload();
  if (!payload) {
    state.latestDurableArchiveWriteGateAction = "Write gate JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestDurableArchiveWriteGateAction = "Write gate JSON copied to clipboard.";
  } catch (error) {
    state.latestDurableArchiveWriteGateAction =
      `Write gate JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyDurableArchiveWriteAuditJson() {
  const statusPayload = getDurableArchiveWriteAuditStatusPayload();
  const recentPayload = getDurableArchiveWriteAuditRecentPayload();
  const latestPayload = getDurableArchiveWriteAuditLatestPayload();
  if (!statusPayload && !recentPayload && !latestPayload) {
    state.latestDurableArchiveWriteAuditAction = "Write audit JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(
      JSON.stringify(
        {
          status: statusPayload || null,
          recent: recentPayload || null,
          latest: latestPayload || null,
        },
        null,
        2
      )
    );
    state.latestDurableArchiveWriteAuditAction =
      "Write audit JSON copied to clipboard.";
  } catch (error) {
    state.latestDurableArchiveWriteAuditAction =
      `Write audit JSON copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicSignalAlertPreviewMarkdown() {
  const payload = getToxicSignalAlertPreviewPayload();
  if (!payload?.markdown) {
    state.latestToxicSignalAlertPreviewAction =
      "Alert preview markdown is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(payload.markdown);
    state.latestToxicSignalAlertPreviewAction =
      "Alert preview markdown copied to clipboard.";
  } catch (error) {
    state.latestToxicSignalAlertPreviewAction =
      `Alert preview markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function loadLatestToxicReplay() {
  const report = getToxicReplayPayload();
  const symbol = report?.selectedSymbol || "BTC-PERP";
  state.toxicReplayDetail = { reason: "Loading replay..." };
  renderAll();
  try {
    state.toxicReplayDetail = await fetchJson(
      toxicReplayLatestEndpointTemplate.replace(":symbol", encodeURIComponent(symbol))
    );
    state.latestToxicReplayAction = "Loaded latest replay signal.";
  } catch (error) {
    state.toxicReplayDetail = { reason: `Replay load failed: ${error.message}` };
    state.latestToxicReplayAction = `Replay load failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicReplayJson() {
  const payload = state.toxicReplayDetail || getToxicReplayPayload();
  if (!payload) {
    state.latestToxicReplayAction = "Replay JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicReplayAction = "Replay JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicReplayAction = `Replay copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicMarkoutJson() {
  const payload = getToxicMarkoutPayload();
  if (!payload) {
    state.latestToxicMarkoutAction = "Markout JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicMarkoutAction = "Markout JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicMarkoutAction = `Markout copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicQualityScorecardJson() {
  const payload = getToxicQualityScorecardPayload();
  if (!payload) {
    state.latestToxicQualityScorecardAction = "Quality scorecard JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicQualityScorecardAction = "Quality scorecard JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicQualityScorecardAction =
      `Quality scorecard copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicWeightRecommendationJson() {
  const payload = getToxicWeightRecommendationPayload();
  if (!payload) {
    state.latestToxicWeightRecommendationAction =
      "Weight recommendation JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicWeightRecommendationAction =
      "Weight recommendation JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicWeightRecommendationAction =
      `Weight recommendation copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicWeightReviewJson() {
  const payload = getToxicWeightReviewPayload();
  if (!payload) {
    state.latestToxicWeightReviewAction = "Weight review JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicWeightReviewAction = "Weight review JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicWeightReviewAction = `Weight review copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicWeightReviewMarkdown() {
  const symbol = getToxicWeightReviewPayload()?.selectedSymbol || "BTC-PERP";
  try {
    const payload = await fetchJson(
      `/api/toxicity/weight-review/${encodeURIComponent(symbol)}/export`
    );
    await navigator.clipboard.writeText(payload.markdownReport || "");
    state.latestToxicWeightReviewAction = "Weight review markdown copied to clipboard.";
  } catch (error) {
    state.latestToxicWeightReviewAction =
      `Weight review markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicGovernanceLedgerJson() {
  const payload = getToxicGovernanceLedgerPayload();
  if (!payload) {
    state.latestToxicGovernanceLedgerAction = "Governance ledger JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicGovernanceLedgerAction = "Governance ledger JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicGovernanceLedgerAction =
      `Governance ledger copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicGovernanceProposalJson() {
  const payload = getToxicGovernanceProposalPayload();
  if (!payload) {
    state.latestToxicGovernanceProposalAction = "Governance proposal JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicGovernanceProposalAction =
      "Governance proposal JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicGovernanceProposalAction =
      `Governance proposal copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicGovernanceProposalMarkdown() {
  const symbol = getToxicGovernanceProposalPayload()?.selectedSymbol || "BTC-PERP";
  try {
    const payload = await fetchJson(
      `/api/toxicity/governance-proposal/export?symbol=${encodeURIComponent(symbol)}`
    );
    await navigator.clipboard.writeText(payload.markdownReport || "");
    state.latestToxicGovernanceProposalAction =
      "Governance proposal markdown copied to clipboard.";
  } catch (error) {
    state.latestToxicGovernanceProposalAction =
      `Governance proposal markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicGovernanceReviewPackJson() {
  const payload = getToxicGovernanceReviewPackPayload();
  if (!payload) {
    state.latestToxicGovernanceReviewPackAction = "Governance review pack JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicGovernanceReviewPackAction =
      "Governance review pack JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicGovernanceReviewPackAction =
      `Governance review pack copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicGovernanceReviewPackMarkdown() {
  const symbol = getToxicGovernanceReviewPackPayload()?.selectedSymbol || "BTC-PERP";
  try {
    const payload = await fetchJson(
      `/api/toxicity/governance-review-pack/export?symbol=${encodeURIComponent(symbol)}`
    );
    await navigator.clipboard.writeText(payload.markdownReport || "");
    state.latestToxicGovernanceReviewPackAction =
      "Governance review pack markdown copied to clipboard.";
  } catch (error) {
    state.latestToxicGovernanceReviewPackAction =
      `Governance review pack markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicGovernanceSignoffPackJson() {
  const payload = getToxicGovernanceSignoffPackPayload();
  if (!payload) {
    state.latestToxicGovernanceSignoffPackAction =
      "Governance signoff pack JSON is unavailable.";
    renderAll();
    return;
  }

  try {
    await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
    state.latestToxicGovernanceSignoffPackAction =
      "Governance signoff pack JSON copied to clipboard.";
  } catch (error) {
    state.latestToxicGovernanceSignoffPackAction =
      `Governance signoff pack copy failed: ${error.message}`;
  }
  renderAll();
}

async function copyToxicGovernanceSignoffPackMarkdown() {
  const symbol = getToxicGovernanceSignoffPackPayload()?.selectedSymbol || "BTC-PERP";
  try {
    const payload = await fetchJson(
      `/api/toxicity/governance-signoff-pack/export?symbol=${encodeURIComponent(symbol)}`
    );
    await navigator.clipboard.writeText(payload.markdownReport || "");
    state.latestToxicGovernanceSignoffPackAction =
      "Governance signoff pack markdown copied to clipboard.";
  } catch (error) {
    state.latestToxicGovernanceSignoffPackAction =
      `Governance signoff pack markdown copy failed: ${error.message}`;
  }
  renderAll();
}

async function ensureMonitoringStarted() {
  try {
    const response = await fetch("/api/runtime/start", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });
    if (!response.ok) {
      throw new Error(`/api/runtime/start -> ${response.status}`);
    }
    const payload = await response.json();
    state.latestRuntimeAction = payload.message || payload.result || "started";
    await refreshFast();
  } catch (error) {
    state.latestRuntimeAction = `start failed: ${error.message}`;
    renderAll();
  }
}

async function ensureMonitoringStopped() {
  try {
    const response = await fetch("/api/runtime/stop", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
    });
    if (!response.ok) {
      throw new Error(`/api/runtime/stop -> ${response.status}`);
    }
    const payload = await response.json();
    state.latestRuntimeAction = payload.message || payload.result || "stopped";
    await refreshFast();
  } catch (error) {
    state.latestRuntimeAction = `stop failed: ${error.message}`;
    renderAll();
  }
}

function selectActiveTradeToxicitySymbol() {
  const input = $("activeTradeToxicitySymbolInput");
  state.activeTradeToxicitySymbol = input?.value?.trim() || null;
  state.latestActiveTradeToxicityAction = state.activeTradeToxicitySymbol
    ? `Selected symbol: ${state.activeTradeToxicitySymbol}`
    : "Reset to runtime symbol.";
  refreshActiveTradeToxicity();
}

function selectOrderbookWallSymbol() {
  const input = $("orderbookWallSymbolInput");
  state.orderbookWallSymbol = input?.value?.trim() || null;
  state.latestOrderbookWallAction = state.orderbookWallSymbol
    ? `Selected symbol: ${state.orderbookWallSymbol}`
    : "Reset to runtime symbol.";
  refreshOrderbookWallLifecycle();
}

function selectOrderbookWallInterpretationSymbol() {
  const input = $("orderbookWallInterpretationSymbolInput");
  state.orderbookWallInterpretationSymbol = input?.value?.trim() || null;
  state.latestOrderbookWallInterpretationAction = state.orderbookWallInterpretationSymbol
    ? `Selected symbol: ${state.orderbookWallInterpretationSymbol}`
    : "Reset to runtime symbol.";
  refreshOrderbookWallInterpretation();
}

function selectStructuralToxicitySymbol() {
  const input = $("structuralToxicitySymbolInput");
  state.structuralToxicitySymbol = input?.value?.trim() || null;
  state.latestStructuralToxicityAction = state.structuralToxicitySymbol
    ? `Selected symbol: ${state.structuralToxicitySymbol}`
    : "Reset to runtime symbol.";
  refreshStructuralToxicity();
}

document.addEventListener("click", (event) => {
  if (event.target.closest("#refreshOperatorHomeButton")) {
    refreshOperatorHome();
    return;
  }

  if (event.target.closest("#filterSignalsButton")) {
    applySignalSymbolFilter();
    return;
  }

  if (event.target.closest("#clearSignalFilterButton")) {
    clearSignalSymbolFilter();
    return;
  }

  if (event.target.closest("#copyFilteredSignalJsonButton")) {
    copyFilteredSignalJson();
    return;
  }

  if (event.target.closest("#loadToxicSignalHealthBySymbolButton")) {
    loadToxicSignalHealthBySymbol();
    return;
  }

  if (event.target.closest("#refreshToxicSignalHealthButton")) {
    refreshToxicSignalHealth();
    return;
  }

  if (event.target.closest("#copyToxicSignalHealthJsonButton")) {
    copyToxicSignalHealthJson();
    return;
  }

  if (event.target.closest("#clearSuspiciousOrdersFilterButton")) {
    clearSuspiciousOrdersFilters();
    return;
  }

  if (event.target.closest("#resetSuspiciousOrdersSortButton")) {
    resetSuspiciousOrdersSort();
    return;
  }

  if (event.target.closest("#oneClickStartButton")) {
    ensureMonitoringStarted();
    return;
  }

  if (event.target.closest("#operatorStopButton")) {
    ensureMonitoringStopped();
    return;
  }

  if (event.target.closest("#operatorRefreshButton")) {
    refreshFast();
    return;
  }

  if (event.target.closest("#refreshVenueDiagnosticsButton")) {
    refreshVenueDiagnostics();
    return;
  }

  if (event.target.closest("#copyVenueDiagnosticsJsonButton")) {
    copyVenueDiagnosticsJson();
    return;
  }

  if (event.target.closest("#refreshSuspiciousReplayButton")) {
    refreshSuspiciousReplay();
    return;
  }

  if (event.target.closest("#loadSuspiciousReplayBySymbolButton")) {
    loadSuspiciousReplayBySymbol();
    return;
  }

  if (event.target.closest("#loadSuspiciousReplayBySignalIdButton")) {
    loadSuspiciousReplayBySignalId();
    return;
  }

  if (event.target.closest("#clearSuspiciousReplayFilterButton")) {
    clearSuspiciousReplayFilter();
    return;
  }

  if (event.target.closest("#copySuspiciousReplayJsonButton")) {
    copySuspiciousReplayJson();
    return;
  }

  const whaleFlowPresetButton = event.target.closest("[data-whale-flow-preset]");
  if (whaleFlowPresetButton) {
    setWhaleFlowCompactPreset(whaleFlowPresetButton.getAttribute("data-whale-flow-preset"));
    return;
  }

  if (event.target.closest("#resetWhaleFlowCompactPresetButton")) {
    resetWhaleFlowCompactPreset();
    return;
  }

  if (event.target.closest("#copyWhaleFlowCompactPresetJsonButton")) {
    copyWhaleFlowCompactPresetJson();
    return;
  }

  if (event.target.closest("#refreshWhaleReplayOverlayButton")) {
    refreshWhaleReplayOverlay();
    return;
  }

  if (event.target.closest("#loadWhaleReplayOverlayBySymbolButton")) {
    loadWhaleReplayOverlayBySymbol();
    return;
  }

  if (event.target.closest("#loadWhaleReplayOverlayBySignalIdButton")) {
    loadWhaleReplayOverlayBySignalId();
    return;
  }

  if (event.target.closest("#copyWhaleReplayOverlayJsonButton")) {
    copyWhaleReplayOverlayJson();
    return;
  }

  if (event.target.closest("#copyWhaleReplayOverlayMarkdownButton")) {
    copyWhaleReplayOverlayMarkdown();
    return;
  }

  if (event.target.closest("#refreshReplayHeatmapButton")) {
    refreshReplayHeatmap();
    return;
  }

  if (event.target.closest("#buildReplayHeatmapButton")) {
    buildReplayHeatmap();
    return;
  }

  if (event.target.closest("#clearReplayHeatmapFilterButton")) {
    clearReplayHeatmapFilter();
    return;
  }

  if (event.target.closest("#copyReplayHeatmapJsonButton")) {
    copyReplayHeatmapJson();
    return;
  }

  if (event.target.closest("#copyReplayHeatmapMarkdownButton")) {
    copyReplayHeatmapMarkdown();
    return;
  }

  const suspiciousReplayButton = event.target.closest("[data-suspicious-replay-signal-id]");
  if (suspiciousReplayButton) {
    state.suspiciousReplaySignalId =
      suspiciousReplayButton.getAttribute("data-suspicious-replay-signal-id") || null;
    state.suspiciousReplaySymbol =
      suspiciousReplayButton.getAttribute("data-suspicious-replay-symbol") || null;
    loadSuspiciousReplayBySignalId();
    return;
  }

  const button = event.target.closest("[data-file-name]");
  if (button) {
    loadReport(button.getAttribute("data-file-name"));
    return;
  }

  const calibrationButton = event.target.closest("[data-calibration-report-id]");
  if (calibrationButton) {
    loadCalibrationReport(calibrationButton.getAttribute("data-calibration-report-id"));
    return;
  }

  const reviewButton = event.target.closest("[data-review-action]");
  if (reviewButton) {
    submitParameterReview(
      reviewButton.getAttribute("data-recommendation-id"),
      reviewButton.getAttribute("data-report-id"),
      reviewButton.getAttribute("data-review-action")
    );
    return;
  }

  if (event.target.closest("#generateManualPatchButton")) {
    generateManualPatch();
    return;
  }

  if (event.target.closest("#refreshExportsButton")) {
    refreshParameterReviews();
    return;
  }

  if (event.target.closest("#refreshPatchDiffButton")) {
    refreshPatchDiffs();
    return;
  }

  const runbookButton = event.target.closest("[data-runbook-export-id]");
  if (runbookButton) {
    loadRunbook(runbookButton.getAttribute("data-runbook-export-id"));
    return;
  }

  const dryRunButton = event.target.closest("[data-dryrun-export-id]");
  if (dryRunButton) {
    loadDryRun(dryRunButton.getAttribute("data-dryrun-export-id"));
    return;
  }

  const evidenceButton = event.target.closest("[data-evidence-export-id]");
  if (evidenceButton) {
    loadEvidencePack(evidenceButton.getAttribute("data-evidence-export-id"));
    return;
  }

  if (event.target.closest("#loadLatestRunbookButton")) {
    loadLatestRunbook();
    return;
  }

  if (event.target.closest("#refreshRunbookButton")) {
    refreshRunbooks();
    return;
  }

  if (event.target.closest("#loadLatestDryRunButton")) {
    loadLatestDryRun();
    return;
  }

  if (event.target.closest("#refreshDryRunButton")) {
    refreshDryRuns();
    return;
  }

  if (event.target.closest("#loadLatestEvidencePackButton")) {
    loadLatestEvidencePack();
    return;
  }

  if (event.target.closest("#refreshEvidencePackButton")) {
    refreshEvidencePacks();
    return;
  }

  if (event.target.closest("#manualStartupCheckButton")) {
    refreshManualStartupCheck();
    return;
  }

  if (event.target.closest("#refreshManualSignoffButton")) {
    refreshManualSignoffs();
    return;
  }

  if (event.target.closest("#refreshManualAuditStoryButton")) {
    refreshManualAuditStory();
    return;
  }

  if (event.target.closest("#copyManualAuditStoryButton")) {
    copyManualAuditStoryMarkdown();
    return;
  }

  if (event.target.closest("#refreshGovernanceIndexButton")) {
    refreshGovernanceIndex();
    return;
  }

  if (event.target.closest("#copyGovernanceMarkdownButton")) {
    copyGovernanceMarkdown();
    return;
  }

  if (event.target.closest("#refreshActiveTradeToxicityButton")) {
    refreshActiveTradeToxicity();
    return;
  }

  if (event.target.closest("#selectActiveTradeToxicitySymbolButton")) {
    selectActiveTradeToxicitySymbol();
    return;
  }

  if (event.target.closest("#copyActiveTradeToxicityJsonButton")) {
    copyActiveTradeToxicityJson();
    return;
  }

  if (event.target.closest("#refreshLiquidationToxicityButton")) {
    refreshLiquidationToxicity();
    return;
  }

  if (event.target.closest("#refreshOrderbookWallLifecycleButton")) {
    refreshOrderbookWallLifecycle();
    return;
  }

  if (event.target.closest("#selectOrderbookWallSymbolButton")) {
    selectOrderbookWallSymbol();
    return;
  }

  if (event.target.closest("#copyOrderbookWallJsonButton")) {
    copyOrderbookWallJson();
    return;
  }

  if (event.target.closest("#refreshOrderbookWallInterpretationButton")) {
    refreshOrderbookWallInterpretation();
    return;
  }

  if (event.target.closest("#selectOrderbookWallInterpretationSymbolButton")) {
    selectOrderbookWallInterpretationSymbol();
    return;
  }

  if (event.target.closest("#copyOrderbookWallInterpretationJsonButton")) {
    copyOrderbookWallInterpretationJson();
    return;
  }

  if (event.target.closest("#refreshStructuralToxicityButton")) {
    refreshStructuralToxicity();
    return;
  }

  if (event.target.closest("#selectStructuralToxicitySymbolButton")) {
    selectStructuralToxicitySymbol();
    return;
  }

  if (event.target.closest("#copyStructuralToxicityJsonButton")) {
    copyStructuralToxicityJson();
    return;
  }

  if (event.target.closest("#refreshWhaleFlowMonitorButton")) {
    refreshWhaleFlowMonitor();
    return;
  }

  if (event.target.closest("#copyWhaleFlowMonitorJsonButton")) {
    copyWhaleFlowMonitorJson();
    return;
  }

  if (event.target.closest("#loadWhaleFlowCalibrationBySymbolButton")) {
    loadWhaleFlowCalibrationBySymbol();
    return;
  }

  if (event.target.closest("#refreshWhaleFlowCalibrationButton")) {
    refreshWhaleFlowCalibration();
    return;
  }

  if (event.target.closest("#copyWhaleFlowCalibrationJsonButton")) {
    copyWhaleFlowCalibrationJson();
    return;
  }

  if (event.target.closest("#copyWhaleFlowCalibrationMarkdownButton")) {
    copyWhaleFlowCalibrationMarkdown();
    return;
  }

  if (event.target.closest("#loadWhaleFlowCandidateHistoryBySymbolButton")) {
    loadWhaleFlowCandidateHistoryBySymbol();
    return;
  }

  if (event.target.closest("#refreshWhaleFlowCandidateHistoryButton")) {
    refreshWhaleFlowCandidateHistory();
    return;
  }

  if (event.target.closest("#copyWhaleFlowCandidateHistoryJsonButton")) {
    copyWhaleFlowCandidateHistoryJson();
    return;
  }

  if (event.target.closest("#refreshToxicSignalFusionButton")) {
    refreshToxicSignalFusion();
    return;
  }

  if (event.target.closest("#copyToxicSignalFusionJsonButton")) {
    copyToxicSignalFusionJson();
    return;
  }

  if (event.target.closest("#refreshToxicSignalInboxButton")) {
    refreshToxicSignalInbox();
    return;
  }

  if (event.target.closest("#copyToxicSignalInboxJsonButton")) {
    copyToxicSignalInboxJson();
    return;
  }

  if (event.target.closest("#refreshToxicSignalGroupsButton")) {
    refreshToxicSignalGroups();
    return;
  }

  if (event.target.closest("#copyToxicSignalGroupsJsonButton")) {
    copyToxicSignalGroupsJson();
    return;
  }

  if (event.target.closest("#loadToxicSignalDetailButton")) {
    loadToxicSignalDetail();
    return;
  }

  if (event.target.closest("#loadToxicSignalGroupDetailButton")) {
    loadToxicSignalGroupDetail();
    return;
  }

  if (event.target.closest("#copyToxicSignalDetailJsonButton")) {
    copyToxicSignalDetailJson();
    return;
  }

  if (event.target.closest("#loadToxicSignalHistoryBySymbolButton")) {
    loadToxicSignalHistoryBySymbol();
    return;
  }

  if (event.target.closest("#loadToxicSignalHistorySignalButton")) {
    loadToxicSignalHistorySignalById();
    return;
  }

  if (event.target.closest("#refreshToxicSignalHistoryButton")) {
    refreshToxicSignalHistory();
    return;
  }

  if (event.target.closest("#copyToxicSignalHistoryJsonButton")) {
    copyToxicSignalHistoryJson();
    return;
  }

  if (event.target.closest("#exportToxicSignalHistoryJsonButton")) {
    exportToxicSignalHistoryJson();
    return;
  }

  if (event.target.closest("#refreshToxicSignalReportButton")) {
    refreshToxicSignalReport();
    return;
  }

  if (event.target.closest("#copyToxicSignalReportJsonButton")) {
    copyToxicSignalReportJson();
    return;
  }

  if (event.target.closest("#copyToxicSignalReportMarkdownButton")) {
    copyToxicSignalReportMarkdown();
    return;
  }

  if (event.target.closest("#refreshToxicSignalRollingButton")) {
    refreshToxicSignalRolling();
    return;
  }

  if (event.target.closest("#copyToxicSignalRollingJsonButton")) {
    copyToxicSignalRollingJson();
    return;
  }

  if (event.target.closest("#copyToxicSignalRollingMarkdownButton")) {
    copyToxicSignalRollingMarkdown();
    return;
  }

  if (event.target.closest("#refreshToxicSignalAlertPreviewButton")) {
    refreshToxicSignalAlertPreview();
    return;
  }

  if (event.target.closest("#loadToxicSignalAlertExplainButton")) {
    loadToxicSignalAlertExplain();
    return;
  }

  if (event.target.closest("#copyToxicSignalAlertExplainJsonButton")) {
    copyToxicSignalAlertExplainJson();
    return;
  }

  if (event.target.closest("#copyToxicSignalAlertPreviewJsonButton")) {
    copyToxicSignalAlertPreviewJson();
    return;
  }

  if (event.target.closest("#copyToxicSignalAlertPreviewMarkdownButton")) {
    copyToxicSignalAlertPreviewMarkdown();
    return;
  }

  if (event.target.closest("#refreshDurableArchiveDryRunButton")) {
    refreshDurableArchiveDryRun();
    return;
  }

  if (event.target.closest("#copyDurableArchiveDryRunJsonButton")) {
    copyDurableArchiveDryRunJson();
    return;
  }

  if (event.target.closest("#refreshDurableArchiveDryRunReviewPackButton")) {
    refreshDurableArchiveDryRunReviewPack();
    return;
  }

  if (event.target.closest("#copyDurableArchiveDryRunReviewPackJsonButton")) {
    copyDurableArchiveDryRunReviewPackJson();
    return;
  }

  if (event.target.closest("#copyDurableArchiveDryRunReviewPackMarkdownButton")) {
    copyDurableArchiveDryRunReviewPackMarkdown();
    return;
  }

  if (event.target.closest("#refreshDurableArchiveWriteGateButton")) {
    refreshDurableArchiveWriteGate();
    return;
  }

  if (event.target.closest("#copyDurableArchiveWriteGateJsonButton")) {
    copyDurableArchiveWriteGateJson();
    return;
  }

  if (event.target.closest("#refreshDurableArchiveWriteAuditButton")) {
    refreshDurableArchiveWriteAudit();
    return;
  }

  if (event.target.closest("#loadLatestDurableArchiveWriteAttemptButton")) {
    loadLatestDurableArchiveWriteAttempt();
    return;
  }

  if (event.target.closest("#copyDurableArchiveWriteAuditJsonButton")) {
    copyDurableArchiveWriteAuditJson();
    return;
  }

  if (event.target.closest("#refreshToxicReplayButton")) {
    refreshToxicReplay();
    return;
  }

  if (event.target.closest("#loadLatestToxicReplayButton")) {
    loadLatestToxicReplay();
    return;
  }

  if (event.target.closest("#copyToxicReplayJsonButton")) {
    copyToxicReplayJson();
    return;
  }

  if (event.target.closest("#refreshToxicMarkoutButton")) {
    refreshToxicMarkout();
    return;
  }

  if (event.target.closest("#copyToxicMarkoutJsonButton")) {
    copyToxicMarkoutJson();
    return;
  }

  if (event.target.closest("#refreshToxicQualityScorecardButton")) {
    refreshToxicQualityScorecard();
    return;
  }

  if (event.target.closest("#copyToxicQualityScorecardJsonButton")) {
    copyToxicQualityScorecardJson();
    return;
  }

  if (event.target.closest("#refreshToxicWeightRecommendationButton")) {
    refreshToxicWeightRecommendation();
    return;
  }

  if (event.target.closest("#copyToxicWeightRecommendationJsonButton")) {
    copyToxicWeightRecommendationJson();
    return;
  }

  if (event.target.closest("#refreshToxicWeightReviewButton")) {
    refreshToxicWeightReview();
    return;
  }

  if (event.target.closest("#refreshToxicGovernanceLedgerButton")) {
    refreshToxicGovernanceLedger();
    return;
  }

  if (event.target.closest("#refreshToxicGovernanceProposalButton")) {
    refreshToxicGovernanceProposal();
    return;
  }

  if (event.target.closest("#refreshToxicGovernanceReviewPackButton")) {
    refreshToxicGovernanceReviewPack();
    return;
  }

  if (event.target.closest("#refreshToxicGovernanceSignoffPackButton")) {
    refreshToxicGovernanceSignoffPack();
    return;
  }

  if (event.target.closest("#copyToxicWeightReviewJsonButton")) {
    copyToxicWeightReviewJson();
    return;
  }

  if (event.target.closest("#copyToxicWeightReviewMarkdownButton")) {
    copyToxicWeightReviewMarkdown();
    return;
  }

  if (event.target.closest("#copyToxicGovernanceLedgerJsonButton")) {
    copyToxicGovernanceLedgerJson();
    return;
  }

  if (event.target.closest("#copyToxicGovernanceProposalJsonButton")) {
    copyToxicGovernanceProposalJson();
    return;
  }

  if (event.target.closest("#copyToxicGovernanceProposalMarkdownButton")) {
    copyToxicGovernanceProposalMarkdown();
    return;
  }

  if (event.target.closest("#copyToxicGovernanceReviewPackJsonButton")) {
    copyToxicGovernanceReviewPackJson();
    return;
  }

  if (event.target.closest("#copyToxicGovernanceReviewPackMarkdownButton")) {
    copyToxicGovernanceReviewPackMarkdown();
    return;
  }

  if (event.target.closest("#copyToxicGovernanceSignoffPackJsonButton")) {
    copyToxicGovernanceSignoffPackJson();
    return;
  }

  if (event.target.closest("#copyToxicGovernanceSignoffPackMarkdownButton")) {
    copyToxicGovernanceSignoffPackMarkdown();
    return;
  }

  if (event.target.closest("#approveManualSignoffButton")) {
    submitManualSignoff("Approved");
    return;
  }

  if (event.target.closest("#rejectManualSignoffButton")) {
    submitManualSignoff("Rejected");
  }
});

document.addEventListener("change", (event) => {
  if (event.target?.id === "toxicSignalHistorySortSelect") {
    state.toxicSignalHistorySortMode = event.target.value || "severity";
    renderToxicSignalHistory();
    return;
  }

  if (
    event.target?.id === "suspiciousOrdersSortSelect" ||
    event.target?.id === "suspiciousOrdersFilterSymbolInput" ||
    event.target?.id === "suspiciousOrdersFilterAlertDecisionInput" ||
    event.target?.id === "suspiciousOrdersHideNotEnoughDataCheckbox" ||
    event.target?.id === "suspiciousOrdersHighSeverityOnlyCheckbox"
  ) {
    updateSuspiciousOrdersViewStateFromControls();
    renderSuspiciousToxicOrders();
    return;
  }

  if (
    event.target?.id === "replayHeatmapSymbolInput" ||
    event.target?.id === "replayHeatmapSignalKindInput" ||
    event.target?.id === "replayHeatmapDirectionSelect"
  ) {
    syncReplayHeatmapFiltersFromControls();
    if (state.replayHeatmapBuiltPayload && state.replayHeatmapLastHistoryUrl === replayHeatmapHistoryUrl()) {
      state.replayHeatmapBuiltPayload = buildReplayHeatmapPayload();
    } else if (state.replayHeatmapLastHistoryUrl !== replayHeatmapHistoryUrl()) {
      state.replayHeatmapBuiltPayload = null;
    }
    renderReplayHeatmap();
  }
});

const refreshButton = $("refreshButton");
if (refreshButton) {
  refreshButton.addEventListener("click", async () => {
    await Promise.all([
      refreshFast(),
      refreshSlow(),
      refreshReplayReports(),
      refreshCalibrationReports(),
      refreshParameterReviews(),
      refreshPatchDiffs(),
      refreshRunbooks(),
      refreshDryRuns(),
      refreshEvidencePacks(),
      refreshManualStartupCheck(),
      refreshManualSignoffs(),
      refreshManualEvidenceFreshness(),
      refreshManualAuditStory(),
      refreshGovernanceIndex(),
      refreshActiveTradeToxicity(),
      refreshLiquidationToxicity(),
      refreshOrderbookWallLifecycle(),
      refreshOrderbookWallInterpretation(),
      refreshStructuralToxicity(),
      refreshToxicSignalHealth(),
      refreshToxicSignalInbox(),
      refreshToxicSignalGroups(),
      refreshToxicSignalDetail(),
      refreshToxicSignalHistory(),
      refreshToxicSignalReport(),
      refreshToxicSignalRolling(),
      refreshReplayHeatmap(),
      refreshDurableArchiveDryRun(),
      refreshDurableArchiveDryRunReviewPack(),
      refreshDurableArchiveWriteGate(),
      refreshDurableArchiveWriteAudit(),
      refreshToxicSignalFusion(),
      refreshToxicReplay(),
      refreshToxicMarkout(),
      refreshToxicQualityScorecard(),
      refreshToxicWeightRecommendation(),
      refreshToxicWeightReview(),
      refreshToxicGovernanceLedger(),
      refreshToxicGovernanceProposal(),
      refreshToxicGovernanceReviewPack(),
      refreshToxicGovernanceSignoffPack(),
      refreshWhaleFlowCandidateHistory(),
    ]);
  });
}

async function init() {
  await Promise.all([
    refreshFast(),
    refreshSlow(),
    refreshReplayReports(),
    refreshCalibrationReports(),
    refreshParameterReviews(),
    refreshPatchDiffs(),
    refreshRunbooks(),
    refreshDryRuns(),
    refreshEvidencePacks(),
    refreshManualStartupCheck(),
    refreshManualSignoffs(),
    refreshManualEvidenceFreshness(),
    refreshManualAuditStory(),
    refreshGovernanceIndex(),
    refreshActiveTradeToxicity(),
    refreshLiquidationToxicity(),
    refreshOrderbookWallLifecycle(),
    refreshOrderbookWallInterpretation(),
    refreshStructuralToxicity(),
    refreshToxicSignalHealth(),
    refreshToxicSignalInbox(),
    refreshToxicSignalGroups(),
    refreshToxicSignalDetail(),
    refreshToxicSignalHistory(),
    refreshToxicSignalReport(),
    refreshToxicSignalRolling(),
    refreshReplayHeatmap(),
    refreshDurableArchiveDryRun(),
    refreshDurableArchiveDryRunReviewPack(),
    refreshDurableArchiveWriteGate(),
    refreshDurableArchiveWriteAudit(),
    refreshToxicSignalFusion(),
    refreshToxicReplay(),
    refreshToxicMarkout(),
    refreshToxicQualityScorecard(),
    refreshToxicWeightRecommendation(),
    refreshToxicWeightReview(),
    refreshToxicGovernanceLedger(),
    refreshToxicGovernanceProposal(),
    refreshToxicGovernanceReviewPack(),
    refreshToxicGovernanceSignoffPack(),
    refreshWhaleFlowCandidateHistory(),
  ]);
  setInterval(refreshFast, 1000);
  setInterval(refreshSlow, 3000);
  setInterval(refreshReplayReports, 10000);
  setInterval(refreshCalibrationReports, 10000);
  setInterval(refreshParameterReviews, 10000);
  setInterval(refreshPatchDiffs, 10000);
  setInterval(refreshRunbooks, 10000);
  setInterval(refreshDryRuns, 10000);
  setInterval(refreshEvidencePacks, 10000);
  setInterval(refreshManualStartupCheck, 10000);
  setInterval(refreshManualSignoffs, 10000);
  setInterval(refreshManualEvidenceFreshness, 10000);
  setInterval(refreshManualAuditStory, 10000);
  setInterval(refreshGovernanceIndex, 10000);
  setInterval(refreshActiveTradeToxicity, 5000);
  setInterval(refreshLiquidationToxicity, 5000);
  setInterval(refreshOrderbookWallLifecycle, 5000);
  setInterval(refreshOrderbookWallInterpretation, 5000);
  setInterval(refreshStructuralToxicity, 5000);
  setInterval(refreshToxicSignalHealth, 5000);
  setInterval(refreshToxicSignalInbox, 5000);
  setInterval(refreshToxicSignalGroups, 5000);
  setInterval(refreshToxicSignalDetail, 5000);
  setInterval(refreshToxicSignalHistory, 5000);
  setInterval(refreshToxicSignalReport, 5000);
  setInterval(refreshToxicSignalRolling, 5000);
  setInterval(refreshReplayHeatmap, 5000);
  setInterval(refreshDurableArchiveDryRunReviewPack, 5000);
  setInterval(refreshDurableArchiveWriteGate, 5000);
  setInterval(refreshDurableArchiveWriteAudit, 5000);
  setInterval(refreshToxicSignalFusion, 5000);
  setInterval(refreshToxicReplay, 5000);
  setInterval(refreshToxicMarkout, 5000);
  setInterval(refreshToxicQualityScorecard, 5000);
  setInterval(refreshToxicWeightRecommendation, 5000);
  setInterval(refreshToxicWeightReview, 5000);
  setInterval(refreshToxicGovernanceLedger, 5000);
  setInterval(refreshToxicGovernanceProposal, 5000);
  setInterval(refreshToxicGovernanceReviewPack, 5000);
  setInterval(refreshToxicGovernanceSignoffPack, 5000);
  setInterval(refreshWhaleFlowCandidateHistory, 5000);
}

init().catch((error) => {
  $("systemHealthContent").innerHTML = `<div class="error">${error.message}</div>`;
});

