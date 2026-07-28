use base64::engine::general_purpose::STANDARD;
use base64::Engine;

// Outil de diagnostic : verifie qu'un refresh token Spotify est toujours valide.
// Les identifiants sont lus dans l'environnement, jamais ecrits en dur.
//
//   set SPOTIFY_CLIENT_ID=...
//   set SPOTIFY_CLIENT_SECRET=...
//   set SPOTIFY_REFRESH_TOKEN=...
//   cargo run -p test_spotify
#[tokio::main]
async fn main() {
    let (id, secret, r_token) = match (
        std::env::var("SPOTIFY_CLIENT_ID"),
        std::env::var("SPOTIFY_CLIENT_SECRET"),
        std::env::var("SPOTIFY_REFRESH_TOKEN"),
    ) {
        (Ok(i), Ok(s), Ok(t)) if !i.is_empty() && !s.is_empty() && !t.is_empty() => (i, s, t),
        _ => {
            eprintln!(
                "Variables manquantes. Definissez SPOTIFY_CLIENT_ID, \
                 SPOTIFY_CLIENT_SECRET et SPOTIFY_REFRESH_TOKEN avant de lancer cet outil."
            );
            std::process::exit(2);
        }
    };

    let client = reqwest::Client::new();
    let auth_str = format!("{}:{}", id, secret);
    let b64 = STANDARD.encode(auth_str.as_bytes());
    let params = [("grant_type", "refresh_token"), ("refresh_token", r_token.as_str())];
    match client.post("https://accounts.spotify.com/api/token")
        .header(reqwest::header::AUTHORIZATION, format!("Basic {}", b64))
        .form(&params).send().await {
        Ok(resp) => {
            println!("Status: {}", resp.status());
            println!("Text: {}", resp.text().await.unwrap());
        },
        Err(e) => { println!("Error: {}", e); }
    }
}
