use std::str::FromStr;
use tauri_plugin_global_shortcut::{Modifiers, Shortcut};

fn key_token(code: tauri_plugin_global_shortcut::Code) -> String {
    let s = format!("{code:?}");
    s.strip_prefix("Key")
        .or_else(|| s.strip_prefix("Digit"))
        .unwrap_or(&s)
        .to_string()
}

fn is_fn_key(code: tauri_plugin_global_shortcut::Code) -> bool {
    let s = format!("{code:?}");
    s.starts_with('F') && s.chars().nth(1).is_some_and(|c| c.is_ascii_digit())
}

pub fn normalize_shortcut(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }
    let hk = Shortcut::from_str(raw).map_err(|e| format!("无效快捷键: {e}"))?;
    let has_cmd = hk.mods.contains(Modifiers::SUPER);
    let has_ctrl = hk.mods.contains(Modifiers::CONTROL);
    let has_alt = hk.mods.contains(Modifiers::ALT);
    let has_shift = hk.mods.contains(Modifiers::SHIFT);
    if !has_cmd && !has_ctrl && !has_alt && !is_fn_key(hk.key) {
        return Err("请加上 Command、Control 或 Option".into());
    }
    let mut parts: Vec<String> = Vec::new();
    if has_cmd {
        parts.push("Command".into());
    }
    if has_ctrl {
        parts.push("Control".into());
    }
    if has_alt {
        parts.push("Alt".into());
    }
    if has_shift {
        parts.push("Shift".into());
    }
    parts.push(key_token(hk.key));
    Ok(parts.join("+"))
}

pub fn same_shortcut(a: &str, b: &str) -> bool {
    match (Shortcut::from_str(a.trim()), Shortcut::from_str(b.trim())) {
        (Ok(x), Ok(y)) => x.key == y.key && x.mods == y.mods,
        _ => a.trim().eq_ignore_ascii_case(b.trim()),
    }
}
