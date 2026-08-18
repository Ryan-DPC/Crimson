//! Verification des droits premium.
//!
//! La decision ne peut pas reposer sur data.json : ce fichier est ecrit par le
//! client et modifiable au Bloc-notes. Elle est donc rattachee a la session
//! Supabase de l'utilisateur et verifiee aupres de Supabase lui-meme.
//!
//! Le jeton d'acces vit uniquement en memoire, mais le jeton de rafraichissement
//! est conserve sur disque. Sans lui, le serveur ne savait pas pour qui il
//! travaillait tant que l'application n'etait pas ouverte : au demarrage de
//! Windows, toutes les actions StreamDock etaient refusees. Il obtient
//! desormais un acces par lui-meme.
//!
//! Ce que ce fichier contient n'est pas un verdict — un verdict sur disque
//! serait falsifiable, ce qui etait precisement le probleme de data.json. Il
//! contient une preuve d'identite que seul Supabase peut emettre, et qui reste
//! soumise a sa verification.
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

/// Point de verification fige a la compilation (voir build.rs). Il ne doit
/// jamais venir du client : celui-ci pourrait designer un serveur complaisant.
const SUPABASE_URL: &str = env!("CRIMSON_SUPABASE_URL");
const SUPABASE_ANON_KEY: &str = env!("CRIMSON_SUPABASE_ANON_KEY");

/// Fichier ou le jeton de rafraichissement survit aux redemarrages.
const SESSION_FILE: &str = "supabase_session.json";

#[derive(Clone)]
struct Session {
    access_token: String,
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
/// Seuls les jetons viennent du client : ils sont ensuite presentes a un point
/// de verification que le client ne choisit pas.
pub fn set_session(access_token: String, refresh_token: Option<String>) {
    if let Ok(mut s) = SESSION.write() {
        *s = Some(Session { access_token });
    }
    if let Some(rt) = refresh_token.filter(|t| !t.is_empty()) {
        store_refresh_token(&rt);
    }
    invalidate();
    tracing::info!("[ENTITLEMENT] Session enregistree, verdict a revalider");
}

fn session_path() -> std::path::PathBuf {
    crate::storage::get_data_dir().join(SESSION_FILE)
}

fn store_refresh_token(token: &str) {
    let json = serde_json::json!({ "refresh_token": token });
    match std::fs::write(session_path(), json.to_string()) {
        Ok(_) => tracing::info!("[ENTITLEMENT] Jeton de rafraichissement conserve pour les prochains demarrages"),
        Err(e) => tracing::warn!("[ENTITLEMENT] Ecriture du jeton de rafraichissement impossible : {}", e),
    }
}

fn read_refresh_token() -> Option<String> {
    let data = std::fs::read_to_string(session_path()).ok()?;
    let json: serde_json::Value = serde_json::from_str(&data).ok()?;
    json.get("refresh_token")?
        .as_str()
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
}

fn forget_refresh_token() {
    let _ = std::fs::remove_file(session_path());
}

/// Echange le jeton de rafraichissement contre un acces neuf. Supabase renvoie
/// un nouveau jeton de rafraichissement a chaque appel : il faut le conserver,
/// l'ancien devenant invalide.
async fn refresh_access_token() -> Result<String, String> {
    let refresh = read_refresh_token().ok_or("aucun jeton de rafraichissement conserve")?;

    if SUPABASE_URL.is_empty() || SUPABASE_ANON_KEY.is_empty() {
        return Err("configuration Supabase absente du binaire".to_string());
    }

    let url = format!(
        "{}/auth/v1/token?grant_type=refresh_token",
        SUPABASE_URL.trim_end_matches('/')
    );

    let resp = reqwest::Client::new()
        .post(&url)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Content-Type", "application/json")
        .timeout(HTTP_TIMEOUT)
        .body(serde_json::json!({ "refresh_token": refresh }).to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    if !status.is_success() {
        // 400 ou 401 : le jeton a ete revoque ou a expire. Le garder ne servirait
        // qu'a rejouer un echec a chaque demarrage.
        if status.as_u16() == 400 || status.as_u16() == 401 {
            forget_refresh_token();
            return Err(format!("jeton de rafraichissement rejete ({}), session oubliee", status));
        }
        return Err(format!("Supabase a repondu {}", status));
    }

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let access = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("reponse sans access_token")?
        .to_string();

    if let Some(new_refresh) = json.get("refresh_token").and_then(|v| v.as_str()) {
        store_refresh_token(new_refresh);
    }

    if let Ok(mut s) = SESSION.write() {
        *s = Some(Session { access_token: access.clone() });
    }
    tracing::info!("[ENTITLEMENT] Acces renouvele sans l'application");
    Ok(access)
}

/// Jeton d'acces courant, obtenu au besoin depuis le jeton conserve.
async fn current_access_token() -> Option<String> {
    let existing = match SESSION.read() {
        Ok(guard) => guard.as_ref().map(|s| s.access_token.clone()),
        Err(_) => None,
    };
    if let Some(t) = existing {
        return Some(t);
    }
    match refresh_access_token().await {
        Ok(t) => Some(t),
        Err(e) => {
            tracing::warn!("[ENTITLEMENT] Renouvellement impossible : {}", e);
            None
        }
    }
}

/// Efface la session (deconnexion). Le premium retombe a false immediatement,
/// et le jeton conserve est supprime : sans cela le serveur se reconnecterait
/// tout seul au demarrage suivant, malgre la deconnexion.
pub fn clear_session() {
    if let Ok(mut s) = SESSION.write() {
        *s = None;
    }
    forget_refresh_token();
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

    let token = match current_access_token().await {
        Some(t) => t,
        None => {
            tracing::warn!("[ENTITLEMENT] Aucune session : acces premium refuse");
            return false;
        }
    };

    let premium = match fetch(&token).await {
        Ok(p) => p,
        Err(FetchError::Unauthorized) => {
            // L'acces a expire. On le renouvelle depuis le jeton conserve et on
            // retente une fois, sans quoi le serveur resterait bloque jusqu'a la
            // prochaine ouverture de l'application.
            tracing::info!("[ENTITLEMENT] Acces expire, renouvellement");
            if let Ok(mut s) = SESSION.write() {
                *s = None;
            }
            match current_access_token().await {
                Some(fresh) => match fetch(&fresh).await {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!("[ENTITLEMENT] Verification impossible apres renouvellement ({}) : acces refuse", e);
                        false
                    }
                },
                None => false,
            }
        }
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

/// Distingue l'acces expire des autres echecs : lui seul justifie un
/// renouvellement puis une seconde tentative.
enum FetchError {
    Unauthorized,
    Other(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Unauthorized => write!(f, "acces refuse par Supabase"),
            FetchError::Other(e) => write!(f, "{}", e),
        }
    }
}

/// Interroge Supabase avec le jeton de l'utilisateur. La RLS ne lui renvoie que
/// sa propre ligne, il n'y a donc pas d'identifiant a passer.
async fn fetch(access_token: &str) -> Result<bool, FetchError> {
    if SUPABASE_URL.is_empty() || SUPABASE_ANON_KEY.is_empty() {
        return Err(FetchError::Other("configuration Supabase absente du binaire".to_string()));
    }

    let url = format!(
        "{}/rest/v1/profiles?select=is_premium",
        SUPABASE_URL.trim_end_matches('/')
    );

    let resp = reqwest::Client::new()
        .get(&url)
        .header("apikey", SUPABASE_ANON_KEY)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Accept", "application/json")
        .timeout(HTTP_TIMEOUT)
        .send()
        .await
        .map_err(|e| FetchError::Other(e.to_string()))?;

    let status = resp.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(FetchError::Unauthorized);
    }
    if !status.is_success() {
        return Err(FetchError::Other(format!("Supabase a repondu {}", status)));
    }

    let rows: serde_json::Value = resp.json().await.map_err(|e| FetchError::Other(e.to_string()))?;

    Ok(rows
        .get(0)
        .and_then(|row| row.get("is_premium"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}
