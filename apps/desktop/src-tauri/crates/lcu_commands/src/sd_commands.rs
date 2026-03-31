use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tauri::AppHandle;
use crate::{storage, lcu, analyzer};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StreamDeckCommand {
    #[serde(rename = "TOGGLE_AUTO_ACCEPT")]
    ToggleAutoAccept,

    #[serde(rename = "TOGGLE_AUTO_BAN")]
    ToggleAutoBan { 
        #[serde(rename = "championId")]
        champion_id: Option<u64> 
    },

    #[serde(rename = "TOGGLE_AUTO_PICK")]
    ToggleAutoPick { 
        #[serde(rename = "championId")]
        champion_id: Option<u64> 
    },

    #[serde(rename = "DODGE_GAME")]
    DodgeGame,

    #[serde(rename = "INJECT_BUILD")]
    InjectBuild {
        #[serde(rename = "championId")]
        champion_id: u32,
        #[serde(rename = "championName")]
        champion_name: Option<String>,
        role: Option<String>
    },

    #[serde(rename = "GET_BUILDS")]
    GetBuilds {
        #[serde(rename = "championId")]
        champion_id: u32,
        #[serde(rename = "championName")]
        champion_name: Option<String>,
    },

    #[serde(rename = "TOGGLE_GLOBAL_AUTOMATION")]
    ToggleGlobalAutomation
}

impl StreamDeckCommand {
    pub async fn execute(
        self, 
        handle: &AppHandle, 
        tx: &broadcast::Sender<String>
    ) -> Result<Option<Value>, String> {
        match self {
            StreamDeckCommand::ToggleAutoAccept => {
                let mut data = storage::load_data(handle);
                data.auto_accept = !data.auto_accept;
                storage::save_data(handle, &data);
                let _ = tx.send(json!({"type": "AUTO_ACCEPT_STATE", "enabled": data.auto_accept}).to_string());
                Ok(None)
            }
            StreamDeckCommand::ToggleAutoBan { champion_id } => {
                let mut data = storage::load_data(handle);
                let id = champion_id.unwrap_or(0);
                if data.auto_ban == Some(id) { 
                    data.auto_ban = None; 
                } else { 
                    data.auto_ban = Some(id); 
                    data.auto_pick = None; 
                }
                storage::save_data(handle, &data);
                let _ = tx.send(json!({"type": "AUTO_BAN_STATE", "championId": data.auto_ban}).to_string());
                Ok(None)
            }
            StreamDeckCommand::ToggleAutoPick { champion_id } => {
                let mut data = storage::load_data(handle);
                let id = champion_id.unwrap_or(0);
                if data.auto_pick == Some(id) { 
                    data.auto_pick = None; 
                } else { 
                    data.auto_pick = Some(id); 
                    data.auto_ban = None; 
                }
                storage::save_data(handle, &data);
                let _ = tx.send(json!({"type": "AUTO_PICK_STATE", "championId": data.auto_pick}).to_string());
                Ok(None)
            }
            StreamDeckCommand::DodgeGame => {
                let _ = lcu::lcu_request("POST".into(), "/lol-login/v1/session/invoke?destination=lcdsServiceProxy&method=call&args=[\"\", \"teambuilder-draft\", \"quitV2\", \"\"]".into(), None).await;
                Ok(None)
            }
            StreamDeckCommand::InjectBuild { champion_id, champion_name, role } => {
                if champion_id == 0 { return Ok(None); }
                let mut c_name = champion_name.unwrap_or_default();
                
                if c_name.is_empty() {
                    if let Ok(c_json) = lcu::lcu_request("GET".into(), format!("/lol-game-data/assets/v1/champions/{}.json", champion_id), None).await {
                        if let Ok(c_data) = serde_json::from_str::<Value>(&c_json) {
                            c_name = c_data["name"].as_str().unwrap_or("").to_string();
                        }
                    }
                }

                if !c_name.is_empty() {
                    let r = role.unwrap_or_else(|| "mid".to_string());
                    if let Ok(builds) = analyzer::fetch_dynamic_runes(handle.clone(), c_name, r, None, None).await {
                        if let Some(best) = builds.first() {
                            let mut selected_perks = best.perk_ids.clone();
                            selected_perks.extend(&best.shards);

                            let rune_page = json!({
                                "name": format!("CRIMSON: {}", best.name),
                                "primaryStyleId": best.primary_style_id,
                                "subStyleId": best.sub_style_id,
                                "selectedPerkIds": selected_perks,
                                "current": true
                            });

                            if let Ok(pages_json) = lcu::lcu_request("GET".into(), "/lol-perks/v1/pages".into(), None).await {
                                if let Ok(pages) = serde_json::from_str::<Value>(&pages_json) {
                                    if let Some(pages_arr) = pages.as_array() {
                                        for p in pages_arr {
                                            if p["name"].as_str().unwrap_or("").starts_with("CRIMSON:") {
                                                let id = p["id"].as_u64().unwrap_or(0);
                                                let _ = lcu::lcu_request("DELETE".into(), format!("/lol-perks/v1/pages/{}", id), None).await;
                                            }
                                        }
                                    }
                                }
                            }
                            let _ = lcu::lcu_request("POST".into(), "/lol-perks/v1/pages".into(), Some(rune_page.to_string())).await;
                        }
                    }
                }
                Ok(None)
            }
            StreamDeckCommand::GetBuilds { champion_id, champion_name } => {
                let mut c_name = champion_name.unwrap_or_default();
                if c_name.is_empty() && champion_id > 0 {
                    if let Ok(c_json) = lcu::lcu_request("GET".into(), format!("/lol-game-data/assets/v1/champions/{}.json", champion_id), None).await {
                        if let Ok(c_data) = serde_json::from_str::<Value>(&c_json) {
                            c_name = c_data["name"].as_str().unwrap_or("").to_string();
                        }
                    }
                }
                if !c_name.is_empty() {
                    if let Ok(builds) = analyzer::fetch_dynamic_runes(handle.clone(), c_name, "mid".to_string(), None, None).await {
                        return Ok(Some(json!({"type": "BUILDS_LIST", "builds": builds})));
                    }
                }
                Ok(None)
            }
            StreamDeckCommand::ToggleGlobalAutomation => {
                let mut data = storage::load_data(handle);
                if data.auto_ban.is_some() || data.auto_pick.is_some() {
                    data.remembered_auto_ban = data.auto_ban;
                    data.remembered_auto_pick = data.auto_pick;
                    data.auto_ban = None;
                    data.auto_pick = None;
                } else {
                    data.auto_ban = data.remembered_auto_ban;
                    data.auto_pick = data.remembered_auto_pick;
                }
                storage::save_data(handle, &data);
                let _ = tx.send(json!({"type": "AUTO_BAN_STATE", "championId": data.auto_ban}).to_string());
                let _ = tx.send(json!({"type": "AUTO_PICK_STATE", "championId": data.auto_pick}).to_string());
                Ok(None)
            }
        }
    }
}
