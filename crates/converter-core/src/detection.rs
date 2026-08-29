//! Magic-byte / structural format sniffing.
//!
//! Deliberately does NOT trust file extensions. Every format is identified
//! by reading its byte signature (and, where signatures collide or are
//! insufficient, a lightweight structural probe). This is what lets the
//! rest of the pipeline refuse to "convert" a `.png` that is actually a
//! renamed `.jpg`.

use crate::errors::DetectError;
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Formats this build of `converter-core` knows how to *identify*.
/// Adapters may support a subset of these for actual conversion — see
/// `registry.rs` for the adapter capability matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Png,
    Jpeg,
    Gif,
    Bmp,
    WebP,
    Tiff,
    Pdf,
    Zip, // also covers docx/xlsx/pptx/odt containers at the container level
    Docx,
    Odt,
    PlainText,
}

impl Format {
    pub fn as_str(&self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpeg",
            Format::Gif => "gif",
            Format::Bmp => "bmp",
            Format::WebP => "webp",
            Format::Tiff => "tiff",
            Format::Pdf => "pdf",
            Format::Zip => "zip",
            Format::Docx => "docx",
            Format::Odt => "odt",
            Format::PlainText => "txt",
        }
    }
}

/// Number of bytes read for signature sniffing. Large enough to cover the
/// longest signatures we check (RIFF/WEBP needs 12) plus headroom for the
/// ZIP-container sub-format probe.
const SNIFF_WINDOW: usize = 512;

/// Read the leading bytes of `path` and classify its format by signature.
///
/// Returns `DetectError::UnknownSignature` rather than falling back to the
/// file extension — an unrecognized signature is a hard stop, not a hint.
pub fn detect(path: &Path) -> Result<Format, DetectError> {
    if !path.exists() {
        return Err(DetectError::NotFound(path.to_path_buf()));
    }

    let mut file = File::open(path).map_err(|e| DetectError::Read {
        path: path.to_path_buf(),
        source: e,
    })?;

    let mut buf = vec![0u8; SNIFF_WINDOW];
    let n = file.read(&mut buf).map_err(|e| DetectError::Read {
        path: path.to_path_buf(),
        source: e,
    })?;

    if n == 0 {
        return Err(DetectError::EmptyFile(path.to_path_buf()));
    }
    buf.truncate(n);

    sniff_bytes(&buf).ok_or_else(|| DetectError::UnknownSignature(path.to_path_buf()))
}

/// Pure byte-signature classifier, split out from `detect` so adapters and
/// tests can sniff in-memory buffers (e.g. post-conversion validation)
/// without touching the filesystem.
pub fn sniff_bytes(buf: &[u8]) -> Option<Format> {
    if buf.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(Format::Png);
    }
    if buf.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(Format::Jpeg);
    }
    if buf.starts_with(b"GIF87a") || buf.starts_with(b"GIF89a") {
        return Some(Format::Gif);
    }
    if buf.starts_with(b"BM") {
        return Some(Format::Bmp);
    }
    if buf.len() >= 12 && &buf[0..4] == b"RIFF" && &buf[8..12] == b"WEBP" {
        return Some(Format::WebP);
    }
    if buf.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || buf.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
        return Some(Format::Tiff);
    }
    if buf.starts_with(b"%PDF-") {
        return Some(Format::Pdf);
    }
    if buf.starts_with(&[0x50, 0x4B, 0x03, 0x04]) || buf.starts_with(&[0x50, 0x4B, 0x05, 0x06]) {
        // Generic ZIP container signature. DOCX/ODT/XLSX are all ZIP
        // containers with a distinguishing internal manifest, so we probe
        // the central directory entries present in the sniff window rather
        // than claiming certainty from the outer signature alone.
        return Some(classify_zip_container(buf));
    }
    if buf.iter().take(256).all(|&b| b == 0x09 || b == 0x0A || b == 0x0D || (0x20..=0x7E).contains(&b) || b >= 0x80) {
        return Some(Format::PlainText);
    }
    None
}

/// Distinguish OOXML (docx) vs ODF (odt) vs a bare ZIP by looking for their
/// characteristic first-entry file names, which both formats place near the
/// start of the archive by convention.
fn classify_zip_container(buf: &[u8]) -> Format {
    let window = String::from_utf8_lossy(buf);
    if window.contains("word/document.xml") || window.contains("[Content_Types].xml") {
        Format::Docx
    } else if window.contains("mimetypeapplication/vnd.oasis.opendocument") {
        Format::Odt
    } else {
        Format::Zip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_png() {
        let sig = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
        assert_eq!(sniff_bytes(&sig), Some(Format::Png));
    }

    #[test]
    fn detects_jpeg() {
        let sig = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(sniff_bytes(&sig), Some(Format::Jpeg));
    }

    #[test]
    fn rejects_extension_spoofed_content() {
        // A file with .png extension but JPEG bytes must sniff as Jpeg,
        // not Png — the whole point of magic-byte detection.
        let sig = [0xFF, 0xD8, 0xFF, 0xE1];
        assert_eq!(sniff_bytes(&sig), Some(Format::Jpeg));
        assert_ne!(sniff_bytes(&sig), Some(Format::Png));
    }

    #[test]
    fn unknown_signature_returns_none() {
        let garbage = [0x00, 0x01, 0x02, 0x03];
        assert_eq!(sniff_bytes(&garbage), None);
    }
}
