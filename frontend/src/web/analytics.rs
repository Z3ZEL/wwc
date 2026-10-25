//! The Umami tracker script (ADR 0022). Loaded only once the visitor accepts analytics
//! (`crate::analytics` decides when, and what to send).
//!
//! Automatic tracking is off: the app is one canvas without routes, so it counts the visit when
//! the script loads and sends its own events. Once consent is withdrawn the script stays loaded
//! but the app stops calling it, and it sends nothing on its own.

use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::HtmlElement;

use crate::analytics::{Prop, TrackEvent};

const SCRIPT_ID: &str = "umami";

/// Adds the script (once) and counts the visit when it has loaded.
pub fn start(website_id: &str, script_url: &str) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else { return };
    if document.get_element_by_id(SCRIPT_ID).is_some() {
        // Accepted again after a withdrawal: the script is already there.
        return pageview();
    }
    let (Ok(script), Some(body)) = (document.create_element("script"), document.body()) else { return };
    for (name, value) in [
        ("id", SCRIPT_ID),
        ("src", script_url),
        ("data-website-id", website_id),
        ("data-auto-track", "false"),
        ("data-do-not-track", "true"),
        ("data-exclude-search", "true"),
        ("data-exclude-hash", "true"),
    ] {
        if script.set_attribute(name, value).is_err() {
            return;
        }
    }
    script.unchecked_ref::<HtmlElement>().set_onload(Some(Closure::once_into_js(pageview).unchecked_ref()));
    if let Err(e) = body.append_child(&script) {
        log::warn!("analytics: {e:?}");
    }
}

/// `umami.track()`: a pageview.
fn pageview() {
    call_track(&[]);
}

/// `umami.track(name, data)`. Dropped while the script hasn't loaded yet.
pub fn track(event: TrackEvent) {
    let data = js_sys::Object::new();
    if let Some((key, value)) = event.prop {
        let value = match value {
            Prop::Text(text) => JsValue::from_str(text),
            Prop::Count(n) => JsValue::from(n),
        };
        let _ = js_sys::Reflect::set(&data, &key.into(), &value);
    }
    call_track(&[JsValue::from_str(event.name), data.into()]);
}

fn call_track(args: &[JsValue]) {
    let Some(window) = web_sys::window() else { return };
    let Ok(umami) = js_sys::Reflect::get(&window, &"umami".into()) else { return };
    let Some(track) =
        js_sys::Reflect::get(&umami, &"track".into()).ok().and_then(|f| f.dyn_into::<js_sys::Function>().ok())
    else {
        return;
    };
    let args: js_sys::Array = args.iter().collect();
    if let Err(e) = track.apply(&umami, &args) {
        log::warn!("analytics: {e:?}");
    }
}
