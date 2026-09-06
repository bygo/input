use serde::Serialize;
use serde_json::json;
use std::process::Child;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use super::catalog::ModelRow;

pub(super) struct Inner {
    pub child: Option<Child>,
    pub busy: bool,
    pub message: String,
    pub percent: u8,
}

pub struct LocalState(pub(super) Mutex<Inner>);

pub fn managed() -> LocalState {
    LocalState(Mutex::new(Inner {
        child: None,
        busy: false,
        message: String::new(),
        percent: 0,
    }))
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub enabled: bool,
    pub installed: bool,
    pub running: bool,
    pub has_model: bool,
    pub has_llm: bool,
    pub model: String,
    pub models: Vec<ModelRow>,
    pub busy: bool,
    pub message: String,
    pub percent: u8,
}

pub(super) fn emit(app: &AppHandle, stage: &str, message: &str, percent: u8) {
    if let Some(st) = app.try_state::<LocalState>() {
        if let Ok(mut g) = st.0.lock() {
            g.message = message.into();
            g.percent = percent;
            g.busy = stage != "done" && stage != "error" && stage != "idle";
        }
    }
    let _ = app.emit(
        "local-model-progress",
        json!({ "stage": stage, "message": message, "percent": percent }),
    );
}
