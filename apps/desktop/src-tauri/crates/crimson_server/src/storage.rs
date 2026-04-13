use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppData {
    pub auto_accept: bool,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            auto_accept: true,
            other: HashMap::new(),
        }
    }
}

pub fn get_data_path_from_env() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let path = PathBuf::from(appdata).join("com.laoy.crimson");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        path.join("data.json")
    } else {
        PathBuf::from("./data.json")
    }
}

pub fn load_data_from_path(path: PathBuf) -> AppData {
    if let Ok(content) = fs::read_to_string(path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AppData::default()
    }
}
