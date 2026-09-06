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

pub fn caret_rect() -> Option<super::InputRect> {
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
        Some(super::InputRect {
            x: tl.x as f64,
            y: tl.y as f64,
            width: (br.x - tl.x).max(1) as f64,
            height: (br.y - tl.y).max(16) as f64,
            physical: true,
        })
    }
}
