use std::fs;
use std::path::PathBuf;

use super::types::Settings;

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
