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
