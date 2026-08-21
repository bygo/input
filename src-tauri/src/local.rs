use crate::settings;
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

pub const MODEL: &str = "qwen2.5:3b";
pub const BASE_URL: &str = "http://127.0.0.1:11434";

struct Inner {
    child: Option<Child>,
    busy: bool,
    message: String,
    percent: u8,
}

pub struct LocalState(Mutex<Inner>);

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
    pub busy: bool,
    pub message: String,
    pub percent: u8,
}

fn ollama_dir() -> Result<PathBuf, String> {
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
    let cmd = if cfg!(windows) { "where" } else { "which" };
    let out = Command::new(cmd).arg("ollama").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let line = s.lines().next()?.trim();
    if line.is_empty() {
        None
    } else {
        Some(PathBuf::from(line))
    }
}

fn resolve_bin() -> Option<PathBuf> {
    bundled_bin()
        .ok()
        .filter(|p| p.is_file())
        .or_else(which_ollama)
}

async fn running() -> bool {
    reqwest::Client::new()
        .get(format!("{BASE_URL}/api/tags"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

async fn has_model() -> bool {
    let Ok(resp) = reqwest::Client::new()
        .get(format!("{BASE_URL}/api/tags"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
    else {
        return false;
    };
    let Ok(v) = resp.json::<serde_json::Value>().await else {
        return false;
    };
    v["models"]
        .as_array()
        .map(|arr| {
            arr.iter().any(|m| {
                m["name"]
                    .as_str()
                    .map(|n| n == MODEL || n.starts_with(&format!("{MODEL}:")))
                    .unwrap_or(false)
                    || m["name"].as_str() == Some(MODEL)
                    || m["model"]
                        .as_str()
                        .is_some_and(|n| n.starts_with("qwen2.5:3b"))
            })
        })
        .unwrap_or(false)
}

fn emit(app: &AppHandle, stage: &str, message: &str, percent: u8) {
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

fn spawn_serve(bin: &Path, app: &AppHandle) -> Result<(), String> {
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

async fn wait_ready() -> Result<(), String> {
    for _ in 0..50 {
        if running().await {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Err("本地模型启动超时".into())
}

pub async fn boot(app: AppHandle) {
    if !settings::read().local_model {
        return;
    }
    let Some(bin) = resolve_bin() else {
        return;
    };
    if running().await {
        return;
    }
    if spawn_serve(&bin, &app).is_ok() {
        let _ = wait_ready().await;
    }
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

async fn download_ollama(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = ollama_dir()?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let archive = dir.join(if cfg!(windows) {
        "ollama.zip"
    } else {
        "ollama.tgz"
    });
    emit(app, "download", "正在下载运行时…", 5);
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
            emit(app, "download", "正在下载运行时…", pct.min(40));
        }
    }
    drop(file);
    emit(app, "extract", "正在解压…", 42);
    let status = Command::new("tar")
        .args([
            "-xf",
            archive.to_str().unwrap(),
            "-C",
            dir.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("解压失败: {e}"))?;
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

async fn pull_model(app: &AppHandle) -> Result<(), String> {
    emit(app, "pull", "正在拉取模型…", 50);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3600))
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .post(format!("{BASE_URL}/api/pull"))
        .json(&json!({ "model": MODEL, "name": MODEL, "stream": true }))
        .send()
        .await
        .map_err(|e| format!("拉取模型失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("拉取模型失败: {}", resp.status()));
    }
    let mut buf = String::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        buf.push_str(&String::from_utf8_lossy(&chunk));
        while let Some(i) = buf.find('\n') {
            let line = buf[..i].trim().to_string();
            buf = buf[i + 1..].to_string();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(err) = v["error"].as_str() {
                    return Err(err.into());
                }
                let total = v["total"].as_u64().unwrap_or(0);
                let done = v["completed"].as_u64().unwrap_or(0);
                if total > 0 {
                    let pct = 50 + (done * 45 / total) as u8;
                    emit(app, "pull", "正在拉取模型…", pct.min(95));
                } else if let Some(st) = v["status"].as_str() {
                    emit(app, "pull", st, 55);
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn local_model_status(app: AppHandle) -> Status {
    let enabled = settings::read().local_model;
    let installed = resolve_bin().is_some();
    let running = running().await;
    let has = if running { has_model().await } else { false };
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
        has_model: has,
        busy,
        message,
        percent,
    }
}

#[tauri::command]
pub async fn install_local_model(app: AppHandle) -> Result<(), String> {
    {
        let Some(st) = app.try_state::<LocalState>() else {
            return Err("内部状态未就绪".into());
        };
        let mut g = st.0.lock().map_err(|e| e.to_string())?;
        if g.busy {
            return Err("正在安装".into());
        }
        g.busy = true;
        g.message = "开始安装…".into();
        g.percent = 1;
    }
    let result = install_inner(&app).await;
    if let Some(st) = app.try_state::<LocalState>() {
        if let Ok(mut g) = st.0.lock() {
            g.busy = false;
        }
    }
    result
}

async fn install_inner(app: &AppHandle) -> Result<(), String> {
    emit(app, "install", "准备安装…", 2);
    let bin = match resolve_bin() {
        Some(p) => p,
        None => download_ollama(app).await?,
    };
    emit(app, "start", "正在启动本地服务…", 45);
    if !running().await {
        spawn_serve(&bin, app)?;
        wait_ready().await?;
    }
    if !has_model().await {
        pull_model(app).await?;
    }
    let mut s = settings::read();
    s.local_model = true;
    settings::write(&s)?;
    emit(app, "done", "本地模型已就绪", 100);
    Ok(())
}

#[tauri::command]
pub async fn set_local_model(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut s = settings::read();
    s.local_model = enabled;
    settings::write(&s)?;
    if enabled {
        boot(app).await;
    } else if let Some(st) = app.try_state::<LocalState>() {
        if let Ok(mut g) = st.0.lock() {
            if let Some(mut child) = g.child.take() {
                let _ = child.kill();
            }
        }
    }
    Ok(())
}

pub async fn ready() -> bool {
    settings::read().local_model && has_model().await
}

pub async fn chat(system: &str, user: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{BASE_URL}/api/chat"))
        .json(&json!({
            "model": MODEL,
            "stream": false,
            "format": "json",
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ]
        }))
        .send()
        .await
        .map_err(|e| format!("本地模型请求失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("本地模型 {status}: {text}"));
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    v["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("本地模型返回缺少 content: {text}"))
}
