mod llm;
mod settings;

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, LogicalSize, Manager, PhysicalPosition, WindowEvent,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

const OVERLAY_W: f64 = 440.0;
const OVERLAY_H: f64 = 56.0;

struct CursorAnchor(Mutex<Option<PhysicalPosition<f64>>>);

fn place_window(win: &tauri::WebviewWindow, cursor: PhysicalPosition<f64>) {
    let scale = win.scale_factor().unwrap_or(1.0);
    let size = win.outer_size().ok();
    let (pw, ph) = match size {
        Some(s) => (s.width as f64, s.height as f64),
        None => (OVERLAY_W * scale, OVERLAY_H * scale),
    };
    let gap = 8.0 * scale;
    let pad = 8.0 * scale;
    let mut x = cursor.x + gap;
    let mut y = cursor.y + gap;

    if let Ok(monitors) = win.available_monitors() {
        let mon = monitors.iter().find(|m| {
            let p = m.position();
            let s = m.size();
            let x0 = p.x as f64;
            let y0 = p.y as f64;
            cursor.x >= x0
                && cursor.x < x0 + s.width as f64
                && cursor.y >= y0
                && cursor.y < y0 + s.height as f64
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
                x = cursor.x - gap - pw;
            }
            if y + ph > bottom {
                y = cursor.y - gap - ph;
            }
            x = x.clamp(left, (right - pw).max(left));
            y = y.clamp(top, (bottom - ph).max(top));
        }
    }

    let _ = win.set_position(PhysicalPosition::new(x, y));
}

fn capture_cursor(app: &tauri::AppHandle) -> Option<PhysicalPosition<f64>> {
    let cursor = app.cursor_position().ok()?;
    if let Some(anchor) = app.try_state::<CursorAnchor>() {
        *anchor.0.lock().unwrap() = Some(cursor);
    }
    Some(cursor)
}

fn show_overlay(app: &tauri::AppHandle, win: &tauri::WebviewWindow, settings: bool) {
    let cursor = capture_cursor(app);
    let _ = win.set_size(LogicalSize::new(OVERLAY_W, OVERLAY_H));
    let _ = win.show();
    let _ = win.set_focus();
    if let Some(cursor) = cursor {
        place_window(win, cursor);
    }
    if settings {
        let _ = win.emit("open-settings", ());
    } else {
        let _ = win.emit("overlay-shown", ());
    }
}

fn toggle_window(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if win.is_visible().unwrap_or(false) {
        let _ = win.hide();
    } else {
        show_overlay(app, &win, false);
    }
}

fn show_window(app: &tauri::AppHandle, settings: bool) {
    if let Some(win) = app.get_webview_window("main") {
        show_overlay(app, &win, settings);
    }
}

pub fn register_hotkey(app: &tauri::AppHandle, shortcut: &str) -> Result<(), String> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    gs.on_shortcut(shortcut, |app, _s, event| {
        if event.state == ShortcutState::Pressed {
            toggle_window(app);
        }
    })
    .map_err(|e| format!("注册快捷键失败: {e}"))
}

#[tauri::command]
fn reposition(app: tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    let cursor = app.try_state::<CursorAnchor>().and_then(|anchor| *anchor.0.lock().unwrap());
    if let Some(cursor) = cursor {
        place_window(&win, cursor);
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(CursorAnchor(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            llm::analyze,
            llm::test_provider,
            settings::load_settings,
            settings::save_settings,
            reposition,
        ])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let show = MenuItem::with_id(app, "show", "显示", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &settings_item, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_window(app, false),
                    "settings" => show_window(app, true),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            let shortcut = settings::read()
                .map(|s| s.shortcut)
                .unwrap_or_else(|_| "CommandOrControl+G".into());
            register_hotkey(app.handle(), &shortcut)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::Focused(false) = event {
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
