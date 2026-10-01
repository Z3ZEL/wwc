//! Photo picker: a detached `<input type="file">`, read with the browser's async APIs so the
//! frame loop never blocks. Each accepted file is prepared by `web::photo` (metadata removed,
//! resized, compressed, plus a small preview) before the app sees it.

use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{File, HtmlInputElement};

use super::photo;
use crate::api::models::ImageBytes;
use crate::state::{PHOTO_MIME_TYPES, PickedPhoto, source_problem};

/// Opens the file dialog. `done` gets every chosen file once they are prepared. Files that are
/// refused (wrong type, too big, beyond `max` accepted ones, or that couldn't be prepared) come
/// back without bytes so the caller can explain why. Nothing happens if the dialog is cancelled.
pub fn pick_photos(max: usize, done: impl FnOnce(Vec<PickedPhoto>) + 'static) {
    if let Err(e) = open_dialog(max, done) {
        log::error!("file picker unavailable: {e:?}");
    }
}

fn open_dialog(max: usize, done: impl FnOnce(Vec<PickedPhoto>) + 'static) -> Result<(), JsValue> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("no document")?;
    let input: HtmlInputElement = document.create_element("input")?.dyn_into()?;
    input.set_type("file");
    input.set_multiple(true);
    input.set_accept(&PHOTO_MIME_TYPES.join(","));

    // The closure keeps `input` alive (and the input keeps the closure) until it fires.
    let handler_input = input.clone();
    let on_change = Closure::once_into_js(move |_: web_sys::Event| {
        let files = handler_input
            .files()
            .map_or_else(Vec::new, |list| (0..list.length()).filter_map(|i| list.get(i)).collect::<Vec<File>>());
        handler_input.set_onchange(None);
        if !files.is_empty() {
            spawn_local(async move { done(read_files(files, max).await) });
        }
    });
    input.set_onchange(Some(on_change.unchecked_ref()));
    input.click();
    Ok(())
}

async fn read_files(files: Vec<File>, max: usize) -> Vec<PickedPhoto> {
    let mut picked = Vec::with_capacity(files.len());
    let mut accepted = 0;
    for file in files {
        let (name, mime) = (file.name(), file.type_());
        // `size()` is a JS number of bytes; files over 2^53 bytes don't exist.
        let original_size = file.size() as u64;
        let mut photo =
            PickedPhoto { name, mime, original_size, bytes: ImageBytes::default(), preview: None, problem: None };
        if accepted < max && source_problem(&photo.name, &photo.mime, original_size).is_none() {
            match photo::prepare(&file).await {
                Ok(prepared) => {
                    accepted += 1;
                    photo.name = prepared.name;
                    photo.mime = prepared.mime;
                    photo.bytes = prepared.bytes.into();
                    photo.preview = prepared.preview;
                }
                Err(problem) => photo.problem = Some(problem),
            }
        }
        picked.push(photo);
    }
    picked
}
