use std::sync::Mutex;
use tauri::{AppHandle, Manager};

pub struct NativeMonitor(Mutex<Slot>);

pub struct Slot {
    #[cfg(target_os = "macos")]
    monitor: Option<objc2::rc::Retained<objc2::runtime::AnyObject>>,
    #[cfg(target_os = "macos")]
    tap: Option<*mut std::ffi::c_void>,
    #[cfg(target_os = "macos")]
    global: Option<objc2::rc::Retained<objc2::runtime::AnyObject>>,
}

unsafe impl Send for Slot {}
unsafe impl Sync for Slot {}

pub struct Bindings(pub Mutex<(String, String)>);

pub fn managed() -> NativeMonitor {
    NativeMonitor(Mutex::new(Slot {
        #[cfg(target_os = "macos")]
        monitor: None,
        #[cfg(target_os = "macos")]
        tap: None,
        #[cfg(target_os = "macos")]
        global: None,
    }))
}

pub fn managed_bindings() -> Bindings {
    Bindings(Mutex::new((
        crate::settings::default_compose_shortcut(),
        crate::settings::default_select_shortcut(),
    )))
}

pub fn set_bindings(app: &AppHandle, compose: String, select: String) {
    if let Some(b) = app.try_state::<Bindings>() {
        *b.0.lock().unwrap() = (compose, select);
    }
    #[cfg(target_os = "macos")]
    macos::listen(app);
}

pub fn listen(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    macos::listen(app);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

pub fn start(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    macos::start(app);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

pub fn stop(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    macos::stop(app);
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[cfg(target_os = "macos")]
mod macos {
    use super::NativeMonitor;
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
            let slot_handle = handle.clone();
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
            if let Some(slot) = slot_handle.try_state::<NativeMonitor>() {
                if let Ok(mut g) = slot.0.lock() {
                    g.monitor = monitor;
                }
            }
        });
    }

    pub fn stop(app: &AppHandle) {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Some(slot) = handle.try_state::<NativeMonitor>() {
                if let Ok(mut g) = slot.0.lock() {
                    if let Some(m) = g.monitor.take() {
                        unsafe { NSEvent::removeMonitor(&m) };
                    }
                }
            }
        });
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: unsafe extern "C" fn(
                *const std::ffi::c_void,
                u32,
                *mut std::ffi::c_void,
                *mut std::ffi::c_void,
            ) -> *mut std::ffi::c_void,
            user_info: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
        fn CGEventTapEnable(tap: *mut std::ffi::c_void, enable: bool);
        fn CGEventGetIntegerValueField(event: *mut std::ffi::c_void, field: u32) -> i64;
        fn CGEventGetFlags(event: *mut std::ffi::c_void) -> u64;
        fn CGRequestListenEventAccess() -> bool;
        fn CGPreflightListenEventAccess() -> bool;
    }

    fn match_hotkey(app: &AppHandle, pressed: &str) -> Option<u8> {
        let bound = app
            .try_state::<super::Bindings>()
            .and_then(|b| b.0.lock().ok().map(|g| g.clone()))?;
        if crate::settings::same_shortcut(pressed, &bound.0) {
            return Some(0);
        }
        if crate::settings::same_shortcut(pressed, &bound.1) {
            return Some(1);
        }
        None
    }

    fn pressed_from(code: u16, cmd: bool, ctrl: bool, opt: bool, shift: bool) -> Option<String> {
        if is_modifier(code) {
            return None;
        }
        let key = token(code)?;
        let fn_key = key.starts_with('F') && key.as_bytes().get(1).is_some_and(u8::is_ascii_digit);
        if !cmd && !ctrl && !opt && !fn_key {
            return None;
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
        Some(parts.join("+"))
    }

    unsafe extern "C" fn tap_cb(
        _proxy: *const std::ffi::c_void,
        etype: u32,
        event: *mut std::ffi::c_void,
        user: *mut std::ffi::c_void,
    ) -> *mut std::ffi::c_void {
        if etype == 0xFFFFFFFE || etype == 0xFFFFFFFF {
            if let Some(slot) = unsafe { &*(user as *const AppHandle) }.try_state::<NativeMonitor>()
            {
                if let Ok(g) = slot.0.lock() {
                    if let Some(tap) = g.tap {
                        unsafe { CGEventTapEnable(tap, true) };
                    }
                }
            }
            return event;
        }
        if etype != 10 {
            return event;
        }
        let app = unsafe { &*(user as *const AppHandle) };
        if crate::capturing_shortcut(app) {
            return event;
        }
        let code = unsafe { CGEventGetIntegerValueField(event, 9) } as u16;
        if is_modifier(code) {
            return event;
        }
        let flags = unsafe { CGEventGetFlags(event) };
        let cmd = flags & 0x00100000 != 0;
        let ctrl = flags & 0x00040000 != 0;
        let opt = flags & 0x00080000 != 0;
        let shift = flags & 0x00020000 != 0;
        let Some(pressed) = pressed_from(code, cmd, ctrl, opt, shift) else {
            return event;
        };
        if let Some(which) = match_hotkey(app, &pressed) {
            crate::dispatch_hotkey(app, which);
            return std::ptr::null_mut();
        }
        event
    }

    fn attach_tap(location: u32, user: *mut std::ffi::c_void) -> *mut std::ffi::c_void {
        unsafe { CGEventTapCreate(location, 0, 0, 1u64 << 10, tap_cb, user) }
    }

    pub fn listen(app: &AppHandle) {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            unsafe {
                if !CGPreflightListenEventAccess() {
                    let _ = CGRequestListenEventAccess();
                }
            }
            if let Some(slot) = handle.try_state::<NativeMonitor>() {
                if let Ok(mut g) = slot.0.lock() {
                    if let Some(old) = g.tap.take() {
                        unsafe { CGEventTapEnable(old, false) };
                    }
                }
            }
            let user = Box::into_raw(Box::new(handle.clone())) as *mut std::ffi::c_void;
            let mut tap = attach_tap(0, user);
            if tap.is_null() {
                tap = attach_tap(1, user);
            }
            if tap.is_null() {
                unsafe { drop(Box::from_raw(user as *mut AppHandle)) };
                let _ = std::process::Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent")
                    .spawn();
                start_global_backup(&handle);
                return;
            }
            use core_foundation::base::TCFType;
            use core_foundation::mach_port::CFMachPort;
            use core_foundation::runloop::{kCFRunLoopCommonModes, CFRunLoop};
            let port = unsafe { CFMachPort::wrap_under_create_rule(tap as _) };
            if let Ok(source) = port.create_runloop_source(0) {
                unsafe {
                    CFRunLoop::get_current().add_source(&source, kCFRunLoopCommonModes);
                }
                std::mem::forget(source);
            }
            unsafe { CGEventTapEnable(tap, true) };
            std::mem::forget(port);
            if let Some(slot) = handle.try_state::<NativeMonitor>() {
                if let Ok(mut g) = slot.0.lock() {
                    g.tap = Some(tap);
                }
            }
            start_global_backup(&handle);
        });
    }

    fn start_global_backup(app: &AppHandle) {
        use std::ptr::NonNull;
        if let Some(slot) = app.try_state::<NativeMonitor>() {
            if let Ok(mut g) = slot.0.lock() {
                if let Some(old) = g.global.take() {
                    unsafe { NSEvent::removeMonitor(&old) };
                }
            }
        }
        let emit = app.clone();
        let block = RcBlock::new(move |ev: NonNull<NSEvent>| {
            let event = unsafe { ev.as_ref() };
            if crate::capturing_shortcut(&emit) || event.isARepeat() {
                return;
            }
            let code = event.keyCode();
            let flags = event.modifierFlags();
            let cmd = flags.contains(NSEventModifierFlags::Command);
            let ctrl = flags.contains(NSEventModifierFlags::Control);
            let opt = flags.contains(NSEventModifierFlags::Option);
            let shift = flags.contains(NSEventModifierFlags::Shift);
            let Some(pressed) = pressed_from(code, cmd, ctrl, opt, shift) else {
                return;
            };
            if let Some(which) = match_hotkey(&emit, &pressed) {
                crate::dispatch_hotkey(&emit, which);
            }
        });
        let monitor =
            NSEvent::addGlobalMonitorForEventsMatchingMask_handler(NSEventMask::KeyDown, &block);
        std::mem::forget(block);
        if let Some(slot) = app.try_state::<NativeMonitor>() {
            if let Ok(mut g) = slot.0.lock() {
                g.global = monitor;
            }
        }
    }
}
