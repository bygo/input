use serde_json::json;
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

pub use crate::llm_parse::Analysis;
use crate::llm_parse::{
    is_empty_result_err, split_translate_chunks, try_parse, try_parse_loose,
};
use crate::llm_prompt::{select_prompt, system_prompt, user_message};

const BASE_URL: &str = "https://node-hk.sssaiapi.com/api";
const API_KEY: &str =
    "sk-sssaicode-c43a7da470198b3556bf569111f5e5be3fa6bbdb04688d4e06d74b604d6ff165";
const CLOUD_MODEL: &str = "claude-haiku-4-5-20251001";

fn resolve_into(into: Option<String>) -> String {
    if let Some(s) = into {
        let t = s.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    let lang = crate::settings::read().target_language;
    if lang.trim().is_empty() {
        "英语".into()
    } else {
        lang
    }
}

fn log_parse_fail(reason: &str, text: &str, into: &str, select: bool, source: &str, raw: &str) {
    let Ok(root) = crate::settings::dir() else {
        return;
    };
    let dir = root.join("logs");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let path = dir.join(format!("parse-{ms}.json"));
    let model = if source == "local-mt" {
        crate::local::MT_MODEL
    } else if source == "local" {
        crate::local::MODEL
    } else {
        CLOUD_MODEL
    };
    let body = json!({
        "reason": reason,
        "text": text,
        "into": into,
        "select": select,
        "source": source,
        "model": model,
        "raw": raw,
    });
    if let Ok(s) = serde_json::to_string_pretty(&body) {
        let _ = fs::write(path, s);
    }
}

async fn chat_with_system(system: &str, user: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/v1/messages", BASE_URL.trim_end_matches('/'));
    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {API_KEY}"))
        .header("anthropic-version", "2023-06-01")
        .header("x-api-key", API_KEY)
        .json(&json!({
            "model": CLOUD_MODEL,
            "max_tokens": 4096,
            "temperature": 0,
            "system": system,
            "stop_sequences": ["```"],
            "messages": [{"role": "user", "content": user}]
        }))
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "请求超时".into()
            } else if e.is_connect() {
                format!("无法连接: {e}")
            } else {
                e.to_string()
            }
        })?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        if status.as_u16() == 401 {
            return Err("API Key 无效".into());
        }
        return Err(format!("模型接口 {status}: {text}"));
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    v["content"]
        .as_array()
        .and_then(|blocks| {
            let text = blocks
                .iter()
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("");
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        })
        .ok_or_else(|| format!("模型返回缺少 content: {text}"))
}

/// 模型常把多行译文拆进 sentences[]；界面只展示 [0]，需先按行合并回整段。

fn emit_analyze_partial(
    app: &AppHandle,
    req_id: Option<u64>,
    index: usize,
    total: usize,
    partial: &str,
    source: &str,
) {
    let _ = app.emit(
        "analyze-partial",
        json!({
            "reqId": req_id,
            "index": index,
            "total": total,
            "partial": partial,
            "source": source,
        }),
    );
}

async fn analyze_one_attempt(
    text: &str,
    into: &str,
    select: bool,
    mt_source: Option<&str>,
) -> Result<Analysis, String> {
    if let Some(source_lang) = mt_source {
        if crate::local::mt_ready().await {
            match crate::local::translate(text, source_lang, into).await {
                Ok(raw) => {
                    // TranslateGemma：原样上屏，不做混排/截断过滤。
                    let out = raw.trim().to_string();
                    if !out.is_empty() {
                        return Ok(Analysis {
                            sentences: vec![out],
                            source: "local-mt".into(),
                        });
                    }
                    log_parse_fail("mt_empty", text, into, select, "local-mt", &raw);
                }
                Err(e) => {
                    log_parse_fail("mt_error", text, into, select, "local-mt", &e);
                    if is_empty_result_err(&e) {
                        return Err(e);
                    }
                }
            }
        }
    }

    let system = if select {
        select_prompt(into)
    } else {
        system_prompt(into)
    };
    let user = user_message(text, into, select);
    eprintln!(
        "\n========== [LLM] select={select} into={into} mt_source={mt_source:?} ==========\n--- system ---\n{system}\n--- user ---\n{user}\n========== [/LLM prompt] ==========\n"
    );

    // Compose (and select fallback): local LLM, then cloud.
    // Empty `{}` / mixed-script output must not block cloud fallback.
    if crate::local::ready().await {
        match crate::local::chat(&system, &user).await {
            Ok(raw) => {
                eprintln!("\n========== [LLM local result] ==========\n{raw}\n========== [/LLM local result] ==========\n");
                if let Some(sentences) = try_parse(&raw, text, into) {
                    return Ok(Analysis {
                        sentences,
                        source: "local".into(),
                    });
                }
                let repair = format!(
                    "上一次输出不合格。请把下面整段改写成纯正{into}，保留换行；禁止残留源语言字符（含人名须音译）。只输出 JSON：{{\"sentences\":[\"...\"]}}。\n不合格译文：\n{raw}"
                );
                eprintln!("\n========== [LLM repair user] ==========\n{repair}\n========== [/LLM repair user] ==========\n");
                if let Ok(raw2) = crate::local::chat(&system, &repair).await {
                    eprintln!("\n========== [LLM local repair result] ==========\n{raw2}\n========== [/LLM local repair result] ==========\n");
                    if let Some(sentences) = try_parse(&raw2, text, into) {
                        return Ok(Analysis {
                            sentences,
                            source: "local".into(),
                        });
                    }
                    if select {
                        if let Some(sentences) = try_parse_loose(&raw2, text, into) {
                            log_parse_fail(
                                "local_repair_accept_loose",
                                text,
                                into,
                                select,
                                "local",
                                &raw2,
                            );
                            return Ok(Analysis {
                                sentences,
                                source: "local".into(),
                            });
                        }
                    }
                    log_parse_fail("local_repair_failed", text, into, select, "local", &raw2);
                } else {
                    log_parse_fail("local_empty_or_unparseable", text, into, select, "local", &raw);
                }
            }
            Err(e) => {
                log_parse_fail("local_error", text, into, select, "local", &e);
                if is_empty_result_err(&e) {
                    // 继续云端；真正空结果由外层重试。
                }
            }
        }
    }

    let raw = chat_with_system(&system, &user).await?;
    eprintln!("\n========== [LLM cloud result] ==========\n{raw}\n========== [/LLM cloud result] ==========\n");
    if let Some(sentences) = try_parse(&raw, text, into) {
        return Ok(Analysis {
            sentences,
            source: "cloud".into(),
        });
    }
    if select {
        if let Some(sentences) = try_parse_loose(&raw, text, into) {
            log_parse_fail(
                "cloud_accept_loose",
                text,
                into,
                select,
                "cloud",
                &raw,
            );
            return Ok(Analysis {
                sentences,
                source: "cloud".into(),
            });
        }
    }
    log_parse_fail("cloud_empty_or_unparseable", text, into, select, "cloud", &raw);
    Err("模型返回为空".into())
}

async fn analyze_one(
    text: &str,
    into: &str,
    select: bool,
    mt_source: Option<&str>,
) -> Result<Analysis, String> {
    let mut last = Err("模型返回为空".into());
    for attempt in 1u8..=3 {
        match analyze_one_attempt(text, into, select, mt_source).await {
            Ok(a) => {
                if a.sentences.iter().any(|s| !s.trim().is_empty()) {
                    return Ok(a);
                }
                log_parse_fail(
                    "empty_retry",
                    text,
                    into,
                    select,
                    "retry",
                    &format!("attempt={attempt} empty sentences"),
                );
                last = Err("模型返回为空".into());
            }
            Err(e) => {
                let empty = is_empty_result_err(&e);
                log_parse_fail(
                    if empty { "empty_retry" } else { "attempt_error" },
                    text,
                    into,
                    select,
                    "retry",
                    &format!("attempt={attempt} err={e}"),
                );
                last = Err(e);
                if !empty {
                    return last;
                }
            }
        }
    }
    last
}

#[tauri::command]
pub async fn analyze(
    app: AppHandle,
    text: String,
    into: Option<String>,
    select: Option<bool>,
    req_id: Option<u64>,
) -> Result<Analysis, String> {
    if text.trim().is_empty() {
        return Err("请输入内容".into());
    }
    let select = select.unwrap_or(false);
    let into = if select {
        crate::lang::into_for_selection(&text)
    } else {
        resolve_into(into)
    };

    // TranslateGemma：select 只用 is_pure_english 分流，禁止 detect。
    let mt_source_owned: Option<String> = if select {
        Some(crate::lang::mt_source_for_selection(&text))
    } else {
        crate::lang::compose_mt_source(&text, &into).map(|s| s.to_string())
    };
    let mt_source = mt_source_owned.as_deref();
    let translating = select || mt_source.is_some();
    let chunks = if translating {
        split_translate_chunks(&text, 200)
    } else {
        vec![text.as_str()]
    };
    let total = chunks.len();
    if total <= 1 {
        let one = analyze_one(&text, &into, select, mt_source).await?;
        emit_analyze_partial(&app, req_id, 0, 1, one.sentences.first().map(|s| s.as_str()).unwrap_or(""), &one.source);
        return Ok(one);
    }

    let mut acc = String::new();
    let mut source = String::new();
    for (index, chunk) in chunks.into_iter().enumerate() {
        if chunk.trim().is_empty() {
            acc.push_str(chunk);
            emit_analyze_partial(&app, req_id, index, total, &acc, if source.is_empty() { "local-mt" } else { &source });
            continue;
        }
        let part = analyze_one(chunk, &into, select, mt_source).await?;
        let piece = part.sentences.first().map(|s| s.as_str()).unwrap_or("").to_string();
        acc.push_str(&piece);
        if chunk.ends_with('\n') && !piece.ends_with('\n') {
            acc.push('\n');
        }
        source = part.source;
        emit_analyze_partial(&app, req_id, index, total, &acc, &source);
    }
    if acc.trim().is_empty() {
        return Err("模型返回为空".into());
    }
    Ok(Analysis {
        sentences: vec![acc],
        source,
    })
}
