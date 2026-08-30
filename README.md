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
| `scripts/install.sh`, `scripts/install.ps1` | ✅ Complete | Cross-platform installers (macOS/Linux/Windows). Try a prebuilt binary from the latest GitHub Release first, fall back to `cargo build --release` from source. |
| `.github/workflows/ci.yml`, `release.yml` | ✅ Complete | CI: fmt/clippy/build/test on Linux+macOS+Windows on every push/PR. Release: on pushing a `vX.Y.Z` tag, cross-compiles `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`, and publishes them as GitHub Release assets. |
| `converter-ui`, `converter-platform` | **Not included in this pass** | Deliberately deferred rather than stubbed — a GUI shell and per-OS shell-integration hooks (registry entries, QuickActions, `.desktop` files) are separate, substantial pieces of engineering that deserve their own real implementations, not placeholder crates that would violate the zero-mock contract. Happy to build either next; tell me which to prioritize. |

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

## Install — a single command, nothing to install first

**Once you've pushed this repo to GitHub** and edited `UFC_REPO` /
`$UfcRepo` at the top of `scripts/install.sh` and `scripts/install.ps1` to
your real `owner/repo` (then committed and pushed that edit), anyone can
install `ufc` with one command — no `git clone` needed first:

```bash
# macOS / Linux
bash -c "$(curl -fsSL https://raw.githubusercontent.com/OWNER/REPO/main/scripts/install.sh)"
```

```powershell
# Windows PowerShell
irm https://raw.githubusercontent.com/OWNER/REPO/main/scripts/install.ps1 -OutFile "$env:TEMP\ufc-install.ps1"; & "$env:TEMP\ufc-install.ps1"
```

(replace `OWNER/REPO` with your actual GitHub path, e.g.
`anish/universal-file-converter`)

Each command does everything in one run:

1. Tries to download a prebuilt `ufc` binary from your latest GitHub
   Release (published by `.github/workflows/release.yml` when you push a
   `vX.Y.Z` tag) — nothing else needed at all if this succeeds.
2. If no prebuilt binary is available yet, bootstraps `git` and `Rust`
   automatically, clones the repo into a small local cache, and builds
   from source.
3. Installs LibreOffice automatically (Homebrew on macOS, apt/dnf/pacman/
   zypper on Linux, winget on Windows) if it isn't already present, so
   docx/odt/pdf-from-office routes work out of the box too.
4. Adds the install directory to your PATH, so `ufc` becomes a normal
   terminal command from anywhere.

You may see your OS's own admin/sudo password prompt if a step needs to
install something (git, LibreOffice) through a system package manager —
that's expected, not something the script is asking for itself.

If you'd rather run it from a clone you already have on disk, that still
works exactly the same way:

```bash
# macOS / Linux
chmod +x scripts/install.sh && ./scripts/install.sh
```

```powershell
# Windows PowerShell
.\scripts\install.ps1
```

Flags: `--from-source` / `-FromSource` always builds locally even if a
prebuilt binary exists; `--skip-libreoffice` / `-SkipLibreOffice` skips
that step entirely (image and PDF-text routes work fine without
LibreOffice).

## Releasing a new version

```bash
git tag v0.1.0
git push origin v0.1.0
```

This triggers `.github/workflows/release.yml`, which cross-compiles for
Linux, both macOS architectures, and Windows, and attaches the archives
to a new GitHub Release automatically.

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
