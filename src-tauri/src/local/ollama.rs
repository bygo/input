use serde_json::json;
use std::time::Duration;

use super::catalog::active_model;
use crate::settings;

pub const BASE_URL: &str = "http://127.0.0.1:11434";


pub(super) const KEEP_ALIVE: i64 = -1;

pub(super) async fn running() -> bool {
    reqwest::Client::new()
        .get(format!("{BASE_URL}/api/tags"))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

pub(super) async fn tags_json() -> Option<serde_json::Value> {
    let resp = reqwest::Client::new()
        .get(format!("{BASE_URL}/api/tags"))
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .ok()?;
    resp.json::<serde_json::Value>().await.ok()
}

pub(super) fn tag_has(v: &serde_json::Value, want: &str) -> bool {
    let Some(arr) = v["models"].as_array() else {
        return false;
    };

    let dash = format!("{want}-");
    let colon = format!("{want}:");
    arr.iter().any(|m| {
        let name = m["name"].as_str().unwrap_or("");
        let model = m["model"].as_str().unwrap_or("");
        [name, model]
            .into_iter()
            .any(|n| n == want || n.starts_with(&dash) || n.starts_with(&colon))
    })
}

pub(super) async fn has_named(name: &str) -> bool {
    tags_json().await.is_some_and(|v| tag_has(&v, name))
}

async fn has_llm_model() -> bool {
    has_named(&active_model()).await
}

pub(super) async fn warm(name: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(format!("{BASE_URL}/api/chat"))
        .json(&json!({
            "model": name,
            "stream": false,
            "think": false,
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

pub(super) async fn pull_named(
    app: &tauri::AppHandle,
    name: &str,
    lo: u8,
    hi: u8,
    label: &str,
) -> Result<(), String> {
    super::state::emit(app, "pull", label, lo);
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
                    super::state::emit(app, "pull", label, pct.min(hi));
                } else if let Some(st) = v["status"].as_str() {
                    super::state::emit(app, "pull", st, lo.saturating_add(2));
                }
            }
        }
    }
    Ok(())
}

pub(super) async fn delete_named(name: &str) {
    let _ = reqwest::Client::new()
        .delete(format!("{BASE_URL}/api/delete"))
        .json(&json!({ "model": name }))
        .timeout(Duration::from_secs(60))
        .send()
        .await;
}

pub async fn ready() -> bool {
    settings::read().local_model && has_llm_model().await
}

pub async fn chat(system: &str, user: &str) -> Result<String, String> {
    chat_inner(system, user, true).await
}

/// 词卡上下文释义等：不强制 JSON schema。
pub async fn chat_plain(system: &str, user: &str) -> Result<String, String> {
    chat_inner(system, user, false).await
}

async fn chat_inner(system: &str, user: &str, json_schema: bool) -> Result<String, String> {
    let model = active_model();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let mut body = json!({
        "model": model,
        "stream": false,
        "think": false,
        "keep_alive": KEEP_ALIVE,
        "options": { "temperature": 0.2 },
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user}
        ]
    });
    if json_schema {
        body["format"] = json!({
            "type": "object",
            "properties": {
                "sentences": {
                    "type": "array",
                    "items": { "type": "string" },
                    "minItems": 1
                },
                "native": { "type": "string" }
            },
            "required": ["sentences"]
        });
    }
    let resp = client
        .post(format!("{BASE_URL}/api/chat"))
        .json(&body)
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

    if !json_schema {
        return Ok(content);
    }
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
