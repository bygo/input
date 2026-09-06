use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize};

use crate::ocr;
use crate::overlay::{
    appear_window, arm_ignore_unfocus, clear_ocr_freeze, disarm_ignore_unfocus, dismiss_overlay,
    ocr_picking, set_ocr_picking, set_overlay_open, show_overlay, take_ocr_freeze, OverlayKind,
    OVERLAY_H, OVERLAY_W,
};
use crate::paste;

use super::flow::OcrRecognizing;

#[tauri::command]
pub fn ocr_show_pick(app: tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        return;
    };
    if !ocr_picking(&app) {
        return;
    }
    if let Some(monitor) = win
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())
    {
        let size = monitor.size();
        let pos = monitor.position();
        let _ = win.set_fullscreen(false);
        let _ = win.set_size(PhysicalSize::new(size.width, size.height));
        let _ = win.set_position(PhysicalPosition::new(pos.x, pos.y));
    }
    appear_window(&win);
}

#[tauri::command]
pub async fn ocr_finish_pick(
    app: tauri::AppHandle,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> Result<(), String> {

    let frame = take_ocr_freeze(&app).ok_or_else(|| "没有定格图".to_string())?;
    let freeze_path = frame.path;
    let _ = (frame.origin_x, frame.origin_y);

    let freeze_for_crop = freeze_path.clone();
    let crop = tokio::task::spawn_blocking(move || {
        ocr::crop_freeze_jpeg(
            &freeze_for_crop,
            x.max(0.0),
            y.max(0.0),
            w.max(0.0),
            h.max(0.0),
        )
    })
    .await
    .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&freeze_path);

    let path = match crop {
        Ok(p) => p,
        Err(e) => {
            set_ocr_picking(&app, false);
            let msg = e.clone();
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                let Some(win) = app2.get_webview_window("main") else {
                    return;
                };
                let _ = win.emit("ocr-pick-end", ());
                show_overlay(&app2, &win, OverlayKind::Compose);
                let _ = win.emit("ocr-error", msg);
            });
            return Err(e);
        }
    };

    let preview_path = path.to_string_lossy().to_string();
    {
        let app2 = app.clone();
        let preview2 = preview_path;
        let _ = app.run_on_main_thread(move || {
            paste::remember(&app2, false);
            set_overlay_open(&app2, true);
            arm_ignore_unfocus(&app2);
            let _ = app2.emit("ocr-recognizing", OcrRecognizing { preview: preview2 });
        });
    }

    let recognized = tokio::task::spawn_blocking(move || {
        let text = ocr::recognize_image(&path);

        text
    })
    .await
    .map_err(|e| e.to_string())?;

    match recognized {
        Ok(text) => {
            let text = text.trim().to_string();
            if text.is_empty() {
                let app2 = app.clone();
                let _ = app.run_on_main_thread(move || {
                    set_ocr_picking(&app2, false);
                    disarm_ignore_unfocus(&app2);
                    let Some(win) = app2.get_webview_window("main") else {
                        return;
                    };
                    let _ = win.set_focus();
                    let _ = win.emit("ocr-error", "未识别到文字".to_string());
                });
                return Err("未识别到文字".into());
            }
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                set_ocr_picking(&app2, false);

                disarm_ignore_unfocus(&app2);
                let Some(win) = app2.get_webview_window("main") else {
                    return;
                };
                let _ = win.set_focus();
                let _ = win.emit("ocr-text", text);
            });
            Ok(())
        }
        Err(e) => {
            let msg = e.clone();
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                set_ocr_picking(&app2, false);
                disarm_ignore_unfocus(&app2);
                let Some(win) = app2.get_webview_window("main") else {
                    return;
                };
                let _ = win.set_focus();
                let _ = win.emit("ocr-error", msg);
            });
            Err(e)
        }
    }
}

#[tauri::command]
pub fn ocr_cancel_pick(app: tauri::AppHandle) {
    clear_ocr_freeze(&app);
    set_ocr_picking(&app, false);
    let _ = app.emit("ocr-pick-end", ());
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_size(LogicalSize::new(OVERLAY_W, OVERLAY_H));
    }
    dismiss_overlay(&app, true);
}
