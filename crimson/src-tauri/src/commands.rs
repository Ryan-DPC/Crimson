use tauri::Manager;
use lcu_commands::storage;
use std::os::windows::process::CommandExt;

#[tauri::command]
pub async fn crimson_quit_app(app: tauri::AppHandle) {
  let _ = crimson_stop_server(app.clone()).await;
  app.exit(0);
}

#[tauri::command]
pub fn check_plugin_presence(plugin_id: String) -> bool {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let elgato_path = std::path::Path::new(&appdata)
            .join("Elgato")
            .join("StreamDeck")
            .join("Plugins")
            .join(format!("{}.sdPlugin", plugin_id));
            
        let streamdock_path = std::path::Path::new(&appdata)
            .join("HotSpot")
            .join("StreamDock")
            .join("plugins")
            .join(format!("{}.sdPlugin", plugin_id));

        return (elgato_path.exists() && elgato_path.is_dir()) || 
               (streamdock_path.exists() && streamdock_path.is_dir());
    }
    false
}



#[tauri::command]
pub fn crimson_toggle_server_autostart(app: tauri::AppHandle, enable: bool) -> Result<(), String> {
  // Update AppData
  let mut data = storage::load_data(&app);
  data.server_launch_on_startup = enable;
  storage::save_data(&app, &data);

  if let Ok(exe_path) = std::env::current_exe() {
      let mut exe_path_str = exe_path.to_string_lossy().to_string();
      if exe_path_str.starts_with("\\\\?\\") {
          exe_path_str = exe_path_str[4..].to_string();
      }
      
      if enable {
          create_server_registry_run(&exe_path_str)
      } else {
          remove_server_registry_run()
      }
  } else {
      Err("Could not find main executable for autostart".to_string())
  }
}

fn create_server_registry_run(exe_path: &str) -> Result<(), String> {
    use std::process::Command;
    
    // Append --autostart so the Tauri app runs hidden
    let ps_command = format!(
        r#"Set-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "CrimsonServer" -Value '"{}" --autostart'"#,
        exe_path
    );

    let output = Command::new("powershell")
        .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_command])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Registry registration failed: {}", stderr))
    }
}

fn remove_server_registry_run() -> Result<(), String> {
    use std::process::Command;
    
    let ps_command = r#"Remove-ItemProperty -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "CrimsonServer" -ErrorAction SilentlyContinue"#;

    let output = Command::new("powershell")
        .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps_command])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("Failed to execute PowerShell: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Registry removal failed: {}", stderr))
    }
}

#[tauri::command]
pub fn crimson_get_server_autostart_info(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    use std::process::Command;
    use serde_json::json;
    
    let mut info = json!({
        "is_enabled": false,
        "server_path": Option::<String>::None,
        "task_exists": false,
        "errors": Vec::<String>::new()
    });
    
    // Check if server executable exists
    if let Some(server_path) = find_server_path(&app) {
        info["server_path"] = json!(server_path.to_string_lossy().to_string());
        
        if !server_path.exists() {
            info["errors"].as_array_mut().unwrap().push("Server executable not found".into());
        }
    } else {
        info["errors"].as_array_mut().unwrap().push("Could not locate server executable".into());
    }
    
    // Check if Registry key exists
    let ps_command = r#"
$val = Get-ItemPropertyValue -Path "HKCU:\Software\Microsoft\Windows\CurrentVersion\Run" -Name "CrimsonServer" -ErrorAction SilentlyContinue
if ($null -ne $val) { Write-Host "EXISTS" } else { Write-Host "NOTFOUND" }
"#;
    
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", ps_command])
        .creation_flags(0x08000000)
        .output()
        .map_err(|e| format!("Failed to check task: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    info["task_exists"] = json!(stdout.contains("EXISTS"));
    
    // Check AppData setting
    let data = storage::load_data(&app);
    info["is_enabled"] = json!(data.server_launch_on_startup);
    
    Ok(info)
}

fn log_to_launch_file(app: &tauri::AppHandle, message: &str) {
    if let Ok(app_data) = app.path().app_data_dir() {
        let log_path = app_data.join("launch_debug.log");
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(log_path) {
            use std::io::Write;
            let _ = writeln!(file, "[{:?}] {}", std::time::SystemTime::now(), message);
        }
    }
}

/// Jeton d'authentification du serveur local, lu depuis le dossier de donnees.
/// Il transite par une commande Tauri plutot que d'etre expose au frontend :
/// la webview n'a pas acces au disque.
#[tauri::command]
pub fn crimson_get_auth_token() -> Option<String> {
    crimson_server::auth::read_token().filter(|t| !t.is_empty())
}

#[tauri::command]
pub fn crimson_get_actual_server_path(app: tauri::AppHandle) -> Option<String> {
    find_server_path(&app).map(|p| {
        let mut s = p.to_string_lossy().into_owned();
        if s.starts_with(r"\\?\") {
            s = s.replacen(r"\\?\", "", 1);
        }
        s
    })
}

#[tauri::command]
pub async fn crimson_start_server(app: tauri::AppHandle) -> Result<(), String> {
    log_to_launch_file(&app, "crimson_start_server command invoked by user");
    crimson_spawn_server(app).await
}

#[tauri::command]
pub async fn crimson_stop_server(app: tauri::AppHandle) -> Result<(), String> {
    log_to_launch_file(&app, "crimson_stop_server command invoked");
    let sidecar_state = app.state::<lcu_commands::SidecarChild>();
    let mut lock = sidecar_state.0.lock().await;
    if let Some(mut child) = lock.take() {
        let _ = child.kill();
        log_to_launch_file(&app, "Killed sidecar process managed by SidecarChild state");
    }
    
    use sysinfo::{System, ProcessRefreshKind, RefreshKind};
    let mut sys = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::new())
    );
    sys.refresh_processes();
    for process in sys.processes_by_exact_name("crimson-server.exe") {
        let _ = process.kill();
        log_to_launch_file(&app, &format!("Killed external crimson-server.exe process (PID: {})", process.pid()));
    }
    for process in sys.processes_by_exact_name("crimson-server-x86_64-pc-windows-msvc.exe") {
        let _ = process.kill();
        log_to_launch_file(&app, &format!("Killed legacy external sidecar process (PID: {})", process.pid()));
    }
    Ok(())
}

#[tauri::command]
pub async fn crimson_restart_server(app: tauri::AppHandle) -> Result<(), String> {
    log_to_launch_file(&app, "crimson_restart_server command invoked by user");
    let _ = crimson_stop_server(app.clone()).await;
    // Brief pause so the mutex / port are released before re-spawn.
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    crimson_spawn_server(app).await
}

pub async fn crimson_spawn_server(handle: tauri::AppHandle) -> Result<(), String> {
    let services = vec!["main"];
    log_to_launch_file(&handle, "crimson_spawn_server started");
    let mut last_error: Option<String> = None;
    
    for service_name in services {
        let port = 40510;

        if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
            log_to_launch_file(&handle, &format!("Service {} is already running on port {}. Skipping spawn.", service_name, port));
            continue;
        }

        let p_opt = find_server_path(&handle);
        log_to_launch_file(&handle, &format!("Resolved server path: {:?}", p_opt));

        if let Some(p) = p_opt {
            const DETACHED_PROCESS: u32 = 0x00000008;
            const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x01000000;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            let mut cmd = std::process::Command::new(&p);
            cmd.arg("--service").arg(service_name);
            // Le serveur en build de developpement refuse de demarrer sans ce
            // signal, pour ne pas etre ressuscite par la restauration de
            // session Windows. Une application de developpement autorise donc
            // explicitement son propre sidecar.
            if cfg!(debug_assertions) {
                cmd.env("CRIMSON_DEV", "1");
            }
            cmd.creation_flags(DETACHED_PROCESS | CREATE_BREAKAWAY_FROM_JOB | CREATE_NO_WINDOW);
            if let Some(parent) = p.parent() { cmd.current_dir(parent); }
            
            log_to_launch_file(&handle, &format!("Attempting first-stage spawn of service {} with breakaway flag...", service_name));
            let spawn_result = match cmd.spawn() {
                Ok(child) => {
                    log_to_launch_file(&handle, &format!("SUCCESS: Spanned service {} (first stage) from {:?}", service_name, p));
                    if service_name == "main" {
                        let sidecar_state = handle.state::<lcu_commands::SidecarChild>();
                        let mut lock = sidecar_state.0.lock().await;
                        *lock = Some(child);
                    }
                    Ok(())
                },
                Err(e) => {
                    log_to_launch_file(&handle, &format!("WARNING: First-stage spawn failed: {}. Retrying without CREATE_BREAKAWAY_FROM_JOB...", e));
                    
                    // Fallback to spawning without CREATE_BREAKAWAY_FROM_JOB in case parent is in a restricted Job Object
                    let mut cmd2 = std::process::Command::new(&p);
                    cmd2.arg("--service").arg(service_name);
                    if cfg!(debug_assertions) {
                        cmd2.env("CRIMSON_DEV", "1");
                    }
                    cmd2.creation_flags(DETACHED_PROCESS | CREATE_NO_WINDOW);
                    if let Some(parent) = p.parent() { cmd2.current_dir(parent); }
                    
                    match cmd2.spawn() {
                        Ok(child2) => {
                            log_to_launch_file(&handle, &format!("SUCCESS: Spanned service {} (fallback stage) from {:?}", service_name, p));
                            if service_name == "main" {
                                let sidecar_state = handle.state::<lcu_commands::SidecarChild>();
                                let mut lock = sidecar_state.0.lock().await;
                                *lock = Some(child2);
                            }
                            Ok(())
                        },
                        Err(e2) => {
                            let msg = format!("Spawn completely failed at both stages. Final error: {}", e2);
                            log_to_launch_file(&handle, &format!("ERROR: {}", msg));
                            Err(msg)
                        }
                    }
                }
            };

            if let Err(e) = spawn_result {
                last_error = Some(e);
                continue;
            }

            // Spawn can report success while a debug binary exits immediately
            // (missing CRIMSON_DEV) or crashes before bind. Confirm the port.
            let mut up = false;
            for _ in 0..25 {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                    up = true;
                    break;
                }
            }
            if up {
                log_to_launch_file(&handle, &format!("Service {} is listening on port {}.", service_name, port));
            } else {
                let msg = format!(
                    "Sidecar spawned from {:?} but port {} never opened (debug build needs CRIMSON_DEV=1, or binary crashed — see AppData launch_debug / crimson-server.log)",
                    p, port
                );
                log_to_launch_file(&handle, &format!("ERROR: {}", msg));
                last_error = Some(msg);
            }
        } else {
            let msg = "No server executable path was found (expected crimson-server.exe or crimson-server-x86_64-pc-windows-msvc.exe next to the app or in src-tauri/bin).".to_string();
            log_to_launch_file(&handle, &format!("ERROR: {}", msg));
            last_error = Some(msg);
        }
    }

    match last_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

fn find_server_path(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    // 0. Check custom path first
    let data = storage::load_data(app);
    if let Some(custom_path) = data.custom_server_path {
        let p = std::path::PathBuf::from(&custom_path);
        if p.is_file() {
            return Some(p);
        }
    }

    let path_resolver = app.path();
    let sidecar_base = "crimson-server.exe";
    let sidecar_arch = "crimson-server-x86_64-pc-windows-msvc.exe";
    
    // Priority order for finding the server executable
    let mut search_paths = vec![];
    
    // 1. Tauri resource directory (bundled with app)
    if let Ok(resource_dir) = path_resolver.resource_dir() {
        search_paths.push(resource_dir.clone());
        search_paths.push(resource_dir.join("bin"));
    }
    
    // 2. Executable directory (app folder)
    if let Ok(exe_dir) = path_resolver.executable_dir() {
        search_paths.push(exe_dir.clone());
        search_paths.push(exe_dir.join("bin"));
    }
    
    // 3. Development paths (for development/debugging)
    // Try to find from the app's parent directories
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Navigate up from bin directory or app directory
            if let Some(parent) = exe_dir.parent() {
                search_paths.push(parent.join("target/release"));
                search_paths.push(parent.join("target/debug"));
                search_paths.push(parent.join("cargo-target-hotfix2/release"));
                search_paths.push(parent.join("cargo-target-hotfix/release"));
                // Workspace layout: target/{debug,release} and crimson/src-tauri/bin
                if let Some(repo) = parent.parent() {
                    search_paths.push(repo.join("target/release"));
                    search_paths.push(repo.join("target/debug"));
                    search_paths.push(repo.join("crimson/src-tauri/bin"));
                    search_paths.push(repo.join("src-tauri/bin"));
                }
            }
            // Direct sibling of the exe (Tauri copies externalBin here on dev/build)
            search_paths.push(exe_dir.to_path_buf());
            search_paths.push(exe_dir.join("bin"));
        }
    }
    
    // 4. Common Crimson installation paths
    if let Ok(user) = std::env::var("USERNAME") {
        let app_data = format!("C:\\Users\\{}\\AppData\\Local\\crimson\\bin", user);
        search_paths.push(std::path::PathBuf::from(&app_data));
    }
    
    // Search for the executable in all paths
    for root in search_paths {
        for name in &[sidecar_base, sidecar_arch] {
            let p = root.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    
    // Last resort: search in parent directories
    if let Ok(exe_path) = std::env::current_exe() {
        let mut current = exe_path.parent();
        let mut depth = 0;
        while let Some(dir) = current {
            if depth > 5 { break; } // Limit search depth
            
            for name in &[sidecar_base, sidecar_arch] {
                let p = dir.join(name);
                if p.is_file() { return Some(p); }
                
                let p = dir.join("bin").join(name);
                if p.is_file() { return Some(p); }
                
                let p = dir.join("target/release").join(name);
                if p.is_file() { return Some(p); }
            }
            
            current = dir.parent();
            depth += 1;
        }
    }
    
    None
}
#[tauri::command]
pub async fn exchange_spotify_token(app: tauri::AppHandle, code: String) -> Result<(), String> {
    // Credentials live only in data.json / the sidecar — never passed from the webview.
    let data = storage::load_data(&app);
    let client_id = data
        .other
        .get("spotifyClientId")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let client_secret = data
        .other
        .get("spotifyClientSecret")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if client_id.is_empty() || client_secret.is_empty() {
        log_to_launch_file(&app, "[SPOTIFY] Echange impossible - identifiants absents de data.json");
        return Err("Identifiants Spotify absents. Renseignez-les dans les parametres.".into());
    }

    log_to_launch_file(
        &app,
        &format!(
            "[SPOTIFY] Echange demarre - client_id len={}, secret len={} (valeurs non journalisees)",
            client_id.len(),
            client_secret.len()
        ),
    );
    let client = reqwest::Client::new();
    let auth = base64::Engine::encode(&base64::prelude::BASE64_STANDARD, format!("{}:{}", client_id, client_secret));
    
    let params = [
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("redirect_uri", "http://127.0.0.1:40510/callback"),
    ];

    let resp = client.post("https://accounts.spotify.com/api/token")
        .header("Authorization", format!("Basic {}", auth))
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let access_token = json["access_token"].as_str().ok_or("No access token")?;
        let refresh_token = json["refresh_token"].as_str().ok_or("No refresh token")?;
        let expires_in = json["expires_in"].as_u64().unwrap_or(3600);

        // Send to sidecar via WS (Spotify is on 40510)
        use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
        use futures_util::SinkExt;
        use serde_json::json;

        // Presente le jeton local, comme tout autre client du serveur.
        let url = match crimson_server::auth::read_token() {
            Some(t) if !t.is_empty() => format!("ws://127.0.0.1:40510/?token={}", t),
            _ => "ws://127.0.0.1:40510".to_string(),
        };
        match connect_async(&url).await {
            Ok((mut ws_stream, _)) => {
                let msg = json!({
                    "type": "SPOTIFY_AUTH",
                    "access_token": access_token,
                    "refresh_token": refresh_token,
                    "expires_in": expires_in,
                    // Transmis au serveur pour qu'il puisse rafraichir le jeton
                    // plus tard : le rafraichissement exige l'authentification
                    // Basic client_id:client_secret.
                    "client_id": client_id,
                    "client_secret": client_secret
                }).to_string();
                if let Err(e) = ws_stream.send(Message::Text(msg.into())).await {
                    log_to_launch_file(&app, &format!("[SPOTIFY] Echange reussi mais transmission au serveur impossible : {}", e));
                    return Err(format!("Serveur local injoignable : {}", e));
                }

                // Fermeture negociee, et non abandon de la connexion. Le serveur
                // pousse plusieurs etats initiaux avant de lire : si la socket
                // est deja fermee, ces ecritures echouent et la trame recue est
                // perdue avec elles. Attendre la reponse de fermeture garantit
                // qu'il a lu nos trames, donc traite SPOTIFY_AUTH.
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    ws_stream.close(None),
                )
                .await
                {
                    Ok(Ok(())) => log_to_launch_file(&app, "[SPOTIFY] Echange reussi, identifiants transmis au serveur"),
                    Ok(Err(e)) => log_to_launch_file(&app, &format!("[SPOTIFY] Identifiants envoyes, fermeture anormale : {}", e)),
                    Err(_) => log_to_launch_file(&app, "[SPOTIFY] Identifiants envoyes, le serveur n'a pas confirme la fermeture en 5 s"),
                }
            }
            Err(e) => {
                // Sans cette transmission le serveur ne peut pas rafraichir le
                // jeton : l'echange serait perdu au bout d'une heure.
                log_to_launch_file(&app, &format!("[SPOTIFY] Echange reussi mais connexion au serveur refusee : {}", e));
                return Err(format!("Serveur local injoignable : {}", e));
            }
        }
        Ok(())
    } else {
        // Le corps de la reponse contient la raison exacte du refus
        // (invalid_client, invalid_grant, redirect_uri_mismatch...).
        let status = resp.status();
        let body = resp.text().await.unwrap_or_else(|_| "<corps illisible>".to_string());
        log_to_launch_file(&app, &format!("[SPOTIFY] Echange refuse - HTTP {} - {}", status, body));
        Err(format!("Spotify a refuse l'echange (HTTP {}) : {}", status, body))
    }
}

#[tauri::command]
pub async fn youtube_search(query: String) -> Result<String, String> {
    let url = reqwest::Url::parse_with_params("https://m.youtube.com/results", &[("search_query", &query)]).map_err(|e| e.to_string())?;
    
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("Cookie", reqwest::header::HeaderValue::from_static("SOCS=CAI"));

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .default_headers(headers)
        .build()
        .map_err(|e| e.to_string())?;
        
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    let html = resp.text().await.map_err(|e| e.to_string())?;
    
    let re = regex::Regex::new(r#"/watch\?v=([a-zA-Z0-9_-]{11})"#).unwrap();
    if let Some(cap) = re.captures(&html) {
        if let Some(m) = cap.get(1) {
            return Ok(m.as_str().to_string());
        }
    }
    
    let re2 = regex::Regex::new(r#""videoId":"([a-zA-Z0-9_-]{11})""#).unwrap();
    if let Some(cap) = re2.captures(&html) {
        if let Some(m) = cap.get(1) {
            return Ok(m.as_str().to_string());
        }
    }

    println!("YOUTUBE HTML: {}", &html[..std::cmp::min(html.len(), 500)]);
    Err("No video found".to_string())
}

#[tauri::command]
pub async fn download_music_video(
    app: tauri::AppHandle,
    video_id: String,
    artist: String,
    track: String,
) -> Result<String, String> {
    use rusty_ytdl::{Video, VideoOptions, VideoQuality, VideoSearchOptions};
    
    // Setup paths
    let app_data = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let videos_dir = app_data.join("videos").join(sanitize_filename(&artist));
    std::fs::create_dir_all(&videos_dir).map_err(|e| e.to_string())?;
    
    let file_path = videos_dir.join(format!("{}.mp4", sanitize_filename(&track)));
    
    // Check if already exists
    if file_path.exists() {
        return Ok(file_path.to_string_lossy().to_string());
    }
    
    let url = format!("https://www.youtube.com/watch?v={}", video_id);
    let video_options = VideoOptions {
        quality: VideoQuality::HighestVideo,
        filter: VideoSearchOptions::Video, // Video only, no audio!
        download_options: rusty_ytdl::DownloadOptions::default(),
        request_options: rusty_ytdl::RequestOptions::default(),
    };
    
    let video = Video::new_with_options(&url, video_options.clone())
        .map_err(|e| e.to_string())?;
    
    video.download(&file_path).await.map_err(|e: rusty_ytdl::VideoError| {
        let _ = std::fs::remove_file(&file_path);
        e.to_string()
    })?;
    
    Ok(file_path.to_string_lossy().to_string())
}

fn sanitize_filename(name: &str) -> String {
    name.replace(&['\\', '/', ':', '*', '?', '"', '<', '>', '|'][..], "")
}
