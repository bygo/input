use serde_json::json;
use std::fs;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

use super::client::{chat_once, cloud_api_key, llm_debug, CLOUD_MODEL};
use crate::llm_parse::{
    bad_excerpt, is_empty_result_err, split_translate_chunks, try_parse, try_parse_loose, Analysis,
};
use crate::llm_prompt::{repair_user, select_prompt, system_prompt, user_message};

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
    let model = if source == "local" {
        crate::local::active_model()
    } else {
        CLOUD_MODEL.to_string()
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

fn passthrough_original(text: &str) -> Analysis {
    Analysis {
        sentences: vec![text.to_string()],
        native: None,
        source: "passthrough".into(),
    }
}

async fn analyze_one_attempt(
    text: &str,
    into: &str,
    select: bool,
) -> Result<Analysis, String> {
    let t0 = Instant::now();
    let debug = llm_debug();

    let mut bad_case: Option<String> = None;


    let t = Instant::now();
    let use_cloud = cloud_api_key().is_ok();
    if !use_cloud && !crate::local::ready().await {
        return Err(
            "未配置 Grok 密钥，且本地千问未就绪（设置里打开本地模型，或写入 ~/input-workspace/xai_api_key）"
                .into(),
        );
    }
    let source = if use_cloud { "cloud" } else { "local" };
    let model = if use_cloud {
        CLOUD_MODEL.to_string()
    } else {
        crate::local::active_model()
    };
    eprintln!(
        "[LLM] route source={source} model={model} {}ms",
        t.elapsed().as_millis()
    );

    let t = Instant::now();
    let system = if select {
        select_prompt(into)
    } else {
        system_prompt(into)
    };
    let user = user_message(text, into, select);
    eprintln!(
        "[LLM] prompt select={select} into={into} sys={}B user={}B {}ms",
        system.len(),
        user.len(),
        t.elapsed().as_millis()
    );
    if debug {
        eprintln!("--- system ---\n{system}\n--- user ---\n{user}");
    }

    let t = Instant::now();
    match chat_once(&system, &user, use_cloud).await {
        Ok(raw) => {
            eprintln!(
                "[LLM] chat ok source={source} out={}B {}ms",
                raw.len(),
                t.elapsed().as_millis()
            );
            if debug {
                eprintln!("--- result ---\n{raw}");
            }
            let t = Instant::now();
            if let Some(mut a) = try_parse(&raw, text, into) {

                if !select && a.native.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
                    eprintln!(
                        "[LLM] parse ok but missing native → repair {}ms",
                        t.elapsed().as_millis()
                    );
                    bad_case = Some(bad_excerpt(&raw));
                    log_parse_fail(
                        &format!("{source}_need_native"),
                        text,
                        into,
                        select,
                        source,
                        &raw,
                    );
                } else {
                    eprintln!(
                        "[LLM] parse ok n={} native={} {}ms | attempt_total={}ms",
                        a.sentences.len(),
                        a.native.is_some(),
                        t.elapsed().as_millis(),
                        t0.elapsed().as_millis()
                    );
                    a.source = source.into();
                    return Ok(a);
                }
            } else {
                eprintln!(
                    "[LLM] parse fail → repair {}ms",
                    t.elapsed().as_millis()
                );
                bad_case = Some(bad_excerpt(&raw));
                log_parse_fail(&format!("{source}_need_repair"), text, into, select, source, &raw);
            }
        }
        Err(e) => {
            eprintln!(
                "[LLM] chat err source={source} {}ms: {e}",
                t.elapsed().as_millis()
            );
            log_parse_fail(&format!("{source}_error"), text, into, select, source, &e);
            return Err(e);
        }
    }

    let Some(mut bad) = bad_case else {
        eprintln!(
            "[LLM] passthrough (no bad) | attempt_total={}ms",
            t0.elapsed().as_millis()
        );
        return Ok(passthrough_original(text));
    };

    for round in 1u8..=2 {
        let repair = repair_user(into, text, &bad);
        eprintln!(
            "[LLM] repair#{round} user={}B",
            repair.len()
        );
        if debug {
            eprintln!("--- repair user ---\n{repair}");
        }
        let t = Instant::now();
        match chat_once(&system, &repair, use_cloud).await {
            Ok(raw2) => {
                eprintln!(
                    "[LLM] repair#{round} chat ok out={}B {}ms",
                    raw2.len(),
                    t.elapsed().as_millis()
                );
                if debug {
                    eprintln!("--- repair result ---\n{raw2}");
                }
                let t = Instant::now();
                if let Some(mut a) = try_parse(&raw2, text, into) {
                    if select
                        || a.native.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false)
                    {
                        eprintln!(
                            "[LLM] repair#{round} parse ok n={} native={} {}ms | attempt_total={}ms",
                            a.sentences.len(),
                            a.native.is_some(),
                            t.elapsed().as_millis(),
                            t0.elapsed().as_millis()
                        );
                        a.source = source.into();
                        return Ok(a);
                    }
                }
                if select {
                    if let Some(mut a) = try_parse_loose(&raw2, text, into) {
                        eprintln!(
                            "[LLM] repair#{round} loose ok n={} {}ms | attempt_total={}ms",
                            a.sentences.len(),
                            t.elapsed().as_millis(),
                            t0.elapsed().as_millis()
                        );
                        log_parse_fail(
                            &format!("{source}_repair_accept_loose"),
                            text,
                            into,
                            select,
                            source,
                            &raw2,
                        );
                        a.source = source.into();
                        return Ok(a);
                    }
                }
                eprintln!(
                    "[LLM] repair#{round} parse fail {}ms",
                    t.elapsed().as_millis()
                );
                bad = bad_excerpt(&raw2);
                log_parse_fail(
                    &format!("{source}_repair_failed_r{round}"),
                    text,
                    into,
                    select,
                    source,
                    &raw2,
                );
            }
            Err(e) => {
                eprintln!(
                    "[LLM] repair#{round} chat err {}ms: {e}",
                    t.elapsed().as_millis()
                );
                log_parse_fail(&format!("{source}_repair_error"), text, into, select, source, &e);
                return Err(e);
            }
        }
    }
    eprintln!(
        "[LLM] passthrough after repair | attempt_total={}ms",
        t0.elapsed().as_millis()
    );
    log_parse_fail(
        &format!("{source}_passthrough"),
        text,
        into,
        select,
        "passthrough",
        &bad,
    );
    Ok(passthrough_original(text))
}

async fn analyze_one(text: &str, into: &str, select: bool) -> Result<Analysis, String> {
    let t0 = Instant::now();
    let mut last = Err("模型返回为空".into());
    for attempt in 1u8..=3 {
        let t = Instant::now();
        match analyze_one_attempt(text, into, select).await {
            Ok(a) => {
                if a.sentences.iter().any(|s| !s.trim().is_empty()) {
                    eprintln!(
                        "[LLM] analyze_one ok attempt={attempt} source={} {}ms | one_total={}ms",
                        a.source,
                        t.elapsed().as_millis(),
                        t0.elapsed().as_millis()
                    );
                    return Ok(a);
                }
                eprintln!(
                    "[LLM] analyze_one empty attempt={attempt} {}ms",
                    t.elapsed().as_millis()
                );
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
                eprintln!(
                    "[LLM] analyze_one err attempt={attempt} empty={empty} {}ms: {e}",
                    t.elapsed().as_millis()
                );
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

    if last.is_err() {
        eprintln!(
            "[LLM] analyze_one passthrough | one_total={}ms",
            t0.elapsed().as_millis()
        );
        log_parse_fail(
            "analyze_one_passthrough",
            text,
            into,
            select,
            "passthrough",
            "empty after retries",
        );
        return Ok(passthrough_original(text));
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
    let t0 = Instant::now();
    if text.trim().is_empty() {
        return Err("请输入内容".into());
    }
    let select = select.unwrap_or(false);

    let into = match into.as_ref().map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(s) => s.to_string(),
        None if select => crate::lang::into_for_selection(&text),
        None => resolve_into(None),
    };


    let chunks = if select {
        split_translate_chunks(&text, 200)
    } else {
        vec![text.as_str()]
    };
    let total = chunks.len();
    eprintln!(
        "[LLM] analyze start select={select} into={into} chars={} chunks={total} req={req_id:?}",
        text.len()
    );
    if total <= 1 {
        let one = analyze_one(&text, &into, select).await?;
        emit_analyze_partial(&app, req_id, 0, 1, one.sentences.first().map(|s| s.as_str()).unwrap_or(""), &one.source);
        eprintln!(
            "[LLM] analyze done source={} total={}ms",
            one.source,
            t0.elapsed().as_millis()
        );
        return Ok(one);
    }

    let mut acc = String::new();
    let mut source = String::new();
    for (index, chunk) in chunks.into_iter().enumerate() {
        if chunk.trim().is_empty() {
            acc.push_str(chunk);
            emit_analyze_partial(&app, req_id, index, total, &acc, if source.is_empty() { "local" } else { &source });
            continue;
        }
        let t = Instant::now();
        let part = analyze_one(chunk, &into, select).await?;
        eprintln!(
            "[LLM] chunk {}/{} chars={} {}ms",
            index + 1,
            total,
            chunk.len(),
            t.elapsed().as_millis()
        );
        let piece = part.sentences.first().map(|s| s.as_str()).unwrap_or("").to_string();
        acc.push_str(&piece);
        if chunk.ends_with('\n') && !piece.ends_with('\n') {
            acc.push('\n');
        }
        source = part.source;
        emit_analyze_partial(&app, req_id, index, total, &acc, &source);
    }
    if acc.trim().is_empty() {
        eprintln!(
            "[LLM] analyze empty total={}ms",
            t0.elapsed().as_millis()
        );
        return Err("模型返回为空".into());
    }
    eprintln!(
        "[LLM] analyze done source={source} chunks={total} total={}ms",
        t0.elapsed().as_millis()
    );
    Ok(Analysis {
        sentences: vec![acc],
        native: None,
        source,
    })
}
