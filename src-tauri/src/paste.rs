use arboard::Clipboard;
#[cfg(target_os = "macos")]
use std::thread;
#[cfg(target_os = "macos")]
use std::time::Instant;
use std::time::Duration;
use tauri::{AppHandle, Manager};

#[cfg(not(target_os = "macos"))]
use enigo::{Direction, Enigo, Key, Keyboard, Settings};

struct Saved {
    handle: Option<Handle>,
    clipboard: Option<String>,
    #[cfg(target_os = "macos")]
    ax: Option<ax::Elem>,
    selected: Option<String>,
}

pub struct PasteTarget(std::sync::Mutex<Option<Saved>>);

pub fn managed() -> PasteTarget {
    PasteTarget(std::sync::Mutex::new(None))
}

// --- platform frontmost window ---

#[cfg(target_os = "macos")]
type Handle = i32;

#[cfg(target_os = "windows")]
type Handle = isize;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
type Handle = ();

#[cfg(target_os = "macos")]
mod os {
    use super::Handle;
    use objc2::msg_send;
    use objc2::runtime::{AnyClass, AnyObject};

    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    pub fn frontmost() -> Option<Handle> {
        unsafe {
            let cls = AnyClass::get(c"NSWorkspace")?;
            let ws: *mut AnyObject = msg_send![cls, sharedWorkspace];
            if ws.is_null() {
                return None;
            }
            let app: *mut AnyObject = msg_send![ws, frontmostApplication];
            if app.is_null() {
                return None;
            }
            let pid: i32 = msg_send![app, processIdentifier];
            if pid == std::process::id() as i32 {
                return None;
            }
            Some(pid)
        }
    }

    pub fn activate(pid: Handle) {
        unsafe {
            let Some(cls) = AnyClass::get(c"NSRunningApplication") else {
                return;
            };
            let app: *mut AnyObject = msg_send![cls, runningApplicationWithProcessIdentifier: pid];
            if app.is_null() {
                return;
            }
            let _: bool = msg_send![app, activateWithOptions: 3u64];
        }
    }

    pub fn pasteboard_count() -> Option<isize> {
        unsafe {
            let cls = AnyClass::get(c"NSPasteboard")?;
            let pb: *mut AnyObject = msg_send![cls, generalPasteboard];
            if pb.is_null() {
                return None;
            }
            let count: isize = msg_send![pb, changeCount];
            Some(count)
        }
    }
}

#[cfg(target_os = "macos")]
mod ax {
    use core_foundation::base::{CFRelease, CFRetain, CFTypeRef, TCFType};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::c_void;
    use std::ptr;

    pub type AXUIElementRef = *const c_void;
    const SUCCESS: i32 = 0;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> AXUIElementRef;
        fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
        fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> i32;
        fn AXUIElementSetAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: CFTypeRef,
        ) -> i32;
        fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32;
        fn AXUIElementGetPid(element: AXUIElementRef, pid: *mut i32) -> i32;
        fn AXUIElementCopyParameterizedAttributeValue(
            element: AXUIElementRef,
            parameterized_attribute: CFStringRef,
            parameter: CFTypeRef,
            value: *mut CFTypeRef,
        ) -> i32;
        fn AXValueCreate(type_: u32, value_ptr: *const c_void) -> CFTypeRef;
        fn AXValueGetValue(value: CFTypeRef, type_: u32, value_ptr: *mut c_void) -> u8;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFArrayGetCount(the_array: CFTypeRef) -> isize;
        fn CFArrayGetValueAtIndex(the_array: CFTypeRef, idx: isize) -> *const c_void;
    }

    const AX_CGPOINT: u32 = 1;
    const AX_CGSIZE: u32 = 2;
    const AX_CGRECT: u32 = 3;
    const AX_CFRANGE: u32 = 4;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CfRange {
        location: isize,
        length: isize,
    }

    pub struct Elem(AXUIElementRef);

    unsafe impl Send for Elem {}
    unsafe impl Sync for Elem {}

    impl Drop for Elem {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { CFRelease(self.0) }
            }
        }
    }

    impl Elem {
        pub fn focused() -> Option<Self> {
            Self::focused_from(unsafe { AXUIElementCreateSystemWide() }).or_else(|| {
                let pid = super::os::frontmost()?;
                Self::focused_from(unsafe { AXUIElementCreateApplication(pid) })
            })
        }

        fn focused_from(from: AXUIElementRef) -> Option<Self> {
            unsafe {
                if from.is_null() {
                    return None;
                }
                let mut value: CFTypeRef = ptr::null();
                let err = AXUIElementCopyAttributeValue(
                    from,
                    CFString::new("AXFocusedUIElement").as_concrete_TypeRef(),
                    &mut value,
                );
                CFRelease(from);
                if err != SUCCESS || value.is_null() {
                    return None;
                }
                let el = value as AXUIElementRef;
                let mut pid = 0i32;
                if AXUIElementGetPid(el, &mut pid) == SUCCESS && pid == std::process::id() as i32 {
                    CFRelease(el);
                    return None;
                }
                Some(Elem(el))
            }
        }

        pub fn focus(&self) -> bool {
            unsafe {
                let mut window: CFTypeRef = ptr::null();
                if AXUIElementCopyAttributeValue(
                    self.0,
                    CFString::new("AXWindow").as_concrete_TypeRef(),
                    &mut window,
                ) == SUCCESS
                    && !window.is_null()
                {
                    let _ = AXUIElementPerformAction(
                        window as AXUIElementRef,
                        CFString::new("AXRaise").as_concrete_TypeRef(),
                    );
                    CFRelease(window);
                }
                AXUIElementSetAttributeValue(
                    self.0,
                    CFString::new("AXFocused").as_concrete_TypeRef(),
                    CFBoolean::true_value().as_CFTypeRef(),
                ) == SUCCESS
            }
        }

        fn set_attr_string(&self, name: &str, text: &str) -> bool {
            unsafe {
                AXUIElementSetAttributeValue(
                    self.0,
                    CFString::new(name).as_concrete_TypeRef(),
                    CFString::new(text).as_CFTypeRef(),
                ) == SUCCESS
            }
        }

        pub fn insert(&self, text: &str) -> bool {
            self.set_attr_string("AXSelectedText", text)
        }

        pub fn value(&self) -> Option<String> {
            let v = self.copy_attr("AXValue")?;
            let s = unsafe { CFString::wrap_under_create_rule(v as CFStringRef) };
            Some(s.to_string())
        }

        fn set_value(&self, text: &str) -> bool {
            self.set_attr_string("AXValue", text)
        }

        fn set_selected_range(&self, range: CfRange) -> bool {
            unsafe {
                let v = AXValueCreate(AX_CFRANGE, &range as *const _ as *const c_void);
                if v.is_null() {
                    return false;
                }
                let ok = AXUIElementSetAttributeValue(
                    self.0,
                    CFString::new("AXSelectedTextRange").as_concrete_TypeRef(),
                    v,
                ) == SUCCESS;
                CFRelease(v);
                ok
            }
        }

        fn expected_after(before: &str, range: CfRange, text: &str) -> String {
            let u: Vec<u16> = before.encode_utf16().collect();
            let loc = range.location.clamp(0, u.len() as isize) as usize;
            let end = (loc + range.length.max(0) as usize).min(u.len());
            let mut out = String::from_utf16_lossy(&u[..loc]);
            out.push_str(text);
            out.push_str(&String::from_utf16_lossy(&u[end..]));
            out
        }

        fn in_web(&self) -> bool {
            if self.role().as_deref() == Some("AXWebArea") {
                return true;
            }
            let mut cur = self.parent();
            for _ in 0..16 {
                let Some(el) = cur else { break };
                if el.role().as_deref() == Some("AXWebArea") {
                    return true;
                }
                cur = el.parent();
            }
            false
        }

        pub fn insert_text(&self, text: &str, replace: bool) -> bool {
            if self.in_web() && !replace {
                return false;
            }
            let before = self.value();
            let range = self.selected_range();
            if self.insert(text) {
                if replace || self.in_web() {
                    return true;
                }
                if let Some(after) = self.value() {
                    if let (Some(before), Some(range)) = (before.as_deref(), range) {
                        if after == Self::expected_after(before, range, text)
                            || after.contains(text)
                        {
                            return true;
                        }
                    } else if after.contains(text) {
                        return true;
                    }
                }
            }
            if self.in_web() {
                return false;
            }
            let Some(before) = self.value().or(before) else {
                return false;
            };
            let Some(range) = range.or_else(|| self.selected_range()) else {
                return false;
            };
            let expected = Self::expected_after(&before, range, text);
            if !self.set_value(&expected) {
                return false;
            }
            let caret = range.location.max(0) + text.encode_utf16().count() as isize;
            let _ = self.set_selected_range(CfRange {
                location: caret,
                length: 0,
            });
            self.value()
                .is_some_and(|v| v == expected || v.contains(text))
        }

        pub fn selected_text(&self) -> Option<String> {
            let v = self.copy_attr("AXSelectedText")?;
            let s = unsafe { CFString::wrap_under_create_rule(v as CFStringRef) };
            let t = s.to_string();
            let t = t.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }

        pub fn capture_selected(&self) -> Option<String> {
            if let Some(t) = self.selected_text() {
                return Some(t);
            }
            if let Some(range) = self.selected_range() {
                if range.length > 0 {
                    if let Some(t) = self.string_for_range(range) {
                        return Some(t);
                    }
                }
            }
            let mut cur = self.parent();
            for _ in 0..8 {
                let Some(el) = cur else { break };
                if let Some(t) = el.selected_text() {
                    return Some(t);
                }
                if let Some(range) = el.selected_range() {
                    if range.length > 0 {
                        if let Some(t) = el.string_for_range(range) {
                            return Some(t);
                        }
                    }
                }
                cur = el.parent();
            }
            self.find_selected_in_descendants(40)
        }

        fn parent(&self) -> Option<Self> {
            let v = self.copy_attr("AXParent")?;
            Some(Elem(v as AXUIElementRef))
        }

        fn children(&self) -> Vec<Self> {
            unsafe {
                let Some(arr) = self.copy_attr("AXChildren") else {
                    return Vec::new();
                };
                let n = CFArrayGetCount(arr);
                let mut out = Vec::new();
                if n > 0 {
                    out.reserve(n as usize);
                }
                for i in 0..n {
                    let item = CFArrayGetValueAtIndex(arr, i);
                    if item.is_null() {
                        continue;
                    }
                    CFRetain(item);
                    out.push(Elem(item as AXUIElementRef));
                }
                CFRelease(arr);
                out
            }
        }

        fn string_for_range(&self, range: CfRange) -> Option<String> {
            unsafe {
                let param = AXValueCreate(AX_CFRANGE, &range as *const _ as *const c_void);
                if param.is_null() {
                    return None;
                }
                let mut out: CFTypeRef = ptr::null();
                let err = AXUIElementCopyParameterizedAttributeValue(
                    self.0,
                    CFString::new("AXStringForRange").as_concrete_TypeRef(),
                    param,
                    &mut out,
                );
                CFRelease(param);
                if err != SUCCESS || out.is_null() {
                    return None;
                }
                let s = CFString::wrap_under_create_rule(out as CFStringRef);
                let t = s.to_string();
                let t = t.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            }
        }

        fn find_selected_in_descendants(&self, max: usize) -> Option<String> {
            let mut stack = self.children();
            let mut seen = 0usize;
            while let Some(el) = stack.pop() {
                seen += 1;
                if seen > max {
                    break;
                }
                if let Some(t) = el.selected_text() {
                    return Some(t);
                }
                if let Some(range) = el.selected_range() {
                    if range.length > 0 {
                        if let Some(t) = el.string_for_range(range) {
                            return Some(t);
                        }
                    }
                }
                stack.extend(el.children());
            }
            None
        }

        fn copy_attr(&self, name: &str) -> Option<CFTypeRef> {
            unsafe {
                let mut value: CFTypeRef = ptr::null();
                let err = AXUIElementCopyAttributeValue(
                    self.0,
                    CFString::new(name).as_concrete_TypeRef(),
                    &mut value,
                );
                if err != SUCCESS || value.is_null() {
                    None
                } else {
                    Some(value)
                }
            }
        }

        fn ax_point(&self, name: &str) -> Option<core_graphics::geometry::CGPoint> {
            let v = self.copy_attr(name)?;
            let mut pt = core_graphics::geometry::CGPoint::new(0.0, 0.0);
            let ok = unsafe { AXValueGetValue(v, AX_CGPOINT, &mut pt as *mut _ as *mut c_void) };
            unsafe { CFRelease(v) };
            (ok != 0).then_some(pt)
        }

        fn ax_size(&self) -> Option<core_graphics::geometry::CGSize> {
            let v = self.copy_attr("AXSize")?;
            let mut size = core_graphics::geometry::CGSize::new(0.0, 0.0);
            let ok = unsafe { AXValueGetValue(v, AX_CGSIZE, &mut size as *mut _ as *mut c_void) };
            unsafe { CFRelease(v) };
            (ok != 0).then_some(size)
        }

        fn role(&self) -> Option<String> {
            let v = self.copy_attr("AXRole")?;
            let s = unsafe { CFString::wrap_under_create_rule(v as CFStringRef) };
            Some(s.to_string())
        }

        fn selected_range(&self) -> Option<CfRange> {
            let v = self.copy_attr("AXSelectedTextRange")?;
            let mut range = CfRange {
                location: 0,
                length: 0,
            };
            let ok = unsafe { AXValueGetValue(v, AX_CFRANGE, &mut range as *mut _ as *mut c_void) };
            unsafe { CFRelease(v) };
            (ok != 0).then_some(range)
        }

        fn bounds_for_range(&self, range: CfRange) -> Option<core_graphics::geometry::CGRect> {
            unsafe {
                let param = AXValueCreate(AX_CFRANGE, &range as *const _ as *const c_void);
                if param.is_null() {
                    return None;
                }
                let mut out: CFTypeRef = ptr::null();
                let err = AXUIElementCopyParameterizedAttributeValue(
                    self.0,
                    CFString::new("AXBoundsForRange").as_concrete_TypeRef(),
                    param,
                    &mut out,
                );
                CFRelease(param);
                if err != SUCCESS || out.is_null() {
                    return None;
                }
                let mut rect = core_graphics::geometry::CGRect::default();
                let ok = AXValueGetValue(out, AX_CGRECT, &mut rect as *mut _ as *mut c_void);
                CFRelease(out);
                (ok != 0).then_some(rect)
            }
        }

        fn caret_rect(&self) -> Option<core_graphics::geometry::CGRect> {
            let range = self.selected_range()?;
            if let Some(rect) = self.bounds_for_range(range) {
                if rect.size.height > 0.0 || range.length == 0 {
                    return Some(rect);
                }
            }
            let loc = range.location.max(0);
            let nearby = [
                CfRange {
                    location: loc,
                    length: 1,
                },
                CfRange {
                    location: loc.saturating_sub(1),
                    length: 1,
                },
            ];
            for probe in nearby {
                if let Some(rect) = self.bounds_for_range(probe) {
                    if rect.size.height <= 0.0 {
                        continue;
                    }
                    let x = if probe.location < loc {
                        rect.origin.x + rect.size.width
                    } else {
                        rect.origin.x
                    };
                    return Some(core_graphics::geometry::CGRect {
                        origin: core_graphics::geometry::CGPoint {
                            x,
                            y: rect.origin.y,
                        },
                        size: core_graphics::geometry::CGSize {
                            width: 1.0,
                            height: rect.size.height,
                        },
                    });
                }
            }
            None
        }

        pub fn input_rect(&self) -> Option<super::InputRect> {
            if let Some(rect) = self.caret_rect() {
                if rect.origin.x.is_finite() && rect.origin.y.is_finite() {
                    return Some(super::InputRect::from_ax(
                        rect.origin.x,
                        rect.origin.y,
                        rect.size.width,
                        rect.size.height,
                    ));
                }
            }
            let pos = self.ax_point("AXPosition")?;
            let size = self.ax_size()?;
            if !pos.x.is_finite() || !pos.y.is_finite() || size.width < 4.0 || size.height < 4.0 {
                return None;
            }
            let role = self.role().unwrap_or_default();
            let is_text = matches!(
                role.as_str(),
                "AXTextField"
                    | "AXTextArea"
                    | "AXComboBox"
                    | "AXSearchField"
                    | "AXStaticText"
                    | "AXEditableText"
            ) || role.contains("Text");
            if !is_text && (role == "AXWebArea" || role == "AXWindow" || role == "AXGroup") {
                return None;
            }
            if size.height > 480.0 && !is_text {
                return None;
            }
            let height = if size.height > 80.0 {
                22.0
            } else {
                size.height
            };
            Some(super::InputRect::from_ax(pos.x, pos.y, size.width, height))
        }
    }
}

#[cfg(target_os = "windows")]
mod os {
    use super::Handle;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic,
        SetForegroundWindow, ShowWindow, SW_RESTORE,
    };

    pub fn frontmost() -> Option<Handle> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if pid == std::process::id() {
                return None;
            }
            Some(hwnd as isize)
        }
    }

    pub fn activate(hwnd: Handle) {
        unsafe {
            let hwnd = hwnd as HWND;
            if hwnd.is_null() {
                return;
            }
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_RESTORE);
            }
            let fg = GetForegroundWindow();
            let target_thread = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            let fg_thread = GetWindowThreadProcessId(fg, std::ptr::null_mut());
            let our_thread = GetCurrentThreadId();
            if fg_thread != 0 && fg_thread != target_thread {
                AttachThreadInput(fg_thread, target_thread, 1);
            }
            if our_thread != target_thread {
                AttachThreadInput(our_thread, target_thread, 1);
            }
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
            if fg_thread != 0 && fg_thread != target_thread {
                AttachThreadInput(fg_thread, target_thread, 0);
            }
            if our_thread != target_thread {
                AttachThreadInput(our_thread, target_thread, 0);
            }
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod os {
    use super::Handle;
    pub fn frontmost() -> Option<Handle> {
        None
    }
    pub fn activate(_: Handle) {}
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
fn ensure_accessibility() -> Result<(), String> {
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

#[cfg(not(target_os = "macos"))]
fn ensure_accessibility() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceKeyState(state_id: i32, key: u16) -> u8;
}

#[cfg(target_os = "macos")]
fn hid_key_down(key: u16) -> bool {
    unsafe { CGEventSourceKeyState(1, key) != 0 }
}

#[cfg(target_os = "macos")]
fn post_key(ev: &core_graphics::event::CGEvent, _pid: Option<Handle>) {
    use core_graphics::event::CGEventTapLocation;
    ev.post(CGEventTapLocation::HID);
}

#[cfg(target_os = "macos")]
fn send_cmd_key(key: u16, press_command: bool, pid: Option<Handle>) -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, KeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .or_else(|_| CGEventSource::new(CGEventSourceStateID::HIDSystemState))
        .map_err(|()| "无法创建按键事件".to_string())?;
    let flags = CGEventFlags::CGEventFlagCommand | CGEventFlags::from_bits_retain(0x8);
    let option_held = hid_key_down(KeyCode::OPTION) || hid_key_down(KeyCode::RIGHT_OPTION);
    if option_held {
        let opt_up = CGEvent::new_keyboard_event(source.clone(), KeyCode::OPTION, false)
            .map_err(|()| "无法创建按键事件".to_string())?;
        opt_up.set_flags(CGEventFlags::from_bits_retain(0x8));
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
        cmd_up.set_flags(CGEventFlags::from_bits_retain(0x8));
        post_key(&cmd_up, pid);
    }
    if option_held {
        let opt_down = CGEvent::new_keyboard_event(source, KeyCode::OPTION, true)
            .map_err(|()| "无法创建按键事件".to_string())?;
        opt_down
            .set_flags(CGEventFlags::CGEventFlagAlternate | CGEventFlags::from_bits_retain(0x8));
        post_key(&opt_down, pid);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn send_paste(pid: Option<Handle>) -> Result<(), String> {
    send_cmd_key(0x09, true, pid)
}

#[cfg(target_os = "macos")]
fn copy_selected_via_clipboard() -> Option<String> {
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
    send_cmd_key(KEY_C, true, None).ok()?;
    let deadline = Instant::now() + Duration::from_millis(70);
    let mut got = None;
    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(12));
        let count_changed = match (before, os::pasteboard_count()) {
            (Some(a), Some(b)) => a != b,
            (Some(_), None) => false,
            (None, _) => true,
        };
        if !count_changed {
            continue;
        }
        let Ok(text) = clipboard.get_text() else {
            continue;
        };
        if use_marker && (text == MARKER || text.trim().is_empty()) {
            continue;
        }
        let t = text.trim();
        if t.is_empty() {
            continue;
        }
        got = Some(t.to_string());
        break;
    }
    if let Some(prev) = previous {
        let _ = clipboard.set_text(&prev);
    }
    got
}

#[cfg(not(target_os = "macos"))]
fn send_paste(_pid: Option<Handle>) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| map_input_err(e.to_string()))?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| map_input_err(e.to_string()))?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| map_input_err(e.to_string()))?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| map_input_err(e.to_string()))?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn map_input_err(e: String) -> String {
    if e.to_lowercase().contains("access") || e.to_lowercase().contains("permission") {
        #[cfg(target_os = "macos")]
        return "无法回写：请在系统设置 › 隐私与安全性 › 辅助功能 中允许 input".into();
        #[cfg(target_os = "windows")]
        return "无法回写：模拟按键失败".into();
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        return e;
    }
    e
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct InputRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub physical: bool,
}

impl InputRect {
    #[cfg(target_os = "macos")]
    fn from_ax(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width: width.max(1.0),
            height: height.max(18.0),
            physical: false,
        }
    }
}

#[cfg(target_os = "windows")]
fn win_caret_rect() -> Option<InputRect> {
    use windows_sys::Win32::Foundation::{HWND, POINT};
    use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GUITHREADINFO};

    unsafe {
        let mut info: GUITHREADINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
        if GetGUIThreadInfo(0, &mut info) == 0 || info.hwndCaret.is_null() {
            return None;
        }
        let hwnd = info.hwndCaret as HWND;
        let mut tl = POINT {
            x: info.rcCaret.left,
            y: info.rcCaret.top,
        };
        let mut br = POINT {
            x: info.rcCaret.right,
            y: info.rcCaret.bottom,
        };
        if ClientToScreen(hwnd, &mut tl) == 0 || ClientToScreen(hwnd, &mut br) == 0 {
            return None;
        }
        Some(InputRect {
            x: tl.x as f64,
            y: tl.y as f64,
            width: (br.x - tl.x).max(1) as f64,
            height: (br.y - tl.y).max(16) as f64,
            physical: true,
        })
    }
}

#[allow(dead_code)]
pub fn focused_input_rect() -> Option<InputRect> {
    #[cfg(target_os = "macos")]
    {
        ax::Elem::focused().and_then(|el| el.input_rect())
    }
    #[cfg(target_os = "windows")]
    {
        win_caret_rect()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

pub fn remember(app: &AppHandle, capture_selection: bool) {
    let clipboard = Clipboard::new().ok().and_then(|mut c| c.get_text().ok());
    #[cfg(target_os = "macos")]
    let ax = ax::Elem::focused();
    #[cfg(target_os = "macos")]
    let selected = if capture_selection {
        ax.as_ref()
            .and_then(|e| e.capture_selected())
            .or_else(copy_selected_via_clipboard)
    } else {
        None
    };
    #[cfg(not(target_os = "macos"))]
    let selected = {
        let _ = capture_selection;
        None
    };
    let saved = Saved {
        handle: os::frontmost(),
        clipboard,
        #[cfg(target_os = "macos")]
        ax,
        selected,
    };
    if let Some(slot) = app.try_state::<PasteTarget>() {
        *slot.0.lock().unwrap() = Some(saved);
    }
}

pub fn remembered_selected_text(app: &AppHandle) -> Option<String> {
    app.try_state::<PasteTarget>()
        .and_then(|slot| slot.0.lock().unwrap().as_ref()?.selected.clone())
}

pub fn activate_remembered(app: &AppHandle) {
    let handle = app
        .try_state::<PasteTarget>()
        .and_then(|slot| slot.0.lock().unwrap().as_ref()?.handle);
    if let Some(handle) = handle {
        os::activate(handle);
    }
}

#[tauri::command]
pub async fn paste_back(app: AppHandle, text: String) -> Result<(), String> {
    if text.is_empty() {
        return Err("没有可填入的内容".into());
    }
    ensure_accessibility()?;
    let saved = app
        .try_state::<PasteTarget>()
        .and_then(|slot| slot.0.lock().unwrap().take());
    crate::overlay::dismiss_overlay(&app, false);
    tokio::time::sleep(Duration::from_millis(80)).await;
    if let Some(handle) = saved.as_ref().and_then(|s| s.handle) {
        os::activate(handle);
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
    #[cfg(target_os = "macos")]
    {
        let fresh = ax::Elem::focused();
        let el = fresh
            .as_ref()
            .or_else(|| saved.as_ref().and_then(|s| s.ax.as_ref()));
        if let Some(el) = el {
            el.focus();
            let replace = saved
                .as_ref()
                .and_then(|s| s.selected.as_ref())
                .is_some_and(|t| !t.trim().is_empty());
            if el.insert_text(&text, replace) {
                return Ok(());
            }
        }
    }
    let mut clipboard = Clipboard::new().map_err(|e| e.to_string())?;
    clipboard.set_text(&text).map_err(|e| e.to_string())?;
    let pid = saved.as_ref().and_then(|s| s.handle);
    if let Err(e) = send_paste(pid) {
        restore_clipboard(&mut clipboard, saved.as_ref());
        if let Some(slot) = app.try_state::<PasteTarget>() {
            *slot.0.lock().unwrap() = saved;
        }
        crate::overlay::reveal_overlay(&app);
        return Err(e);
    }
    tokio::time::sleep(Duration::from_millis(80)).await;
    restore_clipboard(&mut clipboard, saved.as_ref());
    Ok(())
}

fn restore_clipboard(clipboard: &mut Clipboard, saved: Option<&Saved>) {
    if let Some(prev) = saved.and_then(|s| s.clipboard.as_ref()) {
        let _ = clipboard.set_text(prev);
    }
}
