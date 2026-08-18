# Crimsons Server autostart management script
# Primary mechanism: HKCU\...\Run\CrimsonsServer (same as Tauri installer / Settings).
# Optional secondary: Task Scheduler task "CrimsonsServer" (legacy name "CrimsonServer" still removed).

param(
    [ValidateSet('status', 'enable', 'disable', 'reinstall', 'test', 'logs')]
    [string]$Action = 'status'
)

function Write-Banner {
    param([string]$Message)
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host $Message -ForegroundColor Yellow
    Write-Host "========================================" -ForegroundColor Cyan
    Write-Host ""
}

function Find-ServerExecutable {
    # Prefer installed Program Files binary — never fall back to target\debug.
    $paths = @(
        "${env:ProgramFiles}\CRIMSONS\crimsons-server.exe",
        "${env:ProgramFiles}\CRIMSONS\crimson-server.exe",
        "${env:ProgramFiles(x86)}\CRIMSONS\crimsons-server.exe",
        "${env:ProgramFiles(x86)}\CRIMSONS\crimson-server.exe",
        "${env:ProgramFiles}\CRIMSON\crimson-server.exe",
        "${env:ProgramFiles(x86)}\CRIMSON\crimson-server.exe",
        (Join-Path $PSScriptRoot "..\target\release\crimsons-server.exe"),
        (Join-Path $PSScriptRoot "..\target\release\crimson-server.exe"),
        (Join-Path $PSScriptRoot "crimsons-server.exe"),
        (Join-Path $PSScriptRoot "crimson-server.exe"),
        (Join-Path $PSScriptRoot "bin\crimsons-server.exe"),
        (Join-Path $PSScriptRoot "bin\crimson-server.exe")
    )

    foreach ($path in $paths) {
        if ($path -and (Test-Path $path)) {
            $resolved = (Resolve-Path $path).Path
            # Refuse debug builds for login autostart (need CRIMSON_DEV=1, die at login).
            if ($resolved -match '(?i)[\\/]target[\\/]debug[\\/]') {
                Write-Host "  Skipping debug build: $resolved" -ForegroundColor Yellow
                continue
            }
            return $resolved
        }
    }

    return $null
}

function Get-RunValue {
    $new = (Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'CrimsonsServer' -ErrorAction SilentlyContinue).CrimsonsServer
    if ($new) { return $new }
    return (Get-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'CrimsonServer' -ErrorAction SilentlyContinue).CrimsonServer
}

function Set-RunKey {
    param([string]$ServerPath)
    $value = "`"$ServerPath`""
    Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'CrimsonsServer' -Value $value
    Remove-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'CrimsonServer' -ErrorAction SilentlyContinue
    return $value
}

function Remove-RunKey {
    Remove-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'CrimsonsServer' -ErrorAction SilentlyContinue
    Remove-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'CrimsonServer' -ErrorAction SilentlyContinue
}

function Get-TaskStatus {
    $task = Get-ScheduledTask -TaskName "CrimsonsServer" -ErrorAction SilentlyContinue
    if ($null -eq $task) {
        $task = Get-ScheduledTask -TaskName "CrimsonServer" -ErrorAction SilentlyContinue
    }
    
    if ($null -eq $task) {
        return "NOT_FOUND"
    }
    
    return $task.State
}

function Show-Status {
    Write-Banner "Crimsons Server Autostart Status"
    
    $serverPath = Find-ServerExecutable
    $taskStatus = Get-TaskStatus
    $runValue = Get-RunValue
    $startupLnk = Join-Path ([Environment]::GetFolderPath('Startup')) 'CrimsonsServer.lnk'
    $legacyLnk = Join-Path ([Environment]::GetFolderPath('Startup')) 'CrimsonServer.lnk'
    
    Write-Host "Server Executable:" -ForegroundColor Green
    if ($serverPath) {
        Write-Host "  ✓ Found: $serverPath" -ForegroundColor Green
        Write-Host "  Size: $([math]::Round((Get-Item $serverPath).Length / 1MB, 2))MB" -ForegroundColor Gray
    } else {
        Write-Host "  ✗ Not found in known paths" -ForegroundColor Red
    }

    Write-Host ""
    Write-Host "HKCU Run (primary / installer default):" -ForegroundColor Green
    if ($runValue) {
        Write-Host "  ✓ CrimsonsServer = $runValue" -ForegroundColor Green
        $exePath = ($runValue.Trim('"') -split '"')[0]
        if (-not (Test-Path $exePath)) {
            Write-Host "  ✗ Target executable MISSING — re-run enable" -ForegroundColor Red
        } elseif ($exePath -match '(?i)[\\/]target[\\/]debug[\\/]' -or $exePath -match '^[A-Z]:\\$' ) {
            Write-Host "  ✗ Target looks like a stale debug/drive path — re-run enable" -ForegroundColor Red
        }
    } else {
        Write-Host "  ✗ CrimsonsServer Run key not set" -ForegroundColor Yellow
    }

    Write-Host ""
    Write-Host "Startup folder:" -ForegroundColor Green
    if (Test-Path $startupLnk) {
        $shell = New-Object -ComObject WScript.Shell
        $sc = $shell.CreateShortcut($startupLnk)
        Write-Host "  ✓ CrimsonsServer.lnk -> $($sc.TargetPath)" -ForegroundColor Green
    } elseif (Test-Path $legacyLnk) {
        $shell = New-Object -ComObject WScript.Shell
        $sc = $shell.CreateShortcut($legacyLnk)
        Write-Host "  ✓ CrimsonServer.lnk (legacy) -> $($sc.TargetPath)" -ForegroundColor Green
    } else {
        Write-Host "  (no CrimsonsServer.lnk — OK if Run key is set)" -ForegroundColor Gray
    }
    
    Write-Host ""
    Write-Host "Task Scheduler (optional secondary):" -ForegroundColor Green
    Write-Host "  Task Name: CrimsonsServer" -ForegroundColor Gray
    
    if ($taskStatus -eq "NOT_FOUND") {
        Write-Host "  Status: NOT CONFIGURED" -ForegroundColor Yellow
    } else {
        $statusColor = if ($taskStatus -eq "Ready") { "Green" } else { "Yellow" }
        Write-Host "  Status: $taskStatus" -ForegroundColor $statusColor
        
        $task = Get-ScheduledTask -TaskName "CrimsonsServer" -ErrorAction SilentlyContinue
        if ($null -eq $task) { $task = Get-ScheduledTask -TaskName "CrimsonServer" -ErrorAction SilentlyContinue }
        if ($task) {
            Write-Host "  Last Run: $($task.LastRunTime)" -ForegroundColor Gray
            Write-Host "  Next Run: $($task.NextRunTime)" -ForegroundColor Gray
        }
    }
    
    Write-Host ""
    Write-Host "Server Port Status:" -ForegroundColor Green
    $portInUse = $null -ne (Get-NetTCPConnection -LocalPort 40510 -ErrorAction SilentlyContinue)
    if ($portInUse) {
        Write-Host "  ✓ Port 40510 is in use (server is running)" -ForegroundColor Green
    } else {
        Write-Host "  ✗ Port 40510 is not in use (server is not running)" -ForegroundColor Yellow
    }
}

function Enable-ServerAutostart {
    Write-Banner "Enabling Crimsons Server Autostart"
    
    $serverPath = Find-ServerExecutable
    if (-not $serverPath) {
        Write-Host "✗ Server executable not found!" -ForegroundColor Red
        return $false
    }

    Write-Host "Registering HKCU Run key -> $serverPath"
    try {
        $runValue = Set-RunKey -ServerPath $serverPath
        Write-Host "✓ Run key set: $runValue" -ForegroundColor Green
    } catch {
        Write-Host "✗ Failed to set Run key: $_" -ForegroundColor Red
        return $false
    }
    
    Write-Host "Creating Task Scheduler task (optional secondary)..."
    # Run key is the real autostart path. Task Scheduler is best-effort only
    # (UAC / principal quirks must never undo a good Run key).
    $userId = if ($env:USERDOMAIN) { "$env:USERDOMAIN\$env:USERNAME" } else { $env:USERNAME }
    try {
        $principal = New-ScheduledTaskPrincipal -UserId $userId -LogonType Interactive -RunLevel Limited
        $action = New-ScheduledTaskAction -Execute $serverPath -WorkingDirectory (Split-Path $serverPath)
        $trigger = New-ScheduledTaskTrigger -AtLogOn -User $userId
        $settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero)
        Unregister-ScheduledTask -TaskName "CrimsonServer" -Confirm:$false -ErrorAction SilentlyContinue | Out-Null
        Unregister-ScheduledTask -TaskName "CrimsonsServer" -Confirm:$false -ErrorAction SilentlyContinue | Out-Null
        $null = Register-ScheduledTask -TaskName "CrimsonsServer" -Principal $principal -Action $action -Trigger $trigger -Settings $settings -Force -ErrorAction Stop
        Write-Host "✓ Task created successfully" -ForegroundColor Green
    } catch {
        Write-Host "⚠ Task Scheduler optional; Run key is enough. ($_)" -ForegroundColor Yellow
    }
    return $true
}

function Disable-ServerAutostart {
    Write-Banner "Disabling Crimsons Server Autostart"
    
    try {
        Remove-RunKey
        Write-Host "✓ Run key removed" -ForegroundColor Green
    } catch {
        Write-Host "✗ Failed to remove Run key: $_" -ForegroundColor Red
    }

    try {
        Unregister-ScheduledTask -TaskName "CrimsonsServer" -Confirm:$false -ErrorAction SilentlyContinue | Out-Null
        Unregister-ScheduledTask -TaskName "CrimsonServer" -Confirm:$false -ErrorAction SilentlyContinue | Out-Null
        Write-Host "✓ Task removed (if present)" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "✗ Failed to remove task: $_" -ForegroundColor Red
        return $false
    }
}

function Reinstall-ServerAutostart {
    Write-Banner "Reinstalling Crimsons Server Autostart"
    Disable-ServerAutostart | Out-Null
    Start-Sleep -Seconds 1
    Enable-ServerAutostart
}

function Test-ServerStartup {
    Write-Banner "Testing Crimsons Server Startup"
    
    $serverPath = Find-ServerExecutable
    if (-not $serverPath) {
        Write-Host "✗ Server executable not found!" -ForegroundColor Red
        return
    }

    $already = Get-Process -Name "crimsons-server","crimson-server" -ErrorAction SilentlyContinue
    if ($already) {
        Write-Host "✓ Server already running (PID $($already.Id))" -ForegroundColor Green
        return
    }
    
    Write-Host "Attempting to start server: $serverPath"
    try {
        Start-Process -FilePath $serverPath -WorkingDirectory (Split-Path $serverPath) -WindowStyle Hidden
        Write-Host "✓ Server launched" -ForegroundColor Green
        Start-Sleep -Seconds 2
        
        $portInUse = $null -ne (Get-NetTCPConnection -LocalPort 40510 -ErrorAction SilentlyContinue)
        if ($portInUse) {
            Write-Host "✓ Server is responding on port 40510" -ForegroundColor Green
        } else {
            Write-Host "⚠ Server launched but port 40510 not responding yet" -ForegroundColor Yellow
        }
    }
    catch {
        Write-Host "✗ Failed to start server: $_" -ForegroundColor Red
    }
}

function Show-Logs {
    Write-Banner "Crimsons Server Event Logs"
    
    Write-Host "Recent Task Scheduler events for CrimsonsServer:" -ForegroundColor Green
    Get-WinEvent -FilterHashtable @{
        LogName = "Microsoft-Windows-TaskScheduler/Operational"
        Data = "CrimsonsServer"
    } -MaxEvents 20 -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host "$($_.TimeCreated) - $($_.Message)" -ForegroundColor Gray
    }
}

switch ($Action) {
    'status' { Show-Status }
    'enable' { Enable-ServerAutostart }
    'disable' { Disable-ServerAutostart }
    'reinstall' { Reinstall-ServerAutostart }
    'test' { Test-ServerStartup }
    'logs' { Show-Logs }
}
