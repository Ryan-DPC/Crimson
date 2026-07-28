$ErrorActionPreference = "Stop"
Write-Host "Building crimson-server release..."
cargo build --release -p crimson-server

Write-Host "Copying sidecar..."
Copy-Item "f:\CrimsonProject\target\release\crimson-server.exe" "f:\CrimsonProject\crimson\src-tauri\bin\crimson-server-x86_64-pc-windows-msvc.exe" -Force

Write-Host "Building Tauri release bundle..."
cd f:\CrimsonProject\crimson
npm run tauri build

Write-Host "Done!"
