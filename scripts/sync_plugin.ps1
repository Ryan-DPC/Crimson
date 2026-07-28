# sync_plugin.ps1 - Synchronise le plugin LoL vers Documents\Crimson et AppData\Roaming\HotSpot\StreamDock
$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
$ProjectRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path

$src = Join-Path $ProjectRoot "plugins\streamdeck\com.laoy.streamdock.crimson.sdPlugin"
$dst1 = "C:\Users\Chino\Documents\Crimson\plugins\streamdeck\com.laoy.streamdock.crimson.sdPlugin"
$dst2 = "$env:APPDATA\Roaming\HotSpot\StreamDock\plugins\com.laoy.streamdock.crimson.sdPlugin"
# Fallback to standard AppData environment variable if APPDATA contains Roaming or not
if (-not $dst2.Contains("Roaming")) {
    $dst2 = "$env:APPDATA\HotSpot\StreamDock\plugins\com.laoy.streamdock.crimson.sdPlugin"
}

Write-Host "Synchronisation du plugin Crimson..." -ForegroundColor Cyan

# Nettoyer et copier récursivement pour Documents (seulement si le dossier existe)
if (Test-Path (Split-Path (Split-Path $dst1))) {
    Write-Host "Mise à jour du dossier Documents/Crimson..." -ForegroundColor Cyan
    if (Test-Path $dst1) {
        Remove-Item -Path $dst1 -Recurse -Force -ErrorAction SilentlyContinue
    }
    New-Item -ItemType Directory -Path $dst1 -Force | Out-Null
    Copy-Item -Path "$src\*" -Destination $dst1 -Recurse -Force
}

# Nettoyer et copier récursivement pour AppData StreamDock
Write-Host "Mise à jour du plugin dans StreamDock..." -ForegroundColor Cyan
if (Test-Path $dst2) {
    Remove-Item -Path $dst2 -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $dst2 -Force | Out-Null
Copy-Item -Path "$src\*" -Destination $dst2 -Recurse -Force

Write-Host "Plugin synchronise avec succes !" -ForegroundColor Green
