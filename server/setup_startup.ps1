$startupFolder = [Environment]::GetFolderPath('Startup')
$WshShell = New-Object -comObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut("$startupFolder\CrimsonServer.lnk")
$Shortcut.TargetPath = "F:\CrimsonProject\server\crimson-server.exe"
$Shortcut.WorkingDirectory = "F:\CrimsonProject\server"
$Shortcut.WindowStyle = 7
$Shortcut.Save()
Write-Host "Created startup shortcut at $startupFolder\CrimsonServer.lnk"
