use super::Handle;
use super::os;
use arboard::Clipboard;
use std::thread;
use std::time::{Duration, Instant};

fn accessibility_trusted(prompt: bool) -> bool {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::CFDictionary;
    use core_foundation::string::CFString;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        static kAXTrustedCheckOptionPrompt: core_foundation::string::CFStringRef;
        fn AXIsProcessTrusted() -> u8;
        fn AXIsProcessTrustedWithOptions(
            options: core_foundation::dictionary::CFDictionaryRef,
        ) -> u8;
    }

    unsafe {
        if !prompt {
            return AXIsProcessTrusted() != 0;
        }
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let dict: CFDictionary<CFString, CFBoolean> =
            CFDictionary::from_CFType_pairs(&[(key, CFBoolean::true_value())]);
        AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef()) != 0
    }
}

pub(super) fn ensure_accessibility() -> Result<(), String> {
    if accessibility_trusted(false) {
        return Ok(());
    }
    let _ = accessibility_trusted(true);
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
    let path = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "src-tauri/target/debug/input".into());
    Err(format!(
        "无法回写：请在系统设置 › 隐私与安全性 › 辅助功能 中点「+」，添加并打开：\n{path}\n授权后重新运行 tauri dev。"
    ))
}
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceKeyState(state_id: i32, key: u16) -> u8;
}
fn hid_key_down(key: u16) -> bool {
    unsafe { CGEventSourceKeyState(1, key) != 0 }
}

fn post_key(ev: &core_graphics::event::CGEvent, _pid: Option<Handle>) {
    use core_graphics::event::CGEventTapLocation;
    ev.post(CGEventTapLocation::HID);
}

fn send_cmd_key(key: u16, press_command: bool, pid: Option<Handle>) -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, KeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};


    const NON_COALESCED: CGEventFlags = CGEventFlags::from_bits_retain(0x8);

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .or_else(|_| CGEventSource::new(CGEventSourceStateID::HIDSystemState))
        .map_err(|()| "无法创建按键事件".to_string())?;
    let flags = CGEventFlags::CGEventFlagCommand | NON_COALESCED;

    let option_held = hid_key_down(KeyCode::OPTION) || hid_key_down(KeyCode::RIGHT_OPTION);
    if option_held {
        let opt_up = CGEvent::new_keyboard_event(source.clone(), KeyCode::OPTION, false)
            .map_err(|()| "无法创建按键事件".to_string())?;
        opt_up.set_flags(NON_COALESCED);
        post_key(&opt_up, pid);
        thread::sleep(Duration::from_millis(8));
    }
    let command_held = hid_key_down(KeyCode::COMMAND) || hid_key_down(KeyCode::RIGHT_COMMAND);
    let need_command = press_command && !command_held;
    if need_command {
        let cmd_down = CGEvent::new_keyboard_event(source.clone(), KeyCode::COMMAND, true)
            .map_err(|()| "无法创建按键事件".to_string())?;
        cmd_down.set_flags(flags);
        post_key(&cmd_down, pid);
        thread::sleep(Duration::from_millis(16));
    }
    let down = CGEvent::new_keyboard_event(source.clone(), key, true)
        .map_err(|()| "无法创建按键事件".to_string())?;
    let up = CGEvent::new_keyboard_event(source.clone(), key, false)
        .map_err(|()| "无法创建按键事件".to_string())?;
    down.set_flags(flags);
    up.set_flags(flags);
    post_key(&down, pid);
    thread::sleep(Duration::from_millis(8));
    post_key(&up, pid);
    if need_command {
        thread::sleep(Duration::from_millis(16));
        let cmd_up = CGEvent::new_keyboard_event(source.clone(), KeyCode::COMMAND, false)
            .map_err(|()| "无法创建按键事件".to_string())?;
        cmd_up.set_flags(NON_COALESCED);
        post_key(&cmd_up, pid);
    }
    if option_held {
        let opt_down = CGEvent::new_keyboard_event(source, KeyCode::OPTION, true)
            .map_err(|()| "无法创建按键事件".to_string())?;
        opt_down.set_flags(CGEventFlags::CGEventFlagAlternate | NON_COALESCED);
        post_key(&opt_down, pid);
    }
    Ok(())
}

pub(super) fn send_paste(pid: Option<Handle>) -> Result<(), String> {
    send_cmd_key(0x09, true, pid)
}

pub(super) fn copy_selected_via_clipboard() -> Option<String> {
    const KEY_C: u16 = 0x08;
    const MARKER: &str = "\u{200B}§input-sel§";

    let mut clipboard = Clipboard::new().ok()?;
    let previous = clipboard.get_text().ok();
    let before = os::pasteboard_count();

    let use_marker = before.is_none();
    if use_marker {
        clipboard.set_text(MARKER).ok()?;
        thread::sleep(Duration::from_millis(4));
    }
    let sent = send_cmd_key(KEY_C, true, None);
    let mut got = None;
    if sent.is_ok() {
        let deadline = Instant::now() + Duration::from_millis(120);
        while Instant::now() < deadline {
            let count_changed = match (before, os::pasteboard_count()) {
                (Some(a), Some(b)) => a != b,
                (Some(_), None) => false,
                (None, _) => true,
            };
            if count_changed {
                if let Ok(text) = clipboard.get_text() {
                    if !(use_marker && (text == MARKER || text.trim().is_empty())) {
                        let t = text.trim();
                        if !t.is_empty() {
                            got = Some(t.to_string());
                            break;
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(8));
        }
    }

    if let Some(prev) = previous {
        let _ = clipboard.set_text(&prev);
    } else if use_marker {
        let _ = clipboard.set_text("");
    }
    got
}
