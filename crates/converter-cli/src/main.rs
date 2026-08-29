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
    /// Convert one file to a target format.
    Convert {
        /// Path to the input file. Its real format is detected from
        /// content, not from this path's extension.
        input: PathBuf,

        /// Target format, e.g. png, jpeg, webp, pdf, txt, docx, odt.
        #[arg(long = "to")]
        to: String,

        /// Directory to write the output into. Defaults to the input
        /// file's own directory.
        #[arg(long)]
        outdir: Option<PathBuf>,
    },
    /// List every (source -> target) route currently registered.
    Routes,
}

fn parse_format(s: &str) -> Option<Format> {
    match s.to_ascii_lowercase().as_str() {
        "png" => Some(Format::Png),
        "jpeg" | "jpg" => Some(Format::Jpeg),
        "gif" => Some(Format::Gif),
        "bmp" => Some(Format::Bmp),
        "webp" => Some(Format::WebP),
        "tiff" | "tif" => Some(Format::Tiff),
        "pdf" => Some(Format::Pdf),
        "docx" => Some(Format::Docx),
        "odt" => Some(Format::Odt),
        "txt" | "text" => Some(Format::PlainText),
        _ => None,
    }
}

fn main() -> ExitCode {
    env_logger::init();
    let cli = Cli::parse();
    let registry = converter_adapters::build_default_registry();

    match cli.command {
        Commands::Routes => {
            for (from, to, adapter) in registry.all_routes() {
                println!("{:<8} -> {:<8} [{}]", from.as_str(), to.as_str(), adapter);
            }
            ExitCode::SUCCESS
        }
        Commands::Convert { input, to, outdir } => {
            let Some(to_format) = parse_format(&to) else {
                eprintln!("error: unrecognized target format '{to}'. Run `ufc routes` to see supported targets.");
                return ExitCode::FAILURE;
            };

            let target_dir = outdir.unwrap_or_else(|| {
                input.parent().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
            });

            let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("output").to_string();

            match run_job(&registry, &input, to_format, &target_dir, &stem) {
                Ok(report) => {
                    println!(
                        "converted {} ({}) -> {} ({}) via {}",
                        report.input.display(),
                        report.from.as_str(),
                        report.output.display(),
                        report.to.as_str(),
                        report.adapter_name
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
