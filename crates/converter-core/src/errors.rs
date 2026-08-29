//! Central, strongly-typed error hierarchy for `converter-core`.
//!
//! Every fallible operation in the pipeline returns one of these variants
//! rather than an opaque `anyhow`-style error, so callers (CLI, GUI, or
//! embedders of this crate) can pattern-match and react programmatically
//! (e.g. retry on `Io`, surface `Validation` failures distinctly from
//! `Unsupported` routes).

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DetectError {
    #[error("input file does not exist: {0}")]
    NotFound(PathBuf),

    #[error("input file is empty (0 bytes): {0}")]
    EmptyFile(PathBuf),

    #[error("could not read enough bytes to sniff format from {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("no registered sniffer recognized the byte signature of {0}")]
    UnknownSignature(PathBuf),
}

#[derive(Debug, Error)]
pub enum RouteError {
    #[error("no adapter is registered that can convert {from} -> {to}")]
    NoRoute { from: String, to: String },

    #[error("source and target formats are identical ({0}); nothing to convert")]
    IdenticalFormats(String),

    #[error("target format '{0}' is not registered in the format registry")]
    UnknownTargetFormat(String),
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("output file was not created at expected path: {0}")]
    OutputMissing(PathBuf),

    #[error("output file is empty (0 bytes), conversion likely failed silently: {0}")]
    OutputEmpty(PathBuf),

    #[error("output file signature did not match expected target format '{expected}' (got: {actual:?})")]
    SignatureMismatch { expected: String, actual: Option<String> },

    #[error("output failed structural integrity check: {0}")]
    StructuralIntegrity(String),
}

#[derive(Debug, Error)]
pub enum JobError {
    #[error(transparent)]
    Detect(#[from] DetectError),

    #[error(transparent)]
    Route(#[from] RouteError),

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error("adapter '{adapter}' failed to convert {input} -> {output}: {message}")]
    AdapterFailure {
        adapter: &'static str,
        input: PathBuf,
        output: PathBuf,
        message: String,
    },

    #[error("filesystem I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create temporary working file/dir: {0}")]
    Tempfile(#[source] std::io::Error),

    #[error("refusing to overwrite existing output and collision policy exhausted after {attempts} attempts: {path}")]
    CollisionExhausted { path: PathBuf, attempts: u32 },

    #[error("external tool '{tool}' is not installed or not on PATH")]
    MissingExternalTool { tool: &'static str },

    #[error("external tool '{tool}' exited with non-zero status {code:?}: {stderr}")]
    ExternalToolFailed {
        tool: &'static str,
        code: Option<i32>,
        stderr: String,
    },
}
