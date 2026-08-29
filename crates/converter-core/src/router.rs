//! Resolves a requested (source, target) format pair to a concrete
//! adapter, or fails fast with a typed `RouteError` before any filesystem
//! work happens.

use crate::detection::Format;
use crate::errors::RouteError;
use crate::registry::{ConversionAdapter, Registry};

/// Resolve `from -> to` against `registry`, or return a typed error
/// explaining exactly why no conversion can proceed.
pub fn resolve_route<'a>(
    registry: &'a Registry,
    from: Format,
    to: Format,
) -> Result<&'a dyn ConversionAdapter, RouteError> {
    if from == to {
        return Err(RouteError::IdenticalFormats(from.as_str().to_string()));
    }

    registry.find_adapter(from, to).ok_or_else(|| RouteError::NoRoute {
        from: from.as_str().to_string(),
        to: to.as_str().to_string(),
    })
}
