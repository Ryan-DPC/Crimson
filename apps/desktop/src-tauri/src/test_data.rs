use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerNote {
    pub name: String,
    pub note: String,
    pub tag: String,
    pub last_seen: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub player_notes: HashMap<String, PlayerNote>,
    pub auto_accept: bool,
    pub match_ids: Vec<u64>,
    
    #[serde(default)]
    pub auto_ban: Option<u64>,
    #[serde(default)]
    pub auto_pick: Option<u64>,
    #[serde(default)]
    pub gemini_api_key: Option<String>,
    #[serde(default)]
    pub last_lp_delta: Option<i32>,
    #[serde(default)]
    pub hist: Option<serde_json::Value>,

    #[serde(default)]
    pub remembered_auto_ban: Option<u64>,
    #[serde(default)]
    pub remembered_auto_pick: Option<u64>,
}

fn main() {
    let path = "C:\\Users\\ryand\\AppData\\Roaming\\com.laoy.crimson\\data.json";
    let content = fs::read_to_string(path).unwrap();
    match serde_json::from_str::<AppData>(&content) {
        Ok(data) => println!("Parsed successfully! {:?}", data.gemini_api_key),
        Err(e) => println!("Error parsing data.json: {}", e),
    }
}
