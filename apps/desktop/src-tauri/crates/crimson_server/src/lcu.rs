use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use base64::Engine;

static LCU_CACHE: Mutex<Option<LcuInfo>> = Mutex::new(None);
static LCU_FAIL_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LcuInfo {
    pub port: u16,
    pub password: String,
    pub protocol: String,
}

pub fn get_lcu_info() -> Result<LcuInfo, String> {
    // Return cached value if available
    if let Ok(cache) = LCU_CACHE.lock() {
        if let Some(info) = cache.as_ref() {
            return Ok(info.clone());
        }
    }

    // Method: PowerShell (standard for sidecar)
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command",
              "(Get-Process -Name LeagueClientUx -ErrorAction SilentlyContinue | Select-Object -First 1).CommandLine"]);
    
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    if let Ok(output) = cmd.output() {
        let cmdline = String::from_utf8_lossy(&output.stdout);
        let (port, password) = parse_cmdline(&cmdline);
        if port > 0 && !password.is_empty() {
            let info = LcuInfo { port, password, protocol: "https".into() };
            if let Ok(mut cache) = LCU_CACHE.lock() { *cache = Some(info.clone()); }
            return Ok(info);
        }
    }

    // Fallback: Default lockfile path
    let lockfile = "C:\\Riot Games\\League of Legends\\lockfile";
    if let Ok(content) = fs::read_to_string(lockfile) {
        let parts: Vec<&str> = content.split(':').collect();
        if parts.len() >= 5 {
            let info = LcuInfo {
                port: parts[2].trim().parse().unwrap_or(0),
                password: parts[3].trim().to_string(),
                protocol: parts[4].trim().to_string(),
            };
            if let Ok(mut cache) = LCU_CACHE.lock() { *cache = Some(info.clone()); }
            return Ok(info);
        }
    }

    Err("LCU not found".to_string())
}

fn parse_cmdline(cmdline: &str) -> (u16, String) {
    let mut port = 0u16;
    let mut password = String::new();
    for token in cmdline.split_whitespace() {
        if token.starts_with("--app-port=") {
            port = token.trim_start_matches("--app-port=").parse().unwrap_or(0);
        } else if token.starts_with("--remoting-auth-token=") {
            password = token.trim_start_matches("--remoting-auth-token=").to_string();
        }
    }
    (port, password)
}

pub fn lcu_request(method: String, endpoint: String, body: Option<String>) -> Result<String, String> {
    let info = get_lcu_info()?;
    let url = format!("{}://127.0.0.1:{}{}", info.protocol, info.port, endpoint);

    let mut request = ureq::request(&method.to_uppercase(), &url)
        .set("Authorization", &format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", info.password))))
        .timeout(std::time::Duration::from_secs(4));

    if let Some(b) = body {
        request = request.set("Content-Type", "application/json");
        match request.send_string(&b) {
            Ok(r) => handle_response(r),
            Err(e) => handle_error(e)
        }
    } else {
        match request.call() {
            Ok(r) => handle_response(r),
            Err(e) => handle_error(e)
        }
    }
}

fn handle_response(response: ureq::Response) -> Result<String, String> {
    LCU_FAIL_COUNT.store(0, Ordering::Relaxed);
    response.into_string().map_err(|e| e.to_string())
}

fn handle_error(err: ureq::Error) -> Result<String, String> {
    let count = LCU_FAIL_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    if count >= 10 {
        if let Ok(mut cache) = LCU_CACHE.lock() { *cache = None; }
        LCU_FAIL_COUNT.store(0, Ordering::Relaxed);
    }
    Err(err.to_string())
}
