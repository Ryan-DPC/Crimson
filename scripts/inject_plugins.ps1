$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
$ProjectRoot = (Resolve-Path (Join-Path $ScriptDir "..")).Path
$pluginSource = Join-Path $ProjectRoot "plugins\streamdeck"
$pluginDest = "$env:APPDATA\HotSpot\StreamDock\plugins"

$plugins = @(
    "com.laoy.streamdock.crimson.sdPlugin",
    "com.laoy.streamdock.spotify.sdPlugin",
    "com.laoy.streamdock.hue.sdPlugin",
    "com.laoy.streamdock.twitch.sdPlugin",
    "com.laoy.streamdock.discord.sdPlugin"
)

Write-Host "Starting plugin injection..." -ForegroundColor Cyan

foreach ($plugin in $plugins) {
    $src = Join-Path $pluginSource $plugin
    $dst = Join-Path $pluginDest $plugin
    
    if (Test-Path $src) {
        Write-Host "Injecting $plugin..." -ForegroundColor Green
        if (Test-Path $dst) {
            Remove-Item -Recurse -Force $dst
        }
        Copy-Item -Recurse -Force $src $dst
    } else {
        Write-Host "Warning: Source $plugin not found in $pluginSource" -ForegroundColor Yellow
    }
}

Write-Host "Plugin injection complete!" -ForegroundColor Cyan
Write-Host "Please restart StreamDock to apply changes." -ForegroundColor Yellow
