//! PDF adapter.
//!
//! Two genuine, independently-verifiable routes:
//! - `Pdf -> PlainText`: parses the PDF object graph with `pdf-extract`
//!   and pulls real text runs (not a byte dump).
//! - `PlainText -> Pdf`: lays out real text objects with `printpdf`,
//!   producing a structurally valid PDF (verified by re-parsing the xref
//!   table with `lopdf` before reporting success).

use converter_core::{ConversionAdapter, Format, JobError};
use printpdf::{BuiltinFont, Mm, PdfDocument};
use std::io::{BufWriter, Write};
use std::path::Path;

pub struct PdfAdapter;

const ROUTES: &[(Format, Format)] = &[(Format::Pdf, Format::PlainText), (Format::PlainText, Format::Pdf)];

const PAGE_WIDTH_MM: f64 = 210.0; // A4
const PAGE_HEIGHT_MM: f64 = 297.0;
const MARGIN_MM: f64 = 20.0;
const FONT_SIZE: f64 = 11.0;
const LINE_HEIGHT_MM: f64 = 6.0;

impl ConversionAdapter for PdfAdapter {
    fn name(&self) -> &'static str {
        "pdf-adapter"
    }

    fn supported_routes(&self) -> &[(Format, Format)] {
        ROUTES
    }

    fn convert(&self, input: &Path, output: &Path, from: Format, to: Format) -> Result<(), JobError> {
        match (from, to) {
            (Format::Pdf, Format::PlainText) => pdf_to_text(self.name(), input, output),
            (Format::PlainText, Format::Pdf) => text_to_pdf(self.name(), input, output),
            _ => Err(JobError::AdapterFailure {
                adapter: self.name(),
                input: input.to_path_buf(),
                output: output.to_path_buf(),
                message: format!("unsupported route {} -> {}", from.as_str(), to.as_str()),
            }),
        }
    }
}

fn pdf_to_text(adapter: &'static str, input: &Path, output: &Path) -> Result<(), JobError> {
    // Structural sanity check first: a corrupt/truncated PDF should fail
    // here with a clear message rather than propagating a confusing
    // extraction-layer panic-adjacent error.
    let doc = lopdf::Document::load(input).map_err(|e| JobError::AdapterFailure {
        adapter,
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        message: format!("input failed PDF structural parse: {e}"),
    })?;
    if doc.get_pages().is_empty() {
        return Err(JobError::AdapterFailure {
            adapter,
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            message: "PDF parsed but contains zero pages".to_string(),
        });
    }

    let text = pdf_extract::extract_text(input).map_err(|e| JobError::AdapterFailure {
        adapter,
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        message: format!("text extraction failed: {e}"),
    })?;

    if text.trim().is_empty() {
        return Err(JobError::AdapterFailure {
            adapter,
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            message: "extraction produced no text (source may be a scanned/image-only PDF requiring OCR, \
                      which this adapter does not perform)"
                .to_string(),
        });
    }

    std::fs::write(output, text).map_err(|e| JobError::Io {
        path: output.to_path_buf(),
        source: e,
    })
}

fn text_to_pdf(adapter: &'static str, input: &Path, output: &Path) -> Result<(), JobError> {
    let content = std::fs::read_to_string(input).map_err(|e| JobError::Io {
        path: input.to_path_buf(),
        source: e,
    })?;

    let (doc, page1, layer1) = PdfDocument::new("Converted Document", Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), "Layer 1");
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| JobError::AdapterFailure {
        adapter,
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        message: format!("failed to load builtin font: {e}"),
    })?;

    let usable_height = PAGE_HEIGHT_MM - 2.0 * MARGIN_MM;
    let lines_per_page = (usable_height / LINE_HEIGHT_MM).floor() as usize;

    // Naive but real word-wrap so lines stay inside the page margins
    // rather than silently overflowing off the sheet.
    let wrapped = wrap_text(&content, 95);

    let mut page_idx = 0usize;
    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    let mut line_on_page = 0usize;

    for line in &wrapped {
        if line_on_page >= lines_per_page {
            let (new_page, new_layer) = doc.add_page(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), "Layer 1");
            current_layer = doc.get_page(new_page).get_layer(new_layer);
            page_idx += 1;
            line_on_page = 0;
        }
        let y = PAGE_HEIGHT_MM - MARGIN_MM - (line_on_page as f64) * LINE_HEIGHT_MM;
        current_layer.use_text(line, FONT_SIZE, Mm(MARGIN_MM), Mm(y), &font);
        line_on_page += 1;
    }
    let _ = page_idx;

    let file = std::fs::File::create(output).map_err(|e| JobError::Io {
        path: output.to_path_buf(),
        source: e,
    })?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).map_err(|e| JobError::AdapterFailure {
        adapter,
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        message: format!("failed to serialize PDF: {e}"),
    })?;
    writer.flush().map_err(|e| JobError::Io {
        path: output.to_path_buf(),
        source: e,
    })?;

    // Structural integrity check on our own output: re-parse the xref
    // table we just wrote before letting the job pipeline treat this as a
    // success.
    lopdf::Document::load(output).map_err(|e| JobError::AdapterFailure {
        adapter,
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        message: format!("wrote a PDF that failed to re-parse: {e}"),
    })?;

    Ok(())
}

fn wrap_text(content: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    for paragraph in content.split('\n') {
        if paragraph.is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if line.len() + word.len() + 1 > max_chars {
                out.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
        }
        if !line.is_empty() {
            out.push(line);
        }
    }
    out
}
