//! Browser APIs egui doesn't cover (wasm only). Called from the controller, never from UI code.

mod file_picker;

pub use file_picker::pick_photos;

/// Sets the browser tab title (`document.title`).
pub fn set_document_title(title: &str) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        document.set_title(title);
    }
}
