$ErrorActionPreference = 'Stop'
function Invoke-Check { param([scriptblock]$Command) & $Command; if ($LASTEXITCODE -ne 0) { throw "Check failed: $Command" } }
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    Invoke-Check { cargo fmt --all -- --check }
    & "$PSScriptRoot/cli-sidecar.ps1"
    Invoke-Check { cargo clippy --workspace --all-targets -- -D warnings }
    Invoke-Check { cargo test --workspace }
    Push-Location apps/desktop
    try {
        Invoke-Check { npm run format:check }
        Invoke-Check { npm run lint }
        Invoke-Check { npm run typecheck }
        Invoke-Check { npm test }
        Invoke-Check { npm run build }
    } finally { Pop-Location }
} finally { Pop-Location }
