use super::Handle;
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::time::Duration;

pub(super) fn ensure_accessibility() -> Result<(), String> {
    Ok(())
}
pub(super) fn send_paste(_pid: Option<Handle>) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| map_input_err(e.to_string()))?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| map_input_err(e.to_string()))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| map_input_err(e.to_string()))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| map_input_err(e.to_string()))?;
    Ok(())
}

pub(super) fn copy_selected_via_clipboard() -> Option<String> {
    let mut clipboard = Clipboard::new().ok()?;
    let previous = clipboard.get_text().ok();
    let mut enigo = Enigo::new(&Settings::default()).ok()?;
    enigo.key(Key::Control, Direction::Press).ok()?;
    enigo.key(Key::Unicode('c'), Direction::Click).ok()?;
    enigo.key(Key::Control, Direction::Release).ok()?;
    let deadline = std::time::Instant::now() + Duration::from_millis(120);
    let mut got = None;
    while std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(8));
        let Ok(text) = clipboard.get_text() else {
            continue;
        };
        let t = text.trim();
        if t.is_empty() {
            continue;
        }
        if previous.as_deref() == Some(t) {
            continue;
        }
        got = Some(t.to_string());
        break;
    }
    if let Some(prev) = previous {
        let _ = clipboard.set_text(&prev);
    }
    got
}

fn map_input_err(e: String) -> String {
    if e.to_lowercase().contains("access") || e.to_lowercase().contains("permission") {
        #[cfg(target_os = "macos")]
        return "无法回写：请在系统设置 › 隐私与安全性 › 辅助功能 中允许 input".into();
        #[cfg(target_os = "windows")]
        return "无法回写：模拟按键失败".into();
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        return e;
    }
    e
}
