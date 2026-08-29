#Requires -Version 5.1
<#
.SYNOPSIS
    Build ufc in release mode and install it onto the user's PATH on Windows.
.DESCRIPTION
    Safe to re-run. Installs to %LOCALAPPDATA%\ufc\bin by default and adds
    that directory to the current user's PATH environment variable
    (persisted via the registry, not just the current session).
#>

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$InstallDir = if ($env:UFC_INSTALL_DIR) { $env:UFC_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "ufc\bin" }
$BinName = "ufc.exe"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "cargo not found on PATH. Install Rust first from https://rustup.rs (rustup-init.exe), then reopen this shell."
}

Write-Host "==> Building ufc (release) ..." -ForegroundColor Cyan
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
