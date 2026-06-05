$ErrorActionPreference = "Stop"

Write-Host "[live] validating compose without printing resolved secrets..."
docker compose config | Out-Null

Write-Host "[live] checking ignored local secret/data paths..."
$repoDir = (Get-Location).Path -replace "\\", "/"
git -c "safe.directory=$repoDir" check-ignore -q .env
if ($LASTEXITCODE -ne 0) {
    throw ".env is not ignored by git"
}
git -c "safe.directory=$repoDir" check-ignore -q config/replay.production.local.toml
if ($LASTEXITCODE -ne 0) {
    throw "config/replay.production.local.toml is not ignored by git"
}

git -c "safe.directory=$repoDir" ls-files --error-unmatch .env *> $null
if ($LASTEXITCODE -eq 0) {
    throw ".env is tracked by git; remove it from the index before deploying"
}

$forbidden = Select-String `
    -Path docker-compose.yml, .env.example, toxic-order-monitor/vite.config.js `
    -Pattern "VITE_.*TOKEN|VITE_API_TOKEN|VITE_OPERATOR_TOKEN" `
    -ErrorAction SilentlyContinue
if ($forbidden) {
    throw "found forbidden VITE token environment name"
}

Write-Host "[live] checking backend health endpoints if containers are running..."
$runningBackend = docker compose ps -q --status running backend 2>$null
if ($runningBackend) {
    Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:8000/healthz" | Out-Null
    Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:8000/readyz" | Out-Null
}

Write-Host "[live] checking backend notification env presence without printing values..."
$runningBackend = docker compose ps -q --status running backend 2>$null
if ($runningBackend) {
    docker compose exec -T backend sh -lc @'
if [ "${DRY_RUN:-true}" = "false" ]; then echo "[live] DRY_RUN=false"; else echo "[live] DRY_RUN is not false"; fi
if [ -n "${DISCORD_WEBHOOK_URL:-}" ]; then echo "[live] Discord webhook configured"; else echo "[live] Discord webhook not configured"; fi
if [ -n "${TELEGRAM_BOT_TOKEN:-}" ] && [ -n "${TELEGRAM_CHAT_ID:-}" ]; then echo "[live] Telegram configured"; else echo "[live] Telegram not fully configured"; fi
'@
}

Write-Host "[live] done"
