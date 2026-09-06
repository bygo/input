mod state;

#[cfg(target_os = "macos")]
mod os_macos;
#[cfg(target_os = "macos")]
use os_macos as os;

#[cfg(target_os = "windows")]
mod os_windows;
#[cfg(target_os = "windows")]
use os_windows as os;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod os_linux;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
use os_linux as os;

#[cfg(target_os = "macos")]
mod ax;

#[cfg(target_os = "macos")]
mod keys_macos;
#[cfg(target_os = "macos")]
use keys_macos as keys;

#[cfg(not(target_os = "macos"))]
mod keys_other;
#[cfg(not(target_os = "macos"))]
use keys_other as keys;

pub mod paste_back;

use state::{Handle, InputRect};

pub use state::{activate_remembered, managed, remember, remembered_selected_text};
