use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;
use tauri::Manager;


pub const OVERLAY_W: f64 = 384.0;

pub const OVERLAY_H: f64 = 105.0;

#[derive(Clone, serde::Serialize)]
pub struct OverlayShown {
    pub selected: String,
}

#[derive(Clone, Copy, Debug)]
pub struct Anchor {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub struct CursorAnchor(pub Mutex<Option<Anchor>>);
pub struct OverlayOpen(pub Mutex<bool>);
pub struct ShortcutCapture(pub Mutex<bool>);
pub struct IgnoreUnfocusUntil(pub Mutex<Option<Instant>>);
pub struct OcrPicking(pub Mutex<bool>);

pub struct OcrFreezeFrame {
    pub path: PathBuf,
    pub origin_x: f64,
    pub origin_y: f64,
}

pub struct OcrFreeze(pub Mutex<Option<OcrFreezeFrame>>);

pub enum OverlayKind {
    Settings,
    Compose,
    Select,
}

pub fn overlay_is_open(app: &tauri::AppHandle) -> bool {
    app.try_state::<OverlayOpen>()
        .map(|s| *s.0.lock().unwrap())
        .unwrap_or(false)
}

pub fn capturing_shortcut(app: &tauri::AppHandle) -> bool {
    app.try_state::<ShortcutCapture>()
        .map(|s| *s.0.lock().unwrap())
        .unwrap_or(false)
}

pub fn set_capturing_shortcut(app: &tauri::AppHandle, on: bool) {
    if let Some(s) = app.try_state::<ShortcutCapture>() {
        *s.0.lock().unwrap() = on;
    }
}

pub fn set_overlay_open(app: &tauri::AppHandle, open: bool) {
    if let Some(s) = app.try_state::<OverlayOpen>() {
        *s.0.lock().unwrap() = open;
    }
}

pub fn set_ocr_picking(app: &tauri::AppHandle, on: bool) {
    if let Some(s) = app.try_state::<OcrPicking>() {
        *s.0.lock().unwrap() = on;
    }
}

pub fn ocr_picking(app: &tauri::AppHandle) -> bool {
    app.try_state::<OcrPicking>()
        .is_some_and(|s| *s.0.lock().unwrap())
}

pub fn take_ocr_freeze(app: &tauri::AppHandle) -> Option<OcrFreezeFrame> {
    app.try_state::<OcrFreeze>()
        .and_then(|s| s.0.lock().unwrap().take())
}

pub fn ocr_freeze_origin(app: &tauri::AppHandle) -> Option<(f64, f64)> {
    app.try_state::<OcrFreeze>().and_then(|s| {
        s.0.lock()
            .unwrap()
            .as_ref()
            .map(|f| (f.origin_x, f.origin_y))
    })
}

pub fn set_ocr_freeze(app: &tauri::AppHandle, frame: Option<OcrFreezeFrame>) {
    if let Some(prev) = take_ocr_freeze(app) {
        let _ = std::fs::remove_file(prev.path);
    }
    if let Some(s) = app.try_state::<OcrFreeze>() {
        *s.0.lock().unwrap() = frame;
    }
}

pub fn clear_ocr_freeze(app: &tauri::AppHandle) {
    set_ocr_freeze(app, None);
}
