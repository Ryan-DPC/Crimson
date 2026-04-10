use tauri::Manager;
use lcu_commands::storage;

#[tauri::command]
pub fn crimson_quit_app(app: tauri::AppHandle) {
  app.exit(0);
}

#[tauri::command]
pub fn crimson_toggle_autostart(app: tauri::AppHandle, enable: bool) -> Result<(), String> {
  let autostart_manager = app.state::<tauri_plugin_autostart::AutoLaunchManager>();
  if enable {
    let _ = autostart_manager.enable();
  } else {
    let _ = autostart_manager.disable();
  }
  // Also update AppData
  let mut data = storage::load_data(&app);
  data.launch_on_startup = enable;
  storage::save_data(&app, &data);
  Ok(())
}
