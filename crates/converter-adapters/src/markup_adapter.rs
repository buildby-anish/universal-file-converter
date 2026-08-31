//! Markup & web document adapter (Markdown, HTML, PlainText).
//!
//! Provides conversions:
//! - Markdown -> HTML
//! - Markdown -> PlainText
//! - HTML -> Markdown
//! - HTML -> PlainText

use converter_core::{ConversionAdapter, Format, JobError};
use pulldown_cmark::{html, Event, Options, Parser, Tag, TagEnd};
use std::path::Path;

pub struct MarkupAdapter;

const ROUTES: &[(Format, Format)] = &[
    (Format::Markdown, Format::Html),
    (Format::Markdown, Format::PlainText),
    (Format::Html, Format::Markdown),
    (Format::Html, Format::PlainText),
    (Format::PlainText, Format::Html),
    (Format::PlainText, Format::Markdown),
];

impl ConversionAdapter for MarkupAdapter {
    fn name(&self) -> &'static str {
        "markup-adapter"
    }

    fn supported_routes(&self) -> &[(Format, Format)] {
        ROUTES
    }

    fn convert(&self, input: &Path, output: &Path, from: Format, to: Format) -> Result<(), JobError> {
        let content = std::fs::read_to_string(input).map_err(|e| JobError::Io {
            path: input.to_path_buf(),
            source: e,
        })?;

        let result = match (from, to) {
            (Format::Markdown | Format::PlainText, Format::Html) => markdown_to_html(&content),
            (Format::Markdown, Format::PlainText) => markdown_to_plain_text(&content),
            (Format::Html, Format::Markdown) => html_to_markdown(&content),
            (Format::Html, Format::PlainText) => html_to_plain_text(&content),
            (Format::PlainText, Format::Markdown) => content,
            _ => {

                return Err(JobError::AdapterFailure {
                    adapter: self.name(),
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    message: format!("unsupported markup route {} -> {}", from.as_str(), to.as_str()),
                })
            }
        };

        std::fs::write(output, result).map_err(|e| JobError::Io {
            path: output.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }
}

fn markdown_to_html(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(md, options);
    let mut html_body = String::new();
    html::push_html(&mut html_body, parser);

    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"UTF-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n  <title>Document</title>\n  <style>\n    body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; max-width: 800px; margin: 40px auto; padding: 0 20px; color: #24292e; }}\n    pre {{ background: #f6f8fa; padding: 16px; border-radius: 6px; overflow-x: auto; }}\n    code {{ font-family: monospace; background: #f6f8fa; padding: 2px 4px; border-radius: 3px; }}\n    table {{ border-collapse: collapse; width: 100%; margin: 16px 0; }}\n    th, td {{ border: 1px solid #dfe2e5; padding: 8px 12px; text-align: left; }}\n    th {{ background: #f6f8fa; }}\n    blockquote {{ border-left: 4px solid #dfe2e5; margin: 0; padding-left: 16px; color: #6a737d; }}\n  </style>\n</head>\n<body>\n{}\n</body>\n</html>",
        html_body
    )
}

fn markdown_to_plain_text(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(md, options);
    let mut out = String::new();
    let mut last_was_newline = true;

    for event in parser {
        match event {
            Event::Text(t) => {
                out.push_str(&t);
                last_was_newline = false;
            }
            Event::Code(c) => {
                out.push_str(&c);
                last_was_newline = false;
            }
            Event::Start(Tag::Paragraph) | Event::Start(Tag::Heading { .. }) | Event::Start(Tag::Item) => {
                if !last_was_newline && !out.is_empty() {
                    out.push('\n');
                }
            }
            Event::End(TagEnd::Paragraph) | Event::End(TagEnd::Heading(..)) | Event::End(TagEnd::Item) => {
                out.push('\n');
                last_was_newline = true;
            }
            Event::SoftBreak | Event::HardBreak => {
                out.push('\n');
                last_was_newline = true;
            }
            _ => {}
        }
    }

    out.trim().to_string() + "\n"
}


fn html_to_plain_text(html_str: &str) -> String {
    let mut in_tag = false;
    let mut in_script_or_style = false;
    let mut tag_name = String::new();
    let mut result = String::new();

    let mut chars = html_str.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            in_tag = true;
            tag_name.clear();
            while let Some(&next) = chars.peek() {
                if next == '>' || next.is_whitespace() {
                    break;
                }
                tag_name.push(chars.next().unwrap());
            }
            let tag_lower = tag_name.to_lowercase();
            if tag_lower == "script" || tag_lower == "style" {
                in_script_or_style = true;
            } else if tag_lower == "/script" || tag_lower == "/style" {
                in_script_or_style = false;
            } else if tag_lower == "p" || tag_lower == "/p" || tag_lower == "br" || tag_lower.starts_with('h') || tag_lower == "li" {
                if !result.ends_with('\n') {
                    result.push('\n');
                }
            }
            continue;
        }

        if c == '>' {
            in_tag = false;
            continue;
        }

        if !in_tag && !in_script_or_style {
            result.push(c);
        }
    }

    decode_html_entities(&result)
}

fn html_to_markdown(html_str: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    let mut in_script_or_style = false;
    let mut tag_buffer = String::new();

    let mut chars = html_str.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            in_tag = true;
            tag_buffer.clear();
            continue;
        }
        if c == '>' && in_tag {
            in_tag = false;
            let tag = tag_buffer.trim().to_lowercase();
            if tag == "script" || tag == "style" || tag == "head" {
                in_script_or_style = true;
            } else if tag == "/script" || tag == "/style" || tag == "/head" {
                in_script_or_style = false;
            } else if !in_script_or_style {
                if tag == "h1" {
                    if !out.ends_with("\n\n") { out.push_str("\n\n"); }
                    out.push_str("# ");
                } else if tag == "h2" {
                    if !out.ends_with("\n\n") { out.push_str("\n\n"); }
                    out.push_str("## ");
                } else if tag == "h3" {
                    if !out.ends_with("\n\n") { out.push_str("\n\n"); }
                    out.push_str("### ");
                } else if tag == "h4" {
                    if !out.ends_with("\n\n") { out.push_str("\n\n"); }
                    out.push_str("#### ");
                } else if tag == "p" || tag == "div" {
                    if !out.ends_with("\n\n") { out.push_str("\n\n"); }
                } else if tag == "/p" || tag == "/h1" || tag == "/h2" || tag == "/h3" || tag == "/h4" {
                    out.push('\n');
                } else if tag == "br" {
                    out.push('\n');
                } else if tag == "b" || tag == "strong" || tag == "/b" || tag == "/strong" {
                    out.push_str("**");
                } else if tag == "i" || tag == "em" || tag == "/i" || tag == "/em" {
                    out.push('*');
                } else if tag == "code" || tag == "/code" {
                    out.push('`');
                } else if tag == "pre" {
                    out.push_str("\n```\n");
                } else if tag == "/pre" {
                    out.push_str("\n```\n");
                } else if tag == "li" {
                    out.push_str("\n- ");
                } else if tag == "hr" {
                    out.push_str("\n\n---\n\n");
                }
            }
            continue;
        }
        if in_tag {
            tag_buffer.push(c);
        } else if !in_script_or_style {
            out.push(c);
        }
    }

    let decoded = decode_html_entities(&out);

    // Normalize excessive newlines
    let mut cleaned = String::new();
    let mut consecutive_newlines = 0;
    for ch in decoded.chars() {
        if ch == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 2 {
                cleaned.push(ch);
            }
        } else {
            consecutive_newlines = 0;
            cleaned.push(ch);
        }
    }

    cleaned.trim().to_string() + "\n"
}

fn decode_html_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
}
