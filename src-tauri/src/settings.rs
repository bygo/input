use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub shortcut: String,
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub keys: HashMap<String, String>,
    #[serde(default)]
    pub last_models: HashMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: "CommandOrControl+G".into(),
            provider: "spacexai".into(),
            base_url: "https://api.x.ai/v1".into(),
            api_key: String::new(),
            model: "grok-4.5".into(),
            keys: HashMap::new(),
            last_models: HashMap::new(),
        }
    }
}

fn workspace() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let dir = PathBuf::from(home).join("input-workspace");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn path() -> Result<PathBuf, String> {
    Ok(workspace()?.join("settings.json"))
}

pub fn read() -> Result<Settings, String> {
    let p = path()?;
    if !p.exists() {
        return Ok(Settings::default());
    }
    let raw = fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let mut s: Settings = serde_json::from_str(&raw).map_err(|e| format!("设置文件损坏: {e}"))?;
    if s.shortcut.contains("Shift+G") {
        s.shortcut = "CommandOrControl+G".into();
        let _ = write(&s);
    }
    Ok(s)
}

pub fn write(settings: &Settings) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path()?, raw).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_settings() -> Result<Settings, String> {
    read()
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    write(&settings)?;
    crate::register_hotkey(&app, &settings.shortcut)
}
