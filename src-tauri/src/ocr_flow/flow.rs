use tauri::{Emitter, Manager, PhysicalPosition, PhysicalSize};

use crate::ocr;
use crate::overlay::{
    appear_window, arm_ignore_unfocus, clear_ocr_freeze, park_window, set_ocr_freeze,
    set_ocr_picking, set_overlay_open, OcrFreezeFrame,
};

#[derive(Clone, serde::Serialize)]
pub struct OcrPickStart {

    pub path: String,
}

#[derive(Clone, serde::Serialize)]
pub struct OcrRecognizing {

    pub preview: String,
}

pub fn start_ocr_capture(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if let Err(e) = ocr::ensure_screen_permission() {
        set_overlay_open(app, true);
        appear_window(&win);
        let _ = win.emit("ocr-error", e);
        return;
    }
    let monitor = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        let _ = win.emit("ocr-error", "无法获取屏幕信息".to_string());
        return;
    };
    let size = monitor.size();
    let pos = monitor.position();
    let scale = monitor.scale_factor();
    let mon_w = size.width;
    let mon_h = size.height;
    let mon_x = pos.x;
    let mon_y = pos.y;
    let lx = mon_x as f64 / scale;
    let ly = mon_y as f64 / scale;
    let lw = mon_w as f64 / scale;
    let lh = mon_h as f64 / scale;

    park_window(&win);
    set_ocr_picking(app, true);
    let app2 = app.clone();
    tauri::async_runtime::spawn(async move {
        let freeze_path =
            tokio::task::spawn_blocking(move || ocr::capture_screen_rect(lx, ly, lw, lh))
                .await
                .map_err(|e| e.to_string());
        let freeze_path = match freeze_path {
            Ok(Ok(p)) => p,
            Ok(Err(e)) | Err(e) => {
                let msg = e;
                let handle = app2.clone();
                let _ = app2.run_on_main_thread(move || {
                    set_ocr_picking(&handle, false);
                    let Some(win) = handle.get_webview_window("main") else {
                        return;
                    };
                    set_overlay_open(&handle, true);
                    appear_window(&win);
                    let _ = win.emit("ocr-error", msg);
                });
                return;
            }
        };
        let path_str = freeze_path.to_string_lossy().to_string();
        let handle = app2.clone();
        let _ = app2.run_on_main_thread(move || {
            set_ocr_freeze(
                &handle,
                Some(OcrFreezeFrame {
                    path: freeze_path,
                    origin_x: lx,
                    origin_y: ly,
                }),
            );
            let Some(win) = handle.get_webview_window("main") else {
                clear_ocr_freeze(&handle);
                set_ocr_picking(&handle, false);
                return;
            };
            set_overlay_open(&handle, true);
            arm_ignore_unfocus(&handle);
            let _ = win.set_fullscreen(false);
            let _ = win.set_ignore_cursor_events(true);
            let _ = win.set_size(PhysicalSize::new(mon_w, mon_h));
            let _ = win.set_position(PhysicalPosition::new(mon_x, mon_y));
            #[cfg(target_os = "macos")]
            {
                crate::overlay::set_alpha(&win, 0.0);
                let _ = win.show();
            }
            #[cfg(not(target_os = "macos"))]
            {
                let _ = win.hide();
            }
            let _ = win.emit("ocr-pick-start", OcrPickStart { path: path_str });
        });
    });
}
