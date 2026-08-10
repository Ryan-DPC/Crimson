@echo off
REM Launch the CRIMSONS local sidecar (detached).
REM Prefers the official Program Files install, then local build outputs.

setlocal enabledelayedexpansion

set "paths[0]=%ProgramFiles%\CRIMSONS\crimson-server.exe"
set "paths[1]=%ProgramFiles(x86)%\CRIMSONS\crimson-server.exe"
set "paths[2]=%~dp0..\target\release\crimson-server.exe"
set "paths[3]=%~dp0crimson-server.exe"
set "paths[4]=%~dp0bin\crimson-server.exe"

set "found=0"
for /l %%i in (0,1,4) do (
    if exist "!paths[%%i]!" (
        set "exe_path=!paths[%%i]!"
        set "found=1"
        goto :found
    )
)

if !found! equ 0 (
    echo Error: crimson-server.exe not found in any known path
    echo Checked:
    for /l %%i in (0,1,4) do (
        echo   - !paths[%%i]!
    )
    exit /b 1
)

:found
for %%I in ("!exe_path!") do set "exe_dir=%%~dpI"
echo Found crimson-server at: !exe_path!
echo Launching...

start "" /d "!exe_dir!" /b "!exe_path!"

echo crimson-server started successfully
echo Server should be accessible at ws://127.0.0.1:40510
exit /b 0
