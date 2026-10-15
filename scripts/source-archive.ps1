param(
    [string]$SourceRef = 'HEAD',
    [string]$RepositoryPath = (Split-Path $PSScriptRoot -Parent),
    [string]$OutputDirectory
)
$ErrorActionPreference = 'Stop'

$repository = (Resolve-Path -LiteralPath $RepositoryPath).Path
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repository 'target' }
Push-Location $repository
try {
    $commit = (git rev-parse --verify "$SourceRef^{commit}").Trim()
    if ($LASTEXITCODE -ne 0 -or $commit -notmatch '^[0-9a-f]{40}$') { throw "Invalid source ref: $SourceRef" }
    $tree = (git rev-parse --verify "$commit^{tree}").Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Could not resolve source tree' }
    $packageJson = git show "${commit}:apps/desktop/package.json"
    if ($LASTEXITCODE -ne 0) { throw 'Committed package.json is missing' }
    $version = ($packageJson -join "`n" | ConvertFrom-Json).version
    if ($version -notmatch '^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$') { throw 'Invalid committed package version' }
    $tauriJson = git show "${commit}:apps/desktop/src-tauri/tauri.conf.json"
    if ($LASTEXITCODE -ne 0) { throw 'Committed Tauri configuration is missing' }
    if (($tauriJson -join "`n" | ConvertFrom-Json).version -ne $version) { throw 'Committed package and Tauri versions differ' }
    $cargoToml = git show "${commit}:Cargo.toml"
    if ($LASTEXITCODE -ne 0) { throw 'Committed Cargo workspace metadata is missing' }
    $cargoVersion = [regex]::Match(($cargoToml -join "`n"), '(?m)^version\s*=\s*"([^"]+)"').Groups[1].Value
    if ($cargoVersion -ne $version) { throw 'Committed package and Cargo workspace versions differ' }
    foreach ($required in @('LICENSE', 'docs/third-party/README.md')) {
        git cat-file -e "${commit}:$required" 2>$null
        if ($LASTEXITCODE -ne 0) { throw "Committed legal notice is missing: $required" }
    }
    $files = @(git ls-tree -r --name-only $commit)
    if ($LASTEXITCODE -ne 0) { throw 'Source inventory failed' }
    foreach ($file in $files) {
        if ($file -match '(?i)(^|/)(\.env($|\.)|[^/]*\.(db|sqlite|sqlite3|p12|pfx|pem|key)$|[^/]*(credential|secret)[^/]*)') {
            throw "Review sensitive or local-data file before packaging: $file"
        }
    }

    New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
    $archivePath = Join-Path $OutputDirectory "Benchlight-$version-source.zip"
    $manifestPath = Join-Path $OutputDirectory "Benchlight-$version-source-manifest.json"
    if ((Test-Path -LiteralPath $archivePath) -or (Test-Path -LiteralPath $manifestPath)) {
        throw 'Source archive or manifest already exists; move it aside before rebuilding'
    }
    git archive --format=zip --prefix=Benchlight/ --output="$archivePath" $commit
    if ($LASTEXITCODE -ne 0) { throw 'Git source archive failed' }
    $manifest = [ordered]@{
        schemaVersion = 1
        version = $version
        commit = $commit
        tree = $tree
        fileCount = $files.Count
        archiveSha256 = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    $manifest | ConvertTo-Json | Set-Content -LiteralPath $manifestPath -Encoding utf8
    Get-Item -LiteralPath $archivePath, $manifestPath | Select-Object FullName, Length
} finally { Pop-Location }
