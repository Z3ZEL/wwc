//! Photo preparation before upload (ADR 0017). The browser decodes the picked file
//! (createImageBitmap, which applies the EXIF orientation), draws it on an OffscreenCanvas no
//! larger than `UPLOAD_MAX_SIDE` and encodes it again. A canvas only holds pixels, so the new
//! file carries none of the original's metadata (GPS position, date, camera, owner…), and it is
//! usually a fraction of the original's size. The same bitmap gives the small form preview.
//! Everything is async: decoding a full-size phone photo in wasm would stall the frame.

use js_sys::Uint8Array;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Blob, File, ImageBitmap, ImageEncodeOptions, OffscreenCanvas, OffscreenCanvasRenderingContext2d};

use crate::api::models::ImageBytes;
use crate::media::{
    PREVIEW_MAX_SIDE, PREVIEW_QUALITY, UPLOAD_ENCODINGS, UPLOAD_MAX_SIDE, UPLOAD_QUALITY, fitted_size, renamed,
    strip_metadata,
};

/// Fills transparent areas before drawing: they would turn black in a JPEG. This is image
/// content, not UI, so it doesn't come from the theme.
const BACKGROUND: &str = "#FFFFFF";

/// A photo ready to upload.
pub struct Prepared {
    pub name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
    pub preview: Option<ImageBytes>,
}

/// Re-encodes `file` without its metadata. If the browser can't (no OffscreenCanvas, or an
/// encoder error), the original is uploaded with its metadata removed by `strip_metadata`.
/// The error is a message for the user: the file must not be uploaded.
pub async fn prepare(file: &File) -> Result<Prepared, String> {
    let (name, mime) = (file.name(), file.type_());
    let unsafe_file = || format!("{name}: its hidden metadata (GPS position, camera…) could not be removed.");
    match reencode(file).await {
        Ok((bytes, new_mime, preview)) => {
            // A canvas output has no metadata; strip it anyway in case a browser adds some.
            let bytes = strip_metadata(&bytes, &new_mime).ok_or_else(unsafe_file)?;
            Ok(Prepared { name: renamed(&name, &new_mime), mime: new_mime, bytes, preview })
        }
        Err(e) => {
            log::warn!("could not re-encode {name}, uploading it as is without metadata: {e:?}");
            let original = blob_bytes(file).await.map_err(|e| {
                log::error!("could not read {name}: {e:?}");
                format!("{name}: the file could not be read.")
            })?;
            let bytes = strip_metadata(&original, &mime).ok_or_else(unsafe_file)?;
            Ok(Prepared { name, mime, bytes, preview: None })
        }
    }
}

/// The upload (bytes and type) and the preview, encoded from a single decode of `file`.
async fn reencode(file: &File) -> Result<(Vec<u8>, String, Option<ImageBytes>), JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let bitmap: ImageBitmap = JsFuture::from(window.create_image_bitmap_with_blob(file)?).await?.dyn_into()?;
    let upload = encode(&bitmap, UPLOAD_MAX_SIDE, &UPLOAD_ENCODINGS, UPLOAD_QUALITY).await;
    let preview = match &upload {
        Ok(_) => encode(&bitmap, PREVIEW_MAX_SIDE, &["image/jpeg"], PREVIEW_QUALITY)
            .await
            .map(|(bytes, _)| ImageBytes::from(bytes))
            .map_err(|e| log::warn!("no preview: {e:?}"))
            .ok(),
        Err(_) => None,
    };
    bitmap.close();
    let (bytes, mime) = upload?;
    Ok((bytes, mime, preview))
}

/// `bitmap` fitted in `max_side` and encoded in the first of `types` the browser supports.
async fn encode(
    bitmap: &ImageBitmap,
    max_side: u32,
    types: &[&str],
    quality: f64,
) -> Result<(Vec<u8>, String), JsValue> {
    let (w, h) = fitted_size(bitmap.width(), bitmap.height(), max_side);
    let canvas = OffscreenCanvas::new(w, h)?;
    let ctx: OffscreenCanvasRenderingContext2d = canvas.get_context("2d")?.ok_or("no 2d context")?.dyn_into()?;
    let (wf, hf) = (f64::from(w), f64::from(h));
    ctx.set_fill_style_str(BACKGROUND);
    ctx.fill_rect(0.0, 0.0, wf, hf);
    ctx.draw_image_with_image_bitmap_and_dw_and_dh(bitmap, 0.0, 0.0, wf, hf)?;

    for &mime in types {
        let options = ImageEncodeOptions::new();
        options.set_type(mime);
        options.set_quality(quality);
        let blob: Blob = JsFuture::from(canvas.convert_to_blob_with_options(&options)?).await?.dyn_into()?;
        // Browsers that can't encode a type return a PNG instead of failing.
        if blob.type_() == mime {
            return Ok((blob_bytes(&blob).await?, mime.to_owned()));
        }
    }
    Err("no supported image encoder".into())
}

pub async fn blob_bytes(blob: &Blob) -> Result<Vec<u8>, JsValue> {
    let buffer = JsFuture::from(blob.array_buffer()).await?;
    Ok(Uint8Array::new(&buffer).to_vec())
}
