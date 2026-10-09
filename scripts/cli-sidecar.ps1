param([switch]$Release)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    if ($Release) { cargo build -p benchlight-cli --release --locked } else { cargo build -p benchlight-cli --locked }
    if ($LASTEXITCODE -ne 0) { throw 'CLI build failed' }
    $profile = if ($Release) { 'release' } else { 'debug' }
    $destination = Join-Path (Get-Location) 'apps/desktop/src-tauri/bin'
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path (Get-Location) "target/$profile/benchlight.exe") -Destination (Join-Path $destination 'benchlight-x86_64-pc-windows-msvc.exe') -Force
} finally { Pop-Location }
