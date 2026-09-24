use serde_json::json;
use std::fs;
use std::time::Duration;


pub(crate) fn llm_debug() -> bool {
    matches!(
        std::env::var("INPUT_LLM_DEBUG").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}


pub(crate) const CLOUD_BASE_DEFAULT: &str = "https://api.x.ai/v1";
pub(crate) const CLOUD_MODEL: &str = "grok-4.6";

/// 正式：https://api.x.ai/v1；临时代理：如 http://127.0.0.1:8181/v1
pub(crate) fn cloud_base() -> String {
    std::env::var("XAI_BASE_URL")
        .ok()
        .map(|s| s.trim().trim_end_matches('/').to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| CLOUD_BASE_DEFAULT.to_string())
}

pub(crate) fn cloud_api_key() -> Result<String, String> {
    if let Ok(k) = std::env::var("XAI_API_KEY") {
        let t = k.trim();
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    let path = crate::settings::dir()?.join("xai_api_key");
    if path.is_file() {
        let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let t = raw.trim();
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    Err("未配置 Grok 密钥：请设置 XAI_API_KEY，或写入 ~/input-workspace/xai_api_key".into())
}

pub(crate) async fn chat_with_system(system: &str, user: &str) -> Result<String, String> {
    let api_key = cloud_api_key()?;
    let base = cloud_base();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/chat/completions", base.trim_end_matches('/'));
    eprintln!("[LLM] cloud POST {url} model={CLOUD_MODEL}");
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": CLOUD_MODEL,
            "temperature": 0,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ]
        }))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "请求超时".into()
            } else if e.is_connect() {
                format!("无法连接 {base}: {e}")
            } else {
                e.to_string()
            }
        })?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        if status.as_u16() == 401 {
            return Err("Grok API Key 无效（本地代理则填代理显示的本地密钥）".into());
        }
        return Err(format!("Grok 接口 {status}: {text}"));
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    v["choices"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|c| c["message"]["content"].as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Grok 返回缺少 content: {text}"))
}

pub(crate) async fn chat_once(system: &str, user: &str, use_cloud: bool) -> Result<String, String> {
    if use_cloud {
        chat_with_system(system, user).await
    } else {
        crate::local::chat(system, user).await
    }
}

pub(crate) async fn chat_once_plain(
    system: &str,
    user: &str,
    use_cloud: bool,
) -> Result<String, String> {
    if use_cloud {
        chat_with_system(system, user).await
    } else {
        crate::local::chat_plain(system, user).await
    }
}
