fn is_han(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}')
}

fn is_kana(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}')
}

fn is_hangul(c: char) -> bool {
    matches!(c, '\u{1100}'..='\u{11FF}' | '\u{AC00}'..='\u{D7AF}')
}

fn is_cyrillic(c: char) -> bool {
    matches!(c, '\u{0400}'..='\u{04FF}')
}

fn is_arabic(c: char) -> bool {
    matches!(c, '\u{0600}'..='\u{06FF}')
}

fn is_latin(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '\u{00C0}'..='\u{024F}')
}

fn letters(text: &str) -> Vec<char> {
    text.chars().filter(|c| c.is_alphabetic()).collect()
}

fn ratio(chars: &[char], pred: fn(char) -> bool) -> f32 {
    if chars.is_empty() {
        return 0.0;
    }
    let n = chars.iter().copied().filter(|c| pred(*c)).count();
    n as f32 / chars.len() as f32
}

fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphabetic() && c != '\'')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_lowercase())
        .collect()
}

fn hits(text: &str, words: &[&str]) -> usize {
    let set: std::collections::HashSet<&str> = words.iter().copied().collect();
    tokens(text)
        .into_iter()
        .filter(|t| set.contains(t.as_str()))
        .count()
}

const EN: &[&str] = &[
    "the", "and", "of", "to", "in", "is", "it", "you", "that", "for", "on", "are", "was", "this",
    "with", "be", "as", "have", "not", "but", "they", "from", "we", "or", "an", "work", "hours",
    "at", "by", "if", "can", "will",
];

fn guess(text: &str) -> Option<&'static str> {
    let chars = letters(text);
    if chars.is_empty() {
        return None;
    }
    let han = ratio(&chars, is_han);
    let kana = ratio(&chars, is_kana);
    let hangul = ratio(&chars, is_hangul);
    let latin = ratio(&chars, is_latin);
    if hangul >= 0.3 {
        return Some("韩语");
    }
    if kana >= 0.08 {
        return Some("日语");
    }
    if han >= 0.35 && kana < 0.08 && hangul < 0.08 {
        return Some("中文");
    }
    if ratio(&chars, is_arabic) >= 0.35 {
        return Some("阿拉伯语");
    }
    if ratio(&chars, is_cyrillic) >= 0.35 {
        return Some("俄语");
    }
    if latin >= 0.5 {
        if hits(text, EN) >= 1 || latin >= 0.85 {
            return Some("英语");
        }
        return Some("英语");
    }
    None
}

pub fn looks_like(text: &str, lang: &str) -> bool {
    match guess(text) {
        Some(g) => g == lang,
        None => false,
    }
}

pub fn detect(text: &str) -> Option<&'static str> {
    guess(text)
}

/// Compose routing: if source language is confidently detected and differs from
/// the learning language `into`, return that source so the caller can run MT.
/// Returns `None` when same-language (rewrite) or detection is uncertain → LLM.
pub fn compose_mt_source(text: &str, into: &str) -> Option<&'static str> {
    let source = detect(text)?;
    if looks_like(text, into) {
        return None;
    }
    // Detected language equals target name (e.g. both 英语).
    if source == into.trim() || into.contains(source) {
        return None;
    }
    Some(source)
}

/// English name + BCP-47-ish code for TranslateGemma prompts.
pub fn mt_lang_meta(lang: &str) -> (&'static str, &'static str) {
    if lang.contains("中文") {
        ("Chinese", "zh")
    } else if lang.contains("英语") || lang.contains("英文") {
        ("English", "en")
    } else if lang.contains("日语") {
        ("Japanese", "ja")
    } else if lang.contains("韩语") {
        ("Korean", "ko")
    } else if lang.contains("法语") {
        ("French", "fr")
    } else if lang.contains("德语") {
        ("German", "de")
    } else if lang.contains("西班牙语") {
        ("Spanish", "es")
    } else if lang.contains("葡萄牙语") {
        ("Portuguese", "pt")
    } else if lang.contains("意大利语") {
        ("Italian", "it")
    } else if lang.contains("俄语") {
        ("Russian", "ru")
    } else if lang.contains("阿拉伯语") {
        ("Arabic", "ar")
    } else if lang.contains("越南语") {
        ("Vietnamese", "vi")
    } else {
        ("English", "en")
    }
}

/// True if `text` is acceptably pure for the target language.
/// Used to drop model output that keeps source-language words (e.g. 「overnight没有…」).
pub fn fits_target(text: &str, lang: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    let chars = letters(t);
    if chars.is_empty() {
        return true;
    }
    let han = ratio(&chars, is_han);
    let kana = ratio(&chars, is_kana);
    let hangul = ratio(&chars, is_hangul);
    let cyr = ratio(&chars, is_cyrillic);
    let ar = ratio(&chars, is_arabic);
    let latin = ratio(&chars, is_latin);

    if lang.contains("中文") {
        // 拒 overnight/crude 这类英文实词夹杂；允许 APM/Fing 等缩写专名 + 中文（战绩/HUD 常见）。
        if has_forbidden_latin_in_chinese(t) {
            return false;
        }
        return han >= 0.12;
    }
    if lang.contains("日语") {
        return !has_ascii_word_run(t, 4) && (han + kana) >= 0.35 && latin < 0.45;
    }
    if lang.contains("韩语") {
        return !has_ascii_word_run(t, 4) && hangul >= 0.4 && han < 0.2;
    }
    if lang.contains("俄语") {
        // 禁止残留汉字/拉丁实词。
        return !has_ascii_word_run(t, 4) && cyr >= 0.5 && han == 0.0;
    }
    if lang.contains("阿拉伯语") {
        return !has_ascii_word_run(t, 4) && ar >= 0.5 && han == 0.0;
    }
    if lang.contains("越南语") {
        return han == 0.0 && latin >= 0.5 && !has_ascii_cjk_mix(t);
    }
    if lang.contains("英语") || lang.contains("英文") {
        // 人名未音译也会残留汉字；比例再低也不放行。
        return han == 0.0 && latin >= 0.55;
    }
    // Romance / Germanic targets: no Han, mostly Latin.
    if lang.contains("法语")
        || lang.contains("德语")
        || lang.contains("西班牙语")
        || lang.contains("葡萄牙语")
        || lang.contains("意大利语")
    {
        return han == 0.0 && latin >= 0.55;
    }
    true
}

fn has_ascii_cjk_mix(text: &str) -> bool {
    // Vietnamese model sometimes emits Latin + Han in one sentence.
    let has_latin = text.chars().any(|c| c.is_ascii_alphabetic());
    let has_han = text.chars().any(is_han);
    has_latin && has_han
}

fn has_ascii_word_run(text: &str, min_len: usize) -> bool {
    let mut run = 0usize;
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            run += 1;
            if run >= min_len {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// 中文目标里不允许的拉丁残留：全小写实词（overnight/crude）；允许全大写缩写与大小写混合专名。
fn has_forbidden_latin_in_chinese(text: &str) -> bool {
    let mut buf = String::new();
    let check = |tok: &str| -> bool {
        if tok.is_empty() || tok.len() <= 2 {
            return false;
        }
        let chars: Vec<char> = tok.chars().collect();
        let all_upper = chars.iter().all(|c| c.is_ascii_uppercase());
        let all_lower = chars.iter().all(|c| c.is_ascii_lowercase());
        if all_upper && chars.len() <= 6 {
            return false; // APM / FPS / GDP
        }
        if !all_lower {
            return false; // Fing / TTiner / Iran — 专名
        }
        chars.len() >= 3 // the / ms(2 ok) / overnight / crude
    };
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            buf.push(c);
        } else if !buf.is_empty() {
            if check(&buf) {
                return true;
            }
            buf.clear();
        }
    }
    if !buf.is_empty() && check(&buf) {
        return true;
    }
    false
}

pub fn route(text: &str, native: &str, target: &str) -> String {
    let native = if native.trim().is_empty() {
        "中文"
    } else {
        native.trim()
    };
    let target = if target.trim().is_empty() {
        "英语"
    } else {
        target.trim()
    };
    if looks_like(text, native) {
        target.to_string()
    } else {
        native.to_string()
    }
}

pub fn into_for_selection(text: &str) -> String {
    let s = crate::settings::read();
    route(text, &s.native_language, &s.target_language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_selected_goes_to_chinese() {
        assert_eq!(route("Work on it for 14 hours.", "中文", "英语"), "中文");
        assert_eq!(route("Have you eaten yet today?", "中文", "英语"), "中文");
    }

    #[test]
    fn chinese_selected_goes_to_english() {
        assert_eq!(route("你好，今天吃饭了吗", "中文", "英语"), "英语");
    }

    #[test]
    fn japanese_goes_to_native() {
        assert_eq!(route("こんにちは", "中文", "英语"), "中文");
    }

    #[test]
    fn chinese_rejects_english_leftovers() {
        assert!(!fits_target(
            "overnight没有出现重大新冲击，但原油仍在反应。",
            "中文"
        ));
        assert!(!fits_target(
            "隔夜没有重大新冲击，但 crude 仍在反应。",
            "中文"
        ));
        assert!(fits_target(
            "隔夜没有出现重大新冲击，但原油仍在对持续的美伊局势做出反应。",
            "中文"
        ));
        assert!(fits_target("可以，OK。", "中文")); // short Latin ok
        assert!(fits_target(
            "Fing.0.00毫秒\nAPM：244\n8\nTTiner 00:17:15\n时间：\n17:07",
            "中文"
        ));
        assert!(!fits_target(
            "Fing.0.00ms\nAPM: 244\nTime：\n17:07",
            "中文"
        ));
    }

    #[test]
    fn english_rejects_chinese_leftovers() {
        assert!(!fits_target("Hello 你好 world", "英语"));
        assert!(!fits_target(
            "Don't force me\nKaho 0:2 Focus\n辰漫\n浪漫 0:2 Happy",
            "英语"
        ));
        assert!(fits_target("Have you eaten yet today?", "英语"));
        assert!(fits_target(
            "Chen Man\nRomance 0:2 Happy\nKaho 0:2 Focus",
            "英语"
        ));
    }

    #[test]
    fn russian_rejects_latin_and_han_mix() {
        assert!(!fits_target(
            "Потребительские расходы составляют две thirds активности.",
            "俄语"
        ));
        assert!(!fits_target(
            "Экономика США по-прежнему保持着世界最大的规模。",
            "俄语"
        ));
        assert!(fits_target(
            "Инфляция замедляется, но цены на услуги остаются высокими.",
            "俄语"
        ));
    }

    #[test]
    fn arabic_rejects_latin_leftovers() {
        assert!(!fits_target(
            "central bank يحاول تجنب Recession.",
            "阿拉伯语"
        ));
    }

    #[test]
    fn compose_routes_chinese_draft_to_mt_for_english_learning() {
        assert_eq!(
            compose_mt_source("你好，今天吃饭了吗", "英语"),
            Some("中文")
        );
        assert_eq!(
            compose_mt_source("Have you eaten yet today?", "英语"),
            None
        );
        assert_eq!(compose_mt_source("你好", "中文"), None);
        // Too little signal → uncertain → LLM rewrite path
        assert_eq!(compose_mt_source("OK", "英语"), None);
    }
}
