//! Jeton d'authentification du serveur local.
//!
//! Genere a chaque demarrage et depose dans le dossier de donnees, il permet
//! aux clients legitimes de prouver qu'ils ont acces a ce dossier.
//!
//! Portee reelle : ce jeton empeche un programme qui ignore son existence de
//! piloter le serveur. Il n'arrete pas un programme malveillant tournant sous
//! le meme compte, qui peut simplement lire le fichier. La vraie frontiere
//! reste un serveur distant que l'utilisateur ne controle pas.

use std::fs;
use std::sync::RwLock;
use uuid::Uuid;

use crate::storage;

lazy_static::lazy_static! {
    static ref CURRENT: RwLock<Option<String>> = RwLock::new(None);
}

/// Mode strict : refuse les connexions non authentifiees au lieu de se
/// contenter de les journaliser. Active par CRIMSON_STRICT_AUTH=1, le temps de
/// verifier que tous les clients presentent bien le jeton.
pub fn strict_mode() -> bool {
    std::env::var("CRIMSON_STRICT_AUTH").map(|v| v == "1").unwrap_or(false)
}

pub fn generate_and_save_token() -> String {
    let token = Uuid::new_v4().to_string();
    let data_dir = storage::get_data_dir();
    let token_path = data_dir.join("auth.token");

    if let Err(e) = fs::write(&token_path, &token) {
        tracing::error!("Failed to write auth.token: {}", e);
    } else {
        tracing::info!("Auth token generated and saved to {:?}", token_path);
    }

    if let Ok(mut cur) = CURRENT.write() {
        *cur = Some(token.clone());
    }

    token
}

pub fn read_token() -> Option<String> {
    let data_dir = storage::get_data_dir();
    let token_path = data_dir.join("auth.token");
    fs::read_to_string(token_path).ok().map(|s| s.trim().to_string())
}

/// Jeton de la session en cours, sans relire le disque a chaque connexion.
pub fn current_token() -> Option<String> {
    if let Ok(cur) = CURRENT.read() {
        if let Some(t) = cur.as_ref() {
            return Some(t.clone());
        }
    }
    read_token()
}

/// Compare en temps constant, pour ne pas laisser deviner le jeton octet par
/// octet a partir du temps de reponse.
pub fn verify(candidate: &str) -> bool {
    let expected = match current_token() {
        Some(t) if !t.is_empty() => t,
        _ => return false,
    };
    if candidate.len() != expected.len() {
        return false;
    }
    let diff = candidate
        .bytes()
        .zip(expected.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b));
    diff == 0
}
