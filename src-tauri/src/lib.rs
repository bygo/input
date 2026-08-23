mod capture;
mod dict;
mod hotkeys;
mod lang;
mod llm;
mod llm_parse;
mod llm_prompt;
mod local;
mod ocr;
mod ocr_flow;
mod overlay;
mod paste;
mod settings;

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

use hotkeys::register_hotkeys;
use overlay::{
    dismiss_overlay, should_ignore_unfocus, show_overlay, warmup_window, CursorAnchor,
    IgnoreUnfocusUntil, OcrFreeze, OcrPicking, OverlayKind, OverlayOpen, ShortcutCapture,
};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(CursorAnchor(Mutex::new(None)))
        .manage(OverlayOpen(Mutex::new(false)))
        .manage(ShortcutCapture(Mutex::new(false)))
        .manage(IgnoreUnfocusUntil(Mutex::new(None)))
        .manage(OcrPicking(Mutex::new(false)))
        .manage(OcrFreeze(Mutex::new(None)))
        .manage(capture::managed())
        .manage(paste::managed())
        .manage(local::managed())
        .manage(dict::managed())
        .invoke_handler(tauri::generate_handler![
            llm::analyze,
            paste::paste_back,
            local::local_model_status,
            local::install_local_model,
            local::set_local_model,
            settings::load_settings,
            settings::set_target_language,
            settings::set_native_language,
            hotkeys::set_shortcut,
            hotkeys::begin_shortcut_capture,
            hotkeys::end_shortcut_capture,
            ocr_flow::ocr_finish_pick,
            ocr_flow::ocr_cancel_pick,
            ocr_flow::ocr_show_pick,
            overlay::reveal,
            overlay::reposition,
            overlay::dismiss,
            dict::dict_lookup,
            dict::dict_speak
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
            if let Some(win) = app.get_webview_window("main") {
                warmup_window(&win);
            }
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            dict::warm_tts();
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
