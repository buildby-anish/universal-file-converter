//! `converter-core`: format detection, adapter registry, route resolution,
//! and the safe (tempfile -> validate -> atomic rename) job pipeline.
//!
//! This crate defines *only* the traits and mechanics of conversion. It
//! has no knowledge of any specific encoding library or external tool —
//! see `converter-adapters` for concrete `ConversionAdapter`
//! implementations (image codecs, headless LibreOffice, PDF handling).

pub mod detection;
pub mod errors;
pub mod job;
pub mod registry;
pub mod router;
pub mod validation;

pub use detection::{detect, sniff_bytes, Format};
pub use errors::{DetectError, JobError, RouteError, ValidationError};
pub use job::{run_job, JobReport};
pub use registry::{ConversionAdapter, Registry};
