//! Browser APIs egui doesn't cover (wasm only). Called from the controller, never from UI code.

mod file_picker;
mod photo;
mod text_agent;

pub use file_picker::pick_photos;
pub use text_agent::set_password_mode;

/// Wall-clock time in ms since the Unix epoch.
pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

/// Sets the browser tab title (`document.title`).
pub fn set_document_title(title: &str) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        document.set_title(title);
    }
}
