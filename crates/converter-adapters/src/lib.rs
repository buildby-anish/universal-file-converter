pub mod data_adapter;
pub mod doc_adapter;
pub mod image_adapter;
pub mod markup_adapter;
pub mod pdf_adapter;

use converter_core::Registry;
use data_adapter::DataAdapter;
use doc_adapter::LibreOfficeAdapter;
use image_adapter::ImageAdapter;
use markup_adapter::MarkupAdapter;
use pdf_adapter::PdfAdapter;

/// Build a `Registry` with every adapter this crate provides.
/// Native codec adapters are registered before the LibreOffice fallback
/// so a route with two possible implementations prefers the faster,
/// dependency-free path (`Registry::find_adapter` returns the first
/// match).
pub fn build_default_registry() -> Registry {
    let mut registry = Registry::new();
    registry.register(Box::new(ImageAdapter));
    registry.register(Box::new(DataAdapter));
    registry.register(Box::new(MarkupAdapter));
    registry.register(Box::new(PdfAdapter));
    registry.register(Box::new(LibreOfficeAdapter::new()));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use converter_core::{ConversionAdapter, Format};
    use tempfile::tempdir;

    #[test]
    fn test_data_adapter_conversions() {
        let dir = tempdir().unwrap();
        let adapter = DataAdapter;

        // JSON -> YAML
        let json_path = dir.path().join("test.json");
        std::fs::write(&json_path, r#"{"name": "ufc", "stars": 100}"#).unwrap();
        let yaml_path = dir.path().join("test.yaml");
        adapter.convert(&json_path, &yaml_path, Format::Json, Format::Yaml).unwrap();
        let yaml_content = std::fs::read_to_string(&yaml_path).unwrap();
        assert!(yaml_content.contains("name: ufc"));

        // YAML -> TOML
        let toml_path = dir.path().join("test.toml");
        adapter.convert(&yaml_path, &toml_path, Format::Yaml, Format::Toml).unwrap();
        let toml_content = std::fs::read_to_string(&toml_path).unwrap();
        assert!(toml_content.contains("name = \"ufc\""));

        // CSV -> JSON
        let csv_path = dir.path().join("test.csv");
        std::fs::write(&csv_path, "city,country\nTokyo,Japan\nParis,France\n").unwrap();
        let json_from_csv = dir.path().join("from_csv.json");
        adapter.convert(&csv_path, &json_from_csv, Format::Csv, Format::Json).unwrap();
        let json_content = std::fs::read_to_string(&json_from_csv).unwrap();
        assert!(json_content.contains("Tokyo"));
    }

    #[test]
    fn test_markup_adapter_conversions() {
        let dir = tempdir().unwrap();
        let adapter = MarkupAdapter;

        // Markdown -> HTML
        let md_path = dir.path().join("test.md");
        std::fs::write(&md_path, "# Header\n\nThis is **bold** text.").unwrap();
        let html_path = dir.path().join("test.html");
        adapter.convert(&md_path, &html_path, Format::Markdown, Format::Html).unwrap();
        let html_content = std::fs::read_to_string(&html_path).unwrap();
        assert!(html_content.contains("<h1>Header</h1>"));
        assert!(html_content.contains("<strong>bold</strong>"));

        // HTML -> Markdown
        let md_from_html = dir.path().join("from_html.md");
        adapter.convert(&html_path, &md_from_html, Format::Html, Format::Markdown).unwrap();
        let md_content = std::fs::read_to_string(&md_from_html).unwrap();
        assert!(md_content.contains("# Header"));
        assert!(md_content.contains("**bold**"));

        // Markdown -> PlainText
        let txt_path = dir.path().join("test.txt");
        adapter.convert(&md_path, &txt_path, Format::Markdown, Format::PlainText).unwrap();
        let txt_content = std::fs::read_to_string(&txt_path).unwrap();
        assert!(txt_content.contains("Header"));
        assert!(txt_content.contains("This is bold text."));
    }

    #[test]
    fn test_image_adapter_conversions() {
        let dir = tempdir().unwrap();
        let adapter = ImageAdapter;

        // Create small 2x2 PNG
        let png_path = dir.path().join("test.png");
        let img = image::RgbImage::new(2, 2);
        img.save_with_format(&png_path, image::ImageFormat::Png).unwrap();

        // PNG -> WebP
        let webp_path = dir.path().join("test.webp");
        adapter.convert(&png_path, &webp_path, Format::Png, Format::WebP).unwrap();
        assert!(webp_path.exists());

        // WebP -> BMP
        let bmp_path = dir.path().join("test.bmp");
        adapter.convert(&webp_path, &bmp_path, Format::WebP, Format::Bmp).unwrap();
        assert!(bmp_path.exists());
    }
}


