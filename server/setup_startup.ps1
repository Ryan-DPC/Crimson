# Create a Startup-folder shortcut to the official CRIMSONS sidecar.
# Prefer Program Files; fall back to a local release build for dev machines.

$candidates = @(
    "${env:ProgramFiles}\CRIMSONS\crimsons-server.exe",
    "${env:ProgramFiles}\CRIMSONS\crimson-server.exe",
    (Join-Path $PSScriptRoot "..\target\release\crimsons-server.exe"),
    (Join-Path $PSScriptRoot "..\target\release\crimson-server.exe"),
    (Join-Path $PSScriptRoot "crimsons-server.exe"),
    (Join-Path $PSScriptRoot "crimson-server.exe")
)

$serverPath = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $serverPath) {
    Write-Error "crimsons-server.exe not found. Checked:`n  - $($candidates -join "`n  - ")"
    exit 1
}

$serverPath = (Resolve-Path $serverPath).Path
$startupFolder = [Environment]::GetFolderPath('Startup')
$WshShell = New-Object -ComObject WScript.Shell
$legacyLnk = "$startupFolder\CrimsonServer.lnk"
if (Test-Path $legacyLnk) { Remove-Item $legacyLnk -Force }
$Shortcut = $WshShell.CreateShortcut("$startupFolder\CrimsonsServer.lnk")
$Shortcut.TargetPath = $serverPath
$Shortcut.WorkingDirectory = Split-Path $serverPath -Parent
$Shortcut.WindowStyle = 7
$Shortcut.Save()
Write-Host "Created startup shortcut at $startupFolder\CrimsonsServer.lnk"
Write-Host "Target: $serverPath"
