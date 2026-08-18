# AGENTS.md

## Cursor Cloud specific instructions

CRIMSONS is a **Windows-native Tauri 2 desktop app** (React UI + Rust sidecar). See `README.md` for the full architecture. On the Linux cloud VM, the **frontend** (`crimson/`) always runs, and the **Rust sidecar** (`server/`) now builds/tests/runs on Linux too (see below). The full Tauri desktop app and the peripheral/client integrations remain Windows-oriented.

### What runs on Linux (the frontend — `crimson/`)

Standard scripts live in `crimson/package.json`. From `crimson/`:

- `npm run dev` — Vite dev server at `http://localhost:5173/` (this is the dev surface; do NOT use `npm run build`, which requires the Tauri toolchain).
- `npx tsc -b` — typecheck. This is the **blocking** check in CI (`.github/workflows/ci.yml`).
- `npm run lint` — ESLint. There is a large pre-existing lint debt (mostly `@typescript-eslint/no-explicit-any`); CI runs it with `continue-on-error: true`, so a non-zero lint exit is expected and non-blocking. Do not try to fix the whole backlog.

### Required env file (non-obvious gotcha)

The frontend imports `@supabase/supabase-js`'s `createClient` at module load, and it **throws on an empty URL**, which renders as a black screen (the README calls this out). To boot the UI you MUST have `crimson/.env` (git-ignored) with:

```
VITE_SUPABASE_URL=...
VITE_SUPABASE_ANON_KEY=...
```

Placeholder values (e.g. `https://mock.supabase.co` / any string) are enough to render the login screen and exercise the UI. A **real** Supabase project is required to actually sign up / log in; without it, submitting the login form fails with `Failed to fetch`, which is expected on the VM. If you see a black screen, the missing/empty `crimson/.env` is almost always the cause.

The app is a Tauri app, so `@tauri-apps/api/*` calls (window controls, `invoke`, `getVersion`) are no-ops/errors in a plain browser but degrade gracefully — the login screen renders and form interaction works without any mocks.

### Building the Rust sidecar (`server/`) on Linux

The `crimson-server` sidecar builds, unit-tests, and runs on Linux (Discord IPC is transport-abstracted: named pipe on Windows, Unix socket elsewhere; the Win32-only `windows` crate is target-gated). It is NOT in the update script (system deps / toolchain changes are out of scope there). Prerequisites, one-time:

- Rust >= 1.85 (a transitive dep requires edition 2024). The VM's pinned default may be older — `rustup default stable` fixes it.
- System libs: `sudo apt-get install -y pkg-config libssl-dev libx11-dev libxi-dev libxtst-dev` (OpenSSL for `native-tls`; X11 for `rdev` hotkeys).

Then, from the repo root:

- `cargo build -p crimson-server` — compiles on Linux.
- `cargo test -p crimson-server --lib` — the same suite CI runs on Windows (origin/auth + automation + storage tests).
- Run it with `CRIMSON_DEV=1 ./target/debug/crimson-server` (the dev guard refuses to start otherwise). It listens on `127.0.0.1:40510`. Data/logs go to a per-user dir (`~/.local/share/com.laoy.crimsons` on Linux via `$XDG_DATA_HOME`/`$HOME`; `%APPDATA%\com.laoy.crimsons` on Windows) — never the CWD.

### Launching the full desktop app (Linux and macOS)

The Tauri desktop app builds and launches on Linux, and the code paths for macOS are implemented too. From `crimson/` the launch is **one command** on any OS: `npm run dev:desktop` (on the Linux cloud VM prefix `DISPLAY=:1`). That runs `scripts/prepare-sidecar.mjs` (builds `crimson-server` and copies it to `src-tauri/bin/crimson-server-<triple>`, the `externalBin` Tauri expects) and then `tauri dev`. On login the app spawns the sidecar and the in-app SERVER indicator turns green. `npm run build:desktop` does the release build/bundle.

Per-OS one-time prerequisites:

- **Linux**: `sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev librsvg2-dev libayatana-appindicator3-dev`, plus the sidecar's libs, plus a display (the VM has one at `DISPLAY=:1`). Optional runtime tools for full parity: `pactl` (pipewire/pulseaudio-utils) and `xdotool`.
- **macOS**: Xcode Command Line Tools (`xcode-select --install`) — Tauri uses the system WebKit, so no GTK. **Not buildable/verifiable on this Linux VM** (no macOS SDK; deps like `openssl-sys` don't cross-compile); the macOS branches must be compiled and tested on a Mac.

Note: `getVersion()` (the footer version) only resolves inside the real Tauri app; in a plain browser it stays at the React default (`1.1.0`/`0.0.0`). The native app shows the real `3.1.4`.

### Cross-platform feature parity

Each OS integration below has a per-OS implementation (all fail-safe: log + no-op if the underlying tool/app is missing). Validation: `unit` = unit test; `live` = validated against a real PulseAudio/X session on the VM; `unverified` = written but only compilable/testable on that OS.

| Feature | Windows | Linux | macOS |
| --- | --- | --- | --- |
| Sidecar autostart at login | HKCU `Run` key | XDG `~/.config/autostart/*.desktop` (`unit`) | `~/Library/LaunchAgents/*.plist` + `launchctl` (`unverified`) |
| Discord aux volume / mute | Win32 COM audio | `pactl set-sink-input-{volume,mute}` (`unit`+`live`) | no public per-app volume API → logs + no-op |
| Discord screenshare toggle | Win32 `keybd_event` | `xdotool` (`live`) | `osascript`/System Events (`unverified`) |
| Discord IPC (mute/deafen/status) | named pipe | Unix socket (`$XDG_RUNTIME_DIR`) | Unix socket (`$TMPDIR`) — same code |
| Spotify process detection | `Spotify.exe`/`spotifyd.exe` | `spotify`/`spotifyd` | `Spotify`/`spotifyd` |
| spotifyd start / restart | VBScript/exe + `taskkill` | `spotifyd` on PATH + `pkill` | same as Linux |
| Per-user data/config dir | `%APPDATA%` | `$XDG_DATA_HOME`/`~/.local/share` | `~/Library/Application Support` |

The `cfg` split is: `#[cfg(windows)]`, `#[cfg(all(unix, not(target_os = "macos")))]` (Linux/other Unix), `#[cfg(target_os = "macos")]`. The Linux build compiling green verifies the shared/Linux paths; the macOS branches are cfg'd out there, so they are hand-reviewed only until built on a Mac.

### Genuinely OS-bound (no cross-platform equivalent)

- **League of Legends / LCU**: Riot's Vanguard anti-cheat blocks Linux, so the client cannot run there at all. Because of that, the `leagueOfLegends` plugin is **default-off on Linux** (`crimson_server::default_lol_enabled()`; default-on for Windows/macOS). The process detection already handles the macOS name (`LeagueClientUx`, no `.exe`), so LCU should work on macOS.
- **StreamDock**: the host application is Windows/macOS only; there is no Linux host that loads `.sdPlugin` packages. The plugins' JS is cross-platform, but they need the StreamDock host to run.
- `tools/integration_tester` and `tools/mock-lcu` are Node helpers that talk to the sidecar WS (`40510`).
