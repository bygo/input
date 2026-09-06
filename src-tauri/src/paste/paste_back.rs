use arboard::Clipboard;
use std::time::Duration;
use tauri::{AppHandle, Manager};

use super::keys;
use super::os;
use super::state::{PasteTarget, Saved};

#[cfg(target_os = "macos")]
use super::ax;

#[tauri::command]
pub async fn paste_back(app: AppHandle, text: String) -> Result<(), String> {
    if text.is_empty() {
        return Err("没有可填入的内容".into());
    }
    keys::ensure_accessibility()?;
    let saved = app
        .try_state::<PasteTarget>()
        .and_then(|slot| slot.0.lock().unwrap().take());
    crate::overlay::dismiss_overlay(&app, false);
    tokio::time::sleep(Duration::from_millis(40)).await;
    if let Some(handle) = saved.as_ref().and_then(|s| s.handle) {
        os::activate(handle);
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    #[cfg(target_os = "macos")]
    {
        let fresh = ax::Elem::focused();
        let el = fresh
            .as_ref()
            .or_else(|| saved.as_ref().and_then(|s| s.ax.as_ref()));
        if let Some(el) = el {
            el.focus();
            let replace = saved
                .as_ref()
                .and_then(|s| s.selected.as_ref())
                .is_some_and(|t| !t.trim().is_empty());
            if el.insert_text(&text, replace) {
                return Ok(());
            }
        }
    }
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(&text).map_err(|e| e.to_string())?;
    let pid = saved.as_ref().and_then(|s| s.handle);
    if let Err(e) = keys::send_paste(pid) {
        restore_clipboard(&mut clipboard, saved.as_ref());
        if let Some(slot) = app.try_state::<PasteTarget>() {
            *slot.0.lock().unwrap() = saved;
        }
        crate::overlay::reveal_overlay(&app);
        return Err(e);
    }

    tokio::time::sleep(Duration::from_millis(50)).await;
    restore_clipboard(&mut clipboard, saved.as_ref());
    Ok(())
}

fn restore_clipboard(clipboard: &mut Clipboard, saved: Option<&Saved>) {
    if let Some(prev) = saved.and_then(|s| s.clipboard.as_ref()) {
        let _ = clipboard.set_text(prev);
    }
}
