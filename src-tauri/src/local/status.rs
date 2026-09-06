use tauri::{AppHandle, Manager};

use crate::settings;

use super::catalog::{active_model, catalog_rows};
use super::ollama::{running, tag_has, tags_json};
use super::runtime::resolve_bin;
use super::state::{LocalState, Status};

#[tauri::command]
pub async fn local_model_status(app: AppHandle) -> Status {
    let enabled = settings::read().local_model;
    let model = active_model();
    let installed = resolve_bin().is_some();
    let running = running().await;
    let tags = if running { tags_json().await } else { None };
    let has_llm = tags.as_ref().is_some_and(|v| tag_has(v, &model));
    let models = catalog_rows(tags.as_ref());
    let (busy, message, percent) = if let Some(st) = app.try_state::<LocalState>() {
        st.0.lock()
            .ok()
            .map(|g| (g.busy, g.message.clone(), g.percent))
            .unwrap_or((false, String::new(), 0))
    } else {
        (false, String::new(), 0)
    };
    Status {
        enabled,
        installed,
        running,
        has_model: has_llm,
        has_llm,
        model,
        models,
        busy,
        message,
        percent,
    }
}
