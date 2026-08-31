use converter_adapters::build_default_registry;
use converter_core::{run_job, Format};
use image::RgbImage;
use tempfile::tempdir;

#[test]
fn test_end_to_end_data_pipeline() {
    let dir = tempdir().unwrap();
    let registry = build_default_registry();

    // 1. Write JSON
    let json_file = dir.path().join("users.json");
    std::fs::write(&json_file, r#"[{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]"#).unwrap();

    // JSON -> YAML
    let report_yaml = run_job(&registry, &json_file, Format::Yaml, dir.path(), "users").unwrap();
    assert_eq!(report_yaml.from, Format::Json);
    assert_eq!(report_yaml.to, Format::Yaml);
    assert!(report_yaml.output.exists());

    // YAML -> CSV
    let report_csv = run_job(&registry, &report_yaml.output, Format::Csv, dir.path(), "users_from_yaml").unwrap();
    assert_eq!(report_csv.from, Format::Yaml);
    assert_eq!(report_csv.to, Format::Csv);
    assert!(report_csv.output.exists());

    // CSV -> TOML
    let report_toml = run_job(&registry, &report_csv.output, Format::Toml, dir.path(), "users_from_csv").unwrap();
    assert_eq!(report_toml.from, Format::Csv);
    assert_eq!(report_toml.to, Format::Toml);
    assert!(report_toml.output.exists());
}

#[test]
fn test_end_to_end_markup_pipeline() {
    let dir = tempdir().unwrap();
    let registry = build_default_registry();

    let md_file = dir.path().join("doc.md");
    std::fs::write(&md_file, "# Main Title\n\n- Point 1\n- Point 2\n\n```rust\nlet x = 42;\n```\n").unwrap();

    // MD -> HTML
    let report_html = run_job(&registry, &md_file, Format::Html, dir.path(), "doc").unwrap();
    assert_eq!(report_html.from, Format::Markdown);
    assert_eq!(report_html.to, Format::Html);
    assert!(report_html.output.exists());
    let html_str = std::fs::read_to_string(&report_html.output).unwrap();
    assert!(html_str.contains("<h1>Main Title</h1>"));

    // HTML -> PlainText
    let report_txt = run_job(&registry, &report_html.output, Format::PlainText, dir.path(), "doc").unwrap();
    assert_eq!(report_txt.from, Format::Html);
    assert_eq!(report_txt.to, Format::PlainText);
    assert!(report_txt.output.exists());
    let txt_str = std::fs::read_to_string(&report_txt.output).unwrap();
    assert!(txt_str.contains("Main Title"));
}

#[test]
fn test_end_to_end_image_pipeline() {
    let dir = tempdir().unwrap();
    let registry = build_default_registry();

    let png_file = dir.path().join("canvas.png");
    let mut img = RgbImage::new(10, 10);
    img.put_pixel(0, 0, image::Rgb([255, 0, 0]));
    img.save_with_format(&png_file, image::ImageFormat::Png).unwrap();

    // PNG -> WebP
    let report_webp = run_job(&registry, &png_file, Format::WebP, dir.path(), "canvas").unwrap();
    assert_eq!(report_webp.from, Format::Png);
    assert_eq!(report_webp.to, Format::WebP);
    assert!(report_webp.output.exists());

    // WebP -> BMP
    let report_bmp = run_job(&registry, &report_webp.output, Format::Bmp, dir.path(), "canvas").unwrap();
    assert_eq!(report_bmp.from, Format::WebP);
    assert_eq!(report_bmp.to, Format::Bmp);
    assert!(report_bmp.output.exists());

    // BMP -> TIFF
    let report_tiff = run_job(&registry, &report_bmp.output, Format::Tiff, dir.path(), "canvas").unwrap();
    assert_eq!(report_tiff.from, Format::Bmp);
    assert_eq!(report_tiff.to, Format::Tiff);
    assert!(report_tiff.output.exists());
}
