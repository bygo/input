use super::types::Analysis;

pub(crate) fn json_objects(raw: &str) -> Vec<&str> {
    let bytes = raw.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }
        let start = i;
        let mut depth = 0i32;
        let mut in_str = false;
        let mut escape = false;
        while i < bytes.len() {
            let c = bytes[i];
            if in_str {
                if escape {
                    escape = false;
                } else if c == b'\\' {
                    escape = true;
                } else if c == b'"' {
                    in_str = false;
                }
            } else if c == b'"' {
                in_str = true;
            } else if c == b'{' {
                depth += 1;
            } else if c == b'}' {
                depth -= 1;
                if depth == 0 {
                    out.push(&raw[start..=i]);
                    break;
                }
            }
            i += 1;
        }
        i += 1;
    }
    out
}

fn str_field<'a>(v: &'a serde_json::Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|k| v.get(*k).and_then(|x| x.as_str()))
}

fn list_from_arr(arr: &[serde_json::Value]) -> Vec<String> {
    arr.iter()
        .filter_map(|item| {
            if let Some(s) = item.as_str() {
                return Some(s.trim().to_string());
            }
            str_field(item, &["text", "sentence", "corrected", "en", "english"])
                .map(|s| s.trim().to_string())
        })
        .filter(|s| !s.is_empty())
        .collect()
}

pub(crate) fn sentences_from(v: &serde_json::Value) -> Vec<String> {
    if let Some(arr) = v.get("sentences").and_then(|x| x.as_array()) {
        let out = list_from_arr(arr);
        if !out.is_empty() {
            return out;
        }
    }
    if let Some(obj) = v.as_object() {
        for (_, child) in obj {
            let nested = sentences_from(child);
            if !nested.is_empty() {
                return nested;
            }
        }
    }
    if let Some(s) = str_field(v, &["corrected", "text", "english"]) {
        let lines: Vec<String> = s
            .lines()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect();
        if !lines.is_empty() {
            return lines;
        }
    }
    vec![]
}

fn native_from(v: &serde_json::Value) -> Option<String> {
    if let Some(s) = str_field(v, &["native", "zh", "chinese", "中文"]) {
        let t = s.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Some(arr) = v.get("native").and_then(|x| x.as_array()) {
        let joined = list_from_arr(arr).join("\n");
        if !joined.trim().is_empty() {
            return Some(joined);
        }
    }
    None
}

pub(crate) fn extract_json(raw: &str) -> Result<Analysis, String> {
    let mut last = None;
    for slice in json_objects(raw) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(slice) {
            let sentences = sentences_from(&v);
            if !sentences.is_empty() {
                last = Some(Analysis {
                    sentences,
                    native: native_from(&v),
                    source: String::new(),
                });
            }
        }
    }
    last.ok_or_else(|| {
        let preview: String = raw.chars().take(200).collect();
        format!("模型返回无法解析: {preview}")
    })
}

pub(crate) fn merge_linewise(original: &str, sentences: Vec<String>) -> Vec<String> {
    let src_n = original
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .count();
    if src_n <= 1 {
        return sentences;
    }
    let parts: Vec<String> = sentences
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.len() == src_n && parts.iter().all(|s| !s.contains('\n')) {
        return vec![parts.join("\n")];
    }
    parts
}


pub(crate) fn bad_excerpt(raw: &str) -> String {
    if let Ok(a) = extract_json(raw) {
        let joined = a.sentences.join("\n");
        if !joined.trim().is_empty() {
            return joined;
        }
    }
    raw.chars().take(500).collect()
}

pub(crate) fn try_parse(raw: &str, original: &str, into: &str) -> Option<Analysis> {
    let a = extract_json(raw).ok()?;
    let merged = merge_linewise(original, a.sentences);
    let sentences = take_candidates(merged, into);
    if sentences.is_empty() {
        return None;
    }
    let native = a.native.and_then(|n| {
        let t = n.trim().to_string();
        if t.is_empty() || is_instruction_leak(&t) {
            return None;
        }

        if crate::lang::fits_target(&t, "中文") {
            Some(t)
        } else {
            None
        }
    });
    Some(Analysis {
        sentences,
        native,
        source: String::new(),
    })
}


pub(crate) fn try_parse_loose(raw: &str, original: &str, into: &str) -> Option<Analysis> {
    let a = extract_json(raw).ok()?;
    let merged = merge_linewise(original, a.sentences);
    let strict = take_candidates(merged.clone(), into);
    if !strict.is_empty() {
        return Some(Analysis {
            sentences: strict,
            native: None,
            source: String::new(),
        });
    }

    let parts: Vec<String> = merged
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .filter(|s| !is_instruction_leak(s))
        .filter(|s| crate::lang::fits_target(s, into))
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(Analysis {
            sentences: parts,
            native: None,
            source: String::new(),
        })
    }
}

pub(crate) fn is_empty_result_err(err: &str) -> bool {
    err.contains("模型返回为空")
        || err.contains("返回空 JSON")
        || err.contains("缺少 content")
        || err.contains("空 JSON")
}


pub(crate) fn split_translate_chunks(text: &str, limit: usize) -> Vec<&str> {
    if text.chars().count() <= limit {
        return vec![text];
    }
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut n = 0usize;
    let mut over = false;
    for (i, ch) in text.char_indices() {
        if over && ch == '\n' {
            let end = i + ch.len_utf8();
            out.push(&text[start..end]);
            start = end;
            n = 0;
            over = false;
            continue;
        }
        n += 1;
        if n > limit {
            over = true;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    if out.is_empty() {
        vec![text]
    } else {
        out
    }
}

pub(crate) fn canon_token(t: &str) -> &str {
    match t {
        "purchase" | "purchases" => "buy",
        "purchased" => "bought",
        "purchasing" => "buying",
        "obtain" | "obtains" => "get",
        "obtained" => "got",
        "obtaining" => "getting",
        "assist" | "assists" | "assistance" => "help",
        "assisted" => "helped",
        "assisting" => "helping",
        "require" | "requires" => "need",
        "required" => "needed",
        "requiring" => "needing",
        "utilize" | "utilizes" => "use",
        "utilized" => "used",
        "utilizing" => "using",
        "commence" | "commences" => "start",
        "commenced" => "started",
        "inquire" | "inquires" | "enquire" | "enquires" => "ask",
        "inquired" | "enquired" => "asked",
        _ => t,
    }
}

pub(crate) fn register_key(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_alphabetic() && c != '\'')
        .filter(|t| !t.is_empty())
        .map(canon_token)
        .collect::<Vec<_>>()
        .join(" ")
}


fn is_instruction_leak(s: &str) -> bool {
    s.contains("上一次输出不合格")
        || s.contains("下面译文不合格")
        || s.contains("不合格译文")
        || s.contains("请把下面整段改写")
        || s.contains("只输出 JSON")
        || s.contains("{\"sentences\"")
        || s.contains("禁止残留源语言")
}

pub(crate) fn take_candidates(raw: Vec<String>, lang: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut seen_exact = std::collections::HashSet::new();
    let mut seen_reg = std::collections::HashSet::new();
    for s in raw {
        let t = s.trim().to_string();
        if t.is_empty() || !seen_exact.insert(t.clone()) {
            continue;
        }
        if is_instruction_leak(&t) {
            continue;
        }

        if !crate::lang::fits_target(&t, lang) {
            continue;
        }
        let key = register_key(&t);
        if !key.is_empty() && !seen_reg.insert(key.clone()) {
            continue;
        }
        sentences.push(t);
        if sentences.len() == 3 {
            break;
        }
    }
    sentences
}
