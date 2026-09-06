use crate::settings;
use std::fs;
use std::path::PathBuf;

pub(super) fn slim_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    out.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/ecdict.db"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("resources/ecdict.db"));
            out.push(dir.join("../Resources/resources/ecdict.db"));
            out.push(dir.join("../Resources/ecdict.db"));
        }
    }
    out
}

pub(super) fn slim_db_path() -> Option<PathBuf> {
    slim_candidates().into_iter().find(|p| p.is_file())
}

pub(super) fn dict_dir() -> Result<PathBuf, String> {
    let dir = settings::dir()?.join("dict");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub(super) fn full_db_path() -> Result<PathBuf, String> {
    Ok(dict_dir()?.join("ecdict-full.db"))
}

pub(super) fn full_installed() -> bool {
    full_db_path().ok().is_some_and(|p| p.is_file())
}

pub(super) fn active_pack() -> String {
    let pack = settings::read().dict_pack;
    if pack == "full" && full_installed() {
        "full".into()
    } else {
        "slim".into()
    }
}

pub(super) fn resolve_db_path() -> Result<PathBuf, String> {
    if active_pack() == "full" {
        if let Ok(p) = full_db_path() {
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    slim_db_path().ok_or_else(|| "未找到词库 ecdict.db".into())
}
