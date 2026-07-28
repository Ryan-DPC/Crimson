use rdev::{listen, Event, EventType, Key};
use tokio::sync::mpsc;
use std::sync::Arc;
use crate::events::WsSender;
use crate::spotify::SpotifyService;
use crate::discord::DiscordService;
use std::collections::HashSet;
use tokio::sync::Mutex;

pub struct HotkeyManager {
    spotify: Arc<SpotifyService>,
    discord: Arc<DiscordService>,
    pressed_keys: Arc<Mutex<HashSet<Key>>>,
}

impl HotkeyManager {
    pub fn new(_sender: WsSender, spotify: Arc<SpotifyService>, discord: Arc<DiscordService>) -> Self {
        Self { 
            spotify, 
            discord,
            pressed_keys: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn start_listening(&self) {
        let (tx, mut rx) = mpsc::channel::<Event>(100);
        
        // rdev::listen is blocking, so we run it in a separate thread
        std::thread::spawn(move || {
            if let Err(error) = listen(move |event| {
                match event.event_type {
                    EventType::KeyPress(_) | EventType::KeyRelease(_) => {
                        let _ = tx.try_send(event);
                    }
                    _ => {}
                }
            }) {
                eprintln!("Hotkey Error: {:?}", error);
            }
        });

        println!("Hotkey Manager: Global listener active.");
        
        let s_clone = self.spotify.clone();
        let d_clone = self.discord.clone();
        let keys_clone = self.pressed_keys.clone();

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        let mut keys = keys_clone.lock().await;
                        if !keys.contains(&key) {
                            keys.insert(key);
                            
                            // Check for combos
                            let ctrl = keys.contains(&Key::ControlLeft) || keys.contains(&Key::ControlRight);
                            let alt = keys.contains(&Key::Alt) || keys.contains(&Key::AltGr);
                            let _shift = keys.contains(&Key::ShiftLeft) || keys.contains(&Key::ShiftRight);

                             match key {
                                Key::F9 if ctrl && alt => {
                                    if s_clone.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                        crate::storage::log_to_file("hotkeys.log", "Triggered: CTRL+ALT+F9 (Spotify Next)");
                                        let s = s_clone.clone();
                                        tokio::spawn(async move { let _ = s.handle_command("next", None).await; });
                                    }
                                }
                                Key::F8 if ctrl && alt => {
                                    if s_clone.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                        crate::storage::log_to_file("hotkeys.log", "Triggered: CTRL+ALT+F8 (Spotify Prev)");
                                        let s = s_clone.clone();
                                        tokio::spawn(async move { let _ = s.handle_command("previous", None).await; });
                                    }
                                }
                                Key::F7 if ctrl && alt => {
                                    if s_clone.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                        crate::storage::log_to_file("hotkeys.log", "Triggered: CTRL+ALT+F7 (Spotify Play/Pause)");
                                        let s = s_clone.clone();
                                        tokio::spawn(async move { let _ = s.handle_command("playpause", None).await; });
                                    }
                                }
                                Key::F6 if ctrl && alt => {
                                    if d_clone.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                        crate::storage::log_to_file("hotkeys.log", "Triggered: CTRL+ALT+F6 (Discord Toggle Mute)");
                                        let d = d_clone.clone();
                                        tokio::spawn(async move { let _ = d.handle_command("toggleMute", None).await; });
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    EventType::KeyRelease(key) => {
                        let mut keys = keys_clone.lock().await;
                        keys.remove(&key);
                    }
                    _ => {}
                }
            }
        });
    }
}
