mod focus;
mod state;
mod window;

pub mod commands;

pub use focus::{
    arm_ignore_unfocus, clear_ignore_unfocus, disarm_ignore_unfocus, should_ignore_unfocus,
};
pub use state::{
    capturing_shortcut, clear_ocr_freeze, ocr_freeze_origin, ocr_picking, overlay_is_open,
    set_capturing_shortcut, set_ocr_freeze, set_ocr_picking, set_overlay_open, take_ocr_freeze,
    Anchor, CursorAnchor, IgnoreUnfocusUntil, OcrFreeze, OcrFreezeFrame, OcrPicking, OverlayKind,
    OverlayOpen, OverlayShown, ShortcutCapture, OVERLAY_H, OVERLAY_W,
};
pub use window::{
    appear_window, capture_anchor, dismiss_overlay, park_window, place_window, reveal_overlay,
    show_overlay, warmup_window,
};

#[cfg(target_os = "macos")]
pub use window::set_alpha;
