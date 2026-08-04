//! Champ-select / ready-check automation lives in the crimson-server sidecar
//! (`server/src/automation.rs`). This module is retained only for API
//! compatibility and must not run a second LCU loop.

use tauri::AppHandle;
use serde_json::Value;

#[allow(unused_variables)]
pub async fn handle_champ_select(_handle: &AppHandle, _data: &Value) {
    // no-op: sidecar owns automation
}

#[allow(unused_variables)]
pub async fn handle_champ_select_standalone(_data: &Value) {
    // no-op: sidecar owns automation
}
