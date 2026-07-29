extern crate winres;
use std::env;
use std::path::PathBuf;

/// Embarque la configuration Supabase dans le binaire au moment de la
/// compilation.
///
/// Elle ne peut pas etre transmise par le client : n'importe quel client
/// WebSocket local pourrait alors designer son propre serveur, qui repondrait
/// que tout le monde est premium. Le point de verification doit etre fixe.
///
/// La source est crimson/.env, deja utilise par Vite : une seule valeur a
/// maintenir, identique en local et en CI. Absente, la verification echoue
/// et refuse l'acces.
fn embed_supabase_config() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let env_path = manifest_dir.join("..").join("crimson").join(".env");
    println!("cargo:rerun-if-changed={}", env_path.display());

    let contents = std::fs::read_to_string(&env_path).unwrap_or_default();
    for key in ["VITE_SUPABASE_URL", "VITE_SUPABASE_ANON_KEY"] {
        let value = contents
            .lines()
            .find_map(|line| {
                let (k, v) = line.split_once('=')?;
                if k.trim() == key { Some(v.trim().to_string()) } else { None }
            })
            .unwrap_or_default();

        if value.is_empty() {
            println!("cargo:warning={} introuvable : la verification des droits refusera tout acces premium.", key);
        }
        println!("cargo:rustc-env=CRIMSON_{}={}", key.trim_start_matches("VITE_"), value);
    }
}

fn main() {
    // Declarer un rerun-if-changed desactive le comportement par defaut, qui
    // relance le script des qu'un fichier du paquet bouge. Il faut donc lister
    // explicitement ce qui doit le reveiller — dont CARGO_PRIMARY_PACKAGE,
    // sans quoi la sortie mise en cache lors d'une compilation en dependance
    // serait reutilisee et le binaire perdrait nom et icone.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CRIMSON_EMBED_SERVER_RESOURCE");

    embed_supabase_config();

    // winres emet `cargo:rustc-link-lib=dylib=resource`, une directive globale
    // qui s'applique a tout ce qui lie cette bibliotheque. L'application Tauri
    // en dependant, elle heritait de cette ressource VERSION en plus de la
    // sienne, et l'editeur de liens refusait :
    //   CVT1100: duplicate resource. type:VERSION
    //
    // La ressource n'est donc posee que sur demande explicite. Les deux chemins
    // qui produisent le sidecar distribue la reclament : scripts/build_release.ps1
    // et l'etape Build Sidecar de la CI. Le build de l'application Tauri ne la
    // demande pas, et n'entre donc plus en conflit.
    //
    // Sans cette variable, crimson-server.exe se compile sans nom ni icone :
    // c'est voulu, pour un binaire de developpement.
    let embed_resource = env::var("CRIMSON_EMBED_SERVER_RESOURCE").as_deref() == Ok("1");

    if embed_resource && env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winres::WindowsResource::new();
        
        // Find the project root icon path
        let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        
        // Path: server -> crimson/src-tauri/icons/icon.ico
        let icon_path = manifest_dir.join("..").join("crimson").join("src-tauri").join("icons").join("icon.ico");

        if icon_path.exists() {
            res.set_icon(icon_path.to_str().unwrap());
        } else {
            println!("cargo:warning=Icon not found at {:?}", icon_path);
        }

        // U+2019 and not a plain ASCII apostrophe: winres escapes `'` as `\'` when it
        // writes resource.rc, but RC has no such escape, so the backslash would end up
        // verbatim in the binary and Task Manager would show `Server\'s`.
        res.set("FileDescription", "Server\u{2019}s");
        res.set("ProductName", "Server\u{2019}s");
        res.set("OriginalFilename", "crimson-server.exe");
        res.compile().unwrap();
    }
}
