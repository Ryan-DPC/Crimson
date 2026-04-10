use tokio::sync::broadcast;
use lcu_commands::events::WsSender;
use phantom_server::{ws, lcu_ws, service};

#[tokio::main]
async fn main() {
    println!("Starting Crimson Phantom Server...");

    // Setup broadcast channel for internal communications
    let (tx, _) = broadcast::channel(100);
    let sender = WsSender(tx);

    // Start Auto-Accept and State Broadcasting Service
    service::start_auto_accept_service(sender.clone());

    // Start LCU WebSocket Listener (Sidecar -> LCU)
    let lcu_sender = sender.clone();
    tokio::spawn(async move {
        lcu_ws::start_lcu_ws(lcu_sender).await;
    });

    // Start External WebSocket Server (Sidecar -> UI/StreamDeck)
    println!("WebSocket server listening on 127.0.0.1:40509");
    ws::start_ws_server(sender).await;
}
