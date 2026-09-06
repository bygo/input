use super::types::{DictEntry, DictSense};

pub(super) fn normalize(raw: &str) -> String {
    raw.trim()
        .trim_matches(|c: char| {
            matches!(
                c,
                '"' | '\''
                    | '“'
                    | '”'
                    | '‘'
                    | '’'
                    | '.'
                    | ','
                    | '!'
                    | '?'
                    | '。'
                    | '，'
                    | '！'
                    | '？'
                    | ';'
                    | '；'
                    | ':'
                    | '：'
            )
        })
        .to_lowercase()
}

pub(super) fn is_wordlike(raw: &str) -> bool {
    let t = raw.trim();
    if t.is_empty() || t.len() > 64 {
        return false;
    }

    if t.contains(['。', '！', '？', '.', '!', '?', '\n']) {
        return false;
    }
    let parts: Vec<_> = t.split_whitespace().collect();
    parts.len() <= 2 && parts.iter().all(|p| p.chars().any(|c| c.is_alphabetic()))
}

pub(super) fn parse_senses(translation: &str) -> Vec<DictSense> {
    let mut out = Vec::new();
    for line in translation.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }

        let mut pos = String::new();
        let mut gloss = line.to_string();
        if let Some(i) = line.find(". ") {
            let head = &line[..i];
            if head.len() <= 8 && head.chars().all(|c| c.is_ascii_alphabetic() || c == '/') {
                pos = head.to_string();
                gloss = line[i + 2..].trim().to_string();
            }
        } else if let Some(i) = line.find('.') {
            let head = &line[..i];
            if head.len() <= 8 && head.chars().all(|c| c.is_ascii_alphabetic()) {
                pos = head.to_string();
                gloss = line[i + 1..].trim().to_string();
            }
        }
        if gloss.is_empty() {
            continue;
        }
        out.push(DictSense { pos, gloss });
    }
    if out.is_empty() && !translation.trim().is_empty() {
        out.push(DictSense {
            pos: String::new(),
            gloss: translation.trim().replace('\n', "；"),
        });
    }
    out
}

pub(super) fn row_to_entry(
    word: String,
    phonetic: Option<String>,
    translation: Option<String>,
) -> DictEntry {
    let phonetic = phonetic
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .map(|p| {
            if p.starts_with('/') || p.starts_with('[') {
                p
            } else {
                format!("/{p}/")
            }
        });
    let senses = parse_senses(translation.as_deref().unwrap_or(""));
    DictEntry {
        word,
        phonetic,
        senses,
    }
}


pub(super) fn lemma_candidates(word: &str) -> Vec<String> {
    let w = word.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut push = |s: String| {
        if s.len() >= 2 && s != w && !out.iter().any(|x| x == &s) {
            out.push(s);
        }
    };
    if let Some(stem) = w.strip_suffix("ies").filter(|s| s.len() >= 2) {
        push(format!("{stem}y"));
    }
    for suf in ["ing", "ied", "ed", "es", "ly", "er", "est", "s"] {
        let Some(stem) = w.strip_suffix(suf).filter(|s| s.len() >= 2) else {
            continue;
        };
        push(stem.to_string());
        let b = stem.as_bytes();
        let n = b.len();
        if n >= 2 && b[n - 1] == b[n - 2] && (b[n - 1] as char).is_ascii_alphabetic() {
            push(stem[..n - 1].to_string());
        }
        if matches!(suf, "ing" | "ed" | "ied") {
            push(format!("{stem}e"));
        }
    }
    out
}
