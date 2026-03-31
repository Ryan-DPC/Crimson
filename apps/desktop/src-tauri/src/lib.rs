#[cfg_attr(mobile, tauri::mobile_entry_point)]

use tauri::Manager;
use lcu_commands::{lcu, storage, analyzer, db};
use phantom_server::{ws, lcu_ws, service};

/// Returns Some(version_string) if an update is available, None if up-to-date.
#[tauri::command]
async fn check_for_update(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(Some(update.version.clone())),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Downloads and installs the pending update, then restarts.
#[tauri::command]
async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    if let Ok(Some(update)) = updater.check().await {
        update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
        app.restart();
    }
    Ok(())
}

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
        analyzer::fetch_dynamic_runes,
        db::get_all_matches,
        lcu::fetch_ddragon_url,
        check_for_update,
        install_update
    ])
    .setup(|app| {
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
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
