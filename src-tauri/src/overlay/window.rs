use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition};

use crate::paste;

use super::focus::arm_ignore_unfocus;
use super::state::{
    clear_ocr_freeze, set_ocr_picking, set_overlay_open, Anchor, CursorAnchor, OverlayKind,
    OverlayShown, OVERLAY_H, OVERLAY_W,
};

const PARK_X: f64 = -16384.0;
const PARK_Y: f64 = -16384.0;

#[cfg(target_os = "macos")]
const COLLECTION_CAN_JOIN_ALL_SPACES: u64 = 1 << 0;
#[cfg(target_os = "macos")]
const COLLECTION_FULL_SCREEN_AUXILIARY: u64 = 1 << 8;

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

/// 输入法候选条通常在 CGShieldingWindowLevel；我们用其 -1，盖过其它应用、不盖输入法。
/// 绝不调用 set_always_on_top（会打回 floating）。
#[cfg(target_os = "macos")]
fn elevate_overlay(win: &tauri::WebviewWindow) {
    use core_graphics::display::CGShieldingWindowLevel;
    use objc2::msg_send;
    if let Some(w) = ns_window(win) {
        // NSInteger == isize；必须用 isize，否则超大 level 可能被截断成「很低」
        let shield = unsafe { CGShieldingWindowLevel() } as isize;
        let level = shield.saturating_sub(1);
        unsafe {
            let _: () = msg_send![w, setLevel: level];
            let behavior = COLLECTION_CAN_JOIN_ALL_SPACES | COLLECTION_FULL_SCREEN_AUXILIARY;
            let _: () = msg_send![w, setCollectionBehavior: behavior];
            let _: () = msg_send![w, orderFrontRegardless];
            let now: isize = msg_send![w, level];
            eprintln!("[overlay] elevate want={level} now={now} shield={shield}");
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn elevate_overlay(win: &tauri::WebviewWindow) {
    let _ = win.set_always_on_top(true);
}

/// show/focus 后 AppKit/Tauri 可能改 level，主线程延迟再钉几次。
#[cfg(target_os = "macos")]
fn elevate_overlay_sticky(win: &tauri::WebviewWindow) {
    elevate_overlay(win);
    let handle = win.app_handle().clone();
    let label = win.label().to_string();
    std::thread::spawn(move || {
        for ms in [16u64, 50, 120, 300] {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            let app = handle.clone();
            let label = label.clone();
            let _ = handle.run_on_main_thread(move || {
                if let Some(w) = app.get_webview_window(&label) {
                    elevate_overlay(&w);
                }
            });
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn elevate_overlay_sticky(win: &tauri::WebviewWindow) {
    elevate_overlay(win);
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
    elevate_overlay(win);
    let _ = win.show();
    let _ = win.set_focus();
    elevate_overlay_sticky(win);
}

pub fn warmup_window(win: &tauri::WebviewWindow) {
    let _ = win.set_size(LogicalSize::new(OVERLAY_W, OVERLAY_H));
    elevate_overlay(win);
    park_window(win);
    #[cfg(target_os = "macos")]
    {
        order_front_without_key(win);
        elevate_overlay(win);
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
            // work_area 排除 Dock / 菜单栏 / 任务栏，避免卡片被遮挡
            let wa = m.work_area();
            let left = wa.position.x as f64 + pad;
            let top = wa.position.y as f64 + pad;
            let right = wa.position.x as f64 + wa.size.width as f64 - pad;
            let bottom = wa.position.y as f64 + wa.size.height as f64 - pad;
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
    let selected = match kind {
        OverlayKind::Settings => String::new(),
        OverlayKind::Compose => {
            paste::remember(app, false);
            let _ = win.set_size(LogicalSize::new(OVERLAY_W, OVERLAY_H));
            String::new()
        }
        OverlayKind::Select => {

            paste::remember(app, true);
            let t = paste::remembered_selected_text(app).unwrap_or_default();
            if t.trim().is_empty() {
                return;
            }
            let _ = win.set_size(LogicalSize::new(OVERLAY_W, OVERLAY_H));
            t
        }
    };
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
            let _ = win.emit("overlay-shown", OverlayShown { selected });
        }
    }
}
