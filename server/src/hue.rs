use serde::{Deserialize, Serialize};
use crate::events::WsSender;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Explicit product gate: Hue API is not implemented yet.
pub const UNAVAILABLE_MSG: &str = "Philips Hue is not available yet (coming soon)";

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct HueState {
    pub connected: bool,
    pub bridge_ip: Option<String>,
    pub username: Option<String>,
}

pub struct HueService {
    _state: Arc<RwLock<HueState>>,
    _sender: WsSender,
    /// Kept for WS/entitlement wiring; commands always fail until the API exists.
    pub is_enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl HueService {
    pub fn new(_sender: WsSender) -> Self {
        Self {
            _state: Arc::new(RwLock::new(HueState::default())),
            _sender,
            is_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub async fn handle_command(&self, endpoint: &str, _params: Option<serde_json::Value>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tracing::warn!("[HUE] Rejected '{}': {}", endpoint, UNAVAILABLE_MSG);
        Err(UNAVAILABLE_MSG.into())
    }
}
