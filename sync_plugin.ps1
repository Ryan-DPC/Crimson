# sync_plugin.ps1 - Synchronise le plugin LoL vers Documents\Crimson et AppData\Roaming\HotSpot\StreamDock
$src = "f:\CrimsonProject\plugins\streamdeck\com.laoy.streamdock.crimson.sdPlugin"
$dst1 = "C:\Users\Chino\Documents\Crimson\plugins\streamdeck\com.laoy.streamdock.crimson.sdPlugin"
$dst2 = "C:\Users\Chino\AppData\Roaming\HotSpot\StreamDock\plugins\com.laoy.streamdock.crimson.sdPlugin"

Write-Host "Synchronisation du plugin LoL..." -ForegroundColor Cyan

# Nettoyer et copier récursivement pour Documents
if (Test-Path $dst1) {
    Remove-Item -Path $dst1 -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $dst1 -Force | Out-Null
Copy-Item -Path "$src\*" -Destination $dst1 -Recurse -Force

# Nettoyer et copier récursivement pour AppData
if (Test-Path $dst2) {
    Remove-Item -Path $dst2 -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path $dst2 -Force | Out-Null
Copy-Item -Path "$src\*" -Destination $dst2 -Recurse -Force

Write-Host "Plugin synchronise avec succes dans les deux destinations !" -ForegroundColor Green

