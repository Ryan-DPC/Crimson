use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::db::StreamDockDB;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use serde_json::json;
use crate::spotify::SpotifyService;
use crate::discord::DiscordService;
use dashmap::DashMap;
use std::collections::{HashSet, HashMap};
use tokio::sync::{mpsc, broadcast};

lazy_static::lazy_static! {
    pub static ref ACTIVE_BRIDGES: DashMap<String, HardwareBridge> = DashMap::new();
    pub static ref LAST_DIAL_EVENTS: DashMap<String, std::time::Instant> = DashMap::new();
}

#[derive(Clone)]
pub struct HardwareBridge {
    pub tx: mpsc::Sender<String>,
    pub alive: Arc<AtomicBool>,
    pub contexts: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
    pub last_state_cache: Arc<tokio::sync::Mutex<Option<serde_json::Value>>>,
    pub pi_contexts: Arc<tokio::sync::Mutex<HashSet<String>>>,
    pub last_image_per_ctx: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
    pub settings_per_ctx: Arc<tokio::sync::Mutex<HashMap<String, serde_json::Value>>>,
    pub hue: Arc<crate::hue::HueService>,
    pub twitch: Arc<crate::twitch::TwitchService>,
}

pub async fn try_acquire_handover(
    port: u16, 
    uuid: String, 
    register_event: String,
    spotify: Arc<SpotifyService>,
    discord: Arc<DiscordService>,
    hue: Arc<crate::hue::HueService>,
    twitch: Arc<crate::twitch::TwitchService>,
    db: Arc<StreamDockDB>,
    broadcast_tx: broadcast::Sender<String>
) -> bool {
    let mut stale = Vec::new();
    for bridge in ACTIVE_BRIDGES.iter() { if !bridge.alive.load(Ordering::Relaxed) { stale.push(bridge.key().clone()); } }
    for s in stale { ACTIVE_BRIDGES.remove(&s); }
    if let Some(bridge) = ACTIVE_BRIDGES.get(&uuid) {
        if bridge.alive.load(Ordering::Relaxed) {
            tracing::info!("[SD HANDOVER] Bridge for UUID {} is already alive and active. Skipping handover.", uuid);
            return true;
        }
    }
    if let Some((_, old_bridge)) = ACTIVE_BRIDGES.remove(&uuid) {
        old_bridge.alive.store(false, Ordering::Relaxed);
    }
    let (tx_ws, mut rx_ws_master) = mpsc::channel::<String>(500);
    let bridge_alive = Arc::new(AtomicBool::new(true));
    let contexts = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let last_state_cache = Arc::new(tokio::sync::Mutex::new(None));
    let pi_contexts = Arc::new(tokio::sync::Mutex::new(HashSet::new()));
    let last_image_per_ctx = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let settings_per_ctx = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    ACTIVE_BRIDGES.insert(uuid.clone(), HardwareBridge { tx: tx_ws.clone(), alive: bridge_alive.clone(), contexts: contexts.clone(), last_state_cache: last_state_cache.clone(), pi_contexts: pi_contexts.clone(), last_image_per_ctx: last_image_per_ctx.clone(), settings_per_ctx: settings_per_ctx.clone(), hue: hue.clone(), twitch: twitch.clone() });
    let uuid_spawn = uuid.clone();
    let reg_event_spawn = register_event.clone();
    tokio::spawn(async move {
        let ws_url = format!("ws://127.0.0.1:{}", port);
        if let Ok((ws_stream, _)) = connect_async(&ws_url).await {
            let (mut write, mut read) = ws_stream.split();
            let _ = write.send(Message::Text(json!({ "event": reg_event_spawn, "uuid": uuid_spawn }).to_string())).await;
            
            let app_data = crate::storage::load_data_from_path(crate::storage::get_data_path_from_env());
            if let Some(last_rank) = app_data.other.get("last_rank") {
                let _ = write.send(Message::Text(last_rank.to_string())).await;
            }
            if let Some(last_summoner) = app_data.other.get("last_summoner") {
                let _ = write.send(Message::Text(last_summoner.to_string())).await;
            }
            
            let mut rx_broadcast = broadcast_tx.subscribe();
            loop {
                tokio::select! {
                    msg = rx_broadcast.recv() => { if let Ok(text) = msg { if let Err(_) = write.send(Message::Text(text)).await { break; } } }
                    msg = rx_ws_master.recv() => { if let Some(text) = msg { if let Err(_) = write.send(Message::Text(text)).await { break; } } }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(t))) => {
                                // Broadcast raw hardware events to all JS instances (app.js)
                                let _ = broadcast_tx.send(t.clone());
                                
                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&t) {
                                    let event = val["event"].as_str().unwrap_or("");
                                    let context = val["context"].as_str().unwrap_or("");
                                    let action = val["action"].as_str().unwrap_or("");
                                    if event == "willAppear" && !context.is_empty() { contexts.lock().await.insert(context.to_string(), action.to_string()); }
                                    if event == "willDisappear" && !context.is_empty() { contexts.lock().await.remove(context); }
                                    if event == "setSettings" && !context.is_empty() { settings_per_ctx.lock().await.insert(context.to_string(), val["payload"]["settings"].clone()); }
                                    process_streamdeck_event(val, spotify.clone(), discord.clone(), tx_ws.clone(), contexts.clone(), pi_contexts.clone(), last_state_cache.clone(), settings_per_ctx.clone(), hue.clone(), twitch.clone(), db.clone()).await;
                                }
                            }
                            _ => break,
                        }
                    }
                }
            }
        }
        bridge_alive.store(false, Ordering::Relaxed);
        ACTIVE_BRIDGES.remove(&uuid_spawn);
    });
    true
}

pub async fn start_streamdock_client(_p: u16, _u: String, _r: String, _s: Arc<SpotifyService>, _d: Arc<DiscordService>, _h: Arc<crate::hue::HueService>, _t: Arc<crate::twitch::TwitchService>, _db: Arc<StreamDockDB>) {
    // Stub
}

pub async fn start_mirabox_auth_server<T>(_auth: Arc<T>, _tx: Option<mpsc::Sender<String>>, _rx: Option<Arc<tokio::sync::Mutex<HashMap<String, String>>>>) {
    // Stub with generic type
}

pub async fn process_streamdeck_event(value: serde_json::Value, spotify: Arc<SpotifyService>, _d: Arc<DiscordService>, _tx: mpsc::Sender<String>, contexts: Arc<tokio::sync::Mutex<HashMap<String, String>>>, _pi: Arc<tokio::sync::Mutex<HashSet<String>>>, _ls: Arc<tokio::sync::Mutex<Option<serde_json::Value>>>, _spc: Arc<tokio::sync::Mutex<HashMap<String, serde_json::Value>>>, _h: Arc<crate::hue::HueService>, _t: Arc<crate::twitch::TwitchService>, _db: Arc<StreamDockDB>) {
    let event = value["event"].as_str().unwrap_or("");
    let context = value["context"].as_str().unwrap_or("");
    let mut action = value["action"].as_str().unwrap_or("").to_string();
    if action.is_empty() && !context.is_empty() { if let Some(act) = contexts.lock().await.get(context) { action = act.clone(); } }
    
    if action.starts_with("com.laoy.streamdock.spotify") || 
       action.starts_with("com.laoy.streamdock.discord") || 
       action.starts_with("com.laoy.streamdock.hue") || 
       action.starts_with("com.laoy.streamdock.twitch") {
        let app_data = crate::storage::load_data_from_path(crate::storage::get_data_path_from_env());
        if !app_data.is_premium {
            tracing::warn!("[AUTH] Blocked StreamDock action {} for free user", action);
            return;
        }
    }

    let pressed = value["payload"]["pressed"].as_bool().unwrap_or(true);
    if event == "keyDown" || (event == "dialPress" && pressed) {
        match action.as_str() {
            "com.laoy.streamdock.spotify.playpause" |
            "com.laoy.streamdock.spotify.next" |
            "com.laoy.streamdock.spotify.previous" |
            "com.laoy.streamdock.spotify.shuffle" |
            "com.laoy.streamdock.spotify.repeat" |
            "com.laoy.streamdock.spotify.volumecontrol" |
            "com.laoy.streamdock.spotify.previousornext" |
            "com.laoy.streamdock.spotify.likesong" |
            "com.laoy.streamdock.spotify.changedevice" |
            "com.laoy.streamdock.spotify.playuri" |
            "com.laoy.streamdock.spotify.playplaylist" => {
                if !spotify.is_enabled.load(Ordering::Relaxed) {
                    tracing::info!("[SPOTIFY] Key action ignored: service disabled");
                    return;
                }
            }
            _ => {}
        }
        match action.as_str() {
            "com.laoy.streamdock.spotify.playpause" => {
                let s = spotify.clone();
                tokio::spawn(async move { let _ = s.handle_command("playpause", None).await; });
            }
            "com.laoy.streamdock.spotify.next" => {
                let s = spotify.clone();
                tokio::spawn(async move { let _ = s.handle_command("next", None).await; });
            }
            "com.laoy.streamdock.spotify.previous" => {
                let s = spotify.clone();
                tokio::spawn(async move { let _ = s.handle_command("prev", None).await; });
            }
            "com.laoy.streamdock.spotify.shuffle" => {
                let s = spotify.clone();
                tokio::spawn(async move { let _ = s.handle_command("shuffle", None).await; });
            }
            "com.laoy.streamdock.spotify.repeat" => {
                let s = spotify.clone();
                tokio::spawn(async move { let _ = s.handle_command("repeat", None).await; });
            }
            "com.laoy.streamdock.spotify.volumecontrol" => {
                let s = spotify.clone();
                tokio::spawn(async move { let _ = s.handle_command("mute", None).await; });
            }
            "com.laoy.streamdock.spotify.previousornext" => {
                let s = spotify.clone();
                tokio::spawn(async move { let _ = s.handle_command("playpause", None).await; });
            }
            "com.laoy.streamdock.spotify.likesong" => {
                let s = spotify.clone();
                tokio::spawn(async move { let _ = s.handle_command("likesong", None).await; });
            }
            "com.laoy.streamdock.spotify.changedevice" => {
                let s = spotify.clone();
                tokio::spawn(async move { let _ = s.handle_command("changedevice", None).await; });
            }
            "com.laoy.streamdock.spotify.playuri" => {
                let s = spotify.clone();
                let settings = value["payload"]["settings"].clone();
                tokio::spawn(async move { let _ = s.handle_command("playuri", Some(settings)).await; });
            }
            "com.laoy.streamdock.spotify.playplaylist" => {
                let s = spotify.clone();
                let settings = value["payload"]["settings"].clone();
                tokio::spawn(async move { let _ = s.handle_command("play", Some(settings)).await; });
            }
            _ => {}
        }
    }

    if event == "dialRotate" {
        let ticks = value["payload"]["ticks"].as_i64().unwrap_or(0);
        let context_str = context.to_string();
        
        let now = std::time::Instant::now();
        let should_process = {
            if let Some(entry) = LAST_DIAL_EVENTS.get(&context_str) {
                let last_time = entry.value();
                now.checked_duration_since(*last_time).unwrap_or_default() >= std::time::Duration::from_millis(120)
            } else {
                true
            }
        };
        
        if should_process {
            LAST_DIAL_EVENTS.insert(context_str, now);
            match action.as_str() {
                "com.laoy.streamdock.spotify.volumecontrol" => {
                    if spotify.is_enabled.load(Ordering::Relaxed) {
                        let s = spotify.clone();
                        tokio::spawn(async move {
                            let _ = s.handle_command("volumecontrol", Some(json!({ "ticks": ticks }))).await;
                        });
                    }
                }
                "com.laoy.streamdock.spotify.previousornext" => {
                    if spotify.is_enabled.load(Ordering::Relaxed) {
                        let s = spotify.clone();
                        let direction = if ticks > 0 { 1 } else { -1 };
                        tokio::spawn(async move {
                            let _ = s.handle_command("skip", Some(json!({ "ticks": direction }))).await;
                        });
                    }
                }
                _ => {}
            }
        }
    }

    // Explicit command forwarding via JS API
    if value["type"] == "SPOTIFY_COMMAND" {
        if let Some(endpoint) = value["endpoint"].as_str() {
            if spotify.is_enabled.load(Ordering::Relaxed) {
                let s = spotify.clone();
                let endpoint_str = endpoint.to_string();
                let val_clone = value.clone();
                tokio::spawn(async move {
                    let _ = s.handle_command(&endpoint_str, Some(val_clone)).await;
                });
            }
        }
    }
    if value["type"] == "DISCORD_COMMAND" {
        if let Some(endpoint) = value["endpoint"].as_str() {
            if _d.is_enabled.load(Ordering::Relaxed) {
                let d = _d.clone();
                let endpoint_str = endpoint.to_string();
                let val_clone = value.clone();
                tokio::spawn(async move {
                    let _ = d.handle_command(&endpoint_str, Some(val_clone)).await;
                });
            }
        }
    }
    if value["type"] == "HUE_COMMAND" {
        if let Some(endpoint) = value["endpoint"].as_str() {
            if _h.is_enabled.load(Ordering::Relaxed) {
                let h = _h.clone();
                let endpoint_str = endpoint.to_string();
                let val_clone = value.clone();
                tokio::spawn(async move {
                    let _ = h.handle_command(&endpoint_str, Some(val_clone)).await;
                });
            }
        }
    }
    if value["type"] == "TWITCH_COMMAND" {
        if let Some(endpoint) = value["endpoint"].as_str() {
            if _t.is_enabled.load(Ordering::Relaxed) {
                let t = _t.clone();
                let endpoint_str = endpoint.to_string();
                let val_clone = value.clone();
                tokio::spawn(async move {
                    let _ = t.handle_command(&endpoint_str, Some(val_clone)).await;
                });
            }
        }
    }
    if value["type"] == "ADJUST_AUDIO" {
        if _d.is_enabled.load(Ordering::Relaxed) {
            let ticks = value["ticks"].as_i64().unwrap_or(0);
            tokio::spawn(async move {
                let _ = crate::discord::DiscordService::adjust_aux_volume(ticks).await;
            });
        }
    }
    if value["type"] == "TOGGLE_AUDIO_MUTE" {
        if _d.is_enabled.load(Ordering::Relaxed) {
            tokio::spawn(async move {
                let _ = crate::discord::DiscordService::toggle_aux_mute().await;
            });
        }
    }
}
