//! Verification des droits premium.
//!
//! La decision ne peut pas reposer sur data.json : ce fichier est ecrit par le
//! client et modifiable au Bloc-notes. Elle est donc rattachee a la session
//! Supabase de l'utilisateur, verifiee aupres de Supabase lui-meme, et gardee
//! uniquement en memoire — rien n'est ecrit sur disque.
//!
//! Politique en cas de doute : refus. Pas de session, reseau coupe, reponse
//! inattendue de Supabase — dans tous ces cas l'acces premium est refuse.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Duree pendant laquelle un verdict reste valable sans reinterroger Supabase.
const CACHE_TTL: Duration = Duration::from_secs(300);
const HTTP_TIMEOUT: Duration = Duration::from_secs(8);
/// Intervalle de reapplication du verdict aux services premium.
const GUARD_INTERVAL: Duration = Duration::from_secs(20);

#[derive(Clone)]
struct Session {
    access_token: String,
    supabase_url: String,
    supabase_anon_key: String,
}

#[derive(Clone, Copy)]
struct Verdict {
    premium: bool,
    checked_at: Instant,
}

lazy_static::lazy_static! {
    static ref SESSION: RwLock<Option<Session>> = RwLock::new(None);
    static ref VERDICT: RwLock<Option<Verdict>> = RwLock::new(None);
    /// Reveille le garde-fou des qu'une session change, pour ne pas attendre
    /// GUARD_INTERVAL apres une connexion ou une deconnexion.
    static ref WAKE: tokio::sync::Notify = tokio::sync::Notify::new();
}

/// Enregistre la session transmise par l'application apres authentification.
pub fn set_session(access_token: String, supabase_url: String, supabase_anon_key: String) {
    if let Ok(mut s) = SESSION.write() {
        *s = Some(Session { access_token, supabase_url, supabase_anon_key });
    }
    invalidate();
    tracing::info!("[ENTITLEMENT] Session enregistree, verdict a revalider");
}

/// Efface la session (deconnexion). Le premium retombe a false immediatement.
pub fn clear_session() {
    if let Ok(mut s) = SESSION.write() {
        *s = None;
    }
    invalidate();
    tracing::info!("[ENTITLEMENT] Session effacee");
}

fn invalidate() {
    if let Ok(mut v) = VERDICT.write() {
        *v = None;
    }
    WAKE.notify_waiters();
}

/// Applique en continu le verdict aux services premium.
///
/// Le client peut mentir dans data.json et envoyer TOGGLE_PLUGIN : cette boucle
/// ramene systematiquement chaque service a `droits && preference`. Les services
/// demarrent donc desactives et ne s'allument qu'une fois les droits confirmes.
pub fn start_guard(flags: Vec<(&'static str, Arc<AtomicBool>)>) {
    tokio::spawn(async move {
        loop {
            let premium = is_premium().await;
            let data = crate::storage::load_data_from_path(crate::storage::get_data_path_from_env());
            let prefs = data.other.get("plugins").cloned().unwrap_or(serde_json::Value::Null);

            for (name, flag) in &flags {
                let wanted = prefs.get(*name).and_then(|v| v.as_bool()).unwrap_or(false);
                let target = premium && wanted;
                if flag.swap(target, Ordering::Relaxed) != target {
                    tracing::info!("[ENTITLEMENT] service {} -> {}", name, target);
                }
            }

            tokio::select! {
                _ = tokio::time::sleep(GUARD_INTERVAL) => {}
                _ = WAKE.notified() => {}
            }
        }
    });
}

/// Verdict courant, reinterroge aupres de Supabase si le cache a expire.
pub async fn is_premium() -> bool {
    if let Ok(guard) = VERDICT.read() {
        if let Some(v) = *guard {
            if v.checked_at.elapsed() < CACHE_TTL {
                return v.premium;
            }
        }
    }

    let session = match SESSION.read() {
        Ok(guard) => guard.clone(),
        Err(_) => None,
    };

    let session = match session {
        Some(s) => s,
        None => {
            tracing::warn!("[ENTITLEMENT] Aucune session : acces premium refuse");
            return false;
        }
    };

    let premium = match fetch(&session).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("[ENTITLEMENT] Verification impossible ({}) : acces premium refuse", e);
            false
        }
    };

    if let Ok(mut guard) = VERDICT.write() {
        *guard = Some(Verdict { premium, checked_at: Instant::now() });
    }
    premium
}

/// Interroge Supabase avec le jeton de l'utilisateur. La RLS ne lui renvoie que
/// sa propre ligne, il n'y a donc pas d'identifiant a passer.
async fn fetch(s: &Session) -> Result<bool, String> {
    let url = format!(
        "{}/rest/v1/profiles?select=is_premium",
        s.supabase_url.trim_end_matches('/')
    );

    let resp = reqwest::Client::new()
        .get(&url)
        .header("apikey", &s.supabase_anon_key)
        .header("Authorization", format!("Bearer {}", s.access_token))
        .header("Accept", "application/json")
        .timeout(HTTP_TIMEOUT)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("Supabase a repondu {}", status));
    }

    let rows: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    Ok(rows
        .get(0)
        .and_then(|row| row.get("is_premium"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}
