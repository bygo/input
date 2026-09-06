use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::ocr_flow::start_ocr_capture;
use crate::overlay::{
    capturing_shortcut, dismiss_overlay, overlay_is_open, show_overlay, OverlayKind,
};
use crate::settings;

pub fn toggle_kind(app: &tauri::AppHandle, kind: OverlayKind) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if overlay_is_open(app) && matches!(kind, OverlayKind::Compose) {
        dismiss_overlay(app, true);
        return;
    }
    show_overlay(app, &win, kind);
}

pub fn register_hotkeys(app: &tauri::AppHandle) -> Result<(), String> {
    let s = settings::commands::load_settings();
    let compose = settings::normalize_shortcut(&s.compose_shortcut)?;
    let select = settings::normalize_shortcut(&s.select_shortcut)?;
    let ocr = settings::normalize_shortcut(&s.ocr_shortcut)?;
    if (!compose.is_empty() && settings::same_shortcut(&compose, &select))
        || (!compose.is_empty() && settings::same_shortcut(&compose, &ocr))
        || (!select.is_empty() && settings::same_shortcut(&select, &ocr))
    {
        return Err("快捷键不能互相相同".into());
    }
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if !compose.is_empty() {
        gs.on_shortcut(compose.as_str(), |app, _s, event| {
            if event.state != ShortcutState::Pressed || capturing_shortcut(app) {
                return;
            }
            toggle_kind(app, OverlayKind::Compose);
        })
        .map_err(|e| format!("注册输入快捷键失败: {e}"))?;
    }
    if !select.is_empty() {
        gs.on_shortcut(select.as_str(), |app, _s, event| {
            if event.state != ShortcutState::Pressed || capturing_shortcut(app) {
                return;
            }
            toggle_kind(app, OverlayKind::Select);
        })
        .map_err(|e| format!("注册选中翻译快捷键失败: {e}"))?;
    }
    if !ocr.is_empty() {
        gs.on_shortcut(ocr.as_str(), |app, _s, event| {
            if event.state != ShortcutState::Pressed || capturing_shortcut(app) {
                return;
            }
            start_ocr_capture(app);
        })
        .map_err(|e| format!("注册截屏取词快捷键失败: {e}"))?;
    }
    Ok(())
}
