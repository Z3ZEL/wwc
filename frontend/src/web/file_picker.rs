//! Photo picker: a detached `<input type="file">`, read with the browser's async APIs so the
//! frame loop never blocks. The browser also makes the small form preview (createImageBitmap +
//! OffscreenCanvas): decoding a full-size phone photo in egui would stall the frame, and the
//! texture could exceed the GPU's maximum size.

use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Blob, File, HtmlInputElement, ImageBitmap, ImageEncodeOptions, OffscreenCanvas};

use crate::api::models::ImageBytes;
use crate::state::{PHOTO_MIME_TYPES, PickedPhoto, photo_problem};

/// Longest side of the preview image, in pixels (image resolution, not a layout size).
const PREVIEW_MAX_SIDE: f64 = 480.0;

/// Opens the file dialog. `done` gets every chosen file once they are read. Files that are
/// refused (wrong type, too big, or beyond `max` accepted ones) come back without bytes so
/// the caller can explain why. Nothing happens if the dialog is cancelled.
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
        let size = file.size() as u64;
        let mut photo = PickedPhoto { name, mime, size, bytes: ImageBytes::default(), preview: None };
        if accepted < max && photo_problem(&photo.name, &photo.mime, size).is_none() {
            match blob_bytes(&file).await {
                Ok(bytes) => {
                    accepted += 1;
                    photo.bytes = bytes.into();
                    photo.preview = preview(&file).await.map_err(|e| log::warn!("no preview: {e:?}")).ok();
                }
                Err(e) => {
                    log::error!("could not read {}: {e:?}", photo.name);
                    continue;
                }
            }
        }
        picked.push(photo);
    }
    picked
}

async fn blob_bytes(blob: &Blob) -> Result<Vec<u8>, JsValue> {
    let buffer = JsFuture::from(blob.array_buffer()).await?;
    Ok(Uint8Array::new(&buffer).to_vec())
}

/// A JPEG of at most `PREVIEW_MAX_SIDE` pixels, respecting the photo's EXIF orientation.
async fn preview(file: &File) -> Result<ImageBytes, JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let bitmap: ImageBitmap = JsFuture::from(window.create_image_bitmap_with_blob(file)?).await?.dyn_into()?;
    let (w, h) = (f64::from(bitmap.width()), f64::from(bitmap.height()));
    let scale = (PREVIEW_MAX_SIDE / w.max(h)).min(1.0);
    let (pw, ph) = ((w * scale).round().max(1.0), (h * scale).round().max(1.0));

    let canvas = OffscreenCanvas::new(pw as u32, ph as u32)?;
    let ctx: web_sys::OffscreenCanvasRenderingContext2d =
        canvas.get_context("2d")?.ok_or("no 2d context")?.dyn_into()?;
    ctx.draw_image_with_image_bitmap_and_dw_and_dh(&bitmap, 0.0, 0.0, pw, ph)?;
    bitmap.close();

    let options = ImageEncodeOptions::new();
    options.set_type("image/jpeg");
    options.set_quality(0.85);
    let blob: Blob = JsFuture::from(canvas.convert_to_blob_with_options(&options)?).await?.dyn_into()?;
    Ok(blob_bytes(&blob).await?.into())
}
