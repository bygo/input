use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictSense {
    pub pos: String,
    pub gloss: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DictEntry {
    pub word: String,
    pub phonetic: Option<String>,
    pub senses: Vec<DictSense>,
}

pub struct DictDb(pub Mutex<Option<Connection>>);

pub fn managed() -> DictDb {
    DictDb(Mutex::new(None))
}

fn db_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    out.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/ecdict.db"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("resources/ecdict.db"));
            out.push(dir.join("../Resources/resources/ecdict.db"));
            out.push(dir.join("../Resources/ecdict.db"));
        }
    }
    out
}

fn open_db() -> Result<Connection, String> {
    for p in db_candidates() {
        if p.is_file() {
            let c = Connection::open_with_flags(
                &p,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .map_err(|e| format!("打开词库失败: {e}"))?;
            return Ok(c);
        }
    }
    Err("未找到词库 ecdict.db".into())
}

fn with_db<T>(app: &AppHandle, f: impl FnOnce(&Connection) -> Result<T, String>) -> Result<T, String> {
    let state = app.state::<DictDb>();
    let mut guard = state.0.lock().map_err(|_| "词库锁失败".to_string())?;
    if guard.is_none() {
        *guard = Some(open_db()?);
    }
    f(guard.as_ref().unwrap())
}

fn normalize(raw: &str) -> String {
    raw.trim()
        .trim_matches(|c: char| matches!(c, '"' | '\'' | '“' | '”' | '‘' | '’' | '.' | ',' | '!' | '?' | '。' | '，' | '！' | '？' | ';' | '；' | ':' | '：'))
        .to_lowercase()
}

fn is_wordlike(raw: &str) -> bool {
    let t = raw.trim();
    if t.is_empty() || t.len() > 64 {
        return false;
    }
    // 单 token 或短词组（≤2 词），不含句号类标点。
    if t.contains(['。', '！', '？', '.', '!', '?', '\n']) {
        return false;
    }
    let parts: Vec<_> = t.split_whitespace().collect();
    parts.len() <= 2 && parts.iter().all(|p| p.chars().any(|c| c.is_alphabetic()))
}

fn parse_senses(translation: &str) -> Vec<DictSense> {
    let mut out = Vec::new();
    for line in translation.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('[') {
            continue;
        }
        // "n. 苹果, 家伙" / "vt. 拿, 取" / "a. 美丽的"
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

fn row_to_entry(word: String, phonetic: Option<String>, translation: Option<String>) -> DictEntry {
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

fn query_exact(conn: &Connection, word: &str) -> Result<Option<DictEntry>, String> {
    conn.query_row(
        "SELECT word, phonetic, translation FROM stardict WHERE word = ?1 COLLATE NOCASE LIMIT 1",
        [word],
        |r| {
            Ok(row_to_entry(
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

fn query_via_exchange(conn: &Connection, word: &str) -> Result<Option<DictEntry>, String> {
    // exchange 形如 "0:run/1:runs/d:ran/p:run/i:running"
    let like = format!("%:{word}/%");
    let like_end = format!("%:{word}");
    conn.query_row(
        "SELECT word, phonetic, translation FROM stardict
         WHERE exchange LIKE ?1 OR exchange LIKE ?2
         LIMIT 1",
        rusqlite::params![like, like_end],
        |r| {
            Ok(row_to_entry(
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn dict_lookup(app: AppHandle, word: String) -> Result<Option<DictEntry>, String> {
    if !is_wordlike(&word) {
        return Ok(None);
    }
    let key = normalize(&word);
    if key.is_empty() {
        return Ok(None);
    }
    with_db(&app, |conn| {
        if let Some(e) = query_exact(conn, &key)? {
            return Ok(Some(e));
        }
        query_via_exchange(conn, &key)
    })
}

#[tauri::command]
pub fn dict_speak(word: String, lang: Option<String>) -> Result<(), String> {
    let w = word.trim();
    if w.is_empty() {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let mut cmd = std::process::Command::new("say");
        // 简单按目标语选音色；无则系统默认。
        let voice = match lang.as_deref().unwrap_or("").trim() {
            "英语" | "en" | "English" => Some("Samantha"),
            "中文" | "zh" | "Chinese" => Some("Ting-Ting"),
            _ => None,
        };
        if let Some(v) = voice {
            cmd.args(["-v", v]);
        }
        cmd.arg(w)
            .spawn()
            .map_err(|e| format!("发音失败: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        let _ = lang;
        let escaped = w.replace('\'', "''");
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Add-Type -AssemblyName System.Speech; (New-Object System.Speech.Synthesis.SpeechSynthesizer).Speak('{escaped}')"
                ),
            ])
            .spawn()
            .map_err(|e| format!("发音失败: {e}"))?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = lang;
        Err("当前平台暂不支持发音".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wordlike_accepts_single_and_short_phrase() {
        assert!(is_wordlike("apple"));
        assert!(is_wordlike("look up"));
        assert!(!is_wordlike("I have an apple."));
        assert!(!is_wordlike(""));
    }

    #[test]
    fn parse_translation_lines() {
        let s = parse_senses("n. 苹果, 家伙\n[医] 苹果\nvt. 拿, 取");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].pos, "n");
        assert!(s[0].gloss.contains("苹果"));
        assert_eq!(s[1].pos, "vt");
    }

    #[test]
    fn lookup_fixture_if_present() {
        let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/ecdict.db");
        if !p.is_file() {
            return;
        }
        let conn = Connection::open_with_flags(
            &p,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let e = query_exact(&conn, "apple").unwrap().expect("apple");
        assert_eq!(e.word.to_lowercase(), "apple");
        assert!(e.phonetic.is_some());
        assert!(!e.senses.is_empty());
    }
}
