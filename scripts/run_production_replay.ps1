param(
    [string]$Config = "config/replay.production.local.toml"
)

if (-not (Test-Path -LiteralPath $Config)) {
    Write-Error "Replay config not found: $Config. Copy config/replay.production.example.toml to config/replay.production.local.toml and point it at local production data."
    exit 1
}

$hasInput = Get-ChildItem -LiteralPath "data/production_replay" -File -ErrorAction SilentlyContinue |
    Where-Object { $_.Extension -in @(".jsonl", ".csv") } |
    Select-Object -First 1

if (-not $hasInput) {
    Write-Warning "No real JSONL/CSV production replay input found under data/production_replay/."
}

cargo run --bin replay_production -- --config $Config
