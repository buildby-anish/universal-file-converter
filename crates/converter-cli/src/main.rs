use clap::{Parser, Subcommand};
use converter_core::{run_job, Format};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "ufc", version, about = "Universal File Converter — local, offline, extension-agnostic file conversion")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert one or more files to a target format.
    Convert {
        /// Path(s) to the input file(s). Real formats are detected from
        /// content, not from file extensions.
        #[arg(required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,

        /// Target format, e.g. png, jpeg, webp, bmp, gif, tiff, pdf, txt, docx, odt, json, yaml, toml, csv, md, html.
        #[arg(long = "to")]
        to: String,

        /// Directory to write the output into. Defaults to each input
        /// file's own directory.
        #[arg(long)]
        outdir: Option<PathBuf>,
    },
    /// List every (source -> target) route currently registered.
    Routes,
}

fn main() -> ExitCode {
    env_logger::init();
    let cli = Cli::parse();
    let registry = converter_adapters::build_default_registry();

    match cli.command {
        Commands::Routes => {
            let routes = registry.all_routes();
            println!("Universal File Converter — Supported Routes ({} total)\n", routes.len());

            let mut current_adapter = "";
            let mut sorted_routes = routes;
            sorted_routes.sort_by(|a, b| a.2.cmp(b.2).then(a.0.as_str().cmp(b.0.as_str())).then(a.1.as_str().cmp(b.1.as_str())));

            for (from, to, adapter) in sorted_routes {
                if adapter != current_adapter {
                    current_adapter = adapter;
                    let title = match current_adapter {
                        "image-rs" => "Raster Images (image-rs)",
                        "data-adapter" => "Structured Data (data-adapter: JSON, YAML, TOML, CSV)",
                        "markup-adapter" => "Markup & Web (markup-adapter: Markdown, HTML, Text)",
                        "pdf-adapter" => "PDF Engine (pdf-adapter)",
                        "libreoffice-headless" => "Office Documents (libreoffice-headless)",
                        other => other,
                    };
                    println!("\n  [{}]", title);
                }
                println!("    {:<6} -> {:<6}", from.as_str(), to.as_str());
            }
            println!();
            ExitCode::SUCCESS
        }
        Commands::Convert { inputs, to, outdir } => {
            let Some(to_format) = Format::from_str(&to) else {
                eprintln!("error: unrecognized target format '{to}'. Run `ufc routes` to see supported targets.");
                return ExitCode::FAILURE;
            };

            let total = inputs.len();
            let mut successes = 0;
            let mut failures = 0;

            for input in &inputs {
                let target_dir = match &outdir {
                    Some(dir) => dir.clone(),
                    None => input.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")),
                };

                let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output").to_string();

                match run_job(&registry, input, to_format, &target_dir, &stem) {
                    Ok(report) => {
                        successes += 1;
                        println!(
                            "✓ Converted {} ({}) -> {} ({}) via {}",
                            report.input.display(),
                            report.from.as_str(),
                            report.output.display(),
                            report.to.as_str(),
                            report.adapter_name
                        );
                    }
                    Err(e) => {
                        failures += 1;
                        eprintln!("✗ Failed {}: {e}", input.display());
                    }
                }
            }

            if total > 1 {
                println!("\nBatch Summary: {} converted, {} failed ({} total)", successes, failures, total);
            }

            if failures > 0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
    }
}

