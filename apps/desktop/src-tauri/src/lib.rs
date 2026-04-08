#[cfg_attr(mobile, tauri::mobile_entry_point)]

use tauri::Manager;
use lcu_commands::{lcu, storage, analyzer, db};
use phantom_server::{ws, lcu_ws, service};

pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_updater::Builder::new().build())
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
        lcu_commands::updater_cmd::download_and_install_update
    ])
    .setup(|app| {
      let quit_i = tauri::menu::MenuItem::with_id(app, "quit", "Quitter Crimson", true, None::<&str>)?;
      let show_i = tauri::menu::MenuItem::with_id(app, "show", "Panneau de Controle", true, None::<&str>)?;
      let menu = tauri::menu::Menu::with_items(app, &[&show_i, &quit_i])?;

      let _tray = tauri::tray::TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(true)
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quit" => {
                std::process::exit(0);
            }
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
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
                }
            }
        })
        .build(app)?;

      let pool = db::create_pool(app.handle());
      db::init_db(&pool).expect("failed to initialize database");
      app.manage(pool);

      let (tx, _) = tokio::sync::broadcast::channel(100);
      app.manage(lcu_commands::events::WsSender(tx));

      let handle = app.handle().clone();
      
      // Force auto_accept to true on startup
      let mut data = storage::load_data(&handle);
      data.auto_accept = true;
      storage::save_data(&handle, &data);

      service::start_auto_accept_service(handle.clone());

      let ws_handle = handle.clone();
      tauri::async_runtime::spawn(async move {
          ws::start_ws_server(ws_handle).await;
      });

      let lcu_handle = handle.clone();
      tauri::async_runtime::spawn(async move {
          lcu_ws::start_lcu_ws(lcu_handle).await;
      });
      
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .on_window_event(|window, event| match event {
      tauri::WindowEvent::CloseRequested { api, .. } => {
        let _ = window.hide();
        api.prevent_close();
      }
      _ => {}
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
