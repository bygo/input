use tauri::Manager;

use super::focus::arm_ignore_unfocus;
use super::state::{overlay_is_open, CursorAnchor};
use super::window::{dismiss_overlay, place_window, reveal_overlay};

#[tauri::command]
pub fn reveal(app: tauri::AppHandle) {
    arm_ignore_unfocus(&app);
    reveal_overlay(&app);
}

#[tauri::command]
pub fn reposition(app: tauri::AppHandle) {
    if !overlay_is_open(&app) {
        return;
    }
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    let cursor = app
        .try_state::<CursorAnchor>()
        .and_then(|anchor| *anchor.0.lock().unwrap());
    if let Some(anchor) = cursor {
        place_window(&win, anchor);
    }
}

#[tauri::command]
pub fn dismiss(app: tauri::AppHandle) {
    dismiss_overlay(&app, true);
}
