$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $metadata = cargo metadata --locked --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    $output = Join-Path (Get-Location) 'docs/third-party'
    New-Item -ItemType Directory -Path $output -Force | Out-Null
    $rows = [System.Collections.Generic.List[string]]::new()
    $rows.Add('# Third-party notices')
    $rows.Add('')
    $rows.Add('Generated from Cargo.lock and production package-lock entries. This inventory includes build dependencies and packages for other targets; it is broader than the Windows runtime. Original available license and notice files are preserved in the adjacent folders. Benchlight itself is GPL-3.0-or-later.')
    $rows.Add('')
    $rows.Add('| Package | License | Upstream source |')
    $rows.Add('| --- | --- | --- |')
    foreach ($package in $metadata.packages | Where-Object source | Sort-Object name,version) {
        $rows.Add("| $($package.name) $($package.version) | $($package.license) | $($package.repository) |")
        $root = Split-Path $package.manifest_path -Parent
        $destination = Join-Path $output "$($package.name)-$($package.version)"
        $files = @(Get-ChildItem -LiteralPath $root -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)' })
        if ($package.license_file) { $files += Get-Item -LiteralPath (Join-Path $root $package.license_file) }
        if ($files.Count) {
            New-Item -ItemType Directory -Path $destination -Force | Out-Null
            foreach ($file in $files | Sort-Object FullName -Unique) { Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $destination $file.Name) -Force }
        }
    }
    Push-Location apps/desktop
    try { $javascriptPackages = npm ls --omit=dev --parseable --all --silent; if ($LASTEXITCODE -ne 0) { throw 'Production npm inventory failed' } } finally { Pop-Location }
    foreach ($root in $javascriptPackages | Select-Object -Skip 1) {
        $package = Get-Content -LiteralPath (Join-Path $root 'package.json') -Raw | ConvertFrom-Json
        $rows.Add("| $($package.name) $($package.version) (JavaScript) | $($package.license) | npm package source |")
        $destination = Join-Path $output (($package.name -replace '[@/]', '-') + '-' + $package.version)
        New-Item -ItemType Directory -Path $destination -Force | Out-Null
        foreach ($file in Get-ChildItem -LiteralPath $root -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE)' }) { Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $destination $file.Name) -Force }
    }
    $rows | Set-Content -LiteralPath (Join-Path $output 'README.md') -Encoding utf8
    "Wrote notices for $($metadata.packages.Count) Cargo packages and production JavaScript packages."
} finally { Pop-Location }
