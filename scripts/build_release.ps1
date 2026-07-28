$ErrorActionPreference = "Stop"

# $ErrorActionPreference ne s'applique pas au code de retour des executables
# natifs : sans ce test explicite, un echec de cargo ou de npm laissait le
# script afficher "Done!" et renvoyer 0.
function Invoke-Step {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Action
    )
    Write-Host $Name -ForegroundColor Cyan
    & $Action
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Echec : $Name (code $LASTEXITCODE)" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
$ProjectRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path

Write-Host "Project root resolved to: $ProjectRoot" -ForegroundColor Cyan
Set-Location $ProjectRoot

Invoke-Step "Building crimson-server release..." { cargo build --release -p crimson-server }

Write-Host "Copying sidecar..." -ForegroundColor Cyan
$sidecarDir = Join-Path $ProjectRoot "crimson\src-tauri\bin"
if (-not (Test-Path $sidecarDir)) {
    New-Item -ItemType Directory -Path $sidecarDir -Force | Out-Null
}
Copy-Item (Join-Path $ProjectRoot "target\release\crimson-server.exe") (Join-Path $sidecarDir "crimson-server-x86_64-pc-windows-msvc.exe") -Force

Set-Location (Join-Path $ProjectRoot "crimson")
Invoke-Step "Building Tauri release bundle..." { npm run tauri build }

Write-Host "Done!" -ForegroundColor Green
