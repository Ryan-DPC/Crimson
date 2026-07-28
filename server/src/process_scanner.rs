use sysinfo::{System, ProcessRefreshKind, RefreshKind, UpdateKind};
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    pub static ref GLOBAL_SYSTEM: Arc<Mutex<System>> = Arc::new(Mutex::new(System::new_with_specifics(
        RefreshKind::new().with_processes(
            ProcessRefreshKind::new()
                .with_cmd(UpdateKind::Always)
                .with_exe(UpdateKind::Always)
        )
    )));
}

pub async fn start_process_scanner() {
    // Perform initial sync refresh first to avoid race conditions on startup
    let sys = GLOBAL_SYSTEM.clone();
    let _ = tokio::task::spawn_blocking(move || {
        if let Ok(mut s) = sys.lock() {
            s.refresh_processes_specifics(
                ProcessRefreshKind::new()
                    .with_cmd(UpdateKind::Always)
                    .with_exe(UpdateKind::Always)
            );
        }
    }).await;

    // Then spawn the periodic polling loop
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let sys = GLOBAL_SYSTEM.clone();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(mut s) = sys.lock() {
                    s.refresh_processes_specifics(
                        ProcessRefreshKind::new()
                            .with_cmd(UpdateKind::Always)
                            .with_exe(UpdateKind::Always)
                    );
                }
            }).await;
        }
    });
}
