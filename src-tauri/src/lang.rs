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
}
