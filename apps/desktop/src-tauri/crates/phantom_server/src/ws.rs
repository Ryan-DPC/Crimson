use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use tauri::{AppHandle, Manager};
use serde_json::json;
use lcu_commands::storage;
use lcu_commands::events::WsSender;

pub async fn start_ws_server(handle: AppHandle) {
    let addr = "127.0.0.1:40509".parse::<SocketAddr>().expect("Invalid address");
    if let Ok(listener) = TcpListener::bind(&addr).await {
        while let Ok((stream, _)) = listener.accept().await {
            let handle_clone = handle.clone();
            tokio::spawn(handle_connection(stream, handle_clone));
        }
    }
}

    async fn handle_connection(stream: TcpStream, handle: AppHandle) {
        if let Ok(mut ws_stream) = accept_async(stream).await {
            
            let data = storage::load_data(&handle);
            let initial_state = json!({
                "type": "AUTO_ACCEPT_STATE",
                "enabled": data.auto_accept
            });
            let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(initial_state.to_string().into())).await;

            let state = handle.state::<WsSender>();
            let mut rx = state.0.subscribe();

            loop {
                tokio::select! {
                    msg = ws_stream.next() => {
                        match msg {
                            Some(Ok(msg)) => {
                                if let Ok(text) = msg.to_text() {
                                    if text.trim().is_empty() { continue; }
                                    // Parse Command and Dispatch
                                    if let Ok(command) = serde_json::from_str::<lcu_commands::sd_commands::StreamDeckCommand>(text) {
                                        if let Ok(Some(response_json)) = command.execute(&handle, &state.0).await {
                                            let _ = ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(response_json.to_string().into())).await;
                                        }
                                    } else {
                                        println!("Failed to parse command from Stream Deck: {}", text);
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
