//! Safe conversion job pipeline.
//!
//! This is the only place in the codebase allowed to touch the
//! user-facing output path. The sequence is fixed and non-negotiable:
//!
//! 1. Sniff the input's real format (never trust its extension).
//! 2. Resolve a route to a concrete adapter.
//! 3. Convert into an **isolated tempfile**, never the final path.
//! 4. Re-sniff and structurally validate the tempfile's bytes.
//! 5. Only on successful validation, atomically rename the tempfile onto
//!    a collision-safe final path. The input file is never opened for
//!    writing at any point in this sequence.

use crate::detection::{detect, Format};
use crate::errors::JobError;
use crate::registry::Registry;
use crate::router::resolve_route;
use crate::validation::validate_output;
use std::path::{Path, PathBuf};
use tempfile::Builder;

pub struct JobReport {
    pub input: PathBuf,
    pub output: PathBuf,
    pub from: Format,
    pub to: Format,
    pub adapter_name: &'static str,
}

/// Run one conversion job end-to-end.
///
/// `target_dir` is the directory the final file should land in (typically
/// the input's parent directory, but callers may redirect output
/// elsewhere). `desired_stem` is the filename stem (without extension) to
/// use before collision-suffix resolution.
pub fn run_job(
    registry: &Registry,
    input: &Path,
    to: Format,
    target_dir: &Path,
    desired_stem: &str,
) -> Result<JobReport, JobError> {
    let from = detect(input)?;
    let adapter = resolve_route(registry, from, to)?;

    // Step 3: convert into an isolated tempfile living in `target_dir` so
    // the final rename in step 5 stays on the same filesystem (required
    // for a true atomic rename rather than a copy+delete).
    std::fs::create_dir_all(target_dir).map_err(|e| JobError::Io {
        path: target_dir.to_path_buf(),
        source: e,
    })?;

    let tmp = Builder::new()
        .prefix(".ufc-tmp-")
        .suffix(&format!(".{}", to.as_str()))
        .tempfile_in(target_dir)
        .map_err(JobError::Tempfile)?;
    // Persist the tempfile to disk under its randomized name and drop the
    // open handle: the adapter needs to open the path fresh (an external
    // tool takes a bare path argument, and the `image` crate opens its own
    // `File`). `keep()` disables the auto-delete-on-drop guard so the path
    // survives past this scope for the adapter to write into.
    let (_file, tmp_path) = tmp.keep().map_err(|e| JobError::Tempfile(e.error))?;

    let convert_result = adapter.convert(input, &tmp_path, from, to);
    if let Err(e) = convert_result {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }

    // Step 4: validate before this bytes-on-disk artifact is allowed to
    // become a user-visible file.
    if let Err(e) = validate_output(&tmp_path, to) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(JobError::Validation(e));
    }

    // Step 5: resolve a collision-safe final path, then atomically rename.
    let final_path = resolve_collision_path(target_dir, desired_stem, to.as_str())?;
    std::fs::rename(&tmp_path, &final_path).map_err(|e| JobError::Io {
        path: final_path.clone(),
        source: e,
    })?;

    Ok(JobReport {
        input: input.to_path_buf(),
        output: final_path,
        from,
        to,
        adapter_name: adapter.name(),
    })
}

/// Default collision policy: `stem.ext`, then `stem_1.ext`, `stem_2.ext`,
/// ... up to a bounded number of attempts, so a pathological directory
/// full of stale numbered files can't spin forever.
fn resolve_collision_path(dir: &Path, stem: &str, ext: &str) -> Result<PathBuf, JobError> {
    const MAX_ATTEMPTS: u32 = 1000;

    let candidate = dir.join(format!("{stem}.{ext}"));
    if !candidate.exists() {
        return Ok(candidate);
    }

    for n in 1..MAX_ATTEMPTS {
        let candidate = dir.join(format!("{stem}_{n}.{ext}"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(JobError::CollisionExhausted {
        path: dir.join(format!("{stem}.{ext}")),
        attempts: MAX_ATTEMPTS,
    })
}
