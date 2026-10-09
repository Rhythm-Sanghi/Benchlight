$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    Push-Location apps/desktop
    try { npm run build; if ($LASTEXITCODE -ne 0) { throw 'Frontend build failed' } } finally { Pop-Location }
    & "$PSScriptRoot/cli-sidecar.ps1" -Release
    & "$PSScriptRoot/notices.ps1"
} finally { Pop-Location }
