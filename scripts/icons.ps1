$ErrorActionPreference = 'Stop'
Push-Location (Join-Path (Split-Path $PSScriptRoot -Parent) 'apps/desktop')
try {
    npm run tauri icon icon.svg
    if ($LASTEXITCODE -ne 0) { throw 'Icon generation failed' }
} finally { Pop-Location }
