use crate::settings::Settings;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Issue {
    pub span: String,
    pub category: String,
    pub severity: String,
    pub explanation: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Analysis {
    pub corrected: String,
    pub verdict: String,
    pub issues: Vec<Issue>,
}

const SYSTEM: &str = r#"你是英语语法批改器。只分析用户给出的英文。解释必须用中文。
只输出一个 JSON 对象，不要 markdown，不要其它文字。形状：
{"corrected":"改写后的英文","verdict":"clean|needs_work|broken","issues":[{"span":"原文片段","category":"tense|article|agreement|preposition|word_choice|punctuation|other","severity":"error|warning|suggestion","explanation":"中文解释","suggestion":"替换建议"}]}
verdict：没有问题用 clean，有问题用 needs_work，几乎无法理解用 broken。
如果输入不是英文，issues 里说明「请输入英文」，corrected 留空。"#;

fn endpoint(base: &str, path: &str) -> String {
    format!("{}/{}", base.trim_end_matches('/'), path.trim_start_matches('/'))
}

fn is_local(s: &Settings) -> bool {
    s.provider == "ollama"
        || s.base_url.contains("127.0.0.1")
        || s.base_url.contains("localhost")
}

fn extra_headers(s: &Settings) -> Vec<(&'static str, &'static str)> {
    match s.provider.as_str() {
        "claude" => vec![("anthropic-version", "2023-06-01")],
        "openrouter" => vec![("HTTP-Referer", "https://input.local"), ("X-Title", "input")],
        _ => vec![],
    }
}

fn bearer(s: &Settings) -> Result<String, String> {
    if !s.api_key.is_empty() {
        return Ok(s.api_key.clone());
    }
    if is_local(s) {
        return Ok("ollama".into());
    }
    Err("请先在设置中填写 API Key".into())
}

fn map_err(e: reqwest::Error, s: &Settings) -> String {
    if e.is_timeout() {
        return "请求超时".into();
    }
    if e.is_connect() {
        if is_local(s) {
            return "本地模型未启动。Ollama 请先运行 ollama serve，并 ollama pull 对应模型".into();
        }
        return format!("无法连接: {e}");
    }
    e.to_string()
}

fn extract_json(raw: &str) -> Result<Analysis, String> {
    let mut t = raw.trim();
    if let Some(rest) = t.strip_prefix("```json") {
        t = rest;
    } else if let Some(rest) = t.strip_prefix("```") {
        t = rest;
    }
    t = t.strip_suffix("```").unwrap_or(t).trim();
    let start = t.find('{').ok_or("模型返回无法解析")?;
    let end = t.rfind('}').ok_or("模型返回无法解析")?;
    serde_json::from_str(&t[start..=end]).map_err(|e| format!("模型返回无法解析: {e}"))
}

async fn chat(s: &Settings, user: &str, json_mode: bool) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let mut body = json!({
        "model": s.model,
        "temperature": 0,
        "messages": [
            {"role": "system", "content": SYSTEM},
            {"role": "user", "content": user}
        ]
    });
    if json_mode {
        body["response_format"] = json!({"type": "json_object"});
    }
    let mut req = client
        .post(endpoint(&s.base_url, "/chat/completions"))
        .bearer_auth(bearer(s)?)
        .json(&body);
    for (k, v) in extra_headers(s) {
        req = req.header(k, v);
    }
    let resp = req.send().await.map_err(|e| map_err(e, s))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        if status.as_u16() == 401 {
            return Err("API Key 无效".into());
        }
        return Err(format!("模型接口 {status}: {text}"));
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    v["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("模型返回缺少 content: {text}"))
}

pub async fn analyze_text(s: &Settings, text: &str) -> Result<Analysis, String> {
    if text.trim().is_empty() {
        return Err("请输入英文".into());
    }
    let raw = match chat(s, text, true).await {
        Ok(v) => v,
        Err(e) if e.contains("response_format") || e.contains("400") => chat(s, text, false).await?,
        Err(e) => return Err(e),
    };
    extract_json(&raw)
}

#[tauri::command]
pub async fn analyze(text: String) -> Result<Analysis, String> {
    let s = crate::settings::read()?;
    analyze_text(&s, &text).await
}

#[tauri::command]
pub async fn test_provider(settings: Settings) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = client
        .get(endpoint(&settings.base_url, "/models"))
        .bearer_auth(bearer(&settings)?);
    for (k, v) in extra_headers(&settings) {
        req = req.header(k, v);
    }
    let resp = req.send().await.map_err(|e| map_err(e, &settings))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if status.is_success() {
        return Ok("连接成功".into());
    }
    if status.as_u16() == 401 {
        return Err("API Key 无效".into());
    }
    // 部分本地服务没有 /models，再试一次极短 chat
    let raw = chat(&settings, "ping", false).await?;
    if raw.is_empty() && !status.is_success() {
        return Err(format!("测试失败 {status}: {text}"));
    }
    Ok("连接成功".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let raw = r#"{"corrected":"I went.","verdict":"needs_work","issues":[{"span":"go","category":"tense","severity":"error","explanation":"要用过去式","suggestion":"went"}]}"#;
        let a = extract_json(raw).unwrap();
        assert_eq!(a.corrected, "I went.");
        assert_eq!(a.issues[0].span, "go");
    }

    #[test]
    fn parses_fenced_json() {
        let raw = "```json\n{\"corrected\":\"Hi.\",\"verdict\":\"clean\",\"issues\":[]}\n```";
        let a = extract_json(raw).unwrap();
        assert_eq!(a.verdict, "clean");
        assert!(a.issues.is_empty());
    }

    #[test]
    fn rejects_missing_object() {
        assert!(extract_json("not json").is_err());
    }
}
