use serde::{Deserialize, Serialize};
use crate::events::WsSender;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Explicit product gate: Twitch API is not implemented yet.
pub const UNAVAILABLE_MSG: &str = "Twitch is not available yet (coming soon)";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TwitchState {
    pub connected: bool,
    pub username: Option<String>,
    pub viewers: u32,
}

pub struct TwitchService {
    _state: Arc<RwLock<TwitchState>>,
    _sender: WsSender,
    /// Kept for WS/entitlement wiring; commands always fail until the API exists.
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

    pub async fn handle_command(&self, endpoint: &str, _params: Option<serde_json::Value>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::warn!("[TWITCH] Rejected '{}': {}", endpoint, UNAVAILABLE_MSG);
        Err(UNAVAILABLE_MSG.into())
    }

    pub async fn start_background_polling(&self) {
        tracing::info!("[TWITCH] Background polling skipped: {}", UNAVAILABLE_MSG);
    }
}
