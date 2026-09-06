mod db;
mod entry;
pub mod lookup;
pub mod pack;
mod path;
pub mod speak;
mod types;

#[cfg(target_os = "macos")]
mod tts_macos;

#[cfg(target_os = "windows")]
mod tts_windows;

#[cfg(test)]
mod tests;

pub use speak::warm_tts;
pub use types::managed;
