mod catalog;
pub mod install;
mod ollama;
mod runtime;
mod state;
pub mod status;

pub use catalog::{active_model, is_catalog_model};
pub use install::boot;
pub use ollama::{chat, chat_plain, ready};
pub use state::managed;
