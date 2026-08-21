mod capture;
mod lang;
mod llm;
mod local;
mod paste;
mod settings;

use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, LogicalSize, Manager, PhysicalPosition, WindowEvent,
};
#[cfg(not(target_os = "macos"))]
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const OVERLAY_W: f64 = 440.0;
const OVERLAY_H: f64 = 40.0;
const PARK_X: f64 = -16384.0;
const PARK_Y: f64 = -16384.0;

#[derive(Clone, serde::Serialize)]
struct OverlayShown {
    selected: String,
}

#[derive(Clone, Copy, Debug)]
struct Anchor {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

struct CursorAnchor(Mutex<Option<Anchor>>);

struct OverlayOpen(Mutex<bool>);

struct ShortcutCapture(Mutex<bool>);

struct IgnoreUnfocusUntil(Mutex<Option<Instant>>);

fn overlay_is_open(app: &tauri::AppHandle) -> bool {
    app.try_state::<OverlayOpen>()
        .map(|s| *s.0.lock().unwrap())
        .unwrap_or(false)
}

pub(crate) fn capturing_shortcut(app: &tauri::AppHandle) -> bool {
    app.try_state::<ShortcutCapture>()
        .map(|s| *s.0.lock().unwrap())
        .unwrap_or(false)
}

fn set_capturing_shortcut(app: &tauri::AppHandle, on: bool) {
    if let Some(s) = app.try_state::<ShortcutCapture>() {
        *s.0.lock().unwrap() = on;
    }
}

fn set_overlay_open(app: &tauri::AppHandle, open: bool) {
    if let Some(s) = app.try_state::<OverlayOpen>() {
        *s.0.lock().unwrap() = open;
    }
}

fn arm_ignore_unfocus(app: &tauri::AppHandle) {
    if let Some(s) = app.try_state::<IgnoreUnfocusUntil>() {
        *s.0.lock().unwrap() = Some(Instant::now() + Duration::from_millis(500));
    }
}

fn should_ignore_unfocus(app: &tauri::AppHandle) -> bool {
    app.try_state::<IgnoreUnfocusUntil>()
        .and_then(|s| *s.0.lock().unwrap())
        .is_some_and(|until| Instant::now() < until)
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
fn set_alpha(win: &tauri::WebviewWindow, alpha: f64) {
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

fn park_window(win: &tauri::WebviewWindow) {
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

fn appear_window(win: &tauri::WebviewWindow) {
    let _ = win.set_ignore_cursor_events(false);
    #[cfg(target_os = "macos")]
    set_alpha(win, 1.0);
    let _ = win.show();
    let _ = win.set_focus();
}

fn warmup_window(win: &tauri::WebviewWindow) {
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

pub(crate) fn dismiss_overlay(app: &tauri::AppHandle, restore_focus: bool) {
    set_overlay_open(app, false);
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.emit("overlay-hidden", ());
        park_window(&win);
    }
    if restore_focus {
        paste::activate_remembered(app);
    }
}

pub(crate) fn reveal_overlay(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    set_overlay_open(app, true);
    appear_window(&win);
}

fn place_window(win: &tauri::WebviewWindow, anchor: Anchor) {
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

fn capture_anchor(app: &tauri::AppHandle) -> Option<Anchor> {
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

enum OverlayKind {
    Settings,
    Compose,
    Select,
}

fn show_overlay(app: &tauri::AppHandle, win: &tauri::WebviewWindow, kind: OverlayKind) {
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

pub(crate) fn dispatch_hotkey(app: &tauri::AppHandle, which: u8) {
    if capturing_shortcut(app) {
        return;
    }
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    if let Ok(mut g) = LAST.lock() {
        if g.is_some_and(|t| t.elapsed() < Duration::from_millis(250)) {
            return;
        }
        *g = Some(Instant::now());
    }
    let kind = if which == 1 {
        OverlayKind::Select
    } else {
        OverlayKind::Compose
    };
    toggle_kind(app, kind);
}

fn toggle_kind(app: &tauri::AppHandle, kind: OverlayKind) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if overlay_is_open(app) {
        dismiss_overlay(app, true);
    } else {
        show_overlay(app, &win, kind);
    }
}

fn register_hotkeys(app: &tauri::AppHandle) -> Result<(), String> {
    let s = settings::load_settings();
    let compose = settings::normalize_shortcut(&s.compose_shortcut)
        .unwrap_or_else(|_| settings::default_compose_shortcut());
    let select = settings::normalize_shortcut(&s.select_shortcut)
        .unwrap_or_else(|_| settings::default_select_shortcut());
    if settings::same_shortcut(&compose, &select) {
        return Err("两个快捷键不能相同".into());
    }
    capture::set_bindings(app, compose.clone(), select.clone());
    #[cfg(not(target_os = "macos"))]
    {
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
    }
    Ok(())
}

#[tauri::command]
fn reposition(app: tauri::AppHandle) {
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
fn dismiss(app: tauri::AppHandle) {
    dismiss_overlay(&app, true);
}

#[tauri::command]
fn begin_shortcut_capture(app: tauri::AppHandle) {
    set_capturing_shortcut(&app, true);
    capture::start(&app);
}

#[tauri::command]
fn end_shortcut_capture(app: tauri::AppHandle) {
    capture::stop(&app);
    set_capturing_shortcut(&app, false);
}

#[tauri::command]
fn set_shortcut(
    app: tauri::AppHandle,
    kind: String,
    shortcut: String,
) -> Result<settings::Settings, String> {
    let next = settings::normalize_shortcut(&shortcut)?;
    let mut s = settings::load_settings();
    let prev = s.clone();
    match kind.as_str() {
        "compose" => {
            if settings::same_shortcut(&next, &s.select_shortcut) {
                return Err("与选中翻译快捷键相同".into());
            }
            s.compose_shortcut = next;
        }
        "select" => {
            if settings::same_shortcut(&next, &s.compose_shortcut) {
                return Err("与输入快捷键相同".into());
            }
            s.select_shortcut = next;
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

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(CursorAnchor(Mutex::new(None)))
        .manage(OverlayOpen(Mutex::new(false)))
        .manage(ShortcutCapture(Mutex::new(false)))
        .manage(IgnoreUnfocusUntil(Mutex::new(None)))
        .manage(capture::managed())
        .manage(capture::managed_bindings())
        .manage(paste::managed())
        .manage(local::managed())
        .invoke_handler(tauri::generate_handler![
            llm::analyze,
            paste::paste_back,
            local::local_model_status,
            local::install_local_model,
            local::set_local_model,
            settings::load_settings,
            settings::set_target_language,
            settings::set_native_language,
            set_shortcut,
            begin_shortcut_capture,
            end_shortcut_capture,
            reposition,
            dismiss
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_item, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            show_overlay(app, &win, OverlayKind::Settings);
                        }
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => {
                        if let Some(win) = app.get_webview_window("main") {
                            show_overlay(app, &win, OverlayKind::Settings);
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            register_hotkeys(app.handle())?;
            capture::listen(app.handle());
            if let Some(win) = app.get_webview_window("main") {
                warmup_window(&win);
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                local::boot(handle).await;
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                if should_ignore_unfocus(window.app_handle()) {
                    return;
                }
                dismiss_overlay(window.app_handle(), false);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
