use crate::capture;
use crate::overlay::set_capturing_shortcut;
use crate::settings;

use super::register::register_hotkeys;

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
    let mut s = settings::commands::load_settings();
    let prev = s.clone();
    match kind.as_str() {
        "compose" => {
            if !next.is_empty()
                && (settings::same_shortcut(&next, &s.select_shortcut)
                    || settings::same_shortcut(&next, &s.ocr_shortcut))
            {
                return Err("与其它快捷键相同".into());
            }
            s.compose_shortcut = next;
        }
        "select" => {
            if !next.is_empty()
                && (settings::same_shortcut(&next, &s.compose_shortcut)
                    || settings::same_shortcut(&next, &s.ocr_shortcut))
            {
                return Err("与其它快捷键相同".into());
            }
            s.select_shortcut = next;
        }
        "ocr" => {
            if !next.is_empty()
                && (settings::same_shortcut(&next, &s.compose_shortcut)
                    || settings::same_shortcut(&next, &s.select_shortcut))
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
