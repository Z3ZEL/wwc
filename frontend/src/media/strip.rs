//! Lossless metadata removal for JPEG, PNG and WebP files. Pure byte surgery: the image data is
//! copied as is and only the metadata containers are dropped (EXIF with its GPS position, XMP,
//! IPTC, comments, text chunks). Used when the browser can't re-encode a photo (`web/photo.rs`)
//! and on every re-encoded file as a second safety net.

/// The file without its metadata, or `None` if the format is unknown or the file is malformed
/// (the caller must then refuse it: uploading it as is could leak a GPS position).
pub fn strip_metadata(bytes: &[u8], mime: &str) -> Option<Vec<u8>> {
    match mime {
        "image/jpeg" => strip_jpeg(bytes),
        "image/png" => strip_png(bytes),
        "image/webp" => strip_webp(bytes),
        _ => None,
    }
}

/// Drops APP1–APP13, APP15 (EXIF, XMP, IPTC, maker notes, ICC…) and COM segments.
/// Keeps APP0 (JFIF) and APP14 (Adobe: CMYK files need it to decode with the right colors).
fn strip_jpeg(bytes: &[u8]) -> Option<Vec<u8>> {
    const SOI: u8 = 0xD8;
    const SOS: u8 = 0xDA;
    const EOI: u8 = 0xD9;
    const APP14: u8 = 0xEE;
    const COM: u8 = 0xFE;

    if bytes.get(..2)? != [0xFF, SOI] {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&[0xFF, SOI]);
    let mut i = 2;
    loop {
        if *bytes.get(i)? != 0xFF {
            return None;
        }
        // Any number of 0xFF fill bytes may precede a marker.
        while *bytes.get(i + 1)? == 0xFF {
            i += 1;
        }
        let marker = bytes[i + 1];
        match marker {
            EOI => {
                out.extend_from_slice(&[0xFF, EOI]);
                return Some(out);
            }
            // Standalone markers (no length) never carry metadata.
            0x01 | 0xD0..=0xD7 => {
                out.extend_from_slice(&[0xFF, marker]);
                i += 2;
                continue;
            }
            _ => {}
        }
        let len = usize::from(u16::from_be_bytes([*bytes.get(i + 2)?, *bytes.get(i + 3)?]));
        if len < 2 {
            return None;
        }
        let end = i.checked_add(2 + len)?;
        let segment = bytes.get(i..end)?;
        let metadata = matches!(marker, 0xE1..=0xEF | COM) && marker != APP14;
        if !metadata {
            out.extend_from_slice(segment);
        }
        i = end;
        if marker == SOS {
            // Entropy-coded data and everything after it: metadata segments only come
            // before the first scan in the files cameras and phones produce.
            out.extend_from_slice(&bytes[i..]);
            return Some(out);
        }
    }
}

/// Drops the text (`tEXt`, `zTXt`, `iTXt`), `eXIf` and `tIME` chunks.
fn strip_png(bytes: &[u8]) -> Option<Vec<u8>> {
    const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    const DROPPED: [&[u8; 4]; 5] = [b"tEXt", b"zTXt", b"iTXt", b"eXIf", b"tIME"];

    if bytes.get(..8)? != SIGNATURE {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&SIGNATURE);
    let mut i = 8;
    loop {
        let len = usize::try_from(u32::from_be_bytes(bytes.get(i..i + 4)?.try_into().ok()?)).ok()?;
        let kind = bytes.get(i + 4..i + 8)?;
        // length + type + data + CRC
        let end = i.checked_add(12)?.checked_add(len)?;
        let chunk = bytes.get(i..end)?;
        if !DROPPED.iter().any(|d| d.as_slice() == kind) {
            out.extend_from_slice(chunk);
        }
        i = end;
        if kind == b"IEND" {
            return Some(out);
        }
    }
}

/// Drops the `EXIF` and `XMP ` chunks and clears their flags in the `VP8X` header.
fn strip_webp(bytes: &[u8]) -> Option<Vec<u8>> {
    const EXIF_FLAG: u8 = 0x08;
    const XMP_FLAG: u8 = 0x04;

    if bytes.get(..4)? != b"RIFF" || bytes.get(8..12)? != b"WEBP" {
        return None;
    }
    let riff_len = usize::try_from(u32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?)).ok()?;
    let body = bytes.get(12..riff_len.checked_add(8)?)?;

    let mut chunks = Vec::with_capacity(body.len());
    let mut i = 0;
    while i < body.len() {
        let kind = body.get(i..i + 4)?;
        let len = usize::try_from(u32::from_le_bytes(body.get(i + 4..i + 8)?.try_into().ok()?)).ok()?;
        let data_end = (i + 8).checked_add(len)?;
        if data_end > body.len() {
            return None;
        }
        // Chunks are padded to an even size; a last chunk may omit its pad byte.
        let end = (data_end + len % 2).min(body.len());
        let chunk = &body[i..end];
        match kind {
            b"EXIF" | b"XMP " => {}
            b"VP8X" => {
                let start = chunks.len();
                chunks.extend_from_slice(chunk);
                *chunks.get_mut(start + 8)? &= !(EXIF_FLAG | XMP_FLAG);
            }
            _ => chunks.extend_from_slice(chunk),
        }
        i = end;
    }

    let mut out = Vec::with_capacity(chunks.len() + 12);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&u32::try_from(chunks.len() + 4).ok()?.to_le_bytes());
    out.extend_from_slice(b"WEBP");
    out.extend_from_slice(&chunks);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg_segment(marker: u8, data: &[u8]) -> Vec<u8> {
        let len = u16::try_from(data.len() + 2).expect("small segment");
        [&[0xFF, marker][..], &len.to_be_bytes(), data].concat()
    }

    #[test]
    fn jpeg_loses_exif_xmp_and_comments_but_keeps_the_image() {
        let jfif = jpeg_segment(0xE0, b"JFIF\0\x01\x01");
        let exif = jpeg_segment(0xE1, b"Exif\0\0GPS 45.1N 6.2E");
        let xmp = jpeg_segment(0xE1, b"http://ns.adobe.com/xap/1.0/\0<x:xmpmeta/>");
        let adobe = jpeg_segment(0xEE, b"Adobe\0");
        let comment = jpeg_segment(0xFE, b"taken at home");
        let dqt = jpeg_segment(0xDB, &[0; 65]);
        let sos = jpeg_segment(0xDA, &[1, 1, 0, 0, 63, 0]);
        let scan = [0x12, 0xFF, 0x00, 0x34, 0xFF, 0xD0, 0x56, 0xFF, 0xD9];
        let file = [&[0xFF, 0xD8][..], &jfif, &exif, &xmp, &adobe, &comment, &dqt, &sos, &scan].concat();

        let stripped = strip_metadata(&file, "image/jpeg").expect("valid jpeg");
        let expected = [&[0xFF, 0xD8][..], &jfif, &adobe, &dqt, &sos, &scan].concat();
        assert_eq!(stripped, expected);
    }

    #[test]
    fn jpeg_fill_bytes_and_standalone_markers_are_handled() {
        let dqt = jpeg_segment(0xDB, &[0; 3]);
        let exif = jpeg_segment(0xE1, b"Exif\0\0");
        let file = [&[0xFF, 0xD8, 0xFF][..], &exif, &dqt, &[0xFF, 0xD9]].concat();
        let stripped = strip_metadata(&file, "image/jpeg").expect("valid jpeg");
        assert_eq!(stripped, [&[0xFF, 0xD8][..], &dqt, &[0xFF, 0xD9]].concat());
    }

    #[test]
    fn malformed_jpegs_are_refused() {
        assert_eq!(strip_metadata(b"", "image/jpeg"), None);
        assert_eq!(strip_metadata(&[0xFF, 0xD8], "image/jpeg"), None, "no segment");
        assert_eq!(strip_metadata(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x10, 1], "image/jpeg"), None, "truncated");
        assert_eq!(strip_metadata(&[0xFF, 0xD8, 0x00, 0xE1], "image/jpeg"), None, "not a marker");
        assert_eq!(strip_metadata(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x01], "image/jpeg"), None, "bad length");
    }

    fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let len = u32::try_from(data.len()).expect("small chunk");
        [&len.to_be_bytes()[..], kind, data, &[0xAA, 0xBB, 0xCC, 0xDD]].concat()
    }

    #[test]
    fn png_loses_text_exif_and_time_chunks() {
        let sig = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let ihdr = png_chunk(b"IHDR", &[0; 13]);
        let text = png_chunk(b"tEXt", b"Author\0me");
        let exif = png_chunk(b"eXIf", b"MM\0*GPS");
        let time = png_chunk(b"tIME", &[7; 7]);
        let idat = png_chunk(b"IDAT", &[1, 2, 3]);
        let itxt = png_chunk(b"iTXt", b"XML:com.adobe.xmp\0");
        let iend = png_chunk(b"IEND", &[]);
        let file = [&sig[..], &ihdr, &text, &exif, &time, &idat, &itxt, &iend].concat();

        let stripped = strip_metadata(&file, "image/png").expect("valid png");
        assert_eq!(stripped, [&sig[..], &ihdr, &idat, &iend].concat());
        assert_eq!(strip_metadata(&file[..file.len() - 3], "image/png"), None, "truncated");
        assert_eq!(strip_metadata(b"\x89PNG", "image/png"), None);
    }

    fn webp_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let len = u32::try_from(data.len()).expect("small chunk");
        let pad: &[u8] = if data.len() % 2 == 1 { &[0] } else { &[] };
        [kind, &len.to_le_bytes()[..], data, pad].concat()
    }

    fn webp(chunks: &[Vec<u8>]) -> Vec<u8> {
        let body = chunks.concat();
        let len = u32::try_from(body.len() + 4).expect("small file");
        [&b"RIFF"[..], &len.to_le_bytes(), b"WEBP", &body].concat()
    }

    #[test]
    fn webp_loses_exif_and_xmp_and_their_flags() {
        let vp8x = |flags: u8| webp_chunk(b"VP8X", &[flags, 0, 0, 0, 9, 0, 0, 9, 0, 0]);
        let iccp = webp_chunk(b"ICCP", &[5; 4]);
        let image = webp_chunk(b"VP8 ", &[1, 2, 3]);
        let exif = webp_chunk(b"EXIF", b"MM\0*GPS");
        let xmp = webp_chunk(b"XMP ", b"<x:xmpmeta/>");
        let file = webp(&[vp8x(0x20 | 0x08 | 0x04), iccp.clone(), image.clone(), exif, xmp]);

        let stripped = strip_metadata(&file, "image/webp").expect("valid webp");
        assert_eq!(stripped, webp(&[vp8x(0x20), iccp, image]));
    }

    #[test]
    fn malformed_or_unknown_files_are_refused() {
        assert_eq!(strip_metadata(b"RIFF\x04\0\0\0WEBP", "image/webp"), Some(webp(&[])));
        assert_eq!(strip_metadata(b"RIFF\xFF\0\0\0WEBP", "image/webp"), None, "length past the end");
        assert_eq!(strip_metadata(b"RIFF\x0C\0\0\0WEBPEXIF", "image/webp"), None, "truncated chunk");
        assert_eq!(strip_metadata(b"GIF89a", "image/gif"), None);
    }
}
