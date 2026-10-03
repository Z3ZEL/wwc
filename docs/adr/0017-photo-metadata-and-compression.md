# 0017 — Photo metadata removal and compression before upload

Status: accepted (2026-10-01). Amends ADR 0010.

## Context
Photos were uploaded as picked (ADR 0010). Phone photos carry hidden metadata (EXIF, XMP): very often the
GPS position where they were taken, the date and time, the camera or phone model, sometimes the owner's name.
Photo files are public and not protected, so anyone could download the original and read it. The Privacy
Policy asked users to remove it themselves, which few people know how to do. A 10 MB phone photo was also a
10 MB upload and 10 MB of storage, while the app never shows more than the `1200x1200f` server thumb.
Both were on the roadmap (ARCHITECTURE §13: "client-side resize before upload", "strip photo metadata").

## Decision
- **Re-encode in the browser.** `web/photo.rs` decodes each picked file with `createImageBitmap` (which
  applies the EXIF orientation, so the pixels come out upright), draws it on an `OffscreenCanvas` fitted
  in **2048 px** (never upscaled) and encodes it again. A canvas only holds pixels, so the new file has no
  metadata at all. The decode is the one the preview already needed; the preview now comes from the same
  bitmap. Everything stays async, so the frame loop never blocks.
- **Format:** WebP at quality 0.82 first, then JPEG at 0.82. Browsers that can't encode WebP (Safari)
  return a PNG instead of failing, so the result type is checked before accepting it. Transparent areas are
  filled with white (they would be black in a JPEG). The file is renamed after its new format (`.webp`).
- **Safety net:** `media::strip_metadata`, a pure, host-tested function, removes metadata containers
  losslessly from JPEG (APP1–APP13, APP15, COM; APP0 and the Adobe APP14 are kept), PNG (`tEXt`, `zTXt`,
  `iTXt`, `eXIf`, `tIME`) and WebP (`EXIF`, `XMP `, and their `VP8X` flags). It runs on every re-encoded
  file, and on the original when the browser can't re-encode it (no OffscreenCanvas); the original is then
  uploaded at its own size. **A file whose metadata can't be removed is refused, never uploaded as is.**
- **Limits:** the picker reads originals up to **40 MB** (`MAX_SOURCE_BYTES`, to keep huge files out of
  wasm memory); the server's 10 MB limit (`MAX_PHOTO_BYTES`) now applies to the processed file.
- **UI:** the campsite form says that photos are resized and their metadata removed, and shows the size of
  the new photos as picked and as uploaded.
- **Frontend only.** No schema, rule or hook change. The backend still accepts any JPEG/PNG/WebP up to
  10 MB, so a client that bypasses the app can still upload metadata; stripping it server-side would need a
  Go build or an external tool, which ADR 0002 avoids.

## Consequences
- Uploads and stored files are several times smaller: a 4032×3024 phone photo becomes 2048×1536 (about a
  quarter of the pixels) in a more efficient format. Saves with 3 photos stay far from PocketBase's 32 MB
  body limit.
- Uploaded photos lose their color profile (canvas output is sRGB) and some quality; at 2048 px and 0.82
  this is not visible at the sizes the app shows. Originals are not kept anywhere.
- An already small photo may come out slightly larger than it was; it is still re-encoded, because the
  metadata-free original would lose its EXIF orientation and could show sideways.
- The fallback path (no OffscreenCanvas) keeps the original resolution and size, but without metadata.
- Photos uploaded before this change keep their metadata. Owners can remove and re-add them; a server-side
  migration is not planned.
- The Privacy Policy no longer asks users to remove metadata themselves.
