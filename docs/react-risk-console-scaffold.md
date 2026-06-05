# React Risk Console Scaffold

This project keeps the existing Rust dashboard as the production UI. The React scaffold is optional and can be generated when a separate operator console is needed.

## Generate

```powershell
powershell -ExecutionPolicy Bypass -File scripts\scaffold-react-risk-console.ps1
```

To generate into a custom path:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\scaffold-react-risk-console.ps1 -OutDir frontend/toxic-order-monitor
```

To install dependencies after writing files:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\scaffold-react-risk-console.ps1 -Install
```

## Generated Structure

```text
src/
  api/
    client.js
    discord.js
    signals.js
  components/
    Header.jsx
    PushLog.jsx
    RiskCard.jsx
    RiskChart.jsx
    Sidebar.jsx
    SignalDetail.jsx
    SignalTable.jsx
  pages/
    Dashboard.jsx
  store/
    signalsStore.js
  App.jsx
  main.jsx
```

## Safety Notes

- The scaffold intentionally does not use `VITE_DISCORD_WEBHOOK`.
- Browser code calls a backend notification proxy instead of Discord directly.
- No API token is hardcoded in generated files.
- The scaffold is not wired into the Rust monitor until an explicit integration task exists.
