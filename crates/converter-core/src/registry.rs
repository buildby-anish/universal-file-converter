//! Central adapter registry.
//!
//! Concrete engines (image codecs, headless LibreOffice, PDF libraries)
//! live in `converter-adapters` and implement the `ConversionAdapter`
//! trait defined here. `converter-core` never depends on any specific
//! engine — it only knows this trait, which keeps detection/routing/job
//! execution fully decoupled from *how* a byte gets transformed.

use crate::detection::Format;
use crate::errors::JobError;
use std::path::Path;

/// A single concrete conversion capability: "I can turn a `from`-format
/// file into a `to`-format file." Adapters advertise every pair they
/// support via `supported_routes()`, and `convert()` performs the actual
/// engine call for exactly one such pair.
///
/// Implementors MUST:
/// - write output only to the `output` path they are given (a fresh
///   tempfile path chosen by the job pipeline — never the input path),
/// - return `Err` on any engine failure rather than writing a partial or
///   placeholder file,
/// - never shell out via a command string; use `std::process::Command`
///   with an explicit argument array if invoking an external tool.
pub trait ConversionAdapter: Send + Sync {
    /// Stable identifier used in error messages and logs, e.g. "image-rs".
    fn name(&self) -> &'static str;

    /// All (from, to) format pairs this adapter can perform.
    fn supported_routes(&self) -> &[(Format, Format)];

    /// Perform the conversion. `input` is guaranteed to exist and to have
    /// already been sniffed as `from`. `output` is a not-yet-existing
    /// tempfile path the adapter should create.
    fn convert(&self, input: &Path, output: &Path, from: Format, to: Format) -> Result<(), JobError>;
}

/// Holds every registered adapter and answers "who can do this route?".
pub struct Registry {
    adapters: Vec<Box<dyn ConversionAdapter>>,
}

impl Registry {
    pub fn new() -> Self {
        Self { adapters: Vec::new() }
    }

    pub fn register(&mut self, adapter: Box<dyn ConversionAdapter>) {
        self.adapters.push(adapter);
    }

    /// Find the first adapter advertising support for `from -> to`.
    /// If multiple adapters claim the same route, the first registered
    /// wins — callers wanting priority control should register in the
    /// desired order (e.g. native codec adapters before the LibreOffice
    /// fallback adapter).
    pub fn find_adapter(&self, from: Format, to: Format) -> Option<&dyn ConversionAdapter> {
        self.adapters
            .iter()
            .find(|a| a.supported_routes().iter().any(|&(f, t)| f == from && t == to))
            .map(|b| b.as_ref())
    }

    pub fn all_routes(&self) -> Vec<(Format, Format, &'static str)> {
        self.adapters
            .iter()
            .flat_map(|a| a.supported_routes().iter().map(move |&(f, t)| (f, t, a.name())))
            .collect()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}
