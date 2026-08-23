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

/// macOS：主线程常驻 NSSpeechSynthesizer，避免每次 spawn `say`。
#[cfg(target_os = "macos")]
mod tts {
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};
    use std::cell::RefCell;
    use std::ffi::CString;

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    thread_local! {
        static SYNTH: RefCell<Option<(*mut AnyObject, String)>> = const { RefCell::new(None) };
    }

    fn ns_string(s: &str) -> *mut AnyObject {
        let c = CString::new(s).unwrap_or_default();
        unsafe {
            let cls = AnyClass::get(c"NSString").unwrap();
            msg_send![cls, stringWithUTF8String: c.as_ptr()]
        }
    }

    fn synth(voice: Option<&str>) -> *mut AnyObject {
        SYNTH.with(|slot| {
            let key = voice.unwrap_or("").to_string();
            let mut g = slot.borrow_mut();
            if let Some((p, v)) = g.as_ref() {
                if *v == key && !p.is_null() {
                    return *p;
                }
            }
            unsafe {
                let cls = AnyClass::get(c"NSSpeechSynthesizer").unwrap();
                let syn: *mut AnyObject = msg_send![cls, new];
                if let Some(want) = voice {
                    let voices: *mut AnyObject = msg_send![cls, availableVoices];
                    let n: usize = msg_send![voices, count];
                    let name_key = ns_string("VoiceName");
                    for i in 0..n {
                        let vid: *mut AnyObject = msg_send![voices, objectAtIndex: i];
                        let attrs: *mut AnyObject = msg_send![cls, attributesForVoice: vid];
                        if attrs.is_null() {
                            continue;
                        }
                        let name: *mut AnyObject = msg_send![attrs, objectForKey: name_key];
                        if name.is_null() {
                            continue;
                        }
                        let utf8: *const std::ffi::c_char = msg_send![name, UTF8String];
                        if utf8.is_null() {
                            continue;
                        }
                        let s = std::ffi::CStr::from_ptr(utf8).to_string_lossy();
                        if s.eq_ignore_ascii_case(want) {
                            let _: bool = msg_send![syn, setVoice: vid];
                            break;
                        }
                    }
                }
                *g = Some((syn, key));
                syn
            }
        })
    }

    pub fn stop() {
        SYNTH.with(|slot| {
            if let Some((p, _)) = slot.borrow().as_ref() {
                if !p.is_null() {
                    unsafe {
                        let _: () = msg_send![*p, stopSpeaking];
                    }
                }
            }
        });
    }

    pub fn speak(word: &str, voice: Option<&str>) {
        let syn = synth(voice);
        let s = ns_string(word);
        unsafe {
            let _: () = msg_send![syn, stopSpeaking];
            let _: bool = msg_send![syn, startSpeakingString: s];
        }
    }

    /// 进程启动时在主线程调用：创建单例并点一下语音服务。
    pub fn warm(voice: Option<&str>) {
        let syn = synth(voice);
        let s = ns_string(" ");
        unsafe {
            let _: bool = msg_send![syn, startSpeakingString: s];
            let _: () = msg_send![syn, stopSpeaking];
        }
    }
}

/// 在主线程调用（app setup 里）：进程起来就建好单例。
#[cfg(target_os = "macos")]
pub fn warm_tts() {
    tts::warm(Some("Samantha"));
}

/// Windows：进程内 ISpVoice 单例，不弹任何窗口、不启 PowerShell。
#[cfg(target_os = "windows")]
mod tts {
    use std::sync::Mutex;
    use windows::core::PCWSTR;
    use windows::Win32::Media::Speech::{ISpVoice, SpVoice, SPF_ASYNC, SPF_IS_NOT_XML, SPF_PURGEBEFORESPEAK};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};

    static VOICE: Mutex<Option<ISpVoice>> = Mutex::new(None);

    fn voice() -> Result<ISpVoice, String> {
        let mut g = VOICE.lock().map_err(|e| e.to_string())?;
        if let Some(v) = g.as_ref() {
            return Ok(v.clone());
        }
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let v: ISpVoice = CoCreateInstance(&SpVoice, None, CLSCTX_ALL)
                .map_err(|e| format!("初始化语音失败: {e}"))?;
            *g = Some(v.clone());
            Ok(v)
        }
    }

    pub fn warm() {
        let _ = voice();
    }

    pub fn speak(word: &str) -> Result<(), String> {
        let v = voice()?;
        let wide: Vec<u16> = word.encode_utf16().chain(std::iter::once(0)).collect();
        let flags = (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0 | SPF_IS_NOT_XML.0) as u32;
        unsafe {
            v.Speak(PCWSTR(wide.as_ptr()), flags, None)
                .map_err(|e| format!("发音失败: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub fn warm_tts() {
    tts::warm();
}

#[tauri::command]
pub fn dict_speak(app: AppHandle, word: String, lang: Option<String>) -> Result<(), String> {
    let w = word.trim();
    if w.is_empty() {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        use std::hash::{Hash, Hasher};
        use std::os::unix::process::CommandExt;
        // 简单按目标语选音色；无则系统默认。
        let voice = match lang.as_deref().unwrap_or("").trim() {
            "英语" | "en" | "English" => Some("Samantha"),
            "中文" | "zh" | "Chinese" => Some("Tingting"),
            _ => None,
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        w.hash(&mut hasher);
        voice.unwrap_or("").hash(&mut hasher);
        let cache = std::env::temp_dir().join(format!("input-tts-{:x}.aiff", hasher.finish()));
        let miss = !cache.is_file();
        let w2 = w.to_string();
        let voice2 = voice.map(|s| s.to_string());
        let cache2 = cache.clone();
        app.run_on_main_thread(move || {
            tts::stop();
            let _ = std::process::Command::new("killall")
                .args(["-q", "afplay"])
                .status();
            if cache2.is_file() {
                let _ = std::process::Command::new("afplay").arg(&cache2).spawn();
            } else {
                tts::speak(&w2, voice2.as_deref());
            }
        })
        .map_err(|e| format!("发音失败: {e}"))?;
        // 未命中缓存时后台写盘，下次走 afplay。
        if miss {
            let w3 = w.to_string();
            let cache3 = cache.clone();
            let tmp = cache.with_extension("aiff.part");
            let lock = cache.with_extension("aiff.lock");
            std::thread::spawn(move || {
                if cache3.is_file() {
                    return;
                }
                if std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&lock)
                    .is_err()
                {
                    return;
                }
                let mut c = std::process::Command::new("say");
                c.arg0("input-tts-cache");
                if let Some(v) = voice {
                    c.args(["-v", v]);
                }
                let ok = c.arg("-o").arg(&tmp).arg(&w3).status().map(|s| s.success()).unwrap_or(false);
                if ok {
                    let _ = std::fs::rename(&tmp, &cache3);
                } else {
                    let _ = std::fs::remove_file(&tmp);
                }
                let _ = std::fs::remove_file(&lock);
            });
        }
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        let _ = (app, lang);
        tts::speak(w)?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (app, lang);
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
