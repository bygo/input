use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tauri::{AppHandle, Manager};

use crate::settings;

use super::state::LocalState;

pub(super) fn ollama_dir() -> Result<PathBuf, String> {
    Ok(settings::dir()?.join("ollama"))
}

fn bundled_bin() -> Result<PathBuf, String> {
    let name = if cfg!(windows) {
        "ollama.exe"
    } else {
        "ollama"
    };
    Ok(ollama_dir()?.join(name))
}

fn which_ollama() -> Option<PathBuf> {

    let names: &[&str] = if cfg!(windows) {
        &["ollama.exe", "ollama"]
    } else {
        &["ollama"]
    };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for name in names {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

pub(super) fn resolve_bin() -> Option<PathBuf> {
    bundled_bin()
        .ok()
        .filter(|p| p.is_file())
        .or_else(which_ollama)
}

pub(super) fn spawn_serve(bin: &Path, app: &AppHandle) -> Result<(), String> {
    if let Some(st) = app.try_state::<LocalState>() {
        if let Ok(g) = st.0.lock() {
            if g.child.is_some() {
                return Ok(());
            }
        }
    }
    let models = ollama_dir()?.join("models");
    fs::create_dir_all(&models).map_err(|e| e.to_string())?;
    let mut cmd = Command::new(bin);
    cmd.arg("serve")
        .env("OLLAMA_HOST", "127.0.0.1:11434")
        .env("OLLAMA_MODELS", &models)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let child = cmd.spawn().map_err(|e| format!("启动本地模型失败: {e}"))?;
    if let Some(st) = app.try_state::<LocalState>() {
        if let Ok(mut g) = st.0.lock() {
            g.child = Some(child);
        }
    }
    Ok(())
}

pub(super) async fn wait_ready() -> Result<(), String> {
    for _ in 0..50 {
        if super::ollama::running().await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err("本地模型启动超时".into())
}

fn download_url() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "https://github.com/ollama/ollama/releases/latest/download/ollama-windows-amd64.zip"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "https://github.com/ollama/ollama/releases/latest/download/ollama-darwin.tgz"
    }
}

pub(super) async fn download_ollama(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = ollama_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let archive = dir.join(if cfg!(windows) {
        "ollama.zip"
    } else {
        "ollama.tgz"
    });
    super::state::emit(app, "download", "正在下载运行时…", 5);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .get(download_url())
        .send()
        .await
        .map_err(|e| format!("下载失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("下载失败: {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    let mut file = fs::File::create(&archive).map_err(|e| e.to_string())?;
    let mut written = 0u64;
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        written += chunk.len() as u64;
        if total > 0 {
            let pct = 5 + (written * 35 / total) as u8;
            super::state::emit(app, "download", "正在下载运行时…", pct.min(40));
        }
    }
    drop(file);
    super::state::emit(app, "extract", "正在解压…", 42);
    let mut tar = Command::new("tar");
    tar.args([
        "-xf",
        archive.to_str().unwrap(),
        "-C",
        dir.to_str().unwrap(),
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        tar.creation_flags(0x0800_0000);
    }
    let status = tar.status().map_err(|e| format!("解压失败: {e}"))?;
    if !status.success() {
        return Err("解压失败".into());
    }
    let _ = fs::remove_file(&archive);
    let bin = find_extracted(&dir).ok_or("未找到 ollama 可执行文件")?;
    let dest = bundled_bin()?;
    if bin != dest {
        fs::copy(&bin, &dest).map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dest, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
        let _ = Command::new("xattr")
            .args(["-dr", "com.apple.quarantine", dest.to_str().unwrap()])
            .status();
    }
    Ok(dest)
}

fn find_extracted(dir: &Path) -> Option<PathBuf> {
    let names = if cfg!(windows) {
        ["ollama.exe", "ollama"]
    } else {
        ["ollama", "ollama.exe"]
    };
    for name in names {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if let Some(f) = find_extracted(&p) {
                    return Some(f);
                }
            } else if p.file_name()?.to_str()?.starts_with("ollama") && p.is_file() {
                return Some(p);
            }
        }
    }
    None
}
