//! Document conversion via headless LibreOffice (`soffice --headless`).
//!
//! LibreOffice is the only realistic open-source engine that genuinely
//! understands DOCX/ODT layout well enough to re-flow it into PDF or
//! plain text, so this adapter shells out to it — but strictly through
//! `std::process::Command` with an explicit argument array. No argument is
//! ever interpolated into a shell string, so there is no injection
//! surface regardless of filenames.

use converter_core::{ConversionAdapter, Format, JobError};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct LibreOfficeAdapter {
    /// Resolved path to the `soffice` binary, discovered once at
    /// construction so every `convert()` call fails fast and uniformly if
    /// it's missing, rather than re-probing PATH per job.
    binary: Option<PathBuf>,
}

const ROUTES: &[(Format, Format)] = &[
    (Format::Docx, Format::Pdf),
    (Format::Odt, Format::Pdf),
    (Format::Docx, Format::PlainText),
    (Format::Odt, Format::PlainText),
    (Format::Docx, Format::Odt),
    (Format::Odt, Format::Docx),
];

impl LibreOfficeAdapter {
    pub fn new() -> Self {
        // Try common binary names; `which` walks PATH exactly like a shell
        // would, but without invoking a shell.
        let binary = ["soffice", "libreoffice"]
            .iter()
            .find_map(|name| which::which(name).ok());
        Self { binary }
    }
}

impl Default for LibreOfficeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

fn target_filter(to: Format) -> Option<&'static str> {
    match to {
        Format::Pdf => Some("pdf"),
        Format::PlainText => Some("txt:Text"),
        Format::Docx => Some("docx:MS Word 2007 XML"),
        Format::Odt => Some("odt:writer8"),
        _ => None,
    }
}

impl ConversionAdapter for LibreOfficeAdapter {
    fn name(&self) -> &'static str {
        "libreoffice-headless"
    }

    fn supported_routes(&self) -> &[(Format, Format)] {
        ROUTES
    }

    fn convert(&self, input: &Path, output: &Path, _from: Format, to: Format) -> Result<(), JobError> {
        let soffice = self.binary.as_ref().ok_or(JobError::MissingExternalTool { tool: "soffice" })?;

        let filter = target_filter(to).ok_or_else(|| JobError::AdapterFailure {
            adapter: self.name(),
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            message: format!("no LibreOffice export filter mapped for target '{}'", to.as_str()),
        })?;

        // LibreOffice's --convert-to only lets us choose the output
        // *directory* and *extension*, not an exact filename, so we
        // convert into a scratch subdirectory next to the requested
        // output path and then move the single resulting file onto the
        // exact tempfile path the job pipeline handed us.
        let scratch_dir = output.with_extension("ufc-soffice-scratch");
        std::fs::create_dir_all(&scratch_dir).map_err(|e| JobError::Io {
            path: scratch_dir.clone(),
            source: e,
        })?;

        // Explicit argument array — never a shell string — per the safe
        // process execution contract.
        let status = Command::new(soffice)
            .arg("--headless")
            .arg("--norestore")
            .arg("--convert-to")
            .arg(filter)
            .arg("--outdir")
            .arg(&scratch_dir)
            .arg(input)
            .output()
            .map_err(|e| JobError::Io {
                path: soffice.clone(),
                source: e,
            })?;

        if !status.status.success() {
            let _ = std::fs::remove_dir_all(&scratch_dir);
            return Err(JobError::ExternalToolFailed {
                tool: "soffice",
                code: status.status.code(),
                stderr: String::from_utf8_lossy(&status.stderr).into_owned(),
            });
        }

        // LibreOffice names its output `<input-stem>.<ext>`. Find that
        // exact file in the scratch dir (there should be exactly one) and
        // move it onto the tempfile path the job pipeline expects.
        let produced = std::fs::read_dir(&scratch_dir)
            .map_err(|e| JobError::Io { path: scratch_dir.clone(), source: e })?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .find(|p| p.is_file());

        let produced = produced.ok_or_else(|| JobError::AdapterFailure {
            adapter: self.name(),
            input: input.to_path_buf(),
            output: output.to_path_buf(),
            message: "soffice reported success but wrote no output file".to_string(),
        })?;

        std::fs::rename(&produced, output).or_else(|_| std::fs::copy(&produced, output).map(|_| ())).map_err(|e| {
            JobError::Io { path: output.to_path_buf(), source: e }
        })?;

        let _ = std::fs::remove_dir_all(&scratch_dir);
        Ok(())
    }
}
