use crate::settings;


pub const LLM_CATALOG: &[(&str, &str)] = &[
    ("qwen3.5:2b", "千问 2B"),
    ("qwen3.5:4b", "千问 4B"),
    ("qwen3.5:9b", "千问 9B"),
    ("translategemma:4b", "TG 4B"),
];

pub(super) const OBSOLETE_LLM: &[&str] = &["qwen3:4b", "qwen3:4b-instruct", "qwen3:8b"];

pub fn is_catalog_model(id: &str) -> bool {
    LLM_CATALOG.iter().any(|(m, _)| *m == id)
}

pub fn active_model() -> String {
    let id = settings::read().llm_model;
    if is_catalog_model(&id) {
        id
    } else {
        settings::default_llm_model()
    }
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRow {
    pub id: String,
    pub label: String,
    pub installed: bool,
}

pub(super) fn catalog_rows(tags: Option<&serde_json::Value>) -> Vec<ModelRow> {
    LLM_CATALOG
        .iter()
        .map(|(id, label)| ModelRow {
            id: (*id).into(),
            label: (*label).into(),
            installed: tags.is_some_and(|v| super::ollama::tag_has(v, id)),
        })
        .collect()
}
