use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use serde_json::json;
use lcu_commands::storage;
use lcu_commands::events::WsSender;

pub async fn start_ws_server(sender: WsSender) {
    let addr = "127.0.0.1:40509".parse::<SocketAddr>().expect("Invalid address");
    if let Ok(listener) = TcpListener::bind(&addr).await {
        while let Ok((stream, _)) = listener.accept().await {
            let sender_clone = WsSender(sender.0.clone());
            tokio::spawn(handle_connection(stream, sender_clone));
        }
    }
}

async fn handle_connection(stream: TcpStream, sender: WsSender) {
    if let Ok(mut ws_stream) = accept_async(stream).await {
        println!("Client connected to WebSocket server");
        
        let data = storage::load_data_from_path(storage::get_data_path_from_env());
        let initial_state = json!({
            "type": "AUTO_ACCEPT_STATE",
            "enabled": data.auto_accept
        });
        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(initial_state.to_string().into())).await;

        let mut rx = sender.0.subscribe();

            loop {
                tokio::select! {
                    msg = ws_stream.next() => {
                        match msg {
                            Some(Ok(msg)) => {
                                if let Ok(text) = msg.to_text() {
                                    if text.trim().is_empty() { continue; }
                                    // Parse Command and Dispatch
                                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
                                        if value["type"] == "UPDATE_RESOURCE_MODE" {
                                            if let Some(low) = value["low_resource"].as_bool() {
                                                crate::state::set_low_resource_mode(low);
                                                println!("Resource optimization: {}", if low { "ENABLED" } else { "DISABLED" });
                                            }
                                            continue;
                                        }

                                        if let Ok(command) = serde_json::from_str::<lcu_commands::sd_commands::StreamDeckCommand>(text) {
                                            if let Ok(Some(response_json)) = command.execute_standalone(&sender.0).await {
                                                let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(response_json.to_string().into())).await;
                                            }
                                        }
                                    } else {
                                        println!("Failed to parse command from Client: {}", text);
                                    }
                                }
                            }
                            _ => break,
                        }
                    }
                    Ok(broadcast_msg) = rx.recv() => {
                        let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(broadcast_msg.into())).await;
                    }
                }
            }
        }
    }
