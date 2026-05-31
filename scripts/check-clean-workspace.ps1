Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = (Get-Location).Path
$hasGit = Test-Path -LiteralPath (Join-Path $root ".git")

Write-Output "Workspace: $root"
Write-Output "Git repository: $hasGit"

if ($hasGit) {
    Write-Output ""
    Write-Output "Git status:"
    git status --short
}

Write-Output ""
Write-Output "Build artifact directories:"
Get-ChildItem -LiteralPath $root -Directory -Filter "target*" |
    Select-Object Name, LastWriteTime |
    Format-Table -AutoSize

Write-Output ""
Write-Output "Cargo test logs:"
Get-ChildItem -LiteralPath $root -File -Filter "cargo-test-*.log" |
    Select-Object Name, Length, LastWriteTime |
    Format-Table -AutoSize

Write-Output ""
Write-Output "This script is read-only. It does not delete files."
