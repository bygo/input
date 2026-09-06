pub mod commands;
mod shortcut;
mod store;
mod types;

#[cfg(test)]
mod tests;

pub use shortcut::{normalize_shortcut, same_shortcut};
pub use store::{dir, read, write};
pub use types::{
    default_compose_shortcut, default_dict_pack, default_llm_model, default_ocr_shortcut,
    default_select_shortcut, Settings,
};
