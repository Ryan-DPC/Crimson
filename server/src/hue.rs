use serde::{Deserialize, Serialize};
use crate::events::WsSender;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct HueState {
    pub connected: bool,
    pub bridge_ip: Option<String>,
    pub username: Option<String>,
}

pub struct HueService {
    _state: Arc<RwLock<HueState>>,
    _sender: WsSender,
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

    pub async fn handle_command(&self, endpoint: &str, params: Option<serde_json::Value>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            tracing::info!("[HUE] Hue command ignored: service disabled");
            return Ok(());
        }

        match endpoint {
            "toggle" => {
                println!("Hue: Toggling lights...");
                // TODO: Implement actual Hue API call using reqwest/ureq
            }
            "scene" => {
                if let Some(p) = params {
                    let scene_id = p["payload"]["settings"]["sceneId"].as_str().or(p["sceneId"].as_str());
                    if let Some(sid) = scene_id {
                        println!("Hue: Setting scene {}...", sid);
                    }
                }
            }
            _ => return Err("Unknown Hue command".into()),
        }
        Ok(())
    }
}
