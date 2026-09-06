use arboard::Clipboard;
use tauri::{AppHandle, Manager};

use super::keys;
use super::os;

#[cfg(target_os = "macos")]
use super::ax;

#[cfg(target_os = "macos")]
pub(super) type Handle = i32;

#[cfg(target_os = "windows")]
pub(super) type Handle = isize;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) type Handle = ();

pub(super) struct Saved {
    pub handle: Option<Handle>,
    pub clipboard: Option<String>,
    #[cfg(target_os = "macos")]
    pub ax: Option<ax::Elem>,
    pub selected: Option<String>,
}

pub struct PasteTarget(pub(super) std::sync::Mutex<Option<Saved>>);

pub fn managed() -> PasteTarget {
    PasteTarget(std::sync::Mutex::new(None))
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct InputRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub physical: bool,
}

impl InputRect {
    #[cfg(target_os = "macos")]
    pub(crate) fn from_ax(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width: width.max(1.0),
            height: height.max(18.0),
            physical: false,
        }
    }
}

#[allow(dead_code)]
pub fn focused_input_rect() -> Option<InputRect> {
    #[cfg(target_os = "macos")]
    {
        ax::Elem::focused().and_then(|el| el.input_rect())
    }
    #[cfg(target_os = "windows")]
    {
        os::caret_rect()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

pub fn remember(app: &AppHandle, capture_selection: bool) {
    let clipboard = Clipboard::new().ok().and_then(|mut c| c.get_text().ok());
    #[cfg(target_os = "macos")]
    let ax = ax::Elem::focused();
    #[cfg(target_os = "macos")]
    let selected = if capture_selection {
        ax.as_ref()
            .and_then(|e| e.capture_selected())
            .or_else(keys::copy_selected_via_clipboard)
    } else {
        None
    };
    #[cfg(not(target_os = "macos"))]
    let selected = if capture_selection {
        keys::copy_selected_via_clipboard()
    } else {
        None
    };
    let saved = Saved {
        handle: os::frontmost(),
        clipboard,
        #[cfg(target_os = "macos")]
        ax,
        selected,
    };
    if let Some(slot) = app.try_state::<PasteTarget>() {
        *slot.0.lock().unwrap() = Some(saved);
    }
}

pub fn remembered_selected_text(app: &AppHandle) -> Option<String> {
    app.try_state::<PasteTarget>()
        .and_then(|slot| slot.0.lock().unwrap().as_ref()?.selected.clone())
}

pub fn activate_remembered(app: &AppHandle) {
    let handle = app
        .try_state::<PasteTarget>()
        .and_then(|slot| slot.0.lock().unwrap().as_ref()?.handle);
    if let Some(handle) = handle {
        os::activate(handle);
    }
}
