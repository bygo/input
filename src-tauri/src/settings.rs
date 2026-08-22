use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

fn default_language() -> String {
    "英语".into()
}

fn default_native_language() -> String {
    "中文".into()
}

pub fn default_compose_shortcut() -> String {
    "CommandOrControl+G".into()
}

pub fn default_select_shortcut() -> String {
    "CommandOrControl+Shift+G".into()
}

pub fn default_ocr_shortcut() -> String {
    "CommandOrControl+Shift+O".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub local_model: bool,
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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            local_model: false,
            target_language: default_language(),
            native_language: default_native_language(),
            compose_shortcut: default_compose_shortcut(),
            select_shortcut: default_select_shortcut(),
            ocr_shortcut: default_ocr_shortcut(),
        }
    }
}

fn key_token(code: tauri_plugin_global_shortcut::Code) -> String {
    let s = format!("{code:?}");
    s.strip_prefix("Key")
        .or_else(|| s.strip_prefix("Digit"))
        .unwrap_or(&s)
        .to_string()
}

fn is_fn_key(code: tauri_plugin_global_shortcut::Code) -> bool {
    let s = format!("{code:?}");
    s.starts_with('F') && s.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
}

pub fn normalize_shortcut(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("请设置快捷键".into());
    }
    let hk = Shortcut::from_str(raw).map_err(|e| format!("无效快捷键: {e}"))?;
    let has_cmd = hk.mods.contains(Modifiers::SUPER);
    let has_ctrl = hk.mods.contains(Modifiers::CONTROL);
    let has_alt = hk.mods.contains(Modifiers::ALT);
    let has_shift = hk.mods.contains(Modifiers::SHIFT);
    if !has_cmd && !has_ctrl && !has_alt {
        if is_fn_key(hk.key) {
            // F1–F24, optionally with Shift
        } else {
            return Err("请加上 Command、Control 或 Option".into());
        }
    }
    let mut parts: Vec<String> = Vec::new();
    if has_cmd {
        parts.push("Command".into());
    }
    if has_ctrl {
        parts.push("Control".into());
    }
    if has_alt {
        parts.push("Alt".into());
    }
    if has_shift {
        parts.push("Shift".into());
    }
    parts.push(key_token(hk.key));
    Ok(parts.join("+"))
}

pub fn same_shortcut(a: &str, b: &str) -> bool {
    match (Shortcut::from_str(a.trim()), Shortcut::from_str(b.trim())) {
        (Ok(x), Ok(y)) => x.key == y.key && x.mods == y.mods,
        _ => a.trim().eq_ignore_ascii_case(b.trim()),
    }
}

pub fn dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|e| e.to_string())?;
    let dir = PathBuf::from(home).join("input-workspace");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn path() -> Result<PathBuf, String> {
    Ok(dir()?.join("settings.json"))
}

pub fn read() -> Settings {
    let Ok(p) = path() else {
        return Settings::default();
    };
    if !p.exists() {
        return Settings::default();
    }
    fs::read_to_string(&p)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn write(settings: &Settings) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path()?, raw).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_settings() -> Settings {
    let mut s = read();
    let mut dirty = false;
    // 学习目标暂时固定英语（设置页已去掉目标语选项）。
    if s.target_language != default_language() {
        s.target_language = default_language();
        dirty = true;
    }
    if s.native_language.trim().is_empty() {
        s.native_language = default_native_language();
        dirty = true;
    }
    if s.compose_shortcut.trim().is_empty() {
        s.compose_shortcut = default_compose_shortcut();
        dirty = true;
    }
    if s.select_shortcut.trim().is_empty() {
        s.select_shortcut = default_select_shortcut();
        dirty = true;
    }
    if s.ocr_shortcut.trim().is_empty() {
        s.ocr_shortcut = default_ocr_shortcut();
        dirty = true;
    }
    if is_legacy_default_pair(&s.compose_shortcut, &s.select_shortcut) {
        s.compose_shortcut = default_compose_shortcut();
        s.select_shortcut = default_select_shortcut();
        dirty = true;
    }
    if dirty {
        let _ = write(&s);
    }
    s
}

fn is_legacy_default_pair(compose: &str, select: &str) -> bool {
    // migrate off Option defaults that need Input Monitoring / collide with Bob
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
pub fn set_native_language(language: String) -> Result<Settings, String> {
    let lang = language.trim();
    if lang.is_empty() {
        return Err("请选择当前语言".into());
    }
    let mut s = read();
    s.native_language = lang.to_string();
    write(&s)?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_command_or_control_defaults() {
        let got = normalize_shortcut("CommandOrControl+G").unwrap();
        assert!(got == "Command+G" || got == "Control+G", "{got}");
        let got = normalize_shortcut("CommandOrControl+Shift+G").unwrap();
        assert!(
            got == "Command+Shift+G" || got == "Control+Shift+G",
            "{got}"
        );
        assert_eq!(normalize_shortcut("Alt+A").unwrap(), "Alt+A");
    }

    #[test]
    fn normalizes_command_and_control_separately() {
        assert_eq!(normalize_shortcut("Command+A").unwrap(), "Command+A");
        assert_eq!(normalize_shortcut("Control+A").unwrap(), "Control+A");
        assert!(!same_shortcut("Command+A", "Control+A"));
    }

    #[test]
    fn allows_function_keys() {
        assert_eq!(normalize_shortcut("F8").unwrap(), "F8");
        assert_eq!(normalize_shortcut("Shift+F3").unwrap(), "Shift+F3");
    }

    #[test]
    fn rejects_bare_key() {
        assert!(normalize_shortcut("G").is_err());
        assert!(normalize_shortcut("Shift+G").is_err());
    }

    #[test]
    fn treats_cmd_g_as_same_as_command_or_control() {
        assert!(same_shortcut("CommandOrControl+G", "Command+G"));
        assert!(!same_shortcut("Alt+A", "Alt+D"));
    }
}
