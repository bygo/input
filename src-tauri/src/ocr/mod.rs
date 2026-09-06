mod region;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
use std::process::Command;

use std::path::Path;

pub use region::{capture_region_retina, capture_screen_rect, crop_freeze_jpeg, tmp_path};


pub fn recognize_image(path: &Path) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        macos::recognize_file(path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = path;
        Err("截屏取词目前仅支持 macOS".into())
    }
}

pub fn ensure_screen_permission() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::ensure_screen_permission()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(())
    }
}

pub fn open_screen_recording_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
            .spawn();
    }
}
