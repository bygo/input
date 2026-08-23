use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition};

use crate::paste;

/// Compose panel 360 + shadow padding (p-3 × 2).
pub const OVERLAY_W: f64 = 384.0;
/// Input + separator + one-line result + shadow padding (p-3 × 2).
pub const OVERLAY_H: f64 = 105.0;
const PARK_X: f64 = -16384.0;
const PARK_Y: f64 = -16384.0;

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
/// 定格图 + 屏幕原点（CSS points），供框选坐标映射回屏幕。
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

pub fn arm_ignore_unfocus(app: &tauri::AppHandle) {
    if let Some(s) = app.try_state::<IgnoreUnfocusUntil>() {
        // 超时仅兜底；主窗重新获焦时 clear_ignore_unfocus 会提前撤防。
        *s.0.lock().unwrap() = Some(Instant::now() + Duration::from_millis(500));
    }
}

/// 主窗已获焦：把剩余保护窗缩短到 80ms，吸收紧随其后的虚假失焦。
pub fn clear_ignore_unfocus(app: &tauri::AppHandle) {
    if let Some(s) = app.try_state::<IgnoreUnfocusUntil>() {
        let mut g = s.0.lock().unwrap();
        if g.is_some() {
            *g = Some(Instant::now() + Duration::from_millis(80));
        }
    }
}

/// 面板已在前台时撤掉保护，避免「先点卡片再点外侧」才能关。
pub fn disarm_ignore_unfocus(app: &tauri::AppHandle) {
    if let Some(s) = app.try_state::<IgnoreUnfocusUntil>() {
        *s.0.lock().unwrap() = None;
    }
}

pub fn should_ignore_unfocus(app: &tauri::AppHandle) -> bool {
    if app
        .try_state::<OcrPicking>()
        .is_some_and(|s| *s.0.lock().unwrap())
    {
        return true;
    }
    app.try_state::<IgnoreUnfocusUntil>()
        .and_then(|s| *s.0.lock().unwrap())
        .is_some_and(|until| Instant::now() < until)
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

#[cfg(target_os = "macos")]
fn ns_window(win: &tauri::WebviewWindow) -> Option<*mut objc2::runtime::AnyObject> {
    let ns = win.ns_window().ok()?;
    if ns.is_null() {
        None
    } else {
        Some(ns as *mut objc2::runtime::AnyObject)
    }
}

#[cfg(target_os = "macos")]
pub fn set_alpha(win: &tauri::WebviewWindow, alpha: f64) {
    use objc2::msg_send;
    if let Some(w) = ns_window(win) {
        unsafe {
            let _: () = msg_send![w, setAlphaValue: alpha];
        }
    }
}

#[cfg(target_os = "macos")]
fn order_front_without_key(win: &tauri::WebviewWindow) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    if let Some(w) = ns_window(win) {
        unsafe {
            let nil: *mut AnyObject = std::ptr::null_mut();
            let _: () = msg_send![w, orderFront: nil];
        }
    }
}

pub fn park_window(win: &tauri::WebviewWindow) {
    let _ = win.set_ignore_cursor_events(true);
    #[cfg(target_os = "macos")]
    {
        set_alpha(win, 0.0);
        let _ = win.set_position(PhysicalPosition::new(PARK_X, PARK_Y));
        return;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = win.set_position(PhysicalPosition::new(PARK_X, PARK_Y));
        let _ = win.hide();
    }
}

pub fn appear_window(win: &tauri::WebviewWindow) {
    let _ = win.set_ignore_cursor_events(false);
    #[cfg(target_os = "macos")]
    set_alpha(win, 1.0);
    let _ = win.show();
    let _ = win.set_focus();
}

pub fn warmup_window(win: &tauri::WebviewWindow) {
    let _ = win.set_size(LogicalSize::new(OVERLAY_W, OVERLAY_H));
    park_window(win);
    #[cfg(target_os = "macos")]
    {
        order_front_without_key(win);
        park_window(win);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = win.show();
        park_window(win);
    }
}

pub fn dismiss_overlay(app: &tauri::AppHandle, restore_focus: bool) {
    set_overlay_open(app, false);
    set_ocr_picking(app, false);
    clear_ocr_freeze(app);
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.emit("overlay-hidden", ());
        let _ = win.set_fullscreen(false);
        let _ = win.set_size(LogicalSize::new(OVERLAY_W, OVERLAY_H));
        park_window(&win);
    }
    if restore_focus {
        paste::activate_remembered(app);
    }
}

pub fn reveal_overlay(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    set_overlay_open(app, true);
    if let Some(pos) = capture_anchor(app) {
        place_window(&win, pos);
    }
    appear_window(&win);
}

#[tauri::command]
pub fn reveal(app: tauri::AppHandle) {
    arm_ignore_unfocus(&app);
    reveal_overlay(&app);
}

pub fn place_window(win: &tauri::WebviewWindow, anchor: Anchor) {
    let scale = win.scale_factor().unwrap_or(1.0);
    let size = win.outer_size().ok();
    let (pw, ph) = match size {
        Some(s) => (s.width as f64, s.height as f64),
        None => (OVERLAY_W * scale, OVERLAY_H * scale),
    };
    let gap = 4.0 * scale;
    let pad = 8.0 * scale;
    let mut x = anchor.x;
    let mut y = anchor.y + anchor.h + gap;

    if let Ok(monitors) = win.available_monitors() {
        let mon = monitors.iter().find(|m| {
            let p = m.position();
            let s = m.size();
            let x0 = p.x as f64;
            let y0 = p.y as f64;
            anchor.x >= x0
                && anchor.x < x0 + s.width as f64
                && anchor.y >= y0
                && anchor.y < y0 + s.height as f64
        });
        let mon = mon.or(monitors.first());
        if let Some(m) = mon {
            let p = m.position();
            let s = m.size();
            let left = p.x as f64 + pad;
            let top = p.y as f64 + pad;
            let right = p.x as f64 + s.width as f64 - pad;
            let bottom = p.y as f64 + s.height as f64 - pad;
            if x + pw > right {
                x = (anchor.x + anchor.w - pw).min(anchor.x);
            }
            if y + ph > bottom {
                y = anchor.y - gap - ph;
            }
            x = x.clamp(left, (right - pw).max(left));
            y = y.clamp(top, (bottom - ph).max(top));
        }
    }

    let _ = win.set_position(PhysicalPosition::new(x, y));
}

pub fn capture_anchor(app: &tauri::AppHandle) -> Option<Anchor> {
    let pos = app.cursor_position().ok()?;
    let anchor = Anchor {
        x: pos.x,
        y: pos.y,
        w: 0.0,
        h: 0.0,
    };
    if let Some(slot) = app.try_state::<CursorAnchor>() {
        *slot.0.lock().unwrap() = Some(anchor);
    }
    Some(anchor)
}

pub fn show_overlay(app: &tauri::AppHandle, win: &tauri::WebviewWindow, kind: OverlayKind) {
    set_ocr_picking(app, false);
    clear_ocr_freeze(app);
    match kind {
        OverlayKind::Settings => {}
        OverlayKind::Compose => {
            paste::remember(app, false);
            let _ = win.set_size(LogicalSize::new(OVERLAY_W, OVERLAY_H));
        }
        OverlayKind::Select => {
            paste::remember(app, true);
            let _ = win.set_size(LogicalSize::new(OVERLAY_W, OVERLAY_H));
        }
    }
    if let Some(pos) = capture_anchor(app) {
        place_window(win, pos);
    }
    arm_ignore_unfocus(app);
    set_overlay_open(app, true);
    appear_window(win);
    match kind {
        OverlayKind::Settings => {
            let _ = win.emit("open-settings", ());
        }
        OverlayKind::Compose => {
            let _ = win.emit(
                "overlay-shown",
                OverlayShown {
                    selected: String::new(),
                },
            );
        }
        OverlayKind::Select => {
            let selected = paste::remembered_selected_text(app).unwrap_or_default();
            let _ = win.emit("overlay-shown", OverlayShown { selected });
        }
    }
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
