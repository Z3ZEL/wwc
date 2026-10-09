//! eframe's text agent: the hidden `<input>` it puts right after the canvas to receive keyboard
//! and IME input. Phone keyboards compose words in it, and eframe drops composed text while a
//! password field is focused. A `password` input makes them send plain keystrokes (ADR 0018).

use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

/// Same selector as the text agent rule in `index.html`.
const TEXT_AGENT: &str = "canvas#app + input";

/// Switches the text agent to `type="password"` (`on`) or back to `type="text"`.
pub fn set_password_mode(on: bool) {
    let input = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.query_selector(TEXT_AGENT).ok().flatten())
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok());
    let Some(input) = input else {
        log::warn!("text agent `{TEXT_AGENT}` not found");
        return;
    };
    let kind = if on { "password" } else { "text" };
    if input.type_() != kind {
        input.set_type(kind);
    }
}
