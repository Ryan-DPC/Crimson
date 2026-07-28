use uuid::Uuid;
use std::fs;
use crate::storage;

pub fn generate_and_save_token() -> String {
    let token = Uuid::new_v4().to_string();
    let data_dir = storage::get_data_dir();
    let token_path = data_dir.join("auth.token");
    
    if let Err(e) = fs::write(&token_path, &token) {
        tracing::error!("Failed to write auth.token: {}", e);
    } else {
        tracing::info!("Auth token generated and saved to {:?}", token_path);
    }
    
    token
}

pub fn read_token() -> Option<String> {
    let data_dir = storage::get_data_dir();
    let token_path = data_dir.join("auth.token");
    fs::read_to_string(token_path).ok().map(|s| s.trim().to_string())
}
