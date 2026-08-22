use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::capture;
use crate::ocr_flow::start_ocr_capture;
use crate::overlay::{
    capturing_shortcut, dismiss_overlay, overlay_is_open, set_capturing_shortcut, show_overlay,
    OverlayKind,
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
    let s = settings::load_settings();
    let compose = settings::normalize_shortcut(&s.compose_shortcut)
        .unwrap_or_else(|_| settings::default_compose_shortcut());
    let select = settings::normalize_shortcut(&s.select_shortcut)
        .unwrap_or_else(|_| settings::default_select_shortcut());
    let ocr = settings::normalize_shortcut(&s.ocr_shortcut)
        .unwrap_or_else(|_| settings::default_ocr_shortcut());
    if settings::same_shortcut(&compose, &select)
        || settings::same_shortcut(&compose, &ocr)
        || settings::same_shortcut(&select, &ocr)
    {
        return Err("快捷键不能互相相同".into());
    }
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.on_shortcut(compose.as_str(), |app, _s, event| {
        if event.state != ShortcutState::Pressed || capturing_shortcut(app) {
            return;
        }
        toggle_kind(app, OverlayKind::Compose);
    })
    .map_err(|e| format!("注册输入快捷键失败: {e}"))?;
    gs.on_shortcut(select.as_str(), |app, _s, event| {
        if event.state != ShortcutState::Pressed || capturing_shortcut(app) {
            return;
        }
        toggle_kind(app, OverlayKind::Select);
    })
    .map_err(|e| format!("注册选中翻译快捷键失败: {e}"))?;
    gs.on_shortcut(ocr.as_str(), |app, _s, event| {
        if event.state != ShortcutState::Pressed || capturing_shortcut(app) {
            return;
        }
        start_ocr_capture(app);
    })
    .map_err(|e| format!("注册截屏取词快捷键失败: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn begin_shortcut_capture(app: tauri::AppHandle) {
    set_capturing_shortcut(&app, true);
    capture::start(&app);
}

#[tauri::command]
pub fn end_shortcut_capture(app: tauri::AppHandle) {
    capture::stop(&app);
    set_capturing_shortcut(&app, false);
}

#[tauri::command]
pub fn set_shortcut(
    app: tauri::AppHandle,
    kind: String,
    shortcut: String,
) -> Result<settings::Settings, String> {
    let next = settings::normalize_shortcut(&shortcut)?;
    let mut s = settings::load_settings();
    let prev = s.clone();
    match kind.as_str() {
        "compose" => {
            if settings::same_shortcut(&next, &s.select_shortcut)
                || settings::same_shortcut(&next, &s.ocr_shortcut)
            {
                return Err("与其它快捷键相同".into());
            }
            s.compose_shortcut = next;
        }
        "select" => {
            if settings::same_shortcut(&next, &s.compose_shortcut)
                || settings::same_shortcut(&next, &s.ocr_shortcut)
            {
                return Err("与其它快捷键相同".into());
            }
            s.select_shortcut = next;
        }
        "ocr" => {
            if settings::same_shortcut(&next, &s.compose_shortcut)
                || settings::same_shortcut(&next, &s.select_shortcut)
            {
                return Err("与其它快捷键相同".into());
            }
            s.ocr_shortcut = next;
        }
        _ => return Err("未知快捷键类型".into()),
    }
    settings::write(&s)?;
    if let Err(e) = register_hotkeys(&app) {
        let _ = settings::write(&prev);
        let _ = register_hotkeys(&app);
        capture::stop(&app);
        set_capturing_shortcut(&app, false);
        return Err(e);
    }
    capture::stop(&app);
    set_capturing_shortcut(&app, false);
    Ok(s)
}
