use tauri::{AppHandle, Manager};

use crate::settings;

use super::catalog::{active_model, is_catalog_model, OBSOLETE_LLM};
use super::ollama::{
    delete_named, has_named, pull_named, running, tags_json, warm,
};
use super::runtime::{download_ollama, resolve_bin, spawn_serve, wait_ready};
use super::state::{emit, LocalState, Status};
use super::status::local_model_status;

async fn preload_models(app: &AppHandle) {
    let model = active_model();
    if has_named(&model).await {
        emit(app, "warm", &format!("正在加载模型 {model}…"), 90);
        match warm(&model).await {
            Ok(()) => emit(app, "warm", "模型已常驻", 96),
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

    preload_models(&app).await;
}

async fn pull_models(app: &AppHandle) -> Result<(), String> {
    let model = active_model();
    if !has_named(&model).await {
        pull_named(
            app,
            &model,
            50,
            95,
            &format!("正在拉取模型 {model}…"),
        )
        .await?;
    }
    if let Some(v) = tags_json().await {
        for old in OBSOLETE_LLM {
            if *old == model.as_str() {
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

#[tauri::command]
pub async fn set_llm_model(app: AppHandle, id: String) -> Result<Status, String> {
    if !is_catalog_model(&id) {
        return Err(format!("不支持的型号: {id}"));
    }
    if !has_named(&id).await {
        return Err(format!("尚未下载 {id}，请先下载"));
    }
    let mut s = settings::read();
    s.llm_model = id.clone();
    settings::write(&s)?;
    if s.local_model {
        emit(&app, "warm", &format!("正在加载模型 {id}…"), 90);
        warm(&id).await?;
        emit(&app, "done", "模型已常驻", 100);
    }
    Ok(local_model_status(app).await)
}

#[tauri::command]
pub async fn pull_llm_model(app: AppHandle, id: String) -> Result<Status, String> {
    if !is_catalog_model(&id) {
        return Err(format!("不支持的型号: {id}"));
    }
    {
        let Some(st) = app.try_state::<LocalState>() else {
            return Err("内部状态未就绪".into());
        };
        let mut g = st.0.lock().map_err(|e| e.to_string())?;
        if g.busy {
            return Err("正在安装或下载".into());
        }
        g.busy = true;
        g.message = format!("正在拉取 {id}…");
        g.percent = 1;
    }
    let result = async {
        let bin = match resolve_bin() {
            Some(p) => p,
            None => download_ollama(&app).await?,
        };
        if !running().await {
            spawn_serve(&bin, &app)?;
            wait_ready().await?;
        }
        if !has_named(&id).await {
            pull_named(&app, &id, 10, 95, &format!("正在拉取模型 {id}…")).await?;
        }
        emit(&app, "done", "下载完成", 100);
        Ok::<(), String>(())
    }
    .await;
    if let Some(st) = app.try_state::<LocalState>() {
        if let Ok(mut g) = st.0.lock() {
            g.busy = false;
        }
    }
    result?;
    Ok(local_model_status(app).await)
}

#[tauri::command]
pub async fn delete_llm_model(app: AppHandle, id: String) -> Result<Status, String> {
    if !is_catalog_model(&id) {
        return Err(format!("不支持的型号: {id}"));
    }
    if id == active_model() {
        return Err("不能删除当前使用的型号，请先切换".into());
    }
    if running().await {
        delete_named(&id).await;
    }
    Ok(local_model_status(app).await)
}
