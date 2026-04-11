use tauri::Manager;
use lcu_commands::{lcu, storage, db};

mod commands;

pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
    .plugin(tauri_plugin_notification::init())
    .plugin(tauri_plugin_shell::init())
    .invoke_handler(tauri::generate_handler![
        lcu::get_lcu_info,
        lcu::lcu_request,
        lcu::debug_lcu,
        storage::get_app_data,
        storage::set_app_data,
        storage::set_auto_accept,
        storage::set_player_note,
        lcu_commands::analyzer::fetch_single_build,
        lcu_commands::analyzer::fetch_dynamic_runes,
        db::get_all_matches,
        lcu::fetch_ddragon_url,
        lcu_commands::updater_cmd::download_and_install_update,
        commands::crimson_quit_app,
        commands::crimson_toggle_autostart
    ])
    .setup(|app| {
      let handle = app.handle().clone();
      let data = storage::load_data(&handle);

      // Restore window geometry
      if let Some(window) = app.get_webview_window("main") {
          if let (Some(x), Some(y)) = (data.window_x, data.window_y) {
              let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
          }
          if let (Some(width), Some(height)) = (data.window_width, data.window_height) {
              let _ = window.set_size(tauri::Size::Physical(tauri::PhysicalSize { width, height }));
          }
      }

      let quit_i = tauri::menu::MenuItem::with_id(app, "quit", "Quitter Crimson", true, None::<&str>)?;
      let show_i = tauri::menu::MenuItem::with_id(app, "show", "Panneau de Controle", true, None::<&str>)?;
      let menu = tauri::menu::Menu::with_items(app, &[&show_i, &quit_i])?;

      let mut tray_builder = tauri::tray::TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(true);

      if let Some(icon) = app.default_window_icon() {
          tray_builder = tray_builder.icon(icon.clone());
      }

      let _tray = tray_builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quit" => {
                crate::commands::crimson_quit_app(app.clone());
            }
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                    let _ = notify_server_resource_mode(false);
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                    let _ = notify_server_resource_mode(false);
                }
            }
        })
        .build(app)?;

      let pool = db::create_pool(app.handle());
      if let Err(e) = db::init_db(&pool) {
          eprintln!("Failed to initialize database: {}", e);
      }
      app.manage(pool);

      let (tx, _) = tokio::sync::broadcast::channel(100);
      app.manage(lcu_commands::events::WsSender(tx));

      let handle = app.handle().clone();
      
      // Force auto_accept to true on startup
      let mut data = storage::load_data(&handle);
      data.auto_accept = true;
      storage::save_data(&handle, &data);

      // Launch Sidecar (only if not already running)
      use tauri_plugin_shell::ShellExt;
      let sidecar_handle = handle.clone();
      let sidecar_child = std::sync::Arc::new(tokio::sync::Mutex::new(None));
      let sidecar_child_clone = sidecar_child.clone();
      let pid = std::process::id();
      
      // Launch Sidecar as a truly independent standalone process (Windows Only logic)
      let sidecar_handle = handle.clone();
      tauri::async_runtime::spawn(async move {
          // 1. Port Check: Don't spawn if already running
          if std::net::TcpStream::connect("127.0.0.1:40509").is_ok() {
              println!("Crimson Server already running. Skipping spawn.");
              return;
          }

          // 2. Resolve Path
          let path_resolver = sidecar_handle.path();
          let sidecar_path = path_resolver.resolve("bin/crimson_server-x86_64-pc-windows-msvc.exe", tauri::path::BaseDirectory::Resource);
          
          if let Ok(p) = sidecar_path {
              if p.exists() {
                  use std::os::windows::process::CommandExt;
                  const DETACHED_PROCESS: u32 = 0x00000008;
                  const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x01000000;
                  const CREATE_NO_WINDOW: u32 = 0x08000000;

                  println!("Spawning Standalone Crimson Server from: {:?}", p);
                  let mut cmd = std::process::Command::new(p);
                  cmd.creation_flags(DETACHED_PROCESS | CREATE_BREAKAWAY_FROM_JOB | CREATE_NO_WINDOW);
                  let _ = cmd.spawn();
              } else {
                  eprintln!("Sidecar binary not found at {:?}", p);
              }
          }
      });
      
      app.manage(SidecarChild(sidecar_child));

      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .on_window_event(|window, event| {
        match event {
            tauri::WindowEvent::Focused(focused) => {
                let _ = notify_server_resource_mode(!focused);
            }
            tauri::WindowEvent::Resized(size) => {
                let handle = window.app_handle();
                let mut data = storage::load_data(handle);
                data.window_width = Some(size.width);
                data.window_height = Some(size.height);
                storage::save_data(handle, &data);
            }
            tauri::WindowEvent::Moved(pos) => {
                let handle = window.app_handle();
                let mut data = storage::load_data(handle);
                data.window_x = Some(pos.x);
                data.window_y = Some(pos.y);
                storage::save_data(handle, &data);
            }
            _ => {}
        }
    })
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|_app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            // Decoupled: Server stays running in background.
            // No kill() call here.
        }
    });
}

fn notify_server_resource_mode(low: bool) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
        use futures_util::SinkExt;
        use serde_json::json;

        let url = "ws://127.0.0.1:40509";
        if let Ok((mut ws_stream, _)) = connect_async(url).await {
            let msg = json!({
                "type": "UPDATE_RESOURCE_MODE",
                "low_resource": low
            }).to_string();
            let _ = ws_stream.send(Message::Text(msg.into())).await;
        }
    });
    Ok(())
}

