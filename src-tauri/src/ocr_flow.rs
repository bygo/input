use tauri::{Emitter, LogicalSize, Manager, PhysicalPosition, PhysicalSize};

use crate::ocr;
use crate::overlay::{
    appear_window, arm_ignore_unfocus, clear_ocr_freeze, disarm_ignore_unfocus, dismiss_overlay,
    ocr_freeze_origin, ocr_picking, park_window, set_ocr_freeze, set_ocr_picking, set_overlay_open,
    show_overlay, OcrFreezeFrame, OverlayKind, OVERLAY_H, OVERLAY_W,
};
use crate::paste;

#[derive(Clone, serde::Serialize)]
pub struct OcrPickStart {
    /// Absolute path to freeze JPEG — frontend loads via convertFileSrc (no base64).
    pub path: String,
}

#[derive(Clone, serde::Serialize)]
pub struct OcrRecognizing {
    /// 选区 JPEG 绝对路径；前端 convertFileSrc
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
        let freeze_path = tokio::task::spawn_blocking(move || ocr::capture_screen_rect(lx, ly, lw, lh))
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
    let (origin_x, origin_y) =
        ocr_freeze_origin(&app).ok_or_else(|| "没有定格图".to_string())?;
    clear_ocr_freeze(&app);

    // Nominal 定格与 webview 1:1：框选像素 ≈ 屏幕 CSS points
    let sx = origin_x + x.max(0.0);
    let sy = origin_y + y.max(0.0);
    let sw = w.max(0.0);
    let sh = h.max(0.0);

    let crop = tokio::task::spawn_blocking(move || ocr::capture_region_retina(sx, sy, sw, sh))
        .await
        .map_err(|e| e.to_string())?;

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
        // 预览仍占用 path，识别完不立刻删
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
                // 面板已亮过：撤掉 ignore，并抢回焦点，否则点外侧不触发失焦关闭
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
