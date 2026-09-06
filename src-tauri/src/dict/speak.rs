use tauri::AppHandle;

#[cfg(target_os = "macos")]
use super::tts_macos as tts;

#[cfg(target_os = "windows")]
use super::tts_windows as tts;


#[cfg(target_os = "macos")]
pub fn warm_tts() {
    tts::warm(Some("Samantha"));
}

#[cfg(target_os = "windows")]
pub fn warm_tts() {
    tts::warm();
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn warm_tts() {}

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
            tts::stop_afplay();
            if cache2.is_file() {
                tts::play_afplay(&cache2);
            } else {
                tts::speak(&w2, voice2.as_deref());
            }
        })
        .map_err(|e| format!("发音失败: {e}"))?;

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
                let ok = c
                    .arg("-o")
                    .arg(&tmp)
                    .arg(&w3)
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
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

#[tauri::command]
pub fn dict_stop(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        app.run_on_main_thread(|| {
            tts::stop();
            tts::stop_afplay();
        })
        .map_err(|e| format!("停止发音失败: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        let _ = app;
        return tts::stop();
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = app;
        Ok(())
    }
}

#[tauri::command]
pub fn dict_speaking(app: AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let _ = tx.send(tts::is_speaking());
        })
        .map_err(|e| format!("查询发音状态失败: {e}"))?;
        return rx.recv().map_err(|e| e.to_string());
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Ok(false)
    }
}
