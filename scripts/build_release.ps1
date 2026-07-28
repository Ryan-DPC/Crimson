$ErrorActionPreference = "Stop"

$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
$ProjectRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path

Write-Host "Project root resolved to: $ProjectRoot" -ForegroundColor Cyan

# Change directory to project root
cd $ProjectRoot

Write-Host "Building crimson-server release..." -ForegroundColor Cyan
cargo build --release -p crimson-server

Write-Host "Copying sidecar..." -ForegroundColor Cyan
$sidecarDir = Join-Path $ProjectRoot "crimson\src-tauri\bin"
if (-not (Test-Path $sidecarDir)) {
    New-Item -ItemType Directory -Path $sidecarDir -Force | Out-Null
}
Copy-Item (Join-Path $ProjectRoot "target\release\crimson-server.exe") (Join-Path $sidecarDir "crimson-server-x86_64-pc-windows-msvc.exe") -Force

Write-Host "Building Tauri release bundle..." -ForegroundColor Cyan
cd (Join-Path $ProjectRoot "crimson")
npm run tauri build

Write-Host "Done!" -ForegroundColor Green
