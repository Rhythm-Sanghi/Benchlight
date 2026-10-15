$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
$fixture = Join-Path ([System.IO.Path]::GetTempPath()) ("benchlight-source-archive-test-" + [guid]::NewGuid().ToString('N'))
$repository = Join-Path $fixture 'repo'
$outputOne = Join-Path $fixture 'one'
$outputTwo = Join-Path $fixture 'two'
function Invoke-Git {
    param([string[]]$Arguments)
    & git @Arguments | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Fixture Git command failed: $($Arguments -join ' ')" }
}
try {
    New-Item -ItemType Directory -Path (Join-Path $repository 'apps/desktop/src-tauri') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $repository 'docs/third-party') -Force | Out-Null
    Push-Location $repository
    try {
        Invoke-Git -Arguments @('init', '-q')
        Invoke-Git -Arguments @('config', 'user.name', 'Benchlight archive test')
        Invoke-Git -Arguments @('config', 'user.email', 'archive-test@example.invalid')
        '{"version":"0.2.3"}' | Set-Content apps/desktop/package.json
        '{"version":"0.2.3"}' | Set-Content apps/desktop/src-tauri/tauri.conf.json
        'committed content' | Set-Content README.md
        'license fixture' | Set-Content LICENSE
        'dependency notices fixture' | Set-Content docs/third-party/README.md
        Invoke-Git -Arguments @('add', '.')
        Invoke-Git -Arguments @('commit', '-qm', 'Archive fixture')
        $commit = (git rev-parse HEAD).Trim()
        'changed working copy' | Set-Content README.md
        'do not package' | Set-Content untracked-secret.txt
        & "$PSScriptRoot/source-archive.ps1" -RepositoryPath $repository -SourceRef $commit -OutputDirectory $outputOne | Out-Null
        & "$PSScriptRoot/source-archive.ps1" -RepositoryPath $repository -SourceRef $commit -OutputDirectory $outputTwo | Out-Null
        $first = Join-Path $outputOne 'Benchlight-0.2.3-source.zip'
        $second = Join-Path $outputTwo 'Benchlight-0.2.3-source.zip'
        $manifest = Get-Content (Join-Path $outputOne 'Benchlight-0.2.3-source-manifest.json') -Raw | ConvertFrom-Json
        if ($manifest.commit -ne $commit -or $manifest.fileCount -ne 5) { throw 'Archive manifest does not identify the fixture commit' }
        if ($manifest.archiveSha256 -ne (Get-FileHash $first -Algorithm SHA256).Hash.ToLowerInvariant()) { throw 'Archive hash mismatch' }
        if ((Get-FileHash $first -Algorithm SHA256).Hash -ne (Get-FileHash $second -Algorithm SHA256).Hash) { throw 'Archive is not repeatable' }
        $refusedOverwrite = $false
        try { & "$PSScriptRoot/source-archive.ps1" -RepositoryPath $repository -SourceRef $commit -OutputDirectory $outputOne | Out-Null } catch { $refusedOverwrite = $true }
        if (-not $refusedOverwrite) { throw 'Archive unexpectedly overwrote existing output' }
        $zip = [System.IO.Compression.ZipFile]::OpenRead($first)
        try {
            $names = @($zip.Entries | ForEach-Object FullName)
            if ($names -contains 'Benchlight/untracked-secret.txt' -or $names -notcontains 'Benchlight/LICENSE' -or $names -notcontains 'Benchlight/docs/third-party/README.md') { throw 'Archive inventory is unsafe or incomplete' }
            $reader = [System.IO.StreamReader]::new(($zip.GetEntry('Benchlight/README.md')).Open())
            try {
                if ($reader.ReadToEnd().Trim() -ne 'committed content') { throw 'Archive contains uncommitted working-tree content' }
            } finally { $reader.Dispose() }
        } finally { $zip.Dispose() }
        Invoke-Git -Arguments @('add', 'untracked-secret.txt')
        Invoke-Git -Arguments @('commit', '-qm', 'Fixture with sensitive path')
        $refusedSensitive = $false
        try { & "$PSScriptRoot/source-archive.ps1" -RepositoryPath $repository -SourceRef 'HEAD' -OutputDirectory (Join-Path $fixture 'sensitive') | Out-Null } catch { $refusedSensitive = $true }
        if (-not $refusedSensitive) { throw 'Archive unexpectedly accepted a committed sensitive path' }
        Write-Output 'Source archive fixture passed: exact commit, version, legal file, untracked exclusion, dirty-tree exclusion, repeatable hash.'
    } finally { Pop-Location }
} finally {
    $resolvedTemp = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\')
    $resolvedFixture = [System.IO.Path]::GetFullPath($fixture)
    if ($resolvedFixture.StartsWith($resolvedTemp + '\benchlight-source-archive-test-', [System.StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $resolvedFixture)) {
        Remove-Item -LiteralPath $resolvedFixture -Recurse -Force
    }
}
