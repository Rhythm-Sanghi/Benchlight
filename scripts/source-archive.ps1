$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $workspace = (Get-Location).Path
    $files = git ls-files --cached --others --exclude-standard
    if ($LASTEXITCODE -ne 0) { throw 'Source inventory failed' }
    $archivePath = Join-Path $workspace 'target/Benchlight-0.1.0-source.zip'
    $stream = [System.IO.File]::Open($archivePath, [System.IO.FileMode]::Create)
    $archive = [System.IO.Compression.ZipArchive]::new($stream, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($relative in $files | Sort-Object -Unique) {
            $path = [System.IO.Path]::GetFullPath((Join-Path $workspace $relative))
            if (-not $path.StartsWith($workspace + '\', [System.StringComparison]::OrdinalIgnoreCase)) { throw "Source path escaped workspace: $relative" }
            $entry = $archive.CreateEntry("Benchlight/$($relative -replace '\\','/')", [System.IO.Compression.CompressionLevel]::Optimal)
            $input = [System.IO.File]::OpenRead($path)
            $output = $entry.Open()
            try { $input.CopyTo($output) } finally { $output.Dispose(); $input.Dispose() }
        }
    } finally { $archive.Dispose(); $stream.Dispose() }
    Get-Item -LiteralPath $archivePath | Select-Object FullName,Length
} finally { Pop-Location }
