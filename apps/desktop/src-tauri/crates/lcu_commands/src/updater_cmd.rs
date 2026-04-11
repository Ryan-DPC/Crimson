use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub async fn download_and_install_update(app: AppHandle, _url: String) -> Result<(), String> {
    let update = app.updater().map_err(|e| e.to_string())?
        .check().await.map_err(|e| e.to_string())?
        .ok_or("No update available")?;

    let mut downloaded = 0;

    // Download the update
    let app_c = app.clone();
    let bytes = update.download(
        move |chunk_len, total_len| {
            downloaded += chunk_len;
            let _ = app_c.emit("update-progress", serde_json::json!({
                "downloaded": downloaded,
                "total": total_len.unwrap_or(0)
            }));
        },
        || {
            // on_download_finished
        }
    ).await.map_err(|e| e.to_string())?;

    // Just before installing, we should ensure we are ready to quit.
    // In Tauri 2.0, install() will launch the installer and then the app should quit.
    // CRITICAL: We MUST kill the sidecar (crimson_server) because it locks its own binary,
    // which prevents the installer from overwriting the files.
    let sidecar_state = app.state::<crate::SidecarChild>();
    let child_mutex = sidecar_state.0.clone();
    let mut lock = child_mutex.lock().await;
    if let Some(mut child) = lock.take() {
        let _ = child.kill();
    }

    update.install(bytes).map_err(|e| e.to_string())?;

    Ok(())
}
