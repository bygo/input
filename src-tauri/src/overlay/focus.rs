use std::time::{Duration, Instant};
use tauri::Manager;

use super::state::{IgnoreUnfocusUntil, OcrPicking};

pub fn arm_ignore_unfocus(app: &tauri::AppHandle) {
    if let Some(s) = app.try_state::<IgnoreUnfocusUntil>() {

        *s.0.lock().unwrap() = Some(Instant::now() + Duration::from_millis(500));
    }
}


pub fn clear_ignore_unfocus(app: &tauri::AppHandle) {
    if let Some(s) = app.try_state::<IgnoreUnfocusUntil>() {
        let mut g = s.0.lock().unwrap();
        if g.is_some() {
            *g = Some(Instant::now() + Duration::from_millis(80));
        }
    }
}


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
