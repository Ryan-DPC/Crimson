use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::events::WsSender;
use serde_json::json;
use tokio::net::windows::named_pipe::ClientOptions;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DiscordState {
    pub is_muted: bool,
    pub is_deaf: bool,
    pub is_camera_on: bool,
    pub connected: bool,
    pub current_channel_id: Option<String>,
}

pub struct DiscordService {
    state: Arc<RwLock<DiscordState>>,
    sender: WsSender,
    client_id: String,
    cmd_sender: tokio::sync::mpsc::Sender<(String, serde_json::Value)>,
    cmd_receiver: Arc<tokio::sync::Mutex<tokio::sync::mpsc::Receiver<(String, serde_json::Value)>>>,
    pub is_enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl DiscordService {
    pub fn new(sender: WsSender) -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        Self {
            state: Arc::new(RwLock::new(DiscordState::default())),
            sender,
            client_id: "1330663435166412852".to_string(), // Placeholder - User should update
            cmd_sender: tx,
            cmd_receiver: Arc::new(tokio::sync::Mutex::new(rx)),
            is_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn subscribe_events(&self) -> tokio::sync::broadcast::Receiver<String> {
        self.sender.0.subscribe()
    }

    pub async fn is_connected(&self) -> bool {
        self.state.read().await.connected
    }

    pub async fn start_background_polling(&self) {
        let state_clone = self.state.clone();
        let sender_clone = self.sender.clone();
        let cmd_rx = self.cmd_receiver.clone();
        let is_enabled_clone = self.is_enabled.clone();

        tokio::spawn(async move {
            let mut rx = cmd_rx.lock().await;
            loop {
                if !is_enabled_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    continue;
                }

                let app_data = crate::storage::load_data_from_path(crate::storage::get_data_path_from_env());
                let client_id = app_data.discord_client_id.unwrap_or_else(|| "1330663435166412852".to_string());

                if let Ok(mut pipe) = Self::connect_to_ipc().await {
                    println!("Connected to Discord IPC!");
                    
                    // Handshake
                    if Self::send_handshake(&mut pipe, &client_id).await.is_ok() {
                        {
                            let mut s = state_clone.write().await;
                            s.connected = true;
                        }
                        
                        // Subscribe to events (VOICE_SETTINGS_UPDATE)
                        let _ = Self::send_command(&mut pipe, "SUBSCRIBE", json!({ "evt": "VOICE_SETTINGS_UPDATE" })).await;

                        let mut buffer = [0u8; 4096];
                        loop {
                            tokio::select! {
                                result = pipe.read(&mut buffer) => {
                                    match result {
                                        Ok(n) if n > 0 => {
                                            // Handle multi-frame or partial reads if necessary, 
                                            // but for now assume small JSON payloads.
                                            let payload = &buffer[8..n]; // Skip 8-byte header
                                            if let Ok(payload_str) = std::str::from_utf8(payload) {
                                                println!("Discord IPC Payload: {}", payload_str);
                                            }
                                            if let Ok(v) = serde_json::from_slice::<serde_json::Value>(payload) {
                                                if let Some(evt) = v["evt"].as_str() {
                                                    if evt == "VOICE_SETTINGS_UPDATE" {
                                                        let data = &v["data"];
                                                        let mut s = state_clone.write().await;
                                                        s.is_muted = data["mute"].as_bool().unwrap_or(s.is_muted);
                                                        s.is_deaf = data["deaf"].as_bool().unwrap_or(s.is_deaf);
                                                        s.is_camera_on = data["video_enabled"].as_bool().unwrap_or(s.is_camera_on);
                                                        
                                                        let state_json = json!({
                                                            "type": "DISCORD_STATE",
                                                            "data": *s
                                                        }).to_string();
                                                        let _ = sender_clone.0.send(state_json);
                                                    }
                                                }
                                            }
                                        }
                                        _ => break,
                                    }
                                }
                                cmd_opt = rx.recv() => {
                                    if let Some((endpoint, params)) = cmd_opt {
                                        match endpoint.as_str() {
                                            "toggleMute" => {
                                                let current_mute = state_clone.read().await.is_muted;
                                                let _ = Self::send_command(&mut pipe, "SET_VOICE_SETTINGS", json!({ "mute": !current_mute })).await;
                                            }
                                            "toggleDeafen" => {
                                                let current_deaf = state_clone.read().await.is_deaf;
                                                let _ = Self::send_command(&mut pipe, "SET_VOICE_SETTINGS", json!({ "deaf": !current_deaf })).await;
                                            }
                                            "toggleCamera" => {
                                                let current_camera = state_clone.read().await.is_camera_on;
                                                let _ = Self::send_command(&mut pipe, "SET_VOICE_SETTINGS", json!({ "video_enabled": !current_camera })).await;
                                            }
                                            "joinVoiceChannel" => {
                                                if let Some(channel_id) = params.get("payload").and_then(|p| p.get("settings")).and_then(|s| s.get("channelId")).and_then(|c| c.as_str()).or_else(|| params.get("channelId").and_then(|c| c.as_str())) {
                                                    let current_chan = state_clone.read().await.current_channel_id.clone();
                                                    let target = if current_chan.as_deref() == Some(channel_id) { None } else { Some(channel_id) };
                                                    let _ = Self::send_command(&mut pipe, "SELECT_VOICE_CHANNEL", json!({ "channel_id": target, "force": true })).await;
                                                }
                                            }
                                            "playSoundboardSound" => {
                                                let sound_id = params.get("payload").and_then(|p| p.get("settings")).and_then(|s| s.get("soundId")).and_then(|c| c.as_str()).or_else(|| params.get("soundId").and_then(|c| c.as_str()));
                                                let guild_id = params.get("payload").and_then(|p| p.get("settings")).and_then(|s| s.get("guildId")).and_then(|c| c.as_str()).or_else(|| params.get("guildId").and_then(|c| c.as_str()));
                                                if let (Some(s_id), Some(g_id)) = (sound_id, guild_id) {
                                                    let _ = Self::send_command(&mut pipe, "OVERLAY_PLAY_SOUNDBOARD_SOUND", json!({ "sound_id": s_id, "guild_id": g_id })).await;
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                                    if !is_enabled_clone.load(std::sync::atomic::Ordering::Relaxed) {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                {
                    let mut s = state_clone.write().await;
                    s.connected = false;
                }
                println!("Discord IPC disconnected, retrying in 5s...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
    }

    pub async fn handle_command(&self, endpoint: &str, params: Option<serde_json::Value>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            tracing::info!("[DISCORD] Discord command ignored: service disabled");
            return Ok(());
        }

        if endpoint == "toggleScreenshare" {
            tokio::spawn(async move {
                let _ = Self::simulate_screenshare_keybind().await;
            });
            return Ok(());
        }
        
        let _ = self.cmd_sender.send((endpoint.to_string(), params.unwrap_or(json!({})))).await;
        Ok(())
    }

    async fn connect_to_ipc() -> Result<tokio::net::windows::named_pipe::NamedPipeClient, Box<dyn std::error::Error + Send + Sync>> {
        for i in 0..10 {
            let path = format!(r"\\.\pipe\discord-ipc-{}", i);
            if let Ok(client) = ClientOptions::new().open(&path) {
                return Ok(client);
            }
        }
        Err("Could not find Discord IPC pipe".into())
    }

    async fn send_handshake(pipe: &mut tokio::net::windows::named_pipe::NamedPipeClient, client_id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let payload = json!({ "v": 1, "client_id": client_id }).to_string();
        Self::send_raw_packet(pipe, 0, &payload).await
    }

    async fn send_command(pipe: &mut tokio::net::windows::named_pipe::NamedPipeClient, cmd: &str, args: serde_json::Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let nonce = uuid::Uuid::new_v4().to_string();
        let payload = json!({
            "cmd": cmd,
            "args": args,
            "nonce": nonce
        }).to_string();
        Self::send_raw_packet(pipe, 1, &payload).await
    }

    async fn send_raw_packet(pipe: &mut tokio::net::windows::named_pipe::NamedPipeClient, opcode: u32, payload: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut header = Vec::with_capacity(8);
        header.extend_from_slice(&opcode.to_le_bytes());
        header.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        pipe.write_all(&header).await?;
        pipe.write_all(payload.as_bytes()).await?;
        Ok(())
    }

    async fn simulate_screenshare_keybind() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;
        
        let ps_script = r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public class Win32 {
    [DllImport("user32.dll")] public static extern void keybd_event(byte bVk, byte bScan, int dwFlags, int dwExtraInfo);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern IntPtr FindWindow(string lpClassName, string lpWindowName);
}
"@
$prev = [Win32]::GetForegroundWindow()
$discord = [Win32]::FindWindow($null, "Discord")
if ($discord -eq [IntPtr]::Zero) {
    $procs = Get-Process | Where-Object { $_.MainWindowTitle -match "Discord" } | Select-Object -First 1
    if ($procs) { $discord = $procs.MainWindowHandle }
}
if ($discord -ne [IntPtr]::Zero) {
    [Win32]::SetForegroundWindow($discord) | Out-Null
    Start-Sleep -Milliseconds 200
}
# CTRL+SHIFT+F9
[Win32]::keybd_event(0x11, 0, 0, 0) # CTRL
[Win32]::keybd_event(0x10, 0, 0, 0) # SHIFT
[Win32]::keybd_event(0x78, 0, 0, 0) # F9
Start-Sleep -Milliseconds 80
[Win32]::keybd_event(0x78, 0, 2, 0)
[Win32]::keybd_event(0x10, 0, 2, 0)
[Win32]::keybd_event(0x11, 0, 2, 0)
Start-Sleep -Milliseconds 150
if ($prev -ne [IntPtr]::Zero) { [Win32]::SetForegroundWindow($prev) | Out-Null }
"#;

        Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps_script])
            .output()?;
            
        Ok(())
    }

    pub async fn adjust_aux_volume(ticks: i64) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let delta = if ticks > 0 { 0.05f32 } else { -0.05f32 };
        Self::send_vol_command(format!("vol {}\n", delta)).await;
        Ok(())
    }

    pub async fn toggle_aux_mute() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Self::send_vol_command("mute\n".to_string()).await;
        Ok(())
    }

    async fn send_vol_command(cmd: String) {
        if let Some(tx) = &*PS_VOL_TX.lock().await {
            let _ = tx.send(cmd).await;
        } else {
            // Initialize once
            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);
            *PS_VOL_TX.lock().await = Some(tx.clone());
            
            let script = r#"
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
namespace AudioCtrl {
    [ComImport][Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")] internal class MMDeviceEnumerator {}
    [Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)] internal interface IMMDeviceEnumerator { int GetDefaultAudioEndpoint(int d, int r, out IMMDevice p); }
    [Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)] internal interface IMMDevice { int Activate([MarshalAs(UnmanagedType.LPStruct)] Guid i, int c, IntPtr p, [MarshalAs(UnmanagedType.IUnknown)] out object o); }
    [Guid("77AA99A0-1BD6-484F-8BC7-2C654C9A9B6F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)] internal interface IAudioSessionManager2 { int GetSessionEnumerator(out IAudioSessionEnumerator e); }
    [Guid("E2F5BB11-0570-40CA-ACDD-3AA01277DEE8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)] internal interface IAudioSessionEnumerator { int GetCount(out int c); int GetSession(int i, out IAudioSessionControl s); }
    [Guid("F4B1A599-7266-4319-A8CA-E70ACB11E8CD"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)] internal interface IAudioSessionControl {}
    [Guid("87CE5498-68D6-44E5-9215-6DA47EF883D8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)] internal interface ISimpleAudioVolume { int SetMasterVolume(float f, Guid g); int GetMasterVolume(out float f); int SetMute(bool b, Guid g); int GetMute(out bool b); }
    [Guid("BFA971F1-4D5E-40BB-935E-967039BFBEE4"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)] internal interface IAudioSessionControl2 { int GetSessionIdentifier([MarshalAs(UnmanagedType.LPWStr)] out string s); int GetSessionInstanceIdentifier([MarshalAs(UnmanagedType.LPWStr)] out string s); int GetProcessId(out int p); }
    public static class DiscordVol {
        public static void Adjust(float delta, bool toggleMute) {
            IMMDeviceEnumerator e = (IMMDeviceEnumerator)new MMDeviceEnumerator();
            e.GetDefaultAudioEndpoint(0, 0, out IMMDevice dev);
            Guid g = typeof(IAudioSessionManager2).GUID;
            dev.Activate(g, 1, IntPtr.Zero, out object mo);
            IAudioSessionManager2 mgr = (IAudioSessionManager2)mo;
            mgr.GetSessionEnumerator(out IAudioSessionEnumerator se);
            se.GetCount(out int cnt);
            for (int i = 0; i < cnt; i++) {
                se.GetSession(i, out IAudioSessionControl sc);
                IAudioSessionControl2 sc2 = sc as IAudioSessionControl2;
                if (sc2 == null) continue;
                sc2.GetProcessId(out int pid);
                if (pid <= 0) continue;
                try {
                    var pr = System.Diagnostics.Process.GetProcessById(pid);
                    if (pr.ProcessName.ToLower().Contains("discord")) {
                        ISimpleAudioVolume vol = sc as ISimpleAudioVolume;
                        if (vol == null) continue;
                        if (toggleMute) { vol.GetMute(out bool mu); vol.SetMute(!mu, Guid.Empty); }
                        else { vol.GetMasterVolume(out float lv); lv += delta; if (lv > 1f) lv = 1f; if (lv < 0f) lv = 0f; vol.SetMasterVolume(lv, Guid.Empty); }
                    }
                } catch {}
            }
        }
    }
}
"@
while ($line = [Console]::ReadLine()) {
    if ($line -eq "quit") { break }
    if ($line.StartsWith("vol")) {
        $delta = [float]($line.Split(' ')[1].Replace(',','.'))
        [AudioCtrl.DiscordVol]::Adjust($delta, $false)
    } elseif ($line -eq "mute") {
        [AudioCtrl.DiscordVol]::Adjust(0.0, $true)
    }
}
"#;

            tokio::spawn(async move {
                use tokio::process::Command;
                use std::process::Stdio;
                use tokio::io::AsyncWriteExt;
                
                if let Ok(mut child) = Command::new("powershell")
                    .args(&["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn() 
                {
                    if let Some(stdin) = child.stdin.take() {
                        let mut stdin: tokio::process::ChildStdin = stdin;
                        while let Some(msg) = rx.recv().await {
                            if stdin.write_all(msg.as_bytes()).await.is_err() {
                                break;
                            }
                            let _ = stdin.flush().await;
                        }
                    }
                    let _ = child.kill().await;
                }
            });
            
            let _ = tx.send(cmd).await;
        }
    }
}

lazy_static::lazy_static! {
    static ref PS_VOL_TX: tokio::sync::Mutex<Option<tokio::sync::mpsc::Sender<String>>> = tokio::sync::Mutex::new(None);
}
