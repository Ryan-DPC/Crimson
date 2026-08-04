# État des lieux — tracker des problèmes connus (Crimsons v3.1.4)

| | |
| --- | --- |
| **Version suivie** | 3.1.4 |
| **Identifiant canonique** | `com.laoy.crimsons` (AppData + `tauri.conf.json`) |
| **Dernier audit doc** | 2026-08-01 (post-fix consolidation) |
| **Rôle de ce fichier** | Liste honnête des bugs / dettes encore ouverts. Ne pas marquer « corrigé » sans vérification dans le code ou un test manuel. |

Les chemins monorepo ci-dessous remplacent l’ancienne référence au lecteur `F:`.

---

## Corrigé dans le code (à valider manuellement où indiqué)

Ces points sont **présents dans le working tree** au 2026-08-01. Ce n’est pas une validation produit runtime sauf mention contraire.

### Auth WebSocket + CSP
* **Strict auth ON par défaut** (`server/src/auth.rs`) — `CRIMSON_STRICT_AUTH=0` / `false` / `off` pour désactiver.
* Plugins StreamDock (Crimson, Spotify, Discord, …) lisent `%APPDATA%\com.laoy.crimsons\auth.token` et passent `?token=`.
* Property Inspectors HTML **ne** ouvrent plus de WS non authentifié vers `:40510` (données PI via bridge StreamDeck).
* CSP Tauri non-null (`crimson/src-tauri/tauri.conf.json`).
* Capability sidecar alignée : `bin/crimson-server`.

### LCU auto-accept / pick-ban
* Ready-check sur état LCU **`InProgress`** (`server/src/automation.rs` + `service.rs`) ; tests unitaires ajoutés.
* Sync `AtomicBool` ↔ `data.json` pour l’auto-accept (évite les courses UI ↔ boucle).
* Une seule boucle automation : sidecar ; `lcu_commands::automation` est un no-op volontaire.
* **Statut produit :** corrigé dans le code — **test manuel LoL encore requis** avant de clôturer côté utilisateur.

### Spotify
* Secrets retirés de `localStorage` / query d’authorization (persistés via `data.json` / sidecar).
* Cycle shuffle Off → Standard → Smart (flag local) → Off ; **l’API Spotify ne peut pas activer Smart Shuffle** (limitation amont, documentée dans le code).
* Déduplication d’images StreamDock (`push_image_if_changed` / refus des `setImage` vides) pour limiter le flash du logo générique.

### Identité AppData
* Canonique : **`com.laoy.crimsons`**.
* Migration au démarrage Tauri depuis `com.laoy.crimson` et le typo `com.laoy.crimons`.

### Hue / Twitch
* Gated « coming soon » / `FEATURE_UNAVAILABLE` côté serveur ; UI Settings affiche « (Soon) ».

### CI + doc
* `.github/workflows/ci.yml` : ESLint (`continue-on-error`), `tsc -b`, Clippy + `cargo test` (Windows) pour `crimson-server` / `lcu_commands`.
* README racine présent.

---

## Problèmes / limitations encore ouverts

### 1. Smart Shuffle — limitation API Spotify (pas un bug Crimson)
* **Symptôme :** le 3ᵉ état « Smart » est un flag UX local ; Spotify Web API ne propose que shuffle on/off.
* **Statut :** cycle UI corrigé ; **impossible d’activer réellement Smart Shuffle** via l’API.

### 2. Sécurité locale résiduelle — vol de jeton AppData
* **Symptôme :** tout process du même user Windows peut lire `auth.token` / session Supabase sur disque.
* **Statut :** attendu pour un serveur local ; le mode strict bloque les clients **sans** jeton, pas un attaquant local qui lit le fichier.

### 3. Frontend — interface et CSS
* **Symptôme :** superpositions / manque d’air (ex. Auto Selection vs grille de champions).
* **Statut :** Ouvert (non traité dans cette vague de fixes).

### 4. LCU — validation manuelle
* Auto-accept / pick-ban : logique corrigée + tests unitaires, **pas encore confirmé en client LoL réel**.

---

## Dette outillage (hors produit)

* ESLint frontend : ~100 erreurs historiques (surtout `@typescript-eslint/no-explicit-any`) — job CI **visible** mais `continue-on-error: true` (ne bloque pas la PR).
* Clippy : warnings existants (ex. `dead_code` Discord) — Clippy sans `-D warnings`.
* Couverture tests encore faible hors Origin WS / automation ready-check / entitlements ponctuels.
