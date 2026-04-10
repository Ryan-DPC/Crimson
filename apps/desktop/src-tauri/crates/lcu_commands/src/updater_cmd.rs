use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub async fn download_and_install_update(app: AppHandle, _url: String) -> Result<(), String> {
    let update = app.updater().map_err(|e| e.to_string())?
        .check().await.map_err(|e| e.to_string())?
        .ok_or("No update available")?;

    let mut downloaded = 0;
    let total_size = update.content_length;

    update.download_and_install(
        |chunk_len, _total_len| {
            downloaded += chunk_len;
            let _ = app.emit("update-progress", serde_json::json!({
                "downloaded": downloaded,
                "total": total_size
            }));
        },
        || {
            // Optional: on_download_finished
        }
    ).await.map_err(|e| e.to_string())?;

    Ok(())
}
