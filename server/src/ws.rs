use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{StreamExt, SinkExt};
use serde_json::json;
use std::sync::Arc;
use crate::storage;
use crate::events::WsSender;
use crate::sd_commands::StreamDeckCommand;
use crate::spotify::SpotifyService;
use crate::discord::DiscordService;
use crate::hue::HueService;
use crate::twitch::TwitchService;

pub async fn start_ws_server(
    sender: WsSender, 
    spotify_service: Arc<SpotifyService>, 
    discord_service: Arc<DiscordService>, 
    hue_service: Arc<HueService>,
    twitch_service: Arc<TwitchService>,
    db: Arc<crate::db::StreamDockDB>,
    is_lol_enabled: Arc<std::sync::atomic::AtomicBool>
) {
    let is_auto_accept_enabled = Arc::new(std::sync::atomic::AtomicBool::new(true));
    start_ws_server_modular(40510, sender, Some(spotify_service), Some(discord_service), Some(hue_service), Some(twitch_service), db, is_lol_enabled, is_auto_accept_enabled).await;
}

pub async fn start_ws_server_modular(
    port: u16,
    sender: WsSender, 
    spotify_service: Option<Arc<SpotifyService>>, 
    discord_service: Option<Arc<DiscordService>>, 
    hue_service: Option<Arc<HueService>>,
    twitch_service: Option<Arc<TwitchService>>,
    db: Arc<crate::db::StreamDockDB>,
    is_lol_enabled: Arc<std::sync::atomic::AtomicBool>,
    is_auto_accept_enabled: Arc<std::sync::atomic::AtomicBool>
) {
    let addr_str = format!("127.0.0.1:{}", port);
    let addr = addr_str.parse::<SocketAddr>().expect("Invalid address");
    match TcpListener::bind(&addr).await {
        Ok(listener) => {
            while let Ok((stream, _)) = listener.accept().await {
                let sender_clone = WsSender(sender.0.clone());
                let spotify_clone = spotify_service.clone();
                let discord_clone = discord_service.clone();
                let hue_clone = hue_service.clone();
                let twitch_clone = twitch_service.clone();
                let db_clone = db.clone();
                let is_lol_enabled_clone = is_lol_enabled.clone();
                let is_auto_accept_enabled_clone = is_auto_accept_enabled.clone();
                tokio::spawn(async move {
                    // Peek for HTTP GET (Spotify Callback)
                    let mut buffer = [0; 1024];
                    if stream.peek(&mut buffer).await.is_ok() {
                        let request = String::from_utf8_lossy(&buffer);
                        if request.starts_with("GET /callback") {
                            if let Some(code_start) = request.find("code=") {
                                let code = request[code_start + 5..].split_whitespace().next().unwrap_or("");
                                let code = code.split('&').next().unwrap_or(code);
                                let _ = sender_clone.0.send(json!({ "type": "SPOTIFY_CALLBACK_CODE", "code": code }).to_string());
                                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                                let mut stream = stream;
                                let mut drop_buf = [0; 4096];
                                let _ = stream.read(&mut drop_buf).await; // Consume the incoming HTTP request headers to prevent TCP RST on close
                                let body = "<html><body style='background:#111;color:white;font-family:sans-serif;display:flex;justify-content:center;align-items:center;height:100vh;'><div><h1>Spotify Connected!</h1><p>You can close this window now.</p></div></body></html>";
                                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
                                let _ = stream.write_all(response.as_bytes()).await;
                                let _ = stream.shutdown().await;
                                return;
                            }
                        }
                    }
                    handle_connection(stream, sender_clone, spotify_clone, discord_clone, hue_clone, twitch_clone, db_clone, is_lol_enabled_clone, is_auto_accept_enabled_clone).await;
                });
            }
            tracing::error!("WebSocket listener accept loop exited!");
        },
        Err(e) => tracing::error!("Failed to bind to {}: {}", addr, e),
    }
}

/// Vrai si l'origine designe la machine locale. Couvre la webview Tauri, qui
/// se presente sous http://tauri.localhost sur Windows.
fn is_local_origin(origin: &str) -> bool {
    let host = origin
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(origin);
    let host = host.split('/').next().unwrap_or("");
    // Retire le port sans casser une adresse IPv6 entre crochets.
    let host = match host.rfind(':') {
        Some(i) if !host[i..].contains(']') => &host[..i],
        _ => host,
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();

    host == "localhost"
        || host == "127.0.0.1"
        || host == "[::1]"
        || host == "::1"
        || host.ends_with(".localhost")
}

#[cfg(test)]
mod origin_tests {
    use super::is_local_origin;

    #[test]
    fn accepte_les_clients_legitimes() {
        // Webview Tauri (Windows), serveur de dev Vite, pages locales.
        assert!(is_local_origin("http://tauri.localhost"));
        assert!(is_local_origin("https://tauri.localhost"));
        assert!(is_local_origin("tauri://localhost"));
        assert!(is_local_origin("http://localhost:5173"));
        assert!(is_local_origin("http://127.0.0.1:40510"));
        assert!(is_local_origin("http://[::1]:8080"));
    }

    #[test]
    fn refuse_les_pages_distantes() {
        assert!(!is_local_origin("https://evil.com"));
        assert!(!is_local_origin("http://example.org:8080"));
    }

    #[test]
    fn resiste_aux_origines_trompeuses() {
        // Sous-domaine qui imite localhost sans en etre un.
        assert!(!is_local_origin("https://localhost.evil.com"));
        // Chemin qui ressemble a un hote local.
        assert!(!is_local_origin("https://evil.com/localhost"));
        // Identifiants dans l'URL, hote reel a droite du @.
        assert!(!is_local_origin("https://evil.com#localhost"));
    }
}

async fn handle_connection(
    stream: TcpStream,
    sender: WsSender,
    spotify: Option<Arc<SpotifyService>>, 
    discord: Option<Arc<DiscordService>>, 
    hue: Option<Arc<HueService>>,
    twitch: Option<Arc<TwitchService>>,
    db: Arc<crate::db::StreamDockDB>,
    is_lol_enabled: Arc<std::sync::atomic::AtomicBool>,
    is_auto_accept_enabled: Arc<std::sync::atomic::AtomicBool>
) {

    // Les navigateurs autorisent les WebSocket vers 127.0.0.1 sans CORS : sans
    // ce controle, n'importe quelle page web visitee peut piloter le serveur.
    // Un navigateur envoie toujours Origin et ne peut pas le falsifier ; les
    // clients natifs (application Tauri, plugins StreamDock) n'en envoient pas.
    let origin_check = |req: &tokio_tungstenite::tungstenite::handshake::server::Request,
                        res: tokio_tungstenite::tungstenite::handshake::server::Response| {
        let forbid = |motif: &str| {
            let mut err = tokio_tungstenite::tungstenite::handshake::server::ErrorResponse::new(
                Some(motif.to_string()),
            );
            *err.status_mut() = tokio_tungstenite::tungstenite::http::StatusCode::FORBIDDEN;
            err
        };

        if let Some(origin) = req.headers().get("origin").and_then(|v| v.to_str().ok()) {
            if !is_local_origin(origin) {
                tracing::warn!("[WS] Connexion refusee, origine externe : {}", origin);
                return Err(forbid("Origin not allowed"));
            }
        }

        // Jeton passe en parametre d'URL : l'API WebSocket des navigateurs ne
        // permet pas d'en-tetes personnalises, et les plugins StreamDock sont
        // dans ce cas.
        let token = req
            .uri()
            .query()
            .and_then(|q| {
                q.split('&').find_map(|pair| {
                    let (k, v) = pair.split_once('=')?;
                    if k == "token" { Some(v.to_string()) } else { None }
                })
            })
            .unwrap_or_default();

        if !crate::auth::verify(&token) {
            if crate::auth::strict_mode() {
                tracing::warn!("[WS] Connexion refusee, jeton absent ou invalide");
                return Err(forbid("Invalid token"));
            }
            // Phase de transition : on signale sans rompre, le temps que tous
            // les clients soient adaptes. CRIMSON_STRICT_AUTH=1 pour refuser.
            tracing::warn!("[WS] Connexion sans jeton valide acceptee (mode non strict)");
        }

        Ok(res)
    };

    if let Ok(mut ws_stream) = tokio_tungstenite::accept_hdr_async(stream, origin_check).await {
        tracing::info!("[WS] New client connection accepted");
        tracing::info!("");
        
        let (ws_stream_sender, mut ws_stream_receiver) = tokio::sync::mpsc::channel::<String>(100);
        // Load persistent data to push initial state
        let data_path = storage::get_data_path_from_env();
        let data = storage::load_data_from_path(data_path);
        
        // Push initial state immediately
        let initial_state = json!({
            "type": "AUTO_ACCEPT_STATE",
            "enabled": data.auto_accept
        });
        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(initial_state.to_string().into())).await;

        // Push auto_ban state (so StreamDock button shows correct ON/OFF)
        let auto_ban_id = data.other.get("rememberedAutoBan")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(
            json!({ "type": "AUTO_BAN_STATE", "championId": auto_ban_id }).to_string().into()
        )).await;

        // Push auto_pick state
        let auto_pick_id = data.other.get("rememberedAutoPick")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(
            json!({ "type": "AUTO_PICK_STATE", "championId": auto_pick_id }).to_string().into()
        )).await;

        // Push heartbeat immediately
        let heartbeat = json!({
            "type": "HEARTBEAT_STATUS",
            "server": true,
            "lol": is_lol_enabled.load(std::sync::atomic::Ordering::Relaxed) && crate::lcu::is_lcu_connected(),
            "discord": if let Some(d) = &discord { d.is_enabled.load(std::sync::atomic::Ordering::Relaxed) && d.is_connected().await } else { false }
        });
        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(heartbeat.to_string().into())).await;

        // Push current LCU state if connected
        if crate::lcu::is_lcu_connected() {
            if let Ok(phase_str) = crate::lcu::lcu_request("GET".into(), "/lol-gameflow/v1/gameflow-phase".into(), None) {
                if let Ok(phase) = serde_json::from_str::<serde_json::Value>(&phase_str) {
                    if let Some(p) = phase.as_str() {
                        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(json!({ "type": "GAME_PHASE", "phase": p }).to_string().into())).await;
                        if p == "ChampSelect" {
                            if let Ok(cs_str) = crate::lcu::lcu_request("GET".into(), "/lol-champ-select/v1/session".into(), None) {
                                if let Ok(session) = serde_json::from_str::<serde_json::Value>(&cs_str) {
                                    let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(json!({ "type": "CHAMP_SELECT_UPDATE", "data": session.clone() }).to_string().into())).await;
                                    
                                    let my_cell_id = session["localPlayerCellId"].as_i64().unwrap_or(-1);
                                    let mut my_champ_id = 0;
                                    let mut my_champ_name = String::new();
                                    
                                    if let Some(actions) = session["actions"].as_array() {
                                        for group in actions {
                                            if let Some(group_arr) = group.as_array() {
                                                for action in group_arr {
                                                    if action["actorCellId"].as_i64() == Some(my_cell_id) {
                                                        my_champ_id = action["championId"].as_u64().unwrap_or(0) as u32;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    if my_champ_id > 0 {
                                        if let Ok(champ_json) = crate::lcu::lcu_request("GET".into(), format!("/lol-game-data/assets/v1/champions/{}.json", my_champ_id), None) {
                                            if let Ok(champ_data) = serde_json::from_str::<serde_json::Value>(&champ_json) {
                                                my_champ_name = champ_data["name"].as_str().unwrap_or("").to_string();
                                            }
                                        }
                                    }
                                    let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(json!({
                                        "type": "CHAMP_SELECT", 
                                        "championId": my_champ_id,
                                        "championName": my_champ_name
                                    }).to_string().into())).await;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(last_rank) = data.other.get("last_rank") {
            let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(last_rank.to_string().into())).await;
        }

        if let Some(last_summoner) = data.other.get("last_summoner") {
            let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(last_summoner.to_string().into())).await;
        }

        let mut rx = sender.0.subscribe();

        loop {
            tokio::select! {
                // Outgoing messages to this client (direct)
                out_msg = ws_stream_receiver.recv() => {
                    if let Some(text) = out_msg {
                        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(text.into())).await;
                    } else {
                        break;
                    }
                }
                // Broadcasted messages (heartbeats, state updates)
                broadcast_msg = rx.recv() => {
                    if let Ok(text) = broadcast_msg {
                        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(text.into())).await;
                    }
                }
                msg = ws_stream.next() => {
                    match msg {
                        Some(Ok(msg)) => {
                            if let Ok(text) = msg.into_text() {
                                if text.trim().is_empty() { continue; }
                                // Parse Command and Dispatch
                                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
                                    // 1. Log incoming command for diagnostics
                                    let evt_name = value["event"].as_str().unwrap_or("N/A");
                                    let type_name = value["type"].as_str().unwrap_or("N/A");
                                     
                                    // Universal Settings Extraction
                                    if let Some(ctx) = value["context"].as_str() {
                                        if let Some(settings) = value["payload"]["settings"]
                                            .as_object()
                                            .or_else(|| value["payload"].as_object())
                                        {
                                            tracing::info!("[DEBUG SETTINGS] Received for context {}: {:?}", ctx, settings);
                                            
                                            // 1. Persist settings in-memory
                                            for bridge in crate::streamdock::ACTIVE_BRIDGES.iter_mut() {
                                                let mut s = bridge.settings_per_ctx.lock().await;
                                                s.insert(ctx.to_string(), serde_json::Value::Object(settings.clone()));
                                            }

                                            // 2. Persist to DB (v1.2.0)
                                            let db_c = db.clone();
                                            let ctx_c = ctx.to_string();
                                            let settings_c = serde_json::Value::Object(settings.clone());
                                            let action_c = value["action"].as_str().map(|s| s.to_string());
                                            let img_c = settings.get("image").and_then(|v| v.as_str())
                                                .or_else(|| settings.get("playlist_image").and_then(|v| v.as_str()))
                                                .map(|s| s.to_string());
                                            
                                            tokio::spawn(async move {
                                                let _ = db_c.save_button(&ctx_c, action_c.as_deref(), &settings_c, img_c.as_deref());
                                            });

                                            // 3. IMMEDIATE IMAGE PUSH (v1.1.5)
                                            if let Some(img_url) = settings.get("image").and_then(|v| v.as_str())
                                                .or_else(|| settings.get("playlist_image").and_then(|v| v.as_str())) {
                                                
                                                tracing::info!("[SD IMAGE PUSH] Pushing cover to context {}", ctx);
                                                let bridge_tx = ws_stream_sender.clone();
                                                let ctx_str = ctx.to_string();
                                                let img_str = img_url.to_string();
                                                tokio::spawn(async move {
                                                    let _ = bridge_tx.send(json!({
                                                        "event": "setImage",
                                                        "context": ctx_str,
                                                        "payload": { "image": img_str, "target": 0 }
                                                    }).to_string()).await;
                                                });

                                                // ALSO push to all active hardware bridges (Proxy Bridge)
                                                for bridge in crate::streamdock::ACTIVE_BRIDGES.iter() {
                                                    let tx_clone = bridge.tx.clone();
                                                    let ctx_str2 = ctx.to_string();
                                                    let img_str2 = img_url.to_string();
                                                    tokio::spawn(async move {
                                                        let _ = tx_clone.send(json!({
                                                            "event": "setImage",
                                                            "context": ctx_str2,
                                                            "payload": { "image": img_str2, "target": 0 }
                                                        }).to_string()).await;
                                                    });
                                                }
                                            }
                                        }
                                        // Update context mapping if action is present
                                        if let Some(action) = value["action"].as_str() {
                                            for bridge in crate::streamdock::ACTIVE_BRIDGES.iter_mut() {
                                                let mut c = bridge.contexts.lock().await;
                                                c.insert(ctx.to_string(), action.to_string());
                                            }
                                        }
                                    }

                                    tracing::info!("[SD IN] Event: {} Type: {} Payload: {}", evt_name, type_name, text);

                                    if value["type"] == "TOGGLE_PLUGIN" {
                                        if let (Some(plugin), Some(enabled)) = (value["plugin"].as_str(), value["enabled"].as_bool()) {
                                            // Sans ce controle, n'importe quel client WebSocket
                                            // local activait un service premium d'un seul message.
                                            if enabled
                                                && ["spotify", "discord", "hue", "twitch"].contains(&plugin)
                                                && !crate::entitlement::is_premium().await
                                            {
                                                tracing::warn!("[AUTH] Blocked TOGGLE_PLUGIN {} for free user", plugin);
                                                let _ = sender.0.send(json!({ "type": "AUTH_ERROR", "message": "Abonnement Premium Requis" }).to_string());
                                                continue;
                                            }
                                            // Toggle in-memory states
                                            match plugin {
                                                "spotify" => {
                                                    if let Some(s) = &spotify {
                                                        s.is_enabled.store(enabled, std::sync::atomic::Ordering::Relaxed);
                                                        if !enabled {
                                                            let empty_state = crate::spotify::SpotifyState::default();
                                                            let _ = sender.0.send(json!({ "type": "SPOTIFY_STATE", "data": empty_state }).to_string());
                                                        } else {
                                                            s.notify.notify_one();
                                                        }
                                                    }
                                                }
                                                "discord" => {
                                                    if let Some(d) = &discord {
                                                        d.is_enabled.store(enabled, std::sync::atomic::Ordering::Relaxed);
                                                        if !enabled {
                                                            let empty_state = crate::discord::DiscordState::default();
                                                            let _ = sender.0.send(json!({ "type": "DISCORD_STATE", "data": empty_state }).to_string());
                                                        }
                                                    }
                                                }
                                                "twitch" => { if let Some(t) = &twitch { t.is_enabled.store(enabled, std::sync::atomic::Ordering::Relaxed); } }
                                                "hue" => { if let Some(h) = &hue { h.is_enabled.store(enabled, std::sync::atomic::Ordering::Relaxed); } }
                                                "leagueOfLegends" => {
                                                    is_lol_enabled.store(enabled, std::sync::atomic::Ordering::Relaxed);
                                                    if !enabled {
                                                        let _ = sender.0.send(json!({ "type": "GAME_PHASE", "phase": "None" }).to_string());
                                                        let _ = sender.0.send(json!({ "type": "CHAMP_SELECT", "championId": 0, "championName": "" }).to_string());
                                                        let _ = sender.0.send(json!({ "type": "SUMMONER_INFO", "gameName": "", "profileIconId": 0 }).to_string());
                                                    }
                                                }
                                                _ => {}
                                            }
                                            tracing::info!("Toggled plugin {} to {} in memory", plugin, enabled);

                                            // Immediate heartbeat broadcast for snappy visual updates
                                            let hb = json!({
                                                "type": "HEARTBEAT_STATUS",
                                                "server": true,
                                                "lol": is_lol_enabled.load(std::sync::atomic::Ordering::Relaxed) && crate::lcu::is_lcu_connected(),
                                                "discord": if let Some(d) = &discord { d.is_enabled.load(std::sync::atomic::Ordering::Relaxed) && d.is_connected().await } else { false }
                                            }).to_string();
                                            let _ = sender.0.send(hb);
                                        }
                                        continue;
                                    }

                                    if value["type"] == "GET_VERSION" {
                                        let _ = sender.0.send(json!({
                                            "type": "SERVER_VERSION",
                                            "version": env!("CARGO_PKG_VERSION")
                                        }).to_string());
                                        continue;
                                    }

                                    if value["type"] == "UPDATE_RESOURCE_MODE" {
                                        if let Some(low) = value["low_resource"].as_bool() {
                                            crate::state::set_low_resource_mode(low);
                                            tracing::info!("");
                                        }
                                        continue;
                                    }

                                    // Session Supabase transmise par l'application. Gardee en
                                    // memoire uniquement : elle ne doit jamais toucher le disque.
                                    if value["type"] == "AUTH_SESSION" {
                                        // Seul le jeton vient du client. L'URL de verification est
                                        // figee dans le binaire : la laisser au client reviendrait a
                                        // le laisser choisir qui atteste de ses propres droits.
                                        match value["access_token"].as_str() {
                                            Some(token) if !token.is_empty() => {
                                                crate::entitlement::set_session(token.to_string());
                                            }
                                            _ => crate::entitlement::clear_session(),
                                        }
                                        continue;
                                    }

                                    if value["type"] == "SPOTIFY_AUTH" {
                                        if let (Some(access), Some(refresh)) = (value["access_token"].as_str(), value["refresh_token"].as_str()) {
                                            let expires_in = value["expires_in"].as_u64().unwrap_or(3600);
                                            if let Some(s) = &spotify {
                                                // Les identifiants viennent de l'application Spotify de
                                                // l'utilisateur. Ils doivent etre enregistres avant les
                                                // jetons : sans eux, ensure_valid_token() ne peut pas
                                                // rafraichir l'acces une fois l'heure ecoulee.
                                                if let (Some(id), Some(secret)) = (value["client_id"].as_str(), value["client_secret"].as_str()) {
                                                    if !id.is_empty() && !secret.is_empty() {
                                                        s.set_client_credentials(id.to_string(), secret.to_string()).await;
                                                    }
                                                }
                                                s.update_tokens(access.to_string(), refresh.to_string(), expires_in).await;
                                                tracing::info!("");
                                            }
                                        }
                                        continue;
                                    }

                                    if value["type"] == "DEBUG_LOG" {
                                        tracing::info!("");
                                        continue;
                                    }

                                    let cmd_type = value["type"].as_str().unwrap_or("");
                                    if ["SPOTIFY_COMMAND", "DISCORD_COMMAND", "HUE_COMMAND", "TWITCH_COMMAND"].contains(&cmd_type) {
                                        // Le verdict vient de Supabase, plus de data.json : ce
                                        // fichier est ecrit par le client et modifiable a la main.
                                        if !crate::entitlement::is_premium().await {
                                            tracing::warn!("[AUTH] Blocked {} for free user", cmd_type);
                                            let _ = sender.0.send(json!({ "type": "AUTH_ERROR", "message": "Abonnement Premium Requis" }).to_string());
                                            continue;
                                        }
                                    }

                                    if value["type"] == "SPOTIFY_COMMAND" {
                                        if let Some(s) = &spotify {
                                            if !s.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                                tracing::info!("[SPOTIFY] Spotify command ignored: service disabled");
                                                continue;
                                            }
                                            if let Some(endpoint) = value["endpoint"].as_str() {
                                                let command_val = value.clone();
                                                let s_clone = s.clone();
                                                let endpoint_str = endpoint.to_string();
                                                tokio::spawn(async move {
                                                    // Fallback to persisted settings if payload is empty (v1.1.5)
                                                    if let Some(ctx) = command_val["context"].as_str() {
                                                        let mut payload = command_val["payload"].clone();
                                                        if payload["playlist"].is_null() && payload["uri"].is_null() {
                                                            for bridge in crate::streamdock::ACTIVE_BRIDGES.iter() {
                                                                let settings_lock = bridge.settings_per_ctx.lock().await;
                                                                if let Some(s_settings) = settings_lock.get(ctx) {
                                                                    tracing::info!("[SD CMD] Falling back to persisted settings for context {}", ctx);
                                                                    payload = s_settings.clone();
                                                                }
                                                            }
                                                        }
                                                        
                                                        if endpoint_str == "play" {
                                                            if let Some(uri) = payload["playlist"].as_str().or(payload["uri"].as_str()) {
                                                                let mut p = payload.clone();
                                                                p["context_uri"] = json!(uri);
                                                                let _ = s_clone.handle_command(&endpoint_str, Some(p)).await;
                                                            } else {
                                                                tracing::info!("[SD ERROR] Play command missing playlist/uri");
                                                            }
                                                        } else {
                                                            let _ = s_clone.handle_command(&endpoint_str, Some(payload)).await;
                                                        }
                                                    } else {
                                                        let _ = s_clone.handle_command(&endpoint_str, Some(command_val.clone())).await;
                                                    }
                                                });
                                            }
                                        }
                                        continue;
                                    }
 
                                    if value["type"] == "DISCORD_COMMAND" {
                                        if let Some(d) = &discord {
                                            if !d.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                                tracing::info!("[DISCORD] Discord command ignored: service disabled");
                                                continue;
                                            }
                                            if let Some(endpoint) = value["endpoint"].as_str() {
                                                let d_clone = d.clone();
                                                let endpoint_str = endpoint.to_string();
                                                let val_clone = value.clone();
                                                tokio::spawn(async move {
                                                    let _ = d_clone.handle_command(&endpoint_str, Some(val_clone)).await;
                                                });
                                            }
                                        }
                                        continue;
                                    }
 
                                    if value["type"] == "HUE_COMMAND" {
                                        if let Some(h) = &hue {
                                            if !h.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                                tracing::info!("[HUE] Hue command ignored: service disabled");
                                                continue;
                                            }
                                            if let Some(endpoint) = value["endpoint"].as_str() {
                                                let h_clone = h.clone();
                                                let endpoint_str = endpoint.to_string();
                                                let val_clone = value.clone();
                                                tokio::spawn(async move {
                                                    let _ = h_clone.handle_command(&endpoint_str, Some(val_clone)).await;
                                                });
                                            }
                                        }
                                        continue;
                                    }
 
                                    if value["type"] == "TWITCH_COMMAND" {
                                        if let Some(t) = &twitch {
                                            if !t.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                                tracing::info!("[TWITCH] Twitch command ignored: service disabled");
                                                continue;
                                            }
                                            if let Some(endpoint) = value["endpoint"].as_str() {
                                                let t_clone = t.clone();
                                                let endpoint_str = endpoint.to_string();
                                                let val_clone = value.clone();
                                                tokio::spawn(async move {
                                                    let _ = t_clone.handle_command(&endpoint_str, Some(val_clone)).await;
                                                });
                                            }
                                        }
                                        continue;
                                    }

                                    if value["type"] == "REGISTER_STREAMDOCK_BRIDGE" {
                                        let port_id = value["port"].as_u64()
                                            .or_else(|| value["port"].as_str().and_then(|s| s.parse::<u64>().ok()));

                                        if let (Some(_p), Some(u)) = (port_id, value["uuid"].as_str()) {
                                            tracing::info!("");
                                            let s_clone = spotify.clone();
                                            let d_clone = discord.clone();
                                            let h_clone = hue.clone();
                                            let t_clone = twitch.clone();
                                            let uuid_str = u.to_string();
                                            let sender_clone = ws_stream_sender.clone();
                                            let contexts = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
                                            let pi_contexts = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashSet::new()));
                                            let last_state_cache = std::sync::Arc::new(tokio::sync::Mutex::new(None));
                                            let last_image_per_ctx = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
                                            let settings_per_ctx = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
                                            
                                            let bridge_data = crate::streamdock::HardwareBridge {
                                                tx: sender_clone.clone(),
                                                alive: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
                                                contexts: contexts.clone(),
                                                last_state_cache: last_state_cache.clone(),
                                                pi_contexts: pi_contexts.clone(),
                                                last_image_per_ctx: last_image_per_ctx.clone(),
                                                settings_per_ctx: settings_per_ctx.clone(),
                                                hue: h_clone.clone().unwrap_or_else(|| Arc::new(HueService::new(sender.clone()))),
                                                twitch: t_clone.clone().unwrap_or_else(|| Arc::new(TwitchService::new(sender.clone()))),
                                            };

                                            // RESTORE FROM DB (v1.2.0)
                                            if let Ok(saved_buttons) = db.get_all_buttons() {
                                                let mut settings_lock = settings_per_ctx.lock().await;
                                                let mut contexts_lock = contexts.lock().await;
                                                for (ctx, action, mut settings, image) in saved_buttons {
                                                    // v1.2.3: Merge image into settings for better sync persistence
                                                    if let Some(img) = &image {
                                                        if let Some(obj) = settings.as_object_mut() {
                                                            if !obj.contains_key("image") { obj.insert("image".to_string(), json!(img)); }
                                                            if !obj.contains_key("playlist_image") { obj.insert("playlist_image".to_string(), json!(img)); }
                                                        }
                                                    }

                                                    settings_lock.insert(ctx.clone(), settings.clone());
                                                    contexts_lock.insert(ctx.clone(), action);
                                                    
                                                    // Push image if saved
                                                    if let Some(img) = image {
                                                        let _ = sender_clone.send(json!({
                                                            "event": "setImage",
                                                            "context": ctx,
                                                            "payload": { "image": img, "target": 0 }
                                                        }).to_string()).await;
                                                    }
                                                }
                                            }

                                            crate::streamdock::ACTIVE_BRIDGES.insert(uuid_str.clone(), bridge_data);
                                            
                                            // IMMEDIATE SYNC: Push credentials and playlists as soon as the bridge connects
                                            let sync_val = json!({ "event": "registerPropertyInspector", "uuid": &uuid_str });
                                            let s_unwrapped = s_clone.clone().unwrap_or_else(|| Arc::new(SpotifyService::new(sender.clone())));
                                            let d_unwrapped = d_clone.clone().unwrap_or_else(|| Arc::new(DiscordService::new(sender.clone())));
                                            let h_unwrapped = h_clone.clone().unwrap_or_else(|| Arc::new(HueService::new(sender.clone())));
                                            let t_unwrapped = t_clone.clone().unwrap_or_else(|| Arc::new(TwitchService::new(sender.clone())));
                                            crate::streamdock::process_streamdeck_event(sync_val, s_unwrapped.clone(), d_unwrapped.clone(), sender_clone.clone(), contexts.clone(), pi_contexts.clone(), last_state_cache.clone(), settings_per_ctx.clone(), h_unwrapped.clone(), t_unwrapped.clone(), db.clone()).await;

                                            loop {
                                                tokio::select! {
                                                    msg_res = ws_stream.next() => {
                                                        match msg_res {
                                                            Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                                                                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                                                                    crate::streamdock::process_streamdeck_event(val, s_unwrapped.clone(), d_unwrapped.clone(), sender_clone.clone(), contexts.clone(), pi_contexts.clone(), last_state_cache.clone(), settings_per_ctx.clone(), h_unwrapped.clone(), t_unwrapped.clone(), db.clone()).await;
                                                                }
                                                            }
                                                            _ => break,
                                                        }
                                                    }
                                                    Ok(broadcast_msg) = rx.recv() => {
                                                        if let Err(_) = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(broadcast_msg.into())).await {
                                                            break;
                                                        }
                                                    }
                                                }
                                            }
                                            crate::streamdock::ACTIVE_BRIDGES.remove(&uuid_str);
                                            tracing::info!("");
                                            break;
                                        }
                                    }

                                     if value["event"] == "registerPropertyInspector" {
                                        let s_clone = spotify.clone();
                                        let ws_sender = ws_stream_sender.clone();
                                        tokio::spawn(async move {
                                            if let Some(s) = s_clone {
                                                let playlists = s.get_user_playlists().await.unwrap_or_default();
                                                let devices = s.get_user_devices().await.unwrap_or_default();
                                                let data = json!({
                                                    "event": "sendToPropertyInspector",
                                                    "payload": {
                                                        "playlists": playlists,
                                                        "devices": devices,
                                                        "authorized": true
                                                    }
                                                });
                                                let _ = ws_sender.send(data.to_string()).await;
                                            }
                                        });
                                        continue;
                                    }

                                    if value["type"] == "REGISTER_STREAMDOCK" {
                                        let port_opt = value["port"].as_u64()
                                            .or_else(|| value["port"].as_str().and_then(|s| s.parse::<u64>().ok()));

                                        if let (Some(p), Some(u), Some(r)) = (port_opt, value["uuid"].as_str(), value["register_event"].as_str()) {
                                            tracing::info!("[SD HANDOVER] Port: {} UUID: {} Event: {}", p, u, r);
                                            let uuid_str = u.to_string();
                                            let reg_str = r.to_string();
                                            
                                            let s_clone = spotify.clone().unwrap_or_else(|| Arc::new(SpotifyService::new(sender.clone())));
                                            let d_clone = discord.clone().unwrap_or_else(|| Arc::new(DiscordService::new(sender.clone())));
                                            let h_clone = hue.clone().unwrap_or_else(|| Arc::new(crate::hue::HueService::new(sender.clone())));
                                            let t_clone = twitch.clone().unwrap_or_else(|| Arc::new(crate::twitch::TwitchService::new(sender.clone())));

                                            if !crate::streamdock::try_acquire_handover(
                                                p as u16, 
                                                uuid_str.clone(), 
                                                reg_str,
                                                s_clone,
                                                d_clone,
                                                h_clone,
                                                t_clone,
                                                db.clone(),
                                                sender.0.clone()
                                            ).await {
                                                tracing::info!("[SD HANDOVER] Duplicate or failed for {}", uuid_str);
                                            }

                                            let response_json = json!({
                                                "type": "COMMAND_ACK",
                                                "action": "REGISTER_STREAMDOCK",
                                                "status": "ok"
                                            });
                                            let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(response_json.to_string().into())).await;
                                            continue;
                                        }
                                    }

                                    if value["type"] == "FORWARD_TO_STREAMDOCK" {
                                         if let Some(payload) = value["payload"].as_object() {
                                             let payload_str = serde_json::Value::Object(payload.clone()).to_string();
                                             for bridge in crate::streamdock::ACTIVE_BRIDGES.iter() {
                                                 let _ = bridge.tx.send(payload_str.clone()).await;
                                             }
                                         }
                                         continue;
                                    }

                                    // Broadcast direct action requests from StreamDock so the Tauri UI can handle them
                                    // 4. Handle generic StreamDeck events (willAppear, etc.)
                                    if let Some(event) = value["event"].as_str() {
                                        if event == "willAppear" {
                                            if let (Some(ctx), Some(action)) = (value["context"].as_str(), value["action"].as_str()) {
                                                // Update context mapping
                                                for bridge in crate::streamdock::ACTIVE_BRIDGES.iter_mut() {
                                                    let mut c = bridge.contexts.lock().await;
                                                    c.insert(ctx.to_string(), action.to_string());
                                                    
                                                    // Update settings
                                                    if let Some(settings) = value["payload"]["settings"].as_object() {
                                                        let mut s = bridge.settings_per_ctx.lock().await;
                                                        s.insert(ctx.to_string(), serde_json::Value::Object(settings.clone()));
                                                    }
                                                }

                                                // Restore image from DB immediately on willAppear to solve starting cover art race
                                                if let Ok(Some(img)) = db.get_button_image(ctx) {
                                                    let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(json!({
                                                        "event": "setImage",
                                                        "context": ctx,
                                                        "payload": {
                                                            "image": img,
                                                            "target": 0
                                                        }
                                                    }).to_string().into())).await;
                                                }
                                            }
                                        }
                                        if event == "willDisappear" {
                                            if let Some(ctx) = value["context"].as_str() {
                                                for bridge in crate::streamdock::ACTIVE_BRIDGES.iter_mut() {
                                                    let mut c = bridge.contexts.lock().await;
                                                    c.remove(ctx);
                                                }
                                            }
                                        }
                                    }

                                     if value["type"] == "TOGGLE_AUTO_ACCEPT" {
                                         if !is_lol_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                             tracing::info!("LCU TOGGLE_AUTO_ACCEPT ignored: service disabled");
                                             continue;
                                         }
                                         // Toggle in-memory state and save to disk
                                         let new_val = !is_auto_accept_enabled.load(std::sync::atomic::Ordering::Relaxed);
                                         is_auto_accept_enabled.store(new_val, std::sync::atomic::Ordering::Relaxed);

                                         let data_path = storage::get_data_path_from_env();
                                         let mut data = storage::load_data_from_path(data_path.clone());
                                         data.auto_accept = new_val;
                                         storage::save_data_to_path(data_path, &data);

                                         let state_msg = json!({
                                             "type": "AUTO_ACCEPT_STATE",
                                             "enabled": new_val
                                         }).to_string();
                                         let _ = sender.0.send(state_msg);
                                         let response_json = json!({
                                             "type": "COMMAND_ACK",
                                             "action": "TOGGLE_AUTO_ACCEPT",
                                             "status": "ok"
                                         });
                                         let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(response_json.to_string().into())).await;
                                         continue;
                                     }


                                     if value["type"] == "TOGGLE_AUTO_BAN" || value["type"] == "TOGGLE_AUTO_PICK" || value["type"] == "INJECT_BUILD" || value["type"] == "DODGE_GAME" || value["type"] == "RUNE_BUILDS_READY" {
                                         if !is_lol_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                             tracing::info!("LCU command ignored: service disabled");
                                             continue;
                                         }
                                         let _ = sender.0.send(text.clone());
                                         
                                         // EXECUTE IN BACKEND
                                         if value["type"] == "TOGGLE_AUTO_BAN" {
                                             let data_path = storage::get_data_path_from_env();
                                             let mut data = storage::load_data_from_path(data_path.clone());
                                             
                                             let auto_ban_val = if let Some(auto_ban) = data.other.get("autoBan") {
                                                 if auto_ban.is_null() {
                                                     data.other.get("rememberedAutoBan").unwrap_or(&json!(1)).clone()
                                                 } else {
                                                     data.other.insert("rememberedAutoBan".to_string(), auto_ban.clone());
                                                     json!(null)
                                                 }
                                             } else {
                                                 data.other.get("rememberedAutoBan").unwrap_or(&json!(1)).clone()
                                             };
                                             data.other.insert("autoBan".to_string(), auto_ban_val.clone());
                                             let _ = sender.0.send(json!({ "type": "AUTO_BAN_STATE", "championId": auto_ban_val }).to_string());
                                             
                                             storage::save_data_to_path(data_path, &data);
                                         } else if value["type"] == "TOGGLE_AUTO_PICK" {
                                             let data_path = storage::get_data_path_from_env();
                                             let mut data = storage::load_data_from_path(data_path.clone());
                                             
                                             let auto_pick_val = if let Some(auto_pick) = data.other.get("autoPick") {
                                                 if auto_pick.is_null() {
                                                     data.other.get("rememberedAutoPick").unwrap_or(&json!(1)).clone()
                                                 } else {
                                                     data.other.insert("rememberedAutoPick".to_string(), auto_pick.clone());
                                                     json!(null)
                                                 }
                                             } else {
                                                 data.other.get("rememberedAutoPick").unwrap_or(&json!(1)).clone()
                                             };
                                             data.other.insert("autoPick".to_string(), auto_pick_val.clone());
                                             let _ = sender.0.send(json!({ "type": "AUTO_PICK_STATE", "championId": auto_pick_val }).to_string());
                                             
                                             storage::save_data_to_path(data_path, &data);
                                         } else if value["type"] == "DODGE_GAME" {
                                             // Handle dodge directly via LCU
                                             let _ = crate::lcu::lcu_request("POST".into(), "/lol-login/v1/session/invoke?destination=lcdsServiceProxy&method=call&args=[\"\",\"teambuilder-draft\",\"quitV2\",\"\"]".into(), Some("".into()));
                                             let _ = crate::lcu::lcu_request("POST".into(), "/lol-lobby/v1/lobby/custom/cancel-champ-select".into(), Some("".into()));
                                         }

                                         // Also directly acknowledge it
                                         let response_json = json!({
                                             "type": "COMMAND_ACK",
                                             "action": value["type"],
                                             "status": "ok"
                                         });
                                         let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(response_json.to_string().into())).await;
                                         continue;
                                     }

                                    if let Ok(command) = serde_json::from_str::<StreamDeckCommand>(&text) {
                                        // Simple command acknowledgement
                                        let response_json = json!({
                                            "type": "COMMAND_ACK",
                                            "action": command.action,
                                            "status": "ok"
                                        });
                                        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(response_json.to_string().into())).await;
                                    }
                                } else {
                                    // Fallback: Log unknown JSON structure
                                    let log_path = crate::storage::get_data_dir().join("streamdock.log");
                                    if let Ok(mut log_file) = std::fs::OpenOptions::new().create(true).append(true).open(log_path) {
                                        use std::io::Write;
                                        let _ = writeln!(log_file, "[PRIMARY WS UNKNOWN] Payload: {}", text);
                                    }
                                    tracing::info!("");
                                }
                            }
                        }
                        _ => break,
                    }
                }
            }
        }
    }
}
