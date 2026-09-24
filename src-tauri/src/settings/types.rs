use serde::{Deserialize, Serialize};

pub(super) fn default_language() -> String {
    "英语".into()
}

pub(super) fn default_native_language() -> String {
    "中文".into()
}

pub fn default_compose_shortcut() -> String {
    "CommandOrControl+G".into()
}

pub fn default_select_shortcut() -> String {
    "CommandOrControl+2".into()
}

pub fn default_ocr_shortcut() -> String {
    "CommandOrControl+Shift+O".into()
}

fn default_select_speak() -> bool {
    true
}

fn default_show_native() -> bool {
    true
}

fn default_auto_grammar() -> bool {
    false
}

pub fn default_llm_model() -> String {
    "qwen3.5:4b".into()
}

pub fn default_dict_pack() -> String {
    "slim".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub local_model: bool,

    #[serde(default = "default_llm_model")]
    pub llm_model: String,

    #[serde(default = "default_dict_pack")]
    pub dict_pack: String,
    #[serde(default = "default_language")]
    pub target_language: String,
    #[serde(default = "default_native_language")]
    pub native_language: String,
    #[serde(default = "default_compose_shortcut")]
    pub compose_shortcut: String,
    #[serde(default = "default_select_shortcut")]
    pub select_shortcut: String,
    #[serde(default = "default_ocr_shortcut")]
    pub ocr_shortcut: String,

    #[serde(default = "default_select_speak")]
    pub select_speak: bool,
    /// 是否显示中文对照
    #[serde(default = "default_show_native")]
    pub show_native: bool,
    /// 译出英语后是否自动做块级语义解析；关则点词才解析
    #[serde(default = "default_auto_grammar")]
    pub auto_grammar: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            local_model: false,
            llm_model: default_llm_model(),
            dict_pack: default_dict_pack(),
            target_language: default_language(),
            native_language: default_native_language(),
            compose_shortcut: default_compose_shortcut(),
            select_shortcut: default_select_shortcut(),
            ocr_shortcut: default_ocr_shortcut(),
            select_speak: default_select_speak(),
            show_native: default_show_native(),
            auto_grammar: default_auto_grammar(),
        }
    }
}
