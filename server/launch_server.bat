@echo off
REM Launch the CRIMSONS local sidecar (detached).
REM Prefers the official Program Files install, then local build outputs.

setlocal enabledelayedexpansion

set "paths[0]=%ProgramFiles%\CRIMSONS\crimsons-server.exe"
set "paths[1]=%ProgramFiles%\CRIMSONS\crimson-server.exe"
set "paths[2]=%ProgramFiles(x86)%\CRIMSONS\crimsons-server.exe"
set "paths[3]=%ProgramFiles(x86)%\CRIMSONS\crimson-server.exe"
set "paths[4]=%~dp0..\target\release\crimsons-server.exe"
set "paths[5]=%~dp0..\target\release\crimson-server.exe"
set "paths[6]=%~dp0crimsons-server.exe"
set "paths[7]=%~dp0crimson-server.exe"
set "paths[8]=%~dp0bin\crimsons-server.exe"
set "paths[9]=%~dp0bin\crimson-server.exe"

set "found=0"
for /l %%i in (0,1,9) do (
    if exist "!paths[%%i]!" (
        set "exe_path=!paths[%%i]!"
        set "found=1"
        goto :found
    )
)

if !found! equ 0 (
    echo Error: crimsons-server.exe not found in any known path
    echo Checked:
    for /l %%i in (0,1,9) do (
        echo   - !paths[%%i]!
    )
    exit /b 1
)

:found
for %%I in ("!exe_path!") do set "exe_dir=%%~dpI"
echo Found Crimsons Server at: !exe_path!
echo Launching...

start "" /d "!exe_dir!" /b "!exe_path!"

echo crimsons-server started successfully
echo Server should be accessible at ws://127.0.0.1:40510
exit /b 0
