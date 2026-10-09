param([Parameter(Mandatory)][string]$Tag)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $package = Get-Content apps/desktop/package.json -Raw | ConvertFrom-Json
    if ($Tag -ne "v$($package.version)") { throw 'Release tag must match the package version' }
    $notes = "docs/releases/$($package.version).md"
    if (-not (Test-Path -LiteralPath $notes)) { throw 'Release notes are missing' }
    & "$PSScriptRoot/source-archive.ps1"
    $files = @(
        "target/release/bundle/nsis/Benchlight_$($package.version)_x64-setup.exe",
        'target/release/benchlight-desktop.exe',
        'target/release/benchlight.exe',
        "target/Benchlight-$($package.version)-source.zip"
    )
    $checksums = "target/Benchlight-$($package.version)-SHA256SUMS.txt"
    $lines = foreach ($file in $files) {
        $hash = Get-FileHash -LiteralPath $file -Algorithm SHA256
        "$($hash.Hash.ToLower())  $([System.IO.Path]::GetFileName($file))"
    }
    $lines | Set-Content -LiteralPath $checksums -Encoding ascii
    $files += $checksums
    # Upload to a draft first, so an interrupted upload is never a public release.
    gh release create $Tag @files --repo Rhythm-Sanghi/Benchlight --verify-tag --draft --prerelease --title "Benchlight $($package.version) (early release)" --notes-file $notes
    if ($LASTEXITCODE -ne 0) { throw 'Release asset upload failed; inspect the draft before retrying' }
    gh release edit $Tag --repo Rhythm-Sanghi/Benchlight --draft=false
    if ($LASTEXITCODE -ne 0) { throw 'Release is still a draft; publication failed' }
} finally { Pop-Location }
