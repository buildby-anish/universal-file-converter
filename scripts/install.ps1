#Requires -Version 5.1
<#
.SYNOPSIS
    One-shot installer for ufc on Windows.
.DESCRIPTION
    Does everything in a single run:
      1. Bootstraps Rust via rustup-init.exe if `cargo` isn't already on PATH.
      2. Installs LibreOffice via winget if `soffice` isn't already on PATH
         (needed for docx/odt/pdf-from-office routes).
      3. Installs ufc itself: tries a prebuilt binary from the repo's
         latest GitHub Release first, falls back to `cargo build --release`.

    You do not need to install anything yourself beforehand — just run
    this script. Windows may show its own UAC/admin prompt for the winget
    or rustup installers; that's the OS's own prompt, not this script
    collecting credentials.

    Safe to re-run. Pass -FromSource to always build locally, or
    -SkipLibreOffice to skip that step.
#>
param(
    [switch]$FromSource,
    [switch]$SkipLibreOffice
)

$ErrorActionPreference = "Stop"

# EDIT THIS after you push to GitHub, so the prebuilt-binary path (and the
# standalone/downloaded source-build fallback) can find your repo:
# "yourname/universal-file-converter".
$UfcRepo = if ($env:UFC_REPO) { $env:UFC_REPO } else { "buildby-anish/universal-file-converter" }

# Resolve RepoRoot only if this script is actually running from inside a
# cloned checkout (i.e. ..\Cargo.toml exists next to it). When the script
# was instead downloaded standalone (iwr ... -OutFile; & ...), there is no
# on-disk checkout yet, so RepoRoot stays $null and Build-FromSource
# clones one on demand via Ensure-RepoCheckout.
$RepoRoot = $null
if ($PSScriptRoot) {
    $Candidate = Split-Path -Parent $PSScriptRoot
    if (Test-Path (Join-Path $Candidate "Cargo.toml")) { $RepoRoot = $Candidate }
}

$InstallDir = if ($env:UFC_INSTALL_DIR) { $env:UFC_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "ufc\bin" }
$BinName = "ufc.exe"

function Log($msg) { Write-Host "==> $msg" -ForegroundColor Cyan }

# ---------------------------------------------------------------------------
# Step 1: Rust toolchain
# ---------------------------------------------------------------------------
function Ensure-Rust {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Log "Rust toolchain already installed ($(cargo --version))."
        return
    }
    Log "Rust not found — downloading and running rustup-init.exe (non-interactive) ..."
    $RustupInit = Join-Path ([System.IO.Path]::GetTempPath()) "rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $RustupInit
    & $RustupInit -y --default-toolchain stable
    Remove-Item $RustupInit -ErrorAction SilentlyContinue

    # rustup installs to %USERPROFILE%\.cargo\bin; add it to this
    # session's PATH so the rest of this run can use `cargo` immediately
    # without requiring a new shell.
    $CargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if (Test-Path $CargoBin) { $env:Path = "$env:Path;$CargoBin" }

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "rustup install finished but cargo still isn't on PATH. Open a new PowerShell window and re-run this script."
    }
    Log "Rust installed ($(cargo --version))."
}

# ---------------------------------------------------------------------------
# Step 2: LibreOffice (optional dependency, only for docx/odt/pdf-from-office)
# ---------------------------------------------------------------------------
function Ensure-LibreOffice {
    if ($SkipLibreOffice) {
        Log "Skipping LibreOffice install (-SkipLibreOffice passed)."
        return
    }

    $sofficeCandidates = @(
        "$env:ProgramFiles\LibreOffice\program\soffice.exe",
        "${env:ProgramFiles(x86)}\LibreOffice\program\soffice.exe"
    )
    $hasLibreOffice = (Get-Command soffice -ErrorAction SilentlyContinue) -or ($sofficeCandidates | Where-Object { Test-Path $_ })
    if ($hasLibreOffice) {
        Log "LibreOffice already installed."
        return
    }

    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        Log "winget not found — skipping LibreOffice (available by default on Windows 10 2004+/11)."
        Log "Image and PDF-text conversion still work without it; install LibreOffice manually later if needed."
        return
    }

    Log "LibreOffice not found — installing via winget ..."
    winget install --id TheDocumentFoundation.LibreOffice --silent --accept-package-agreements --accept-source-agreements
    Log "LibreOffice installed."
}

# ---------------------------------------------------------------------------
# Step 3: ufc itself
# ---------------------------------------------------------------------------
function Try-Prebuilt {
    if ([string]::IsNullOrWhiteSpace($UfcRepo)) { return $false }

    $Target = "x86_64-pc-windows-msvc"
    $Asset = "ufc-$Target.zip"
    $Url = "https://github.com/$UfcRepo/releases/latest/download/$Asset"
    Log "Trying prebuilt binary: $Url"

    $Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory -Force -Path $Tmp | Out-Null
    $ZipPath = Join-Path $Tmp $Asset

    try {
        Invoke-WebRequest -Uri $Url -OutFile $ZipPath -ErrorAction Stop
    } catch {
        Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
        return $false
    }

    Expand-Archive -Path $ZipPath -DestinationPath $Tmp -Force
    $Extracted = Get-ChildItem -Path $Tmp -Filter $BinName -Recurse | Select-Object -First 1
    if (-not $Extracted) {
        Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
        return $false
    }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Path $Extracted.FullName -Destination (Join-Path $InstallDir $BinName) -Force
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
    Log "Installed prebuilt $BinName to $InstallDir (no local compile needed)"
    return $true
}

function Ensure-Git {
    if (Get-Command git -ErrorAction SilentlyContinue) { return }
    Log "git not found — installing it (needed to fetch source for the local build) ..."
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        Write-Error "git is required and winget isn't available to install it automatically. Install git manually (https://git-scm.com) and re-run."
    }
    winget install --id Git.Git --silent --accept-package-agreements --accept-source-agreements
    # winget installs to a machine PATH location but this session won't
    # see it without a refresh; add the common install dir directly.
    $GitBin = "$env:ProgramFiles\Git\cmd"
    if (Test-Path $GitBin) { $env:Path = "$env:Path;$GitBin" }
    if (-not (Get-Command git -ErrorAction SilentlyContinue)) {
        Write-Error "git installed but isn't on PATH in this session. Open a new PowerShell window and re-run this script."
    }
}

# When running standalone (downloaded to a temp file, no local checkout),
# clone one into a reusable cache directory so Build-FromSource has
# something to build.
function Ensure-RepoCheckout {
    if ($RepoRoot) { return }
    if ([string]::IsNullOrWhiteSpace($UfcRepo)) {
        Write-Error "No local checkout found and UFC_REPO is unset. Re-run with: `$env:UFC_REPO='buildby-anish/universal-file-converter'; <script invocation>"
    }
    Ensure-Git
    $CheckoutDir = Join-Path $env:LOCALAPPDATA "ufc\src"
    if (Test-Path (Join-Path $CheckoutDir ".git")) {
        Log "Updating cached source checkout at $CheckoutDir ..."
        git -C $CheckoutDir fetch --depth 1 origin main
        git -C $CheckoutDir reset --hard origin/main
    } else {
        Log "Cloning https://github.com/$UfcRepo for a source build ..."
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $CheckoutDir) | Out-Null
        git clone --depth 1 "https://github.com/$UfcRepo.git" $CheckoutDir
    }
    $script:RepoRoot = $CheckoutDir
}

function Build-FromSource {
    Ensure-RepoCheckout
    Log "Building ufc from source (release) ..."
    Push-Location $RepoRoot
    try {
        cargo build --release --workspace
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }

    $BuiltBin = Join-Path $RepoRoot "target\release\$BinName"
    if (-not (Test-Path $BuiltBin)) {
        Write-Error "Expected binary not found at $BuiltBin"
    }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Path $BuiltBin -Destination (Join-Path $InstallDir $BinName) -Force
    Log "Installed $BinName to $InstallDir"
}

function Install-Ufc {
    if (-not $FromSource -and (Try-Prebuilt)) { return }
    if (-not $FromSource) { Log "No prebuilt binary available, falling back to source build." }
    Ensure-Rust
    Build-FromSource
}

# ---------------------------------------------------------------------------
# Run everything
# ---------------------------------------------------------------------------
Install-Ufc
Ensure-LibreOffice

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    $NewPath = if ([string]::IsNullOrEmpty($UserPath)) { $InstallDir } else { "$UserPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    $env:Path = "$env:Path;$InstallDir"
    Log "Added $InstallDir to your user PATH (persisted) and to this session."
} else {
    Log "$InstallDir is already on PATH."
}

Log "Done. Verifying:"
& (Join-Path $InstallDir $BinName) routes
Write-Host ""
Log "ufc is installed and ready to use in this session. New windows will pick it up automatically too."
