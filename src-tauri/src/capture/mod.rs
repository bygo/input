mod state;

#[cfg(target_os = "macos")]
mod macos;

pub use state::managed;

use tauri::AppHandle;

pub fn start(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    macos::start(app);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

pub fn stop(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    macos::stop(app);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}
