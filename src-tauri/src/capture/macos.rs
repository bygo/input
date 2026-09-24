use super::state::NativeMonitor;
use block2::RcBlock;
use objc2_app_kit::{NSEvent, NSEventMask, NSEventModifierFlags};
use std::ptr::NonNull;
use tauri::{AppHandle, Emitter, Manager};

fn is_modifier(code: u16) -> bool {
    matches!(
        code,
        0x36 | 0x37 | 0x38 | 0x39 | 0x3A | 0x3B | 0x3C | 0x3D | 0x3E | 0x3F
    )
}

fn token(code: u16) -> Option<&'static str> {
    Some(match code {
        0x00 => "A",
        0x01 => "S",
        0x02 => "D",
        0x03 => "F",
        0x04 => "H",
        0x05 => "G",
        0x06 => "Z",
        0x07 => "X",
        0x08 => "C",
        0x09 => "V",
        0x0B => "B",
        0x0C => "Q",
        0x0D => "W",
        0x0E => "E",
        0x0F => "R",
        0x10 => "Y",
        0x11 => "T",
        0x12 => "1",
        0x13 => "2",
        0x14 => "3",
        0x15 => "4",
        0x16 => "6",
        0x17 => "5",
        0x18 => "Equal",
        0x19 => "9",
        0x1A => "7",
        0x1B => "Minus",
        0x1C => "8",
        0x1D => "0",
        0x1E => "BracketRight",
        0x1F => "O",
        0x20 => "U",
        0x21 => "BracketLeft",
        0x22 => "I",
        0x23 => "P",
        0x24 => "Enter",
        0x25 => "L",
        0x26 => "J",
        0x27 => "Quote",
        0x28 => "K",
        0x29 => "Semicolon",
        0x2A => "Backslash",
        0x2B => "Comma",
        0x2C => "Slash",
        0x2D => "N",
        0x2E => "M",
        0x2F => "Period",
        0x30 => "Tab",
        0x31 => "Space",
        0x32 => "Backquote",
        0x33 => "Backspace",
        0x35 => "Escape",
        0x60 => "F5",
        0x61 => "F6",
        0x62 => "F7",
        0x63 => "F3",
        0x64 => "F8",
        0x65 => "F9",
        0x67 => "F11",
        0x69 => "F13",
        0x6A => "F16",
        0x6B => "F14",
        0x6D => "F10",
        0x6F => "F12",
        0x71 => "F15",
        0x76 => "F4",
        0x78 => "F2",
        0x7A => "F1",
        0x7B => "ArrowLeft",
        0x7C => "ArrowRight",
        0x7D => "ArrowDown",
        0x7E => "ArrowUp",
        _ => return None,
    })
}

pub fn start(app: &AppHandle) {
    stop(app);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let emit = handle.clone();
        let block = RcBlock::new(move |ev: NonNull<NSEvent>| -> *mut NSEvent {
            let event = unsafe { ev.as_ref() };
            if event.isARepeat() {
                return std::ptr::null_mut();
            }
            let code = event.keyCode();
            if is_modifier(code) {
                return ev.as_ptr();
            }
            let flags = event.modifierFlags();
            let cmd = flags.contains(NSEventModifierFlags::Command);
            let ctrl = flags.contains(NSEventModifierFlags::Control);
            let opt = flags.contains(NSEventModifierFlags::Option);
            let shift = flags.contains(NSEventModifierFlags::Shift);
            if code == 0x35 && !cmd && !ctrl && !opt && !shift {
                let _ = emit.emit("shortcut-capture-cancel", ());
                return std::ptr::null_mut();
            }
            let Some(key) = token(code) else {
                return ev.as_ptr();
            };
            let fn_key =
                key.starts_with('F') && key.as_bytes().get(1).is_some_and(u8::is_ascii_digit);
            if !cmd && !ctrl && !opt && !fn_key {
                return std::ptr::null_mut();
            }
            let mut parts = Vec::new();
            if cmd {
                parts.push("Command");
            }
            if ctrl {
                parts.push("Control");
            }
            if opt {
                parts.push("Alt");
            }
            if shift {
                parts.push("Shift");
            }
            parts.push(key);
            let _ = emit.emit("shortcut-captured", parts.join("+"));
            std::ptr::null_mut()
        });
        let monitor = unsafe {
            NSEvent::addLocalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, &block)
        };
        let _ = block;
        let Some(slot) = handle.try_state::<NativeMonitor>() else {
            return;
        };
        let Ok(mut g) = slot.0.lock() else {
            return;
        };
        g.monitor = monitor;
    });
}

pub fn stop(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(slot) = handle.try_state::<NativeMonitor>() else {
            return;
        };
        let Ok(mut g) = slot.0.lock() else {
            return;
        };
        let Some(m) = g.monitor.take() else {
            return;
        };
        unsafe { NSEvent::removeMonitor(&m) };
    });
}
