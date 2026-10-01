//! Photo preparation before upload (ADR 0017): picked photos are re-encoded by the browser
//! (`web/photo.rs`), which drops their metadata and shrinks them, and `strip_metadata` removes
//! what's left. This module is the pure part: settings, sizes and names, tested on the host.

mod strip;

pub use strip::strip_metadata;

/// Longest side of an uploaded photo, in pixels. The largest server thumb is `1200x1200f`
/// (`api::PhotoSize::Large`); the margin keeps photos sharp on high-density screens.
pub const UPLOAD_MAX_SIDE: u32 = 2048;

/// Longest side of the form preview, in pixels.
pub const PREVIEW_MAX_SIDE: u32 = 480;

/// Encoder quality (0–1) for uploads. Above ~0.85 files grow fast for no visible gain.
pub const UPLOAD_QUALITY: f64 = 0.82;

/// Encoder quality for previews, which are only shown small in the form.
pub const PREVIEW_QUALITY: f64 = 0.85;

/// Formats tried in order. WebP is ~30% smaller than JPEG at the same quality, but some
/// browsers (Safari) can't encode it and silently return a PNG: the encoder checks the result
/// type and moves on to JPEG, which every browser encodes.
pub const UPLOAD_ENCODINGS: [&str; 2] = ["image/webp", "image/jpeg"];

/// `(width, height)` scaled down to fit in `max_side` × `max_side`, never up, at least 1×1.
pub fn fitted_size(width: u32, height: u32, max_side: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= max_side {
        return (width.max(1), height.max(1));
    }
    let scale = |side: u32| {
        let scaled = u64::from(side) * u64::from(max_side) / u64::from(longest);
        u32::try_from(scaled).unwrap_or(max_side).max(1)
    };
    (scale(width), scale(height))
}

/// `name` with the extension of `mime` (a JPEG re-encoded as WebP must not keep `.jpg`).
pub fn renamed(name: &str, mime: &str) -> String {
    let ext = match mime {
        "image/webp" => "webp",
        "image/png" => "png",
        _ => "jpg",
    };
    let stem = match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => name,
    };
    format!("{stem}.{ext}")
}

/// A file size for people: `850 KB`, `4.2 MB`.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    if bytes >= MB {
        // Tenths of a MB, rounded, in integers: exact and no float formatting quirks.
        let tenths = (bytes * 10 + MB / 2) / MB;
        format!("{}.{} MB", tenths / 10, tenths % 10)
    } else {
        format!("{} KB", bytes.div_ceil(KB))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_fitted_without_upscaling() {
        assert_eq!(fitted_size(4032, 3024, 2048), (2048, 1536));
        assert_eq!(fitted_size(3024, 4032, 2048), (1536, 2048));
        assert_eq!(fitted_size(800, 600, 2048), (800, 600));
        assert_eq!(fitted_size(10000, 3, 480), (480, 1));
        assert_eq!(fitted_size(0, 0, 480), (1, 1));
    }

    #[test]
    fn files_are_renamed_after_their_new_format() {
        assert_eq!(renamed("IMG_1234.JPG", "image/webp"), "IMG_1234.webp");
        assert_eq!(renamed("lake.at.dawn.png", "image/jpeg"), "lake.at.dawn.jpg");
        assert_eq!(renamed("noext", "image/webp"), "noext.webp");
        assert_eq!(renamed(".hidden", "image/jpeg"), ".hidden.jpg");
    }

    #[test]
    fn byte_counts_are_readable() {
        assert_eq!(format_bytes(0), "0 KB");
        assert_eq!(format_bytes(1), "1 KB");
        assert_eq!(format_bytes(850 * 1024), "850 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(4_404_019), "4.2 MB");
    }
}
