use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

#[tauri::command]
pub async fn download_and_install_update(app: AppHandle, _url: String) -> Result<(), String> {
    let update = app.updater().map_err(|e| e.to_string())?
        .check().await.map_err(|e| e.to_string())?
        .ok_or("No update available")?;

    let mut downloaded = 0;

    update.download_and_install(
        move |chunk_len, total_len| {
            downloaded += chunk_len;
            let _ = app.emit("update-progress", serde_json::json!({
                "downloaded": downloaded,
                "total": total_len.unwrap_or(0)
            }));
        },
        || {
            // Optional: on_download_finished
        }
    ).await.map_err(|e| e.to_string())?;

    Ok(())
}
