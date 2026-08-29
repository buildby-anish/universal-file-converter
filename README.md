# Universal File Converter (`ufc`)

Local, offline, extension-agnostic file conversion utility.

## What's implemented in this pass

This pass delivers a **complete, functional conversion engine**: detection,
routing, safe execution, and validation, plus three real adapter
backends. Nothing here is a stub — every path either performs a genuine
conversion or returns a typed error.

| Crate | Status | Notes |
|---|---|---|
| `converter-core` | ✅ Complete | Magic-byte detection, adapter registry, route resolution, tempfile→validate→atomic-rename job pipeline, collision-suffix policy, `thiserror` error hierarchy. |
| `converter-adapters` | ✅ Complete | `ImageAdapter` (image-rs, real pixel decode/re-encode across png/jpeg/webp/bmp/gif/tiff), `LibreOfficeAdapter` (headless `soffice --convert-to`, explicit arg arrays, no shell interpolation, requires `soffice`/`libreoffice` on `PATH`), `PdfAdapter` (PDF↔text: `pdf-extract` for extraction, `printpdf` for generation, both directions structurally re-validated with `lopdf`). |
| `converter-cli` (`ufc`) | ✅ Complete | `ufc convert <input> --to <format> [--outdir <dir>]`, `ufc routes`. |
| `converter-ui`, `converter-platform`, `scripts/`, `.github/workflows/` | **Not included in this pass** | Deliberately deferred rather than stubbed — a GUI shell, per-OS shell-integration hooks, and CI matrices are separate, substantial pieces of engineering that deserve their own real implementations, not placeholder crates that would violate the zero-mock contract. Happy to build any of these next; tell me which to prioritize. |

## Build

Requires network access to fetch crates from crates.io (this sandbox has
network disabled, so the code here has **not** been compiled or tested in
this session — please run `cargo build` and `cargo test` locally before
relying on it):

```bash
cargo build --workspace
cargo test --workspace
```

`LibreOfficeAdapter` additionally requires `soffice` (LibreOffice) on
`PATH` at runtime for docx/odt/pdf-from-office routes — image and
PDF-text routes have no external runtime dependency.

## Usage

```bash
ufc routes                                  # list every supported (from -> to) pair
ufc convert photo.jpg --to png               # writes photo.png next to photo.jpg
ufc convert report.docx --to pdf --outdir out/
ufc convert scan.pdf --to txt
```

## Design notes worth knowing before extending this

- **Format is never taken from the file extension.** `detection.rs` sniffs
  magic bytes (and, for ZIP-based containers, probes for the
  distinguishing internal manifest entry) on every input and on every
  adapter's output before that output is allowed to become a final file.
- **The job pipeline (`job.rs`) is the only code path allowed to write to
  a user-visible output path.** Adapters only ever see a fresh tempfile
  path created in the same directory as the eventual output (so the final
  `rename` is a same-filesystem atomic rename, not a copy).
- **Collision policy** is `stem.ext` → `stem_1.ext` → `stem_2.ext` ...,
  bounded at 1000 attempts, and never touches or overwrites an existing
  file.
- **Adding a new adapter:** implement `ConversionAdapter` in
  `converter-adapters`, list its genuine `(Format, Format)` routes, and
  register it in `build_default_registry()`. If it needs a new `Format`
  variant, add the variant and a real signature check in
  `detection::sniff_bytes`.
