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

/// Rewrite / compose LLM.
pub const MODEL: &str = "qwen3:4b-instruct";
/// Select-translate dedicated MT (TranslateGemma).
pub const MT_MODEL: &str = "translategemma:4b";
pub const BASE_URL: &str = "http://127.0.0.1:11434";
/// Old rewrite tags to drop after switching.
const OBSOLETE_LLM: &[&str] = &["qwen3:4b"];

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
    /// Both LLM + MT present (legacy "ready" for UI).
    pub has_model: bool,
    pub has_llm: bool,
    pub has_mt: bool,
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
    // 扫 PATH，避免 Windows 上 `where` 弹出控制台窗口。
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

async fn tags_json() -> Option<serde_json::Value> {
    let resp = reqwest::Client::new()
        .get(format!("{BASE_URL}/api/tags"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .ok()?;
    resp.json::<serde_json::Value>().await.ok()
}

fn tag_has(v: &serde_json::Value, want: &str) -> bool {
    let Some(arr) = v["models"].as_array() else {
        return false;
    };
    // Exact or more-specific quant tags only — avoid "qwen3:4b" matching "qwen3:4b-instruct".
    let dash = format!("{want}-");
    let colon = format!("{want}:");
    arr.iter().any(|m| {
        let name = m["name"].as_str().unwrap_or("");
        let model = m["model"].as_str().unwrap_or("");
        [name, model].into_iter().any(|n| n == want || n.starts_with(&dash) || n.starts_with(&colon))
    })
}

async fn has_llm_model() -> bool {
    tags_json().await.is_some_and(|v| tag_has(&v, MODEL))
}

async fn has_mt_model() -> bool {
    tags_json().await.is_some_and(|v| tag_has(&v, MT_MODEL))
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

/// Keep both models resident forever (until Ollama exits).
const KEEP_ALIVE: i64 = -1;

async fn warm(name: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{BASE_URL}/api/chat"))
        .json(&json!({
            "model": name,
            "stream": false,
            "keep_alive": KEEP_ALIVE,
            "messages": [{"role": "user", "content": "."}],
            "options": { "num_predict": 1, "temperature": 0 }
        }))
        .send()
        .await
        .map_err(|e| format!("预热 {name} 失败: {e}"))?;
    if !resp.status().is_success() {
        let t = resp.text().await.unwrap_or_default();
        return Err(format!("预热 {name} 失败: {t}"));
    }
    Ok(())
}

async fn preload_models(app: &AppHandle) {
    if has_mt_model().await {
        emit(app, "warm", &format!("正在加载翻译模型 {MT_MODEL}…"), 80);
        match warm(MT_MODEL).await {
            Ok(()) => emit(app, "warm", "翻译模型已常驻", 88),
            Err(e) => emit(app, "error", &e, 88),
        }
    }
    if has_llm_model().await {
        emit(app, "warm", &format!("正在加载改写模型 {MODEL}…"), 90);
        match warm(MODEL).await {
            Ok(()) => emit(app, "warm", "改写模型已常驻", 96),
            Err(e) => emit(app, "error", &e, 96),
        }
    }
    emit(app, "done", "本地模型已常驻内存", 100);
}

pub async fn boot(app: AppHandle) {
    if !settings::read().local_model {
        return;
    }
    if !running().await {
        let Some(bin) = resolve_bin() else {
            return;
        };
        if spawn_serve(&bin, &app).is_ok() {
            let _ = wait_ready().await;
        }
    }
    if !running().await {
        return;
    }
    // Always warm both so first translate/rewrite has no load delay.
    preload_models(&app).await;
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

async fn pull_named(app: &AppHandle, name: &str, lo: u8, hi: u8, label: &str) -> Result<(), String> {
    emit(app, "pull", label, lo);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3600))
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .post(format!("{BASE_URL}/api/pull"))
        .json(&json!({ "model": name, "name": name, "stream": true }))
        .send()
        .await
        .map_err(|e| format!("拉取 {name} 失败: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("拉取 {name} 失败: {}", resp.status()));
    }
    let span = (hi - lo) as u64;
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
                    let pct = lo + (done * span / total) as u8;
                    emit(app, "pull", label, pct.min(hi));
                } else if let Some(st) = v["status"].as_str() {
                    emit(app, "pull", st, lo.saturating_add(2));
                }
            }
        }
    }
    Ok(())
}

async fn delete_named(name: &str) {
    let _ = reqwest::Client::new()
        .delete(format!("{BASE_URL}/api/delete"))
        .json(&json!({ "model": name }))
        .timeout(Duration::from_secs(60))
        .send()
        .await;
}

async fn pull_models(app: &AppHandle) -> Result<(), String> {
    if !has_mt_model().await {
        pull_named(
            app,
            MT_MODEL,
            50,
            72,
            &format!("正在拉取翻译模型 {MT_MODEL}…"),
        )
        .await?;
    }
    if !has_llm_model().await {
        pull_named(
            app,
            MODEL,
            73,
            95,
            &format!("正在拉取改写模型 {MODEL}…"),
        )
        .await?;
    }
    if let Some(v) = tags_json().await {
        for old in OBSOLETE_LLM {
            if *old == MODEL {
                continue;
            }
            let present = v["models"].as_array().is_some_and(|arr| {
                arr.iter().any(|m| {
                    m["name"].as_str() == Some(*old) || m["model"].as_str() == Some(*old)
                })
            });
            if present {
                emit(app, "pull", &format!("正在删除旧模型 {old}…"), 96);
                delete_named(old).await;
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
    let (has_llm, has_mt) = if running {
        (has_llm_model().await, has_mt_model().await)
    } else {
        (false, false)
    };
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
        has_model: has_llm && has_mt,
        has_llm,
        has_mt,
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
    pull_models(app).await?;
    let mut s = settings::read();
    s.local_model = true;
    settings::write(&s)?;
    preload_models(app).await;
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
    settings::read().local_model && has_llm_model().await
}

pub async fn mt_ready() -> bool {
    settings::read().local_model && has_mt_model().await
}

/// Dedicated select-translate via TranslateGemma (plain text out, not JSON).
pub async fn translate(text: &str, source_lang: &str, target_lang: &str) -> Result<String, String> {
    let (src_name, src_code) = crate::lang::mt_lang_meta(source_lang);
    let (tgt_name, tgt_code) = crate::lang::mt_lang_meta(target_lang);
    // Official TranslateGemma shape (two blank lines), with strict fidelity.
    let prompt = format!(
        "You are a strict {src_name} ({src_code}) to {tgt_name} ({tgt_code}) translator.\n\
Translate literally and faithfully. Keep the same meaning, person, tense, tone, and structure as much as possible.\n\
Do NOT add any subject, object, possessive, location, cause, or relationship that is not explicit in the source.\n\
Do NOT paraphrase for fluency, do NOT expand, soften, explain, or reinterpret. If a choice is between smoother wording and closer wording, choose closer wording.\n\
If the source has multiple lines, translate EVERY line and preserve the same line breaks. Do not translate only the first line.\n\
Output must be entirely in {tgt_name} script/vocabulary. Transliterate personal names and terms; do NOT leave source-language characters (e.g. Chinese Han) in the output.\n\
Produce only the {tgt_name} translation, with no commentary.\n\
Please translate the following {src_name} text into {tgt_name}:\n\
\n\
\n\
{text}"
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{BASE_URL}/api/chat"))
        .json(&json!({
            "model": MT_MODEL,
            "stream": false,
            "keep_alive": KEEP_ALIVE,
            "options": { "temperature": 0.1 },
            "messages": [
                {"role": "user", "content": prompt}
            ]
        }))
        .send()
        .await
        .map_err(|e| format!("翻译模型请求失败: {e}"))?;
    let status = resp.status();
    let body = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("翻译模型 {status}: {body}"));
    }
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let content = v["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("翻译模型返回缺少 content: {body}"))?;
    Ok(content)
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
            "keep_alive": KEEP_ALIVE,
            // Must be a JSON schema — plain "json" often returns {} on small chat models.
            "format": {
                "type": "object",
                "properties": {
                    "sentences": {
                        "type": "array",
                        "items": { "type": "string" },
                        "minItems": 1
                    }
                },
                "required": ["sentences"]
            },
            "options": { "temperature": 0.2 },
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
    let content = v["message"]["content"]
        .as_str()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("本地模型返回缺少 content: {text}"))?;
    // Reject empty objects / empty arrays before callers treat them as success.
    if content == "{}" || content == "{\"sentences\":[]}" || content == "{ \"sentences\": [] }" {
        return Err(format!("本地模型返回空 JSON: {content}"));
    }
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content) {
        let empty = parsed
            .get("sentences")
            .and_then(|s| s.as_array())
            .map(|a| a.is_empty())
            .unwrap_or(parsed.as_object().map(|o| o.is_empty()).unwrap_or(false));
        if empty {
            return Err(format!("本地模型返回空 JSON: {content}"));
        }
    }
    Ok(content)
}
