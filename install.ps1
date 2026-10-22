# txt2tex installer for Windows.
#
#   irm https://raw.githubusercontent.com/whynotdeep7/txt2tex/main/install.ps1 | iex
#
# Downloads the prebuilt binary from the latest GitHub release and puts it on
# your PATH. That is all it does: no Rust toolchain, no editor, no other
# changes to your machine.

$ErrorActionPreference = 'Stop'

$repo = 'whynotdeep7/txt2tex'
$installDir = if ($env:TXT2TEX_INSTALL_DIR) {
    $env:TXT2TEX_INSTALL_DIR
} else {
    "$env:LOCALAPPDATA\Programs\txt2tex"
}

function Fail($message) {
    Write-Host ""
    Write-Host "  x $message" -ForegroundColor Red
    Write-Host ""
    exit 1
}

if (-not [Environment]::Is64BitOperatingSystem) {
    Fail "txt2tex needs 64-bit Windows."
}

Write-Host ""
Write-Host "  txt2tex" -ForegroundColor Cyan -NoNewline
Write-Host "  Windows x86_64"
Write-Host ""

# 'latest/download' always redirects to the newest release, so this URL never
# needs updating when a version ships.
$url = "https://github.com/$repo/releases/latest/download/txt2tex-windows-x86_64.exe"
$temp = Join-Path $env:TEMP "txt2tex-$([guid]::NewGuid()).exe"

Write-Host "  downloading txt2tex-windows-x86_64.exe"
try {
    Invoke-WebRequest -Uri $url -OutFile $temp -UseBasicParsing
} catch {
    Fail "download failed - no binary at`n     $url`n`n     Check that a release exists:  https://github.com/$repo/releases"
}

New-Item -ItemType Directory -Path $installDir -Force | Out-Null
# Move into place last, so a failed download never leaves a broken exe on PATH.
Move-Item -Path $temp -Destination "$installDir\txt2tex.exe" -Force

Write-Host "  + installed to $installDir\txt2tex.exe" -ForegroundColor Green

# Persist PATH for the user, without duplicating an existing entry.
$userPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable('PATH', "$userPath;$installDir", 'User')
    Write-Host "  ! added to PATH - restart your terminal" -ForegroundColor Yellow
}

# tectonic is a real requirement, but installing it is not this script's
# business.
if (-not (Get-Command tectonic -ErrorAction SilentlyContinue)) {
    Write-Host ""
    Write-Host "  ! tectonic is not installed. txt2tex needs it to typeset PDFs:" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "      choco install tectonic"
    Write-Host ""
}

Write-Host "  Run txt2tex to get started."
Write-Host ""
