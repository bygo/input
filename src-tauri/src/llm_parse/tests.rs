use super::parse::*;
use super::Analysis;

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
fn try_parse_rejects_english_with_han_particle() {
    let raw = r#"{"sentences":["How to read books呢"]}"#;
    assert!(try_parse(raw, "如何读书呢", "英语").is_none());
    let ok = r#"{"sentences":["How to read books?"],"native":"你怎么读书？"}"#;
    let a = try_parse(ok, "如何读书呢", "英语").unwrap();
    assert_eq!(a.sentences, ["How to read books?"]);
    assert_eq!(a.native.as_deref(), Some("你怎么读书？"));
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
        "英语",
    );
    assert_eq!(got, ["Hi.", "Hello.", "Hey."]);
}

#[test]
fn dedupes_purchase_buy_stem() {
    let got = take_candidates(
        vec![
            "Which fish should I purchase and how should I prepare it? Do I need to purchase any vegetables?".into(),
            "Which fish should I buy and how should I prepare it? Do I need to buy any vegetables?".into(),
            "Which fish should I get, and how do I cook it?".into(),
        ],
        "英语",
    );
    assert_eq!(got.len(), 2);
    assert!(got[0].contains("purchase"));
    assert!(got[1].contains("get"));
}

#[test]
fn keeps_grammar_fix_ahead_of_broken_original() {
    let got = take_candidates(
        vec!["I don't love you.".into(), "I do not love you.".into()],
        "英语",
    );
    assert_eq!(got[0], "I don't love you.");
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
    let got = take_candidates(
        vec![
            "overnight没有出现重大新冲击，但crude仍在反应。".into(),
            "隔夜没有出现重大新冲击，但原油仍在对持续的美伊局势做出反应。".into(),
            "隔夜并无重大新冲击，原油仍在对美伊局势持续反应。".into(),
        ],
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
    assert_eq!(parts[0].chars().count(), 202);
    assert_eq!(parts[1], "第二段\n第三段");
}

#[test]
pub(crate) fn split_translate_chunks_no_newline_stays_whole() {
    let s = "汉".repeat(400);
    assert_eq!(split_translate_chunks(&s, 200), vec![s.as_str()]);
}
