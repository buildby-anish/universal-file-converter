pub mod doc_adapter;
pub mod image_adapter;
pub mod pdf_adapter;

use converter_core::Registry;
use doc_adapter::LibreOfficeAdapter;
use image_adapter::ImageAdapter;
use pdf_adapter::PdfAdapter;

/// Build a `Registry` with every adapter this crate provides.
/// Native codec adapters are registered before the LibreOffice fallback
/// so a route with two possible implementations prefers the faster,
/// dependency-free path (`Registry::find_adapter` returns the first
/// match).
pub fn build_default_registry() -> Registry {
    let mut registry = Registry::new();
    registry.register(Box::new(ImageAdapter));
    registry.register(Box::new(PdfAdapter));
    registry.register(Box::new(LibreOfficeAdapter::new()));
    registry
}
