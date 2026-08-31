//! Structured data conversion adapter (JSON, YAML, TOML, CSV).
//!
//! Provides bidirectional conversion across structured data formats:
//! - JSON <-> YAML
//! - JSON <-> TOML
//! - JSON <-> CSV
//! - YAML <-> TOML
//! - YAML <-> CSV
//! - TOML <-> CSV

use converter_core::{ConversionAdapter, Format, JobError};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

pub struct DataAdapter;

const ROUTES: &[(Format, Format)] = &[
    (Format::Json, Format::Yaml),
    (Format::Json, Format::Toml),
    (Format::Json, Format::Csv),
    (Format::Yaml, Format::Json),
    (Format::Yaml, Format::Toml),
    (Format::Yaml, Format::Csv),
    (Format::Toml, Format::Json),
    (Format::Toml, Format::Yaml),
    (Format::Toml, Format::Csv),
    (Format::Csv, Format::Json),
    (Format::Csv, Format::Yaml),
    (Format::Csv, Format::Toml),
    (Format::PlainText, Format::Json),
    (Format::PlainText, Format::Yaml),
    (Format::PlainText, Format::Toml),
    (Format::PlainText, Format::Csv),
];

impl ConversionAdapter for DataAdapter {
    fn name(&self) -> &'static str {
        "data-adapter"
    }

    fn supported_routes(&self) -> &[(Format, Format)] {
        ROUTES
    }

    fn convert(&self, input: &Path, output: &Path, from: Format, to: Format) -> Result<(), JobError> {
        let text = std::fs::read_to_string(input).map_err(|e| JobError::Io {
            path: input.to_path_buf(),
            source: e,
        })?;

        // 1. Decode input into generic serde_json::Value
        let value = match from {
            Format::Json => serde_json::from_str::<Value>(&text).map_err(|e| JobError::AdapterFailure {
                adapter: self.name(),
                input: input.to_path_buf(),
                output: output.to_path_buf(),
                message: format!("invalid JSON syntax: {e}"),
            })?,
            Format::Yaml => serde_yaml::from_str::<Value>(&text).map_err(|e| JobError::AdapterFailure {
                adapter: self.name(),
                input: input.to_path_buf(),
                output: output.to_path_buf(),
                message: format!("invalid YAML syntax: {e}"),
            })?,
            Format::Toml => {
                let toml_val: toml::Value = toml::from_str(&text).map_err(|e| JobError::AdapterFailure {
                    adapter: self.name(),
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    message: format!("invalid TOML syntax: {e}"),
                })?;
                serde_json::to_value(toml_val).map_err(|e| JobError::AdapterFailure {
                    adapter: self.name(),
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    message: format!("TOML to JSON conversion error: {e}"),
                })?
            }
            Format::Csv => csv_to_value(input, self.name(), output)?,
            Format::PlainText => {
                // Auto-detect structured format from plain text
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    v
                } else if let Ok(v) = serde_yaml::from_str::<Value>(&text) {
                    v
                } else if let Ok(toml_val) = toml::from_str::<toml::Value>(&text) {
                    serde_json::to_value(toml_val).unwrap_or(Value::Null)
                } else if let Ok(v) = csv_to_value(input, self.name(), output) {
                    v
                } else {
                    return Err(JobError::AdapterFailure {
                        adapter: self.name(),
                        input: input.to_path_buf(),
                        output: output.to_path_buf(),
                        message: "plain text file does not contain valid JSON, YAML, TOML, or CSV data".to_string(),
                    });
                }
            }
            _ => {
                return Err(JobError::AdapterFailure {
                    adapter: self.name(),
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    message: format!("unsupported source data format '{}'", from.as_str()),
                })
            }
        };


        // 2. Encode value to destination format
        let out_text = match to {
            Format::Json => serde_json::to_string_pretty(&value).map_err(|e| JobError::AdapterFailure {
                adapter: self.name(),
                input: input.to_path_buf(),
                output: output.to_path_buf(),
                message: format!("JSON serialization failed: {e}"),
            })?,
            Format::Yaml => serde_yaml::to_string(&value).map_err(|e| JobError::AdapterFailure {
                adapter: self.name(),
                input: input.to_path_buf(),
                output: output.to_path_buf(),
                message: format!("YAML serialization failed: {e}"),
            })?,
            Format::Toml => {
                // TOML requires the root value to be a table (map/object)
                let toml_val = if value.is_object() {
                    toml::to_string_pretty(&value)
                } else {
                    // Wrap in a root object if scalar or array
                    let wrapped = serde_json::json!({ "data": value });
                    toml::to_string_pretty(&wrapped)
                };
                toml_val.map_err(|e| JobError::AdapterFailure {
                    adapter: self.name(),
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    message: format!("TOML serialization failed: {e}"),
                })?
            }
            Format::Csv => value_to_csv(&value, self.name(), input, output)?,
            _ => {
                return Err(JobError::AdapterFailure {
                    adapter: self.name(),
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    message: format!("unsupported target data format '{}'", to.as_str()),
                })
            }
        };

        std::fs::write(output, out_text).map_err(|e| JobError::Io {
            path: output.to_path_buf(),
            source: e,
        })?;

        Ok(())
    }
}

fn csv_to_value(input: &Path, adapter: &'static str, output: &Path) -> Result<Value, JobError> {
    let mut rdr = csv::ReaderBuilder::new().has_headers(true).from_path(input).map_err(|e| JobError::AdapterFailure {
        adapter,
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        message: format!("failed to open CSV: {e}"),
    })?;

    let headers = rdr.headers().map_err(|e| JobError::AdapterFailure {
        adapter,
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        message: format!("failed to read CSV headers: {e}"),
    })?.clone();

    let mut records = Vec::new();
    for result in rdr.records() {
        let record = result.map_err(|e| JobError::AdapterFailure {
            adapter,
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            message: format!("CSV parse record error: {e}"),
        })?;

        let mut obj = serde_json::Map::new();
        for (i, field) in record.iter().enumerate() {
            let header = headers.get(i).unwrap_or("column");
            let parsed_field = if let Ok(n) = field.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(f) = field.parse::<f64>() {
                serde_json::Number::from_f64(f).map(Value::Number).unwrap_or_else(|| Value::String(field.to_string()))
            } else if field == "true" || field == "TRUE" {
                Value::Bool(true)
            } else if field == "false" || field == "FALSE" {
                Value::Bool(false)
            } else {
                Value::String(field.to_string())
            };
            obj.insert(header.to_string(), parsed_field);
        }
        records.push(Value::Object(obj));
    }

    Ok(Value::Array(records))
}

fn value_to_csv(value: &Value, adapter: &'static str, input: &Path, output: &Path) -> Result<String, JobError> {
    let mut wtr = csv::WriterBuilder::new().has_headers(true).from_writer(vec![]);

    match value {
        Value::Array(items) => {
            if items.is_empty() {
                return Ok(String::new());
            }

            let mut headers_set = BTreeSet::new();
            for item in items {
                if let Value::Object(map) = item {
                    for key in map.keys() {
                        headers_set.insert(key.clone());
                    }
                }
            }

            let headers: Vec<String> = headers_set.into_iter().collect();
            if headers.is_empty() {
                wtr.write_record(["value"]).map_err(|e| JobError::AdapterFailure {
                    adapter,
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    message: format!("CSV write header error: {e}"),
                })?;
                for item in items {
                    wtr.write_record([value_to_cell_string(item)]).map_err(|e| JobError::AdapterFailure {
                        adapter,
                        input: input.to_path_buf(),
                        output: output.to_path_buf(),
                        message: format!("CSV write row error: {e}"),
                    })?;
                }
            } else {
                wtr.write_record(&headers).map_err(|e| JobError::AdapterFailure {
                    adapter,
                    input: input.to_path_buf(),
                    output: output.to_path_buf(),
                    message: format!("CSV write headers error: {e}"),
                })?;

                for item in items {
                    let mut row = Vec::new();
                    if let Value::Object(map) = item {
                        for h in &headers {
                            let cell = map.get(h).map(value_to_cell_string).unwrap_or_default();
                            row.push(cell);
                        }
                    } else {
                        row.push(value_to_cell_string(item));
                    }
                    wtr.write_record(&row).map_err(|e| JobError::AdapterFailure {
                        adapter,
                        input: input.to_path_buf(),
                        output: output.to_path_buf(),
                        message: format!("CSV write record error: {e}"),
                    })?;
                }
            }
        }
        Value::Object(map) => {
            let headers: Vec<&String> = map.keys().collect();
            wtr.write_record(&headers).map_err(|e| JobError::AdapterFailure {
                adapter,
                input: input.to_path_buf(),
                output: output.to_path_buf(),
                message: format!("CSV write headers error: {e}"),
            })?;
            let row: Vec<String> = headers.iter().map(|k| map.get(*k).map(value_to_cell_string).unwrap_or_default()).collect();
            wtr.write_record(&row).map_err(|e| JobError::AdapterFailure {
                adapter,
                input: input.to_path_buf(),
                output: output.to_path_buf(),
                message: format!("CSV write record error: {e}"),
            })?;
        }
        scalar => {
            wtr.write_record(["value"]).map_err(|e| JobError::AdapterFailure {
                adapter,
                input: input.to_path_buf(),
                output: output.to_path_buf(),
                message: format!("CSV write header error: {e}"),
            })?;
            wtr.write_record([value_to_cell_string(scalar)]).map_err(|e| JobError::AdapterFailure {
                adapter,
                input: input.to_path_buf(),
                output: output.to_path_buf(),
                message: format!("CSV write row error: {e}"),
            })?;
        }
    }

    let bytes = wtr.into_inner().map_err(|e| JobError::AdapterFailure {
        adapter,
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        message: format!("CSV finish error: {e}"),
    })?;

    String::from_utf8(bytes).map_err(|e| JobError::AdapterFailure {
        adapter,
        input: input.to_path_buf(),
        output: output.to_path_buf(),
        message: format!("CSV utf-8 error: {e}"),
    })
}

fn value_to_cell_string(val: &Value) -> String {
    match val {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(val).unwrap_or_default(),
    }
}
