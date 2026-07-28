use tauri::{AppHandle, Manager};
use reqwest::Client;
use serde_json::{json, Value};
use crate::storage;
// SQLite Persistent Cache is used instead of in-memory BUILD_CACHE
// SQLite Persistent Cache is used instead of in-memory BUILD_CACHE

// SQLite Persistent Cache is used instead of in-memory BUILD_CACHE

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CounterSuggestion {
    pub name: String,
    #[serde(rename = "keystoneId")]
    pub keystone_id: u32,
}

fn deserialize_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(f64),
    }

    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => Ok(s),
        StringOrNumber::Number(n) => Ok(n.to_string()),
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct RuneBuild {
    pub name: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub winrate: String,
    #[serde(deserialize_with = "deserialize_string_or_number")]
    pub banrate: String,
    #[serde(rename = "primaryStyleId")]
    pub primary_style_id: u32,
    #[serde(rename = "subStyleId")]
    pub sub_style_id: u32,
    #[serde(rename = "perkIds")]
    pub perk_ids: Vec<u32>,
    pub shards: Vec<u32>,
    pub spells: Vec<u32>,
    pub counters: Option<Vec<CounterSuggestion>>,
}

#[tauri::command]
pub async fn fetch_single_build(app: AppHandle, champion_name: String, role: String, opponent: Option<String>, patch: Option<String>, index: i32, champion_id: Option<u32>) -> Result<RuneBuild, String> {
    let pool = app.state::<crate::db::DbPool>();
    let data = storage::load_data(&app);
    let key = data.gemini_api_key.unwrap_or_default();
    
    fetch_single_build_standalone(&pool, key, champion_name, role, opponent, patch, index, champion_id).await
}

pub async fn fetch_lcu_recommended_pages(champion_id: u32, role: &str) -> Result<Vec<RuneBuild>, String> {
    let lcu_position = match role {
        "top" => "top",
        "jungle" => "jungle",
        "mid" => "middle",
        "adc" => "bottom",
        "support" => "utility",
        _ => "middle",
    };

    let endpoint = format!("/lol-perks/v1/recommended-pages/champion/{}/position/{}/map/11", champion_id, lcu_position);
    
    let response_str = crate::lcu::lcu_request("GET".to_string(), endpoint, None).await?;
    
    let raw_pages: Vec<serde_json::Value> = serde_json::from_str(&response_str)
        .map_err(|e| format!("Failed to parse LCU response: {}", e))?;
        
    let mut builds = Vec::new();

    for page in raw_pages {
        let keystone_name = page["keystone"]["name"].as_str().unwrap_or("Méta");
        let name = format!("Méta LCU - {}", keystone_name);
        
        let primary_style_id = page["primaryPerkStyleId"].as_u64().unwrap_or(0) as u32;
        let sub_style_id = page["secondaryPerkStyleId"].as_u64().unwrap_or(0) as u32;
        
        let mut all_ids = Vec::new();
        if let Some(perks_arr) = page["perks"].as_array() {
            for perk in perks_arr {
                if let Some(id) = perk["id"].as_u64() {
                    all_ids.push(id as u32);
                }
            }
        }
        
        if all_ids.len() >= 9 {
            let perk_ids = all_ids[0..6].to_vec();
            let shards = all_ids[6..9].to_vec();
            
            let mut spells = match role {
                "jungle" => vec![4, 11],
                "support" => vec![4, 3],
                "adc" => vec![4, 7],
                "top" => vec![4, 12],
                _ => vec![4, 14],
            };
            
            if let Some(spells_arr) = page["summonerSpellIds"].as_array() {
                let parsed_spells: Vec<u32> = spells_arr.iter().filter_map(|v| v.as_u64().map(|n| n as u32)).collect();
                if parsed_spells.len() >= 2 {
                    spells = parsed_spells[0..2].to_vec();
                }
            }
            
            builds.push(RuneBuild {
                name,
                winrate: "LCU Méta".to_string(),
                banrate: "--".to_string(),
                primary_style_id,
                sub_style_id,
                perk_ids,
                shards,
                spells,
                counters: None,
            });
        }
    }
    
    Ok(builds)
}

pub async fn fetch_single_build_standalone(pool: &crate::db::DbPool, api_key: String, champion_name: String, role: String, opponent: Option<String>, patch: Option<String>, index: i32, champion_id: Option<u32>) -> Result<RuneBuild, String> {
    let normalized_role = match role.to_lowercase().as_str() {
        "top" => "top",
        "jungle" => "jungle",
        "mid" | "middle" => "mid",
        "adc" | "bottom" => "adc",
        "support" | "utility" => "support",
        _ => "mid",
    };

    let current_patch = patch.unwrap_or_else(|| "15.5.1".to_string());
    
    let cache_opponent = if index == 2 {
        opponent.as_deref().unwrap_or("none").to_lowercase()
    } else {
        "none".to_string()
    };

    // 1. Check Cache (for all indexes, using opponent-aware cache_opponent)
    let cached_map = crate::db::get_cached_runes(pool, &champion_name, &normalized_role, &current_patch, &cache_opponent).unwrap_or_default();
    if let Some(data) = cached_map.get(&index) {
        if let Ok(b) = serde_json::from_str::<RuneBuild>(data) {
            // If the user has a Gemini API key configured, we only want to return cached Gemini builds.
            // If the cached build is a fallback preset or a local LCU recommended page, skip the cache
            // so we can query Gemini and cache the optimal build.
            let is_fallback = b.name.contains("(Fallback)") || b.name.starts_with("Méta LCU -");
            if api_key.is_empty() || !is_fallback {
                return Ok(b);
            }
        }
    }

    // 2. Fetch from Gemini
    if !api_key.is_empty() {
        let build_type = match index {
            1 => "Most Popular/Meta",
            2 => "Situational Counter",
            3 => "Scaling/Late Game",
            _ => "Standard",
        };

        match fetch_gemini_single(&api_key, &champion_name, normalized_role, opponent.clone(), build_type).await {
            Ok(b) => {
                // Save to Cache
                if let Ok(serialized) = serde_json::to_string(&b) {
                    let _ = crate::db::save_rune_cache(pool, &champion_name, &normalized_role, &current_patch, index, &cache_opponent, &serialized);
                }
                return Ok(b);
            }
            Err(e) => {
                println!("Gemini single fetch failed for index {}: {}", index, e);
            }
        }
    } else if let Some(cid) = champion_id {
        // 3. Fallback to LCU Recommended Pages (Champion-specific, real meta builds from client)
        match fetch_lcu_recommended_pages(cid, normalized_role).await {
            Ok(pages) => {
                if !pages.is_empty() {
                    let fallback_idx = (index as usize - 1) % pages.len();
                    let b = pages[fallback_idx].clone();
                    // Do NOT cache local LCU recommended pages to avoid blocking future Gemini queries
                    return Ok(b);
                }
            }
            Err(e) => {
                println!("LCU recommended pages fetch failed for index {}: {}", index, e);
            }
        }
    }

    // 4. Fallback to Meta Presets
    let presets = get_meta_presets(normalized_role);
    let fallback_idx = (index as usize - 1) % presets.len();
    Ok(presets[fallback_idx].clone())
}

#[tauri::command]
pub async fn fetch_dynamic_runes(app: AppHandle, champion_name: String, role: String, opponent: Option<String>, patch: Option<String>, champion_id: Option<u32>) -> Result<Vec<RuneBuild>, String> {
    let pool = app.state::<crate::db::DbPool>();
    let data = storage::load_data(&app);
    let key = data.gemini_api_key.unwrap_or_default();
    
    fetch_dynamic_runes_standalone(&pool, key, champion_name, role, opponent, patch, champion_id).await
}

pub async fn fetch_dynamic_runes_standalone(pool: &crate::db::DbPool, api_key: String, champion_name: String, role: String, opponent: Option<String>, patch: Option<String>, champion_id: Option<u32>) -> Result<Vec<RuneBuild>, String> {
    let mut builds = Vec::new();
    for i in 1..=3 {
        if let Ok(b) = fetch_single_build_standalone(pool, api_key.clone(), champion_name.clone(), role.clone(), opponent.clone(), patch.clone(), i, champion_id).await {
            builds.push(b);
        }
    }
    Ok(builds)
}

/// Returns 2 solid meta builds for the given role with accurate rune IDs.
/// These are based on common Season 2025 meta and serve as a reliable offline fallback.
fn get_meta_presets(role: &str) -> Vec<RuneBuild> {
    match role {
        "top" => vec![
            RuneBuild {
                name: "Conqueror Bruiser (Fallback)".into(),
                winrate: "Top Méta".into(), banrate: "--".into(),
                primary_style_id: 8000, sub_style_id: 8200,
                perk_ids: vec![8010, 9111, 9104, 8014, 8233, 8237],
                shards: vec![5005, 5008, 5001],
                spells: vec![4, 14],
                counters: None,
            },
            RuneBuild {
                name: "Grasp Tank (Fallback)".into(),
                winrate: "Top Tank".into(), banrate: "--".into(),
                primary_style_id: 8400, sub_style_id: 8000,
                perk_ids: vec![8437, 8446, 8473, 8451, 9111, 9104],
                shards: vec![5007, 5001, 5001],
                spells: vec![4, 14],
                counters: None,
            },
        ],
        "jungle" => vec![
            RuneBuild {
                name: "Domination Carry (Fallback)".into(),
                winrate: "JG Méta".into(), banrate: "--".into(),
                primary_style_id: 8100, sub_style_id: 8000,
                perk_ids: vec![8112, 8139, 8138, 8136, 8010, 9104],
                shards: vec![5005, 5008, 5001],
                spells: vec![4, 11],
                counters: None,
            },
            RuneBuild {
                name: "Conqueror Fighter JG (Fallback)".into(),
                winrate: "JG Bruiser".into(), banrate: "--".into(),
                primary_style_id: 8000, sub_style_id: 8300,
                perk_ids: vec![8010, 9111, 9104, 8014, 8345, 8347],
                shards: vec![5005, 5008, 5001],
                spells: vec![4, 11],
                counters: None,
            },
        ],
        "mid" => vec![
            RuneBuild {
                name: "Electrocute Assassin (Fallback)".into(),
                winrate: "Mid Assassin".into(), banrate: "--".into(),
                primary_style_id: 8100, sub_style_id: 8200,
                perk_ids: vec![8112, 8139, 8138, 8136, 8233, 8237],
                shards: vec![5005, 5008, 5001],
                spells: vec![4, 14],
                counters: Some(vec![
                    CounterSuggestion { name: "Vex".into(), keystone_id: 8112 },
                    CounterSuggestion { name: "Fizz".into(), keystone_id: 8112 },
                    CounterSuggestion { name: "Pantheon".into(), keystone_id: 8010 },
                ]),
            },
            RuneBuild {
                name: "Phase Rush Mage (Fallback)".into(),
                winrate: "Mid Mage".into(), banrate: "--".into(),
                primary_style_id: 8200, sub_style_id: 8100,
                perk_ids: vec![8214, 8226, 8210, 8237, 8126, 8138],
                shards: vec![5007, 5008, 5001],
                spells: vec![4, 3],
                counters: Some(vec![
                    CounterSuggestion { name: "Vex".into(), keystone_id: 8112 },
                    CounterSuggestion { name: "Fizz".into(), keystone_id: 8112 },
                    CounterSuggestion { name: "Pantheon".into(), keystone_id: 8010 },
                ]),
            },
        ],
        "adc" => vec![
            RuneBuild {
                name: "Lethal Tempo ADC (Fallback)".into(),
                winrate: "ADC Méta".into(), banrate: "--".into(),
                primary_style_id: 8000, sub_style_id: 8300,
                perk_ids: vec![8008, 9101, 9104, 8014, 8345, 8347],
                shards: vec![5005, 5008, 5001],
                spells: vec![4, 7],
                counters: None,
            },
            RuneBuild {
                name: "Fleet Footwork ADC (Fallback)".into(),
                winrate: "ADC Safe".into(), banrate: "--".into(),
                primary_style_id: 8000, sub_style_id: 8400,
                perk_ids: vec![8021, 9101, 9104, 8014, 8473, 8451],
                shards: vec![5005, 5008, 5001],
                spells: vec![4, 7],
                counters: None,
            },
        ],
        "support" | _ => vec![
            RuneBuild {
                name: "Guardian Support (Fallback)".into(),
                winrate: "Support Tanky".into(), banrate: "--".into(),
                primary_style_id: 8400, sub_style_id: 8200,
                perk_ids: vec![8465, 8446, 8473, 8451, 8233, 8237],
                shards: vec![5007, 5008, 5011],
                spells: vec![4, 3],
                counters: None,
            },
            RuneBuild {
                name: "Glacial Enchanter (Fallback)".into(),
                winrate: "Support Utility".into(), banrate: "--".into(),
                primary_style_id: 8300, sub_style_id: 8400,
                perk_ids: vec![8351, 8306, 8316, 8347, 8444, 8451],
                shards: vec![5007, 5008, 5011],
                spells: vec![4, 3],
                counters: None,
            },
        ],
    }
}

async fn fetch_gemini_single(api_key: &str, champion: &str, role: &str, opponent: Option<String>, build_type: &str) -> Result<RuneBuild, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30)) // Lowered timeout for single build
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-3.0-flash:generateContent?key={}", api_key);

    
    // Mask key for logging
    let masked_key = if api_key.len() > 8 {
        format!("{}...{}", &api_key[..4], &api_key[api_key.len()-4..])
    } else {
        "***".to_string()
    };

    let opponent_context_name = opponent.as_deref().unwrap_or("the enemy team");

    let prompt = format!(
        "Provide the absolute latest, highest win rate, or most popular meta build for {champion} {role} (Current Patch). \
        Return ONLY ONE JSON object for a {build_type} build for {champion} {role}. Strictly follow this schema. \
        SCHEMA: {{ \
          \"name\": \"string (short and descriptive, e.g. 'OP.GG Conqueror Méta')\", \
          \"winrate\": \"string (e.g. 52.1%)\", \
          \"banrate\": \"string\", \
          \"primaryStyleId\": int, \
          \"subStyleId\": int, \
          \"perkIds\": [exactly 6 ints], \
          \"shards\": [exactly 3 ints], \
          \"spells\": [exactly 2 ints], \
          \"counters\": [{{ \"name\": \"ChampionName\", \"keystoneId\": int }}] \
        }} \
        CRITICAL RULES FOR PERKIDS (ARRAY OF 6): \
        - Primary Tree (4 total): [Keystone ID, Row 1 Perk ID, Row 2 Perk ID, Row 3 Perk ID]. Exactly one from EACH row. \
        - Secondary Tree (2 total): [Perk ID 1, Perk ID 2]. Choose exactly 2 from DIFFERENT rows. \
        - TOTAL perkIds MUST BE EXACTLY 6. \
        CRITICAL RULES FOR SHARDS: \
        - Exactly 3 IDs. Row 1: (5007, 5005, or 5008). Row 2: (5008, 5010, or 5001). Row 3: (5011, 5013, or 5001). \
        STRICT: No markdown, no text, just JSON. If opponent is {opponent_context_name}, you MUST optimize the build specifically to counter them.",
        champion = champion, role = role, 
        build_type = build_type,
        opponent_context_name = opponent_context_name
    );

    let body = json!({
        "contents": [{"parts": [{"text": prompt}]}],
        "tools": [{"google_search": {}}],
        "generationConfig": { "responseMimeType": "application/json" }
    });

    println!("Sending Gemini 3.0 Flash request for '{}' (key: {})...", build_type, masked_key);

    let res: Value = client.post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await
        .map_err(|e| {
            let err_msg = e.to_string();
            // Sanitize key if present in the error message
            if err_msg.contains("key=") {
                "Request failed (URL hidden for security)".to_string()
            } else {
                format!("Request failed: {}", err_msg)
            }
        })?
        .json().await.map_err(|e| format!("JSON parse failed: {}", e))?;

    // Debug: success log
    if let Some(candidates) = res["candidates"].as_array() {
        if !candidates.is_empty() {
             println!("🎉 Success: Gemini 3.0 Flash generated '{}' build", build_type);
        }
    }

    let text_val = &res["candidates"][0]["content"]["parts"][0]["text"];
    if text_val.is_null() {
        return Err("La génération de builds AI a échoué (réponse vide)".into());
    }
    let mut text = text_val.as_str().unwrap().to_string();

    // Clean up markdown or preambles if AI ignored "ONLY JSON" instruction
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        text = text[start..=end].to_string();
    } else {
        if text.contains("```json") {
            text = text.replace("```json", "");
        }
        if text.contains("```") {
            text = text.replace("```", "");
        }
        text = text.trim().to_string();
    }

    let mut build: RuneBuild = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Gemini build: {} — raw: {}", e, &text[..text.len().min(300)]))?;

    // Sanitize shard IDs to match valid S14/S15 Client shards:
    // Row 1: CDR (5007), Atk Spd (5005), Adaptive (5008)
    // Row 2: Adaptive (5008), MS (5010), Scale HP (5001)
    // Row 3: Flat HP (5011), Tenacity (5013), Scale HP (5001)
    if build.shards.len() == 3 {
        if build.shards[0] != 5007 && build.shards[0] != 5005 && build.shards[0] != 5008 {
            build.shards[0] = 5008; // fallback to Adaptive
        }
        if build.shards[1] != 5008 && build.shards[1] != 5010 && build.shards[1] != 5001 {
            build.shards[1] = 5008; // fallback to Adaptive
        }
        if build.shards[2] != 5011 && build.shards[2] != 5013 && build.shards[2] != 5001 {
            build.shards[2] = 5001; // fallback to Scale HP
        }
    } else {
        build.shards = vec![5008, 5008, 5001]; // standard solid default
    }

    Ok(build)
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct DraftAnalysisResult {
    pub counters: Vec<CounterSuggestion>,
    pub synergy: String,
}

#[tauri::command]
pub async fn analyze_draft(
    app: AppHandle,
    my_team: Vec<String>,
    their_team: Vec<String>,
    role: String
) -> Result<DraftAnalysisResult, String> {
    let data = storage::load_data(&app);
    let key = data.gemini_api_key.unwrap_or_default();
    
    if key.is_empty() {
        return Err("Veuillez configurer votre clé API Gemini dans les paramètres.".to_string());
    }

    let prompt = format!(
        "Tu es un coach Challenger de League of Legends (S15). 
Analyse cette draft en temps réel.
Mon rôle : {}
Mon équipe a verrouillé : {:?}
L'équipe adverse a verrouillé : {:?}

Retourne UNIQUEMENT un objet JSON valide avec cette structure exacte (SANS AUCUN MARKDOWN, ni ```json, juste l'objet brut) :
{{
  \"counters\": [
    {{ \"name\": \"Champion1\", \"keystoneId\": 8112 }},
    {{ \"name\": \"Champion2\", \"keystoneId\": 8008 }},
    {{ \"name\": \"Champion3\", \"keystoneId\": 8229 }}
  ],
  \"synergy\": \"Une ou deux phrases percutantes sur les faiblesses de l'ennemi ou ce qui manque cruellement à notre équipe (ex: manque d'AP, manque de tank, besoin de CC).\"
}}
Choisis 3 champions parfaits pour moi à jouer en 'counter-pick' face à leur composition.",
        role, my_team, their_team
    );

    let client = Client::new();
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-3.0-flash:generateContent?key={}", key);
    let body = json!({
        "contents": [{"parts": [{"text": prompt}]}],
        "generationConfig": { "responseMimeType": "application/json" }
    });

    let res: Value = client.post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await
        .map_err(|e| e.to_string())?
        .json().await.map_err(|e| format!("JSON parse failed: {}", e))?;

    let text_val = &res["candidates"][0]["content"]["parts"][0]["text"];
    if text_val.is_null() {
        return Err("Erreur lors de la génération de l'analyse.".into());
    }
    
    let mut text = text_val.as_str().unwrap().to_string();
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        text = text[start..=end].to_string();
    } else {
        text = text.replace("```json", "").replace("```", "").trim().to_string();
    }

    let analysis: DraftAnalysisResult = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse Draft Analysis: {}", e))?;

    Ok(analysis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meta_presets_have_correct_perk_counts() {
        let roles = vec!["top", "jungle", "mid", "adc", "support"];
        
        for role in roles {
            let builds = get_meta_presets(role);
            assert!(!builds.is_empty(), "Role {} should have at least one preset", role);
            
            for build in builds {
                assert_eq!(build.perk_ids.len(), 6, "Build '{}' for role {} must have exactly 6 perks (1 keystone, 3 primary, 2 secondary)", build.name, role);
                assert_eq!(build.shards.len(), 3, "Build '{}' for role {} must have exactly 3 shards", build.name, role);
                assert_eq!(build.spells.len(), 2, "Build '{}' for role {} must have exactly 2 spells", build.name, role);
                assert!(build.primary_style_id > 0, "Build '{}' needs a valid primary style", build.name);
                assert!(build.sub_style_id > 0, "Build '{}' needs a valid sub style", build.name);
            }
        }
    }
}
