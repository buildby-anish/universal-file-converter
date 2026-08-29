#Requires -Version 5.1
<#
.SYNOPSIS
    Install ufc on Windows.
.DESCRIPTION
    By default, tries to download a prebuilt binary from the repo's latest
    GitHub Release (published by .github/workflows/release.yml). Falls back
    to building from source with cargo if no matching release asset is
    found. Pass -FromSource to always build locally.
    Safe to re-run. Installs to %LOCALAPPDATA%\ufc\bin by default and adds
    that directory to the current user's PATH environment variable
    (persisted via the registry, not just the current session).
#>
param(
    [switch]$FromSource
)

$ErrorActionPreference = "Stop"

# EDIT THIS after you push to GitHub, so the prebuilt-binary path can find
# your release assets: "yourname/universal-file-converter".
$UfcRepo = if ($env:UFC_REPO) { $env:UFC_REPO } else { "CHANGEME/universal-file-converter" }

$RepoRoot = Split-Path -Parent $PSScriptRoot
$InstallDir = if ($env:UFC_INSTALL_DIR) { $env:UFC_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "ufc\bin" }
$BinName = "ufc.exe"

function Try-Prebuilt {
    if ($UfcRepo -eq "CHANGEME/universal-file-converter") { return $false }

    $Target = "x86_64-pc-windows-msvc"
    $Asset = "ufc-$Target.zip"
    $Url = "https://github.com/$UfcRepo/releases/latest/download/$Asset"
    Write-Host "==> Trying prebuilt binary: $Url" -ForegroundColor Cyan

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
    Write-Host "==> Installed prebuilt $BinName to $InstallDir (no Rust toolchain needed)" -ForegroundColor Green
    return $true
}

function Build-FromSource {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error "cargo not found on PATH. Install Rust first from https://rustup.rs (rustup-init.exe), then reopen this shell."
    }

    Write-Host "==> Building ufc from source (release) ..." -ForegroundColor Cyan
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
    Write-Host "==> Installed $BinName to $InstallDir" -ForegroundColor Green
}

if (-not $FromSource -and (Try-Prebuilt)) {
    # prebuilt install succeeded
} else {
    if (-not $FromSource) { Write-Host "==> No prebuilt binary available, falling back to source build." -ForegroundColor Yellow }
    Build-FromSource
}

$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    $NewPath = if ([string]::IsNullOrEmpty($UserPath)) { $InstallDir } else { "$UserPath;$InstallDir" }
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    # Also update the current session so `ufc` works immediately without
    # reopening the shell.
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "==> Added $InstallDir to your user PATH (persisted). Open a NEW terminal for it to apply everywhere." -ForegroundColor Yellow
} else {
    Write-Host "==> $InstallDir is already on PATH." -ForegroundColor Cyan
}

Write-Host "==> Done. Verify with: ufc routes" -ForegroundColor Green

$sofficeCandidates = @(
    "$env:ProgramFiles\LibreOffice\program\soffice.exe",
    "${env:ProgramFiles(x86)}\LibreOffice\program\soffice.exe"
)
$hasLibreOffice = (Get-Command soffice -ErrorAction SilentlyContinue) -or ($sofficeCandidates | Where-Object { Test-Path $_ })
if (-not $hasLibreOffice) {
    Write-Host ""
    Write-Host "NOTE: LibreOffice not found — docx/odt/pdf-from-office routes will be" -ForegroundColor Yellow
    Write-Host "unavailable until it's installed (image and PDF-text routes work regardless)." -ForegroundColor Yellow
    Write-Host "  winget install --id TheDocumentFoundation.LibreOffice" -ForegroundColor Yellow
}
