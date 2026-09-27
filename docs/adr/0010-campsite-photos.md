# 0010 — Campsite photos

Status: accepted (2026-09-26)

## Context
Campsite owners should be able to add up to 3 photos, shown at the top of the Campsite panel.
The frontend had no multipart support, no file picker and no image decoding. It draws to a single canvas
(ADR 0001), so there is no `<img>` element to lean on.

## Decision
- **Storage:** a `photos` file field on `campsites`: at most 3 files, JPEG/PNG/WebP, 10 MB each,
  thumbs `320x240` (thumbnail strips) and `1200x1200f` (the viewer, fitted inside 1200×1200, never upscaled). The files inherit the
  campsite rules (owner-only writes), and the admin hook already rejects `photos+`/`photos-` from admins.
- **One request per save:** when photos change, create/update is a single `multipart/form-data` request with
  PocketBase's `@jsonPayload` part (the usual JSON body plus `"photos-": [...]`) and one `photos+` part per new
  file. The rules see the same body as with JSON, so none of them changed. The multipart body is built by hand in
  `api/client.rs` (a pure, unit-tested function), because `ehttp` has no multipart helper.
- **Picking files:** `web/file_picker.rs` uses web-sys directly: a detached `<input type="file">` read through
  async browser APIs. The UI emits `Action::PickPhotos`, and the files come back as `Event::PhotosPicked`.
  `rfd` was not added: on the web it only wraps the same input, plus an overlay we don't need.
- **Previews:** the browser shrinks each picked photo to a small JPEG (createImageBitmap + OffscreenCanvas)
  before egui sees it. Decoding a full-size phone photo in wasm would block the frame, and its texture could
  exceed the GPU limit, which makes egui_glow panic. The app only ever decodes previews and server thumbs.
- **Viewing:** the Campsite panel shows thumbnails. A click opens a full-page carousel (an egui `Modal` over
  the map, top bar and panel), the only overlay besides toasts. That amends ADR 0008's "everything is a side
  panel": a photo needs the whole window, and the modal has no state of its own beyond
  `CampsiteDetail::photo_open`.
- **Displaying:** `egui_extras` image loaders (`http`, `image`, `webp`), installed once in `app.rs`.
  `egui::Image::new(url)` fetches and caches the thumbnail, the same way walkers fetches map tiles. This is
  a deliberate exception to "UI never does I/O" (ARCHITECTURE §5.2): it's read-only, cached, and never touches
  `AppState`. The loader only accepts absolute `http(s)://` URLs, so `AppState.origin` holds the page origin.

## Consequences
- `egui_extras` joins egui/eframe/walkers in the lockstep upgrade rule.
- Photo files are **not protected**. Anyone who has a file URL can fetch it, even after the campsite is
  hidden by moderation. Filenames carry random suffixes, so URLs can't be guessed. Protected files with file
  tokens can come later if moderation needs it.
- Photos are uploaded as picked: there is no client-side resize or compression yet, so a 10 MB photo is a
  10 MB upload. A save with 3 large photos is close to PocketBase's default body limit (32 MB).
- Browsers without OffscreenCanvas still upload fine, but the form shows the filename instead of a preview.
