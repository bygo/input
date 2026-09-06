use super::shortcut::same_shortcut;
use super::store::{read, write};
use super::types::{
    default_compose_shortcut, default_dict_pack, default_language, default_llm_model,
    default_native_language, default_select_shortcut, Settings,
};

#[tauri::command]
pub fn load_settings() -> Settings {
    let mut s = read();
    let mut dirty = false;

    if s.target_language != default_language() {
        s.target_language = default_language();
        dirty = true;
    }
    if s.native_language != default_native_language() {
        s.native_language = default_native_language();
        dirty = true;
    }
    if is_legacy_default_pair(&s.compose_shortcut, &s.select_shortcut) {
        s.compose_shortcut = default_compose_shortcut();
        s.select_shortcut = default_select_shortcut();
        dirty = true;
    }

    if same_shortcut(&s.compose_shortcut, &s.select_shortcut) {
        s.select_shortcut = default_select_shortcut();
        if same_shortcut(&s.compose_shortcut, &s.select_shortcut)
            || same_shortcut(&s.select_shortcut, &s.ocr_shortcut)
        {
            s.select_shortcut = "CommandOrControl+Shift+T".into();
        }
        dirty = true;
    }
    if !crate::local::is_catalog_model(&s.llm_model) {
        s.llm_model = default_llm_model();
        dirty = true;
    }
    if s.dict_pack != "slim" && s.dict_pack != "full" {
        s.dict_pack = default_dict_pack();
        dirty = true;
    }
    if dirty {
        let _ = write(&s);
    }
    s
}

fn is_legacy_default_pair(compose: &str, select: &str) -> bool {

    same_shortcut(compose, "Alt+A") && same_shortcut(select, "Alt+D")
}

#[tauri::command]
pub fn set_target_language(_language: String) -> Result<Settings, String> {
    let mut s = read();
    s.target_language = default_language();
    write(&s)?;
    Ok(s)
}

#[tauri::command]
pub fn set_native_language(_language: String) -> Result<Settings, String> {
    let mut s = read();
    s.native_language = default_native_language();
    s.target_language = default_language();
    write(&s)?;
    Ok(s)
}

#[tauri::command]
pub fn set_select_speak(enabled: bool) -> Result<Settings, String> {
    let mut s = read();
    s.select_speak = enabled;
    write(&s)?;
    Ok(s)
}
