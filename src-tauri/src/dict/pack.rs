use super::db::{open_at, open_db, reset_conn};
use super::path::{active_pack, dict_dir, full_db_path, full_installed, slim_db_path};
use super::types::{DictDb, DictPackRow, DictStatus};
use crate::settings;
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

const FULL_URL: &str =
    "https://github.com/skywind3000/ECDICT/releases/download/1.0.28/ecdict-sqlite-28.zip";

fn emit_progress(app: &AppHandle, stage: &str, message: &str, percent: u8) {
    if let Ok(mut g) = app.state::<DictDb>().progress.lock() {
        g.message = message.into();
        g.percent = percent;
        g.busy = stage != "done" && stage != "error" && stage != "idle";
    }
    let _ = app.emit(
        "dict-progress",
        json!({ "stage": stage, "message": message, "percent": percent }),
    );
}

fn file_size_label(path: &Path) -> String {
    fs::metadata(path)
        .ok()
        .map(|m| {
            let mb = m.len() as f64 / (1024.0 * 1024.0);
            if mb >= 10.0 {
                format!("{mb:.0} MB")
            } else {
                format!("{mb:.1} MB")
            }
        })
        .unwrap_or_else(|| "—".into())
}

fn find_extracted_db(root: &Path) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    let mut found = Vec::new();
    while let Some(dir) = stack.pop() {
        let Ok(rd) = fs::read_dir(&dir) else {
            continue;
        };
        for ent in rd.flatten() {
            let p = ent.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".db") || lower.ends_with(".sqlite") || lower.ends_with(".sqlite3") {
                found.push(p);
            }
        }
    }
    found
        .into_iter()
        .max_by_key(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
}

pub(super) fn validate_db(path: &Path) -> Result<(), String> {
    let len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    if len < 1_000_000 {
        return Err(format!("词库异常：文件过小（{len} 字节）"));
    }
    let conn = open_at(path)?;
    let _: String = conn
        .query_row("SELECT word FROM stardict LIMIT 1", [], |r| r.get(0))
        .map_err(|e| format!("词库无效（缺 stardict 表）: {e}"))?;
    Ok(())
}

async fn download_full(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = dict_dir()?;
    let archive = dir.join("ecdict-sqlite-28.zip");
    let extract_dir = dir.join("extract");
    let _ = fs::remove_dir_all(&extract_dir);
    fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;

    emit_progress(app, "download", "正在下载完整词库…", 3);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(1800))
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .get(FULL_URL)
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
            let pct = 3 + (written * 72 / total) as u8;
            emit_progress(app, "download", "正在下载完整词库…", pct.min(75));
        }
    }
    drop(file);

    emit_progress(app, "extract", "正在解压…", 78);
    let mut tar = Command::new("tar");
    tar.args([
        "-xf",
        archive.to_str().unwrap(),
        "-C",
        extract_dir.to_str().unwrap(),
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        tar.creation_flags(0x0800_0000);
    }
    let status = tar.status().map_err(|e| format!("解压失败: {e}"))?;
    if !status.success() {
        let mut unzip = Command::new("unzip");
        unzip.args([
            "-o",
            archive.to_str().unwrap(),
            "-d",
            extract_dir.to_str().unwrap(),
        ]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            unzip.creation_flags(0x0800_0000);
        }
        let st2 = unzip.status().map_err(|e| format!("解压失败: {e}"))?;
        if !st2.success() {
            return Err("解压失败".into());
        }
    }

    emit_progress(app, "validate", "正在校验词库…", 90);
    let found = find_extracted_db(&extract_dir).ok_or("压缩包内未找到 sqlite 词库")?;
    validate_db(&found)?;
    let dest = full_db_path()?;
    let _ = fs::remove_file(&dest);
    if fs::rename(&found, &dest).is_err() {
        fs::copy(&found, &dest).map_err(|e| format!("写入词库失败: {e}"))?;
    }
    let _ = fs::remove_file(&archive);
    let _ = fs::remove_dir_all(&extract_dir);
    Ok(dest)
}

#[tauri::command]
pub fn dict_status(app: AppHandle) -> DictStatus {
    let pack = active_pack();
    let slim_detail = slim_db_path()
        .map(|p| format!("2.1 万 · 内置 · {}", file_size_label(&p)))
        .unwrap_or_else(|| "2.1 万 · 内置".into());
    let full_detail = if full_installed() {
        full_db_path()
            .ok()
            .map(|p| format!("340 万 · {}", file_size_label(&p)))
            .unwrap_or_else(|| "340 万 · 已安装".into())
    } else {
        "340 万 · 下载约 207 MB".into()
    };
    let (busy, message, percent) = match app.try_state::<DictDb>() {
        Some(st) => match st.progress.lock() {
            Ok(g) => (g.busy, g.message.clone(), g.percent),
            Err(_) => (false, String::new(), 0),
        },
        None => (false, String::new(), 0),
    };
    DictStatus {
        pack,
        packs: vec![
            DictPackRow {
                id: "slim".into(),
                label: "精简版".into(),
                detail: slim_detail,
                installed: slim_db_path().is_some(),
            },
            DictPackRow {
                id: "full".into(),
                label: "完整版".into(),
                detail: full_detail,
                installed: full_installed(),
            },
        ],
        busy,
        message,
        percent,
    }
}

#[tauri::command]
pub async fn pull_dict_pack(app: AppHandle, id: String) -> Result<DictStatus, String> {
    if id != "full" {
        return Err("精简版已内置，无需下载".into());
    }
    {
        let st = app.try_state::<DictDb>().ok_or("内部状态未就绪")?;
        let mut g = st.progress.lock().map_err(|e| e.to_string())?;
        if g.busy {
            return Err("正在下载或切换".into());
        }
        g.busy = true;
        g.message = "开始下载…".into();
        g.percent = 1;
    }
    let result = async {
        if !full_installed() {
            download_full(&app).await?;
        }
        emit_progress(&app, "done", "下载完成", 100);
        Ok::<(), String>(())
    }
    .await;
    if let Some(st) = app.try_state::<DictDb>() {
        if let Ok(mut g) = st.progress.lock() {
            g.busy = false;
        }
    }
    if let Err(e) = result {
        emit_progress(&app, "error", &e, 0);
        return Err(e);
    }
    Ok(dict_status(app))
}

#[tauri::command]
pub fn set_dict_pack(app: AppHandle, id: String) -> Result<DictStatus, String> {
    if id != "slim" && id != "full" {
        return Err(format!("不支持的词库: {id}"));
    }
    if id == "full" && !full_installed() {
        return Err("尚未下载完整版，请先下载".into());
    }
    let mut s = settings::read();
    s.dict_pack = id;
    settings::write(&s)?;
    reset_conn(&app);

    if let Err(e) = open_db() {
        if s.dict_pack == "full" {
            s.dict_pack = "slim".into();
            let _ = settings::write(&s);
            reset_conn(&app);
        }
        return Err(e);
    }
    reset_conn(&app);
    Ok(dict_status(app))
}

#[tauri::command]
pub fn delete_dict_pack(app: AppHandle, id: String) -> Result<DictStatus, String> {
    if id != "full" {
        return Err("不能删除内置精简版".into());
    }
    if active_pack() == "full" {
        return Err("不能删除当前使用的词库，请先切换到精简版".into());
    }
    if let Ok(p) = full_db_path() {
        let _ = fs::remove_file(p);
    }
    let dir = dict_dir()?;
    let _ = fs::remove_file(dir.join("ecdict-sqlite-28.zip"));
    let _ = fs::remove_dir_all(dir.join("extract"));
    Ok(dict_status(app))
}
