use serde::{Deserialize, Serialize};
use crate::events::WsSender;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TwitchState {
    pub connected: bool,
    pub username: Option<String>,
    pub viewers: u32,
}

pub struct TwitchService {
    _state: Arc<RwLock<TwitchState>>,
    _sender: WsSender,
    pub is_enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl TwitchService {
    pub fn new(_sender: WsSender) -> Self {
        Self {
            _state: Arc::new(RwLock::new(TwitchState::default())),
            _sender,
            is_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub async fn handle_command(&self, endpoint: &str, params: Option<serde_json::Value>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            tracing::info!("[TWITCH] Twitch command ignored: service disabled");
            return Ok(());
        }

        match endpoint {
            "chat" => {
                if let Some(p) = params {
                    let message = p["payload"]["settings"]["message"].as_str().or(p["message"].as_str());
                    if let Some(msg) = message {
                        println!("Twitch: Sending chat message: {}", msg);
                    }
                }
            }
            "ad" => {
                println!("Twitch: Running ad...");
            }
            _ => return Err("Unknown Twitch command".into()),
        }
        Ok(())
    }

    pub async fn start_background_polling(&self) {
        println!("Twitch: Starting background polling for viewer count...");
        // TODO: Implement actual Twitch API polling
    }
}
