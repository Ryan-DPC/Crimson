pub mod ws;
pub mod lcu_ws;
pub mod service;
pub mod state;
// Lite modules to avoid lcu_commands dependency
pub mod lcu;
pub mod storage;
pub mod automation;
pub mod events;
pub mod sd_commands;
pub mod spotify;
pub mod streamdock;
pub mod proxy;
pub mod discord;
pub mod db;
pub mod hotkeys;
pub mod updater;
pub mod hue;
pub mod twitch;
pub mod process_scanner;
pub mod auth;
pub mod entitlement;

/// Whether the League of Legends plugin should be enabled by default.
///
/// LoL cannot run on Linux (Riot's Vanguard anti-cheat blocks it), so the
/// plugin defaults off there. Windows and macOS — where the client runs — keep
/// it enabled by default. A user's stored config still overrides this.
pub fn default_lol_enabled() -> bool {
    !cfg!(target_os = "linux")
}

#[cfg(test)]
mod default_plugin_tests {
    #[test]
    fn lol_default_matches_platform() {
        let enabled = crate::default_lol_enabled();
        #[cfg(target_os = "linux")]
        assert!(!enabled, "LoL must NOT be a default plugin on Linux");
        #[cfg(not(target_os = "linux"))]
        assert!(enabled, "LoL defaults on where the client can run");
    }
}
