use tauri::{AppHandle, Manager, Emitter};
use reqwest::Client;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use futures_util::StreamExt;

#[tauri::command]
pub async fn download_and_install_update(app: AppHandle, url: String) -> Result<(), String> {
    let client = Client::new();
    
    // Build the request
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    let total_size: u64 = res.content_length().unwrap_or(0);

    // Save to temp folder
    let temp_dir = std::env::temp_dir();
    let exe_path: PathBuf = temp_dir.join("Crimson_Update_Setup.exe");
    let mut file = File::create(&exe_path).map_err(|e| format!("Failed to create temp file: {}", e))?;

    let mut stream = res.bytes_stream();
    let mut downloaded: u64 = 0;

    // Send initial event
    let _ = app.emit("update-progress", serde_json::json!({
        "downloaded": downloaded,
        "total": total_size
    }));

    while let Some(item) = stream.next().await {
        let chunk = item.map_err(|e| format!("Error while downloading: {}", e))?;
        file.write_all(&chunk).map_err(|e| format!("Error while writing to file: {}", e))?;
        downloaded += chunk.len() as u64;

        // Emit progress
        let _ = app.emit("update-progress", serde_json::json!({
            "downloaded": downloaded,
            "total": total_size
        }));
    }

    // Finished downloading. Now we launch it.
    // .exe installer can be run directly. 
    // We launch it independently so it survives when we exit.
    Command::new(&exe_path)
        .arg("/S") // Optional: silent install if NSIS supports it. Without it, the UI appears. User wants simple, let's omit /S to let them click install in case, or add it for transparent. Actually user said "ça s'installe bam c'est fini", so /S is best if it is NSIS.
        // Wait, Tauri v2 uses NSIS by default. The /S switch makes NSIS install silently.
        // I will just open the exe and exit.
        .spawn()
        .map_err(|e| format!("Failed to launch installer: {}", e))?;

    // Exit the app so the installer can overwrite files.
    std::process::exit(0);
}
