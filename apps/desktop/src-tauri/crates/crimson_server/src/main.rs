use tokio::sync::broadcast;
use lcu_commands::events::WsSender;
use phantom_server::{ws, lcu_ws, service};
use clap::Parser;
use sysinfo::{System, Pid};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    parent_pid: Option<u32>,

    #[arg(long)]
    data_path: Option<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    println!("Starting Crimson Phantom Server...");

    if let Some(pid) = args.parent_pid {
        println!("Monitoring parent PID: {}", pid);
        tokio::spawn(async move {
            let mut sys = System::new_all();
            let pid = Pid::from(pid as usize);
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                sys.refresh_processes(sysinfo::ProcessesToRefresh::All);
                if sys.process(pid).is_none() {
                    println!("Parent process {} not found. Exiting sidecar.", pid);
                    std::process::exit(0);
                }
            }
        });
    }

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
