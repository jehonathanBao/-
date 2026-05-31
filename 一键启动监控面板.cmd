@echo off
setlocal EnableExtensions

rem Normalize %~dp0 so the nested backend window does not receive a path
rem ending in a backslash inside doubled quotes.
set "ROOT_DIR=%~dp0"
for %%I in ("%ROOT_DIR%.") do set "ROOT_DIR=%%~fI"
cd /d "%ROOT_DIR%" || (
  echo [launcher] Failed to enter project directory:
  echo [launcher] %ROOT_DIR%
  pause
  exit /b 1
)

set "API_HOST=127.0.0.1"
set "API_PORT=3000"
set "DASHBOARD_URL=http://%API_HOST%:%API_PORT%/dashboard"

set "DISCORD_GATEWAY_DIR="
for /d %%D in ("%USERPROFILE%\Documents\discord*") do (
  if not defined DISCORD_GATEWAY_DIR set "DISCORD_GATEWAY_DIR=%%~fD"
)
if defined DISCORD_GATEWAY_DIR (
  set "TOXIC_FLOW_SIDECAR_EVENTS_PATH=%DISCORD_GATEWAY_DIR%\data\sidecar\toxic-flow-rs\events.jsonl"
) else (
  set "TOXIC_FLOW_SIDECAR_EVENTS_PATH=%ROOT_DIR%\.runtime\sidecar\toxic-flow-rs\events.jsonl"
)

echo [launcher] Checking dashboard at %DASHBOARD_URL%
powershell -NoProfile -ExecutionPolicy Bypass -Command "try { $r = Invoke-WebRequest -Uri '%DASHBOARD_URL%' -UseBasicParsing -TimeoutSec 2; if ($r.StatusCode -ge 200 -and $r.StatusCode -lt 500) { exit 0 } else { exit 1 } } catch { exit 1 }"
if not errorlevel 1 (
  echo [launcher] Service is already running. Opening dashboard...
  start "" "%DASHBOARD_URL%"
  exit /b 0
)

where cargo >nul 2>nul
if errorlevel 1 (
  echo [launcher] cargo was not found in PATH. Please install Rust or open this from a Rust-enabled terminal.
  pause
  exit /b 1
)

set "LAUNCHER_DIR=%ROOT_DIR%\.runtime\launcher"
if not exist "%LAUNCHER_DIR%" mkdir "%LAUNCHER_DIR%" >nul 2>nul
set "BACKEND_CMD=%LAUNCHER_DIR%\start-backend.cmd"

(
  echo @echo off
  echo setlocal EnableExtensions
  echo cd /d "%%ROOT_DIR%%"
  echo if errorlevel 1 exit /b 1
  echo set "READ_ONLY=true"
  echo set "API_HOST=%API_HOST%"
  echo set "API_PORT=%API_PORT%"
  echo set "ENABLE_BINANCE=true"
  echo set "ENABLE_BYBIT=false"
  echo set "ENABLE_OKX=false"
  echo set "TOXIC_FLOW_SIDECAR_ENABLED=true"
  echo echo [backend] Starting btc-toxic-flow-monitor-rs on %DASHBOARD_URL%
  echo echo [backend] Sidecar events: %%TOXIC_FLOW_SIDECAR_EVENTS_PATH%%
  echo cargo run -- serve
) > "%BACKEND_CMD%"

echo [launcher] Starting backend service in a new window...
echo [launcher] Sidecar events: %TOXIC_FLOW_SIDECAR_EVENTS_PATH%
start "btc-toxic-flow-monitor-rs" cmd.exe /k call "%BACKEND_CMD%"

set "READY="
for /l %%I in (1,1,90) do (
  powershell -NoProfile -ExecutionPolicy Bypass -Command "try { $r = Invoke-WebRequest -Uri '%DASHBOARD_URL%' -UseBasicParsing -TimeoutSec 2; if ($r.StatusCode -ge 200 -and $r.StatusCode -lt 500) { exit 0 } else { exit 1 } } catch { exit 1 }"
  if not errorlevel 1 (
    set "READY=1"
    goto :open_dashboard
  )
  timeout /t 1 /nobreak >nul
)

echo [launcher] Service did not become ready within 90 seconds.
echo [launcher] The backend window is still open for logs. Start the dashboard manually later:
echo [launcher] %DASHBOARD_URL%
pause
exit /b 1

:open_dashboard
echo [launcher] Dashboard is ready. Opening browser...
start "" "%DASHBOARD_URL%"
exit /b 0
