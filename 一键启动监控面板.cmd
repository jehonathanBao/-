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
set "API_HEALTH_URL=http://127.0.0.1:%API_PORT%/healthz"
set "FRONTEND_HOST=0.0.0.0"
set "FRONTEND_PORT=5173"
set "FRONTEND_URL=http://127.0.0.1:%FRONTEND_PORT%/contract-whale"
set "FRONTEND_API_PROXY_TARGET=http://127.0.0.1:%API_PORT%"
set "FRONTEND_WS_PROXY_TARGET=ws://127.0.0.1:%API_PORT%"

set "DISCORD_GATEWAY_DIR="
for /d %%D in ("%USERPROFILE%\Documents\discord*") do (
  if not defined DISCORD_GATEWAY_DIR set "DISCORD_GATEWAY_DIR=%%~fD"
)
if defined DISCORD_GATEWAY_DIR (
  set "TOXIC_FLOW_SIDECAR_EVENTS_PATH=%DISCORD_GATEWAY_DIR%\data\sidecar\toxic-flow-rs\events.jsonl"
) else (
  set "TOXIC_FLOW_SIDECAR_EVENTS_PATH=%ROOT_DIR%\.runtime\sidecar\toxic-flow-rs\events.jsonl"
)

echo [launcher] Checking frontend at %FRONTEND_URL%
powershell -NoProfile -ExecutionPolicy Bypass -Command "try { $r = Invoke-WebRequest -Uri '%FRONTEND_URL%' -UseBasicParsing -TimeoutSec 2; if ($r.StatusCode -ge 200 -and $r.StatusCode -lt 500) { exit 0 } else { exit 1 } } catch { exit 1 }"
set "FRONTEND_ALREADY_RUNNING="
if not errorlevel 1 (
  set "FRONTEND_ALREADY_RUNNING=1"
  echo [launcher] Frontend is already running.
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
set "FRONTEND_CMD=%LAUNCHER_DIR%\start-frontend.cmd"

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
  echo set "BACKEND_EXE=%%ROOT_DIR%%\target\debug\btc-toxic-flow-monitor-rs.exe"
  echo echo [backend] Starting btc-toxic-flow-monitor-rs on http://127.0.0.1:%API_PORT%/dashboard
  echo echo [backend] Sidecar events: %%TOXIC_FLOW_SIDECAR_EVENTS_PATH%%
  echo if exist "%%BACKEND_EXE%%" ^(
  echo   "%%BACKEND_EXE%%" serve
  echo ^) else ^(
  echo   cargo run --bin btc-toxic-flow-monitor-rs -- serve
  echo ^)
) > "%BACKEND_CMD%"

(
  echo @echo off
  echo setlocal EnableExtensions
  echo cd /d "%%ROOT_DIR%%\\toxic-order-monitor"
  echo if errorlevel 1 exit /b 1
  echo set "VITE_PROXY_API_TARGET=%FRONTEND_API_PROXY_TARGET%"
  echo set "VITE_PROXY_WS_TARGET=%FRONTEND_WS_PROXY_TARGET%"
  echo echo [frontend] Starting Vite dashboard on http://127.0.0.1:%FRONTEND_PORT%/contract-whale
  echo echo [frontend] LAN bind host: %FRONTEND_HOST%
  echo npm run dev -- --host %FRONTEND_HOST% --port %FRONTEND_PORT% --strictPort
) > "%FRONTEND_CMD%"

echo [launcher] Checking backend at %API_HEALTH_URL%
powershell -NoProfile -ExecutionPolicy Bypass -Command "try { $r = Invoke-WebRequest -Uri '%API_HEALTH_URL%' -UseBasicParsing -TimeoutSec 2; if ($r.StatusCode -ge 200 -and $r.StatusCode -lt 500) { exit 0 } else { exit 1 } } catch { exit 1 }"
if errorlevel 1 (
  echo [launcher] Starting backend service in a new window...
  echo [launcher] Sidecar events: %TOXIC_FLOW_SIDECAR_EVENTS_PATH%
  start "btc-toxic-flow-monitor-rs" cmd.exe /k call "%BACKEND_CMD%"
) else (
  echo [launcher] Backend already running.
)

if defined FRONTEND_ALREADY_RUNNING (
  echo [launcher] Frontend already running.
) else (
  echo [launcher] Starting frontend service in a new window...
  start "toxic-order-monitor-frontend" cmd.exe /k call "%FRONTEND_CMD%"
)

set "READY="
for /l %%I in (1,1,90) do (
  powershell -NoProfile -ExecutionPolicy Bypass -Command "try { $r = Invoke-WebRequest -Uri '%FRONTEND_URL%' -UseBasicParsing -TimeoutSec 2; if ($r.StatusCode -ge 200 -and $r.StatusCode -lt 500) { exit 0 } else { exit 1 } } catch { exit 1 }"
  if not errorlevel 1 (
    set "READY=1"
    goto :open_dashboard
  )
  timeout /t 1 /nobreak >nul
)

echo [launcher] Service did not become ready within 90 seconds.
echo [launcher] Backend/frontend did not become ready within 90 seconds.
echo [launcher] Start the dashboard manually later:
echo [launcher] %FRONTEND_URL%
pause
exit /b 1

:open_dashboard
echo [launcher] Dashboard is ready. Opening browser...
start "" "%FRONTEND_URL%"
exit /b 0
