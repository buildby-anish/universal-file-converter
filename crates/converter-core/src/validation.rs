//! Post-conversion validation.
//!
//! An adapter returning `Ok(())` is not, by itself, proof that a real
//! conversion happened — an external tool can exit 0 while writing an
//! empty or truncated file. This module re-sniffs the *output* bytes and
//! confirms they actually match the target format's signature before the
//! job pipeline will report success or perform the atomic rename.

use crate::detection::{sniff_bytes, Format};
use crate::errors::ValidationError;
use std::path::Path;

/// Validate that `output` exists, is non-empty, and its byte signature
/// matches `expected`. Container formats (docx/odt/zip) are validated at
/// the container level only here; adapters with stronger guarantees (e.g.
/// re-parsing a PDF's xref table) should layer additional checks in their
/// own `convert()` before returning.
pub fn validate_output(output: &Path, expected: Format) -> Result<(), ValidationError> {
    if !output.exists() {
        return Err(ValidationError::OutputMissing(output.to_path_buf()));
    }

    let metadata = std::fs::metadata(output).map_err(|_| ValidationError::OutputMissing(output.to_path_buf()))?;
    if metadata.len() == 0 {
        return Err(ValidationError::OutputEmpty(output.to_path_buf()));
    }

    let bytes = std::fs::read(output).map_err(|_| ValidationError::OutputMissing(output.to_path_buf()))?;
    let window = &bytes[..bytes.len().min(512)];

    let actual = sniff_bytes(window);
    if is_compatible(actual, expected) {
        Ok(())
    } else {
        Err(ValidationError::SignatureMismatch {
            expected: expected.as_str().to_string(),
            actual: actual.map(|a| a.as_str().to_string()),
        })
    }
}

fn is_compatible(actual: Option<Format>, expected: Format) -> bool {
    match (actual, expected) {
        (Some(a), e) if a == e => true,
        // Any text format output satisfies any text-based expected format
        (
            Some(Format::Json | Format::Yaml | Format::Toml | Format::Csv | Format::Markdown | Format::Html | Format::PlainText),
            Format::Json | Format::Yaml | Format::Toml | Format::Csv | Format::Markdown | Format::Html | Format::PlainText,
        ) => true,
        _ => false,
    }
}


