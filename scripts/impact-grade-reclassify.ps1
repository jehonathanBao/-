param(
  [string]$Database = $env:CWM_SQLITE_PATH,
  [switch]$Apply
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Database)) {
  throw 'Set -Database or CWM_SQLITE_PATH before running the reclassification preview.'
}

$mode = if ($Apply) { 'APPLY' } else { 'DRY-RUN' }
Write-Output "Impact Grade V3 reclassification mode: $mode"
Write-Output "Database: $Database"
if ($Apply) {
  throw 'Apply is intentionally disabled in this first rollout. Export and approve the dry-run report before enabling mutation.'
}

$sqlite = Get-Command sqlite3 -ErrorAction SilentlyContinue
if (-not $sqlite) {
  throw 'sqlite3 executable is required for the read-only report.'
}

& $sqlite.Source $Database @'
SELECT COALESCE(impact_level, 'NULL') AS legacy_level,
       COUNT(*) AS rows,
       SUM(CASE WHEN liquidation_notional_usd > 0 THEN 1 ELSE 0 END) AS live_liquidation_rows
FROM contract_whale_signals
GROUP BY legacy_level
ORDER BY legacy_level;
'@
