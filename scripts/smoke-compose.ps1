$ErrorActionPreference = "Stop"

if (-not $env:OPERATOR_TOKEN) {
    $env:OPERATOR_TOKEN = "dummy-local-smoke-token"
}
if (-not $env:WS_SIGNAL_INTERVAL_MS) {
    $env:WS_SIGNAL_INTERVAL_MS = "1000"
}
if (-not $env:DRY_RUN) {
    $env:DRY_RUN = "true"
}
$env:DISCORD_WEBHOOK_URL = ""
$env:TELEGRAM_BOT_TOKEN = ""
$env:TELEGRAM_CHAT_ID = ""

docker compose config
docker compose up -d --build

function Wait-Http {
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [Parameter(Mandatory = $true)][string]$Label,
        [int]$Attempts = 60
    )
    Write-Host "[smoke] waiting for $Label..."
    for ($i = 0; $i -lt $Attempts; $i++) {
        try {
            Invoke-WebRequest -UseBasicParsing -Uri $Uri | Out-Null
            return
        } catch {
            Start-Sleep -Seconds 1
        }
    }
    throw "$Label did not become ready: $Uri"
}

function Get-BackendStartedAt {
    docker inspect -f '{{.State.StartedAt}}' toxic-bot
}

function Get-BackendRestartCount {
    docker inspect -f '{{.RestartCount}}' toxic-bot
}

Write-Host "[smoke] checking containers..."
docker compose ps

Write-Host "[smoke] checking frontend..."
Wait-Http -Uri "http://127.0.0.1:5173/" -Label "frontend"

Write-Host "[smoke] checking backend health..."
Wait-Http -Uri "http://127.0.0.1:8000/healthz" -Label "backend /healthz"
Wait-Http -Uri "http://127.0.0.1:8000/readyz" -Label "backend /readyz"

Write-Host "[smoke] checking backend API with server-side token..."
Invoke-WebRequest `
    -UseBasicParsing `
    -Headers @{ "x-operator-api-token" = $env:OPERATOR_TOKEN } `
    -Uri "http://127.0.0.1:8000/api/toxicity/signal-inbox/recent" | Out-Null

Write-Host "[smoke] checking token is not exposed in frontend HTML..."
$html = (Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:5173/").Content
if ($html -like "*$env:OPERATOR_TOKEN*") {
    throw "OPERATOR_TOKEN leaked in frontend HTML"
}

Write-Host "[smoke] checking token is not present in frontend app files..."
docker compose exec -T -e "SMOKE_TOKEN=$env:OPERATOR_TOKEN" frontend sh -lc 'if grep -R -- "$SMOKE_TOKEN" /app --exclude-dir=node_modules --exclude-dir=.vite --exclude-dir=dist >/dev/null 2>&1; then echo "OPERATOR_TOKEN leaked in frontend files"; exit 1; fi'

Write-Host "[smoke] checking websocket through frontend proxy..."
docker compose exec -T frontend node --input-type=module -e @'
const url = "ws://127.0.0.1:5173/ws/signals";
const timeout = setTimeout(() => {
  console.error("websocket smoke timed out");
  process.exit(1);
}, 10000);
const ws = new WebSocket(url);
ws.onmessage = (event) => {
  const payload = JSON.parse(event.data);
  if (payload.type !== "signal_snapshot") {
    console.error("unexpected websocket payload type", payload.type);
    process.exit(1);
  }
  clearTimeout(timeout);
  ws.close();
  process.exit(0);
};
ws.onerror = (event) => {
  console.error("websocket smoke failed", event?.message || "");
  process.exit(1);
};
'@

$startedBefore = Get-BackendStartedAt
$restartBefore = Get-BackendRestartCount
Write-Host "[smoke] backend StartedAt before frontend refresh: $startedBefore"
Write-Host "[smoke] backend RestartCount before frontend refresh: $restartBefore"

Write-Host "[smoke] simulating frontend refresh requests..."
for ($i = 0; $i -lt 20; $i++) {
    Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:5173/" | Out-Null
    Start-Sleep -Seconds 1
}

$startedAfterRefresh = Get-BackendStartedAt
$restartAfterRefresh = Get-BackendRestartCount
if ($startedAfterRefresh -ne $startedBefore) {
    throw "backend StartedAt changed after frontend refresh"
}
if ($restartAfterRefresh -ne $restartBefore) {
    throw "backend RestartCount changed after frontend refresh"
}

Write-Host "[smoke] restarting frontend and checking backend stays up..."
docker compose restart frontend
Wait-Http -Uri "http://127.0.0.1:5173/" -Label "frontend after restart"

$startedAfterFrontendRestart = Get-BackendStartedAt
$restartAfterFrontendRestart = Get-BackendRestartCount
if ($startedAfterFrontendRestart -ne $startedBefore) {
    throw "backend StartedAt changed after frontend restart"
}
if ($restartAfterFrontendRestart -ne $restartBefore) {
    throw "backend RestartCount changed after frontend restart"
}

Write-Host "[smoke] checking data volume mount..."
docker compose exec -T backend sh -lc "test -d /app/data && test -d /app/config"

Write-Host "[smoke] final backend container state..."
docker compose ps backend

Write-Host "[smoke] done"
