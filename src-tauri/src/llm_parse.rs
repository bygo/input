use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Analysis {
    pub sentences: Vec<String>,
    #[serde(default)]
    pub source: String,
}

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

pub(crate) fn extract_json(raw: &str) -> Result<Analysis, String> {
    let mut last = None;
    for slice in json_objects(raw) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(slice) {
            let sentences = sentences_from(&v);
            if !sentences.is_empty() {
                last = Some(Analysis {
                    sentences,
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

pub(crate) fn try_parse(raw: &str, original: &str, into: &str) -> Option<Vec<String>> {
    let a = extract_json(raw).ok()?;
    let merged = merge_linewise(original, a.sentences);
    let sentences = take_candidates(merged, original, into);
    if sentences.is_empty() {
        None
    } else {
        Some(sentences)
    }
}

/// 选中翻译兜底：允许代码标识符等 fits_target 误杀，但拒绝明显源语言整段。
pub(crate) fn try_parse_loose(raw: &str, original: &str, into: &str) -> Option<Vec<String>> {
    let a = extract_json(raw).ok()?;
    let merged = merge_linewise(original, a.sentences);
    let strict = take_candidates(merged.clone(), original, into);
    if !strict.is_empty() {
        return Some(strict);
    }
    let parts: Vec<String> = merged
        .into_iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .filter(|s| !is_instruction_leak(s))
        .filter(|s| {
            if into.contains("英语") || into.contains("英文") {
                !crate::lang::looks_like(s, "中文")
            } else if into.contains("中文") {
                true
            } else {
                crate::lang::fits_target(s, into)
            }
        })
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts)
    }
}

pub(crate) fn is_empty_result_err(err: &str) -> bool {
    err.contains("模型返回为空")
        || err.contains("返回空 JSON")
        || err.contains("缺少 content")
        || err.contains("空 JSON")
}

/// 超过 limit 字后，在下一个换行处切开；后面继续同样规则。无换行则整段不切。
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

/// 模型把 repair/系统提示原样回灌时的特征（尤其中文目标下易被 loose 误收）。
fn is_instruction_leak(s: &str) -> bool {
    s.contains("上一次输出不合格")
        || s.contains("不合格译文")
        || s.contains("请把下面整段改写")
        || s.contains("只输出 JSON")
        || s.contains("{\"sentences\"")
        || s.contains("禁止残留源语言")
}

pub(crate) fn take_candidates(raw: Vec<String>, original: &str, lang: &str) -> Vec<String> {
    let orig = original.trim();
    // 原文已是目标语时：同词干候选保留用户用词（防 buy→purchase）；语法已改则不要把错句硬插第一位。
    let prefer_orig = !orig.is_empty() && crate::lang::looks_like(orig, lang);
    let orig_key = if prefer_orig {
        register_key(orig)
    } else {
        String::new()
    };
    let mut sentences = Vec::new();
    let mut seen_exact = std::collections::HashSet::new();
    let mut seen_reg = std::collections::HashSet::new();
    let mut used_orig_surface = false;
    for s in raw {
        let t = s.trim().to_string();
        if t.is_empty() || !seen_exact.insert(t.clone()) {
            continue;
        }
        if is_instruction_leak(&t) {
            continue;
        }
        // Drop mixed-language leftovers (e.g. Chinese with "overnight" kept).
        if !crate::lang::fits_target(&t, lang) {
            continue;
        }
        let key = register_key(&t);
        if !key.is_empty() && !seen_reg.insert(key.clone()) {
            continue;
        }
        if prefer_orig && !orig_key.is_empty() && key == orig_key && !used_orig_surface {
            used_orig_surface = true;
            if seen_exact.insert(orig.to_string()) {
                sentences.push(orig.to_string());
            } else {
                sentences.push(t);
            }
        } else {
            sentences.push(t);
        }
        if sentences.len() == 3 {
            break;
        }
    }
    sentences
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let raw = r#"{"sentences":["I went to school.","It was fine."]}"#;
        let a = extract_json(raw).unwrap();
        assert_eq!(a.sentences, ["I went to school.", "It was fine."]);
    }

    #[test]
    fn parses_fenced_json() {
        let raw = "```json\n{\"sentences\":[\"Hi.\"]}\n```";
        let a = extract_json(raw).unwrap();
        assert_eq!(a.sentences, ["Hi."]);
    }

    #[test]
    fn parses_object_sentences() {
        let raw = r#"{"sentences":[{"text":"Hi there."}]}"#;
        let a = extract_json(raw).unwrap();
        assert_eq!(a.sentences, ["Hi there."]);
    }

    #[test]
    fn parses_corrected_fallback() {
        let raw = r#"{"corrected":"It's fine.","verdict":"needs_work","issues":[{"span":"ok","category":"other","severity":"error","explanation":"x"}]}"#;
        let a = extract_json(raw).unwrap();
        assert_eq!(a.sentences, ["It's fine."]);
    }

    #[test]
    fn rejects_missing_object() {
        assert!(extract_json("not json").is_err());
    }

    #[test]
    fn parses_nested_shape() {
        let raw = r#"{"shape":{"sentences":["But choose which one to use after giving more consideration."]}}"#;
        let a = extract_json(raw).unwrap();
        assert_eq!(
            a.sentences,
            ["But choose which one to use after giving more consideration."]
        );
    }

    #[test]
    fn takes_last_json_after_ramble() {
        let raw = r#"{"sentences":["Why aren't you moving faster?","Why is the pace so slow?"]} ``` Wait, I need to reconsider. Here's my response: ```json {"sentences":["Why aren't you going faster?"]}"#;
        let a = extract_json(raw).unwrap();
        assert_eq!(a.sentences, ["Why aren't you going faster?"]);
    }

    #[test]
    fn candidates_dedupe_and_cap_at_three() {
        let got = take_candidates(
            vec![
                "  Hi.  ".into(),
                "Hi.".into(),
                "Hello.".into(),
                "Hey.".into(),
                "Yo.".into(),
                "".into(),
            ],
            "",
            "英语",
        );
        assert_eq!(got, ["Hi.", "Hello.", "Hey."]);
    }

    #[test]
    fn drops_purchase_when_original_already_says_buy() {
        let orig =
            "Which fish should I buy and how should I prepare it? Do I need to buy any vegetables?";
        let got = take_candidates(
            vec![
                "Which fish should I purchase and how should I prepare it? Do I need to purchase any vegetables?".into(),
                "Which fish should I get, and how do I cook it?".into(),
            ],
            orig,
            "英语",
        );
        assert_eq!(
            got,
            [orig, "Which fish should I get, and how do I cook it?",]
        );
    }

    #[test]
    fn keeps_grammar_fix_ahead_of_broken_original() {
        let orig = "I aren’t love you";
        let got = take_candidates(
            vec!["I don't love you.".into(), "I do not love you.".into()],
            orig,
            "英语",
        );
        assert_eq!(got[0], "I don't love you.");
        assert!(!got.iter().any(|s| s.contains("aren’t love") || s.contains("aren't love")));
    }

    #[test]
    fn merges_linewise_translation_into_one_block() {
        let orig = "小段等12人来了\n常想一二等11人来了\nDK哥等10人来了";
        let parts = vec![
            "A waited for 12 people.".into(),
            "B waited for 11 people.".into(),
            "C waited for 10 people.".into(),
        ];
        let merged = merge_linewise(orig, parts);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0],
            "A waited for 12 people.\nB waited for 11 people.\nC waited for 10 people."
        );
    }

    #[test]
    fn rejects_empty_json_object() {
        assert!(try_parse("{}", "hello", "英语").is_none());
        assert!(try_parse(r#"{"sentences":[]}"#, "hello", "英语").is_none());
        assert!(try_parse(r#"{"sentences":["Hi."]}"#, "hello", "英语").is_some());
    }

    #[test]
    fn rejects_repair_instruction_leak() {
        let leak = "上一次输出不合格。请把下面整段改写成纯正中文，保留换行；禁止残留源语言字符（含人名须音译）。只输出 JSON：{\"sentences\":[\"...\"]}。";
        let raw = format!(r#"{{"sentences":[{}]}}"#, serde_json::to_string(leak).unwrap());
        assert!(try_parse(&raw, "Hello world", "中文").is_none());
        assert!(try_parse_loose(&raw, "Hello world", "中文").is_none());
    }

    #[test]
    fn drops_chinese_candidates_with_english_leftovers() {
        let orig = "No major brand-new shock overnight, but crude is still reacting to the ongoing US-Iran developments.";
        let got = take_candidates(
            vec![
                "overnight没有出现重大新冲击，但crude仍在反应。".into(),
                "隔夜没有出现重大新冲击，但原油仍在对持续的美伊局势做出反应。".into(),
                "隔夜并无重大新冲击，原油仍在对美伊局势持续反应。".into(),
            ],
            orig,
            "中文",
        );
        assert_eq!(
            got,
            [
                "隔夜没有出现重大新冲击，但原油仍在对持续的美伊局势做出反应。",
                "隔夜并无重大新冲击，原油仍在对美伊局势持续反应。",
            ]
        );
    }

    #[test]
    pub(crate) fn split_translate_chunks_under_limit_stays_one() {
        let s = "a\nb\nc";
        assert_eq!(split_translate_chunks(s, 200), vec![s]);
    }

    #[test]
    pub(crate) fn split_translate_chunks_cuts_at_next_newline_after_limit() {
        let head = "汉".repeat(201);
        let s = format!("{head}\n第二段\n第三段");
        let parts = split_translate_chunks(&s, 200);
        assert_eq!(parts.len(), 2);
        assert!(parts[0].ends_with('\n'));
        assert_eq!(parts[0].chars().count(), 202); // 201 hans + newline
        assert_eq!(parts[1], "第二段\n第三段");
    }

    #[test]
    pub(crate) fn split_translate_chunks_no_newline_stays_whole() {
        let s = "汉".repeat(400);
        assert_eq!(split_translate_chunks(&s, 200), vec![s.as_str()]);
    }


}
