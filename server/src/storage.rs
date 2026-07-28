use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    #[serde(default = "default_true")]
    pub auto_accept: bool,
    // Renseignes par l'utilisateur depuis les parametres de l'app : chaque
    // installation utilise sa propre application Spotify. Aucune valeur par
    // defaut, un secret OAuth n'a pas a etre distribue dans le binaire.
    #[serde(default)]
    pub spotify_client_id: String,
    #[serde(default)]
    pub spotify_client_secret: String,
    
    // Premium and Custom Settings
    #[serde(default)]
    pub is_premium: bool,
    #[serde(default)]
    pub premium_token: Option<String>,
    #[serde(default)]
    pub custom_server_path: Option<String>,
    #[serde(default)]
    pub discord_client_id: Option<String>,

    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool { true }

impl Default for AppData {
    fn default() -> Self {
        Self {
            auto_accept: default_true(),
            spotify_client_id: String::new(),
            spotify_client_secret: String::new(),
            is_premium: false,
            premium_token: None,
            custom_server_path: None,
            discord_client_id: None,
            other: HashMap::new(),
        }
    }
}

lazy_static::lazy_static! {
    static ref CACHED_APP_DATA: std::sync::RwLock<Option<(AppData, std::time::SystemTime)>> = std::sync::RwLock::new(None);
}

pub fn get_data_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    // Must match the Tauri frontend's path (lcu_commands crate uses "com.laoy.crimson")
    let path = PathBuf::from(appdata).join("com.laoy.crimson");
    if !path.exists() {
        let _ = fs::create_dir_all(&path);
    }
    path
}

pub fn get_data_path_from_env() -> PathBuf {
    get_data_dir().join("data.json")
}

pub fn load_data_from_path(path: PathBuf) -> AppData {
    let disk_mtime = fs::metadata(&path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    if let Ok(cache) = CACHED_APP_DATA.read() {
        if let Some((data, cached_mtime)) = cache.as_ref() {
            if disk_mtime <= *cached_mtime {
                return data.clone();
            }
        }
    }

    let mut loaded_data = AppData::default();
    for _ in 0..5 {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(data) = serde_json::from_str(&content) {
                loaded_data = data;
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    let final_mtime = fs::metadata(&path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    if let Ok(mut cache) = CACHED_APP_DATA.write() {
        *cache = Some((loaded_data.clone(), final_mtime));
    }
    loaded_data
}

pub fn save_data_to_path(path: PathBuf, data: &AppData) {
    if let Ok(content) = serde_json::to_string_pretty(data) {
        for _ in 0..5 {
            if fs::write(&path, &content).is_ok() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    let final_mtime = fs::metadata(&path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    if let Ok(mut cache) = CACHED_APP_DATA.write() {
        *cache = Some((data.clone(), final_mtime));
    }
}



pub fn rotate_log(log_name: &str, max_size_mb: u64) {
    let log_path = get_data_dir().join(log_name);
    let mut should_rotate = false;
    
    if let Ok(metadata) = fs::metadata(&log_path) {
        // Rotate if too big
        if metadata.len() > max_size_mb * 1024 * 1024 {
            should_rotate = true;
        }
        
        // Rotate if older than 24h
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                if elapsed.as_secs() > 86400 {
                    should_rotate = true;
                }
            }
        }
    }
    
    if should_rotate {
        let old_path = get_data_dir().join(format!("{}.old", log_name));
        let _ = fs::remove_file(&old_path); // Clear previous old
        let _ = fs::rename(&log_path, &old_path);
    }
}

pub fn log_to_file(log_name: &str, message: &str) {
    rotate_log(log_name, 5); // Max 5MB / 24h
    let log_path = get_data_dir().join(log_name);
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(log_path) {
        use std::io::Write;
        let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), message);
    }
}

