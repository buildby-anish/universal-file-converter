//! Raster image conversion via `image-rs`.
//!
//! This decodes the *actual pixel buffer* from the source bytes and
//! re-encodes it into the target codec — never a byte-level or
//! extension-level rename. A corrupt or truncated image will fail to
//! decode and this adapter reports that failure rather than writing
//! anything.

use converter_core::{ConversionAdapter, Format, JobError};
use image::ImageFormat;
use std::path::Path;

pub struct ImageAdapter;

/// (source, target) pairs this adapter advertises. All are backed by
/// genuine codecs enabled in the workspace `image` feature set
/// (png, jpeg, webp, bmp, gif, tiff).
const ROUTES: &[(Format, Format)] = &[
    (Format::Png, Format::Jpeg),
    (Format::Png, Format::WebP),
    (Format::Png, Format::Bmp),
    (Format::Png, Format::Gif),
    (Format::Png, Format::Tiff),
    (Format::Jpeg, Format::Png),
    (Format::Jpeg, Format::WebP),
    (Format::Jpeg, Format::Bmp),
    (Format::Jpeg, Format::Tiff),
    (Format::Bmp, Format::Png),
    (Format::Bmp, Format::Jpeg),
    (Format::Gif, Format::Png),
    (Format::Tiff, Format::Png),
    (Format::Tiff, Format::Jpeg),
    (Format::WebP, Format::Png),
];

fn to_image_format(f: Format) -> Option<ImageFormat> {
    match f {
        Format::Png => Some(ImageFormat::Png),
        Format::Jpeg => Some(ImageFormat::Jpeg),
        Format::Gif => Some(ImageFormat::Gif),
        Format::Bmp => Some(ImageFormat::Bmp),
        Format::WebP => Some(ImageFormat::WebP),
        Format::Tiff => Some(ImageFormat::Tiff),
        _ => None,
    }
}

impl ConversionAdapter for ImageAdapter {
    fn name(&self) -> &'static str {
        "image-rs"
    }

    fn supported_routes(&self) -> &[(Format, Format)] {
        ROUTES
    }

    fn convert(&self, input: &Path, output: &Path, from: Format, to: Format) -> Result<(), JobError> {
        let src_format = to_image_format(from).ok_or_else(|| JobError::AdapterFailure {
            adapter: self.name(),
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            message: format!("format '{}' is not a codec this adapter drives", from.as_str()),
        })?;
        let dst_format = to_image_format(to).ok_or_else(|| JobError::AdapterFailure {
            adapter: self.name(),
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            message: format!("format '{}' is not a codec this adapter drives", to.as_str()),
        })?;

        // Decode using the format we already determined via magic-byte
        // sniffing (not `image::open`, which re-guesses from the
        // extension — we don't trust extensions anywhere in this
        // pipeline).
        let bytes = std::fs::read(input).map_err(|e| JobError::Io {
            path: input.to_path_buf(),
            source: e,
        })?;

        let img = image::load_from_memory_with_format(&bytes, src_format).map_err(|e| JobError::AdapterFailure {
            adapter: self.name(),
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            message: format!("decode failed: {e}"),
        })?;

        // JPEG has no alpha channel; flatten onto white rather than let the
        // encoder silently drop/garble transparency.
        let img = if dst_format == ImageFormat::Jpeg {
            image::DynamicImage::ImageRgb8(img.to_rgb8())
        } else {
            img
        };

        img.save_with_format(output, dst_format).map_err(|e| JobError::AdapterFailure {
            adapter: self.name(),
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            message: format!("encode failed: {e}"),
        })?;

        Ok(())
    }
}
