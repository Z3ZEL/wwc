//! Browser APIs egui doesn't cover (wasm only). Called from the controller, never from UI code.

mod file_picker;

pub use file_picker::pick_photos;
