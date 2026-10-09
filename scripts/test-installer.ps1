param([Parameter(Mandatory = $true)][string]$Installer)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $workspace = (Get-Location).Path
    $installation = [System.IO.Path]::GetFullPath((Join-Path $workspace 'target/installer-test'))
    $data = [System.IO.Path]::GetFullPath((Join-Path $workspace 'target/installer-test-data'))
    foreach ($path in @($installation,$data)) {
        if (-not $path.StartsWith($workspace + '\', [System.StringComparison]::OrdinalIgnoreCase)) { throw 'Installer test path escaped workspace' }
    }
    $existing = Get-ChildItem -LiteralPath HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall | Get-ItemProperty | Where-Object DisplayName -eq Benchlight
    if ($existing) { throw 'Benchlight is already installed. Refusing to replace a user installation.' }
    if (Test-Path -LiteralPath $installation) { throw 'The isolated installer test directory already exists; inspect it before retrying.' }
    $installerPath = (Resolve-Path -LiteralPath $Installer).Path
    $installed = $false
    try {
        $process = Start-Process -FilePath $installerPath -ArgumentList @('/S',"/D=$installation") -PassThru -WindowStyle Hidden
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "Install failed: $($process.ExitCode)" }
        $installed = $true
        $cli = Join-Path $installation 'benchlight.exe'
        $desktop = Join-Path $installation 'benchlight-desktop.exe'
        if (-not (Test-Path -LiteralPath $desktop) -or -not (Test-Path -LiteralPath $cli)) { throw 'Installed desktop or CLI is missing' }
        foreach ($binary in @('benchlight.exe','benchlight-desktop.exe')) {
            $sourcePath = Join-Path $workspace "target/release/$binary"
            $sourceHash = (Get-FileHash -LiteralPath $sourcePath -Algorithm SHA256).Hash
            if ($binary -eq 'benchlight-desktop.exe') {
                # Tauri patches this fixed token for NSIS, then restores the
                # standalone output. Compare every byte against that known patch.
                $bytes = [System.IO.File]::ReadAllBytes($sourcePath)
                $text = [System.Text.Encoding]::ASCII.GetString($bytes)
                $token = '__TAURI_BUNDLE_TYPE_VAR_UNK'
                $offset = $text.IndexOf($token, [System.StringComparison]::Ordinal)
                if ($offset -lt 0 -or $text.IndexOf($token, $offset + 1, [System.StringComparison]::Ordinal) -ge 0) { throw 'Expected one Tauri bundle marker; review bundler behavior' }
                $patch = [System.Text.Encoding]::ASCII.GetBytes('__TAURI_BUNDLE_TYPE_VAR_NSS')
                [System.Array]::Copy($patch, 0, $bytes, $offset, $patch.Length)
                $algorithm = [System.Security.Cryptography.SHA256]::Create()
                try { $sourceHash = [System.BitConverter]::ToString($algorithm.ComputeHash($bytes)).Replace('-','') } finally { $algorithm.Dispose() }
            }
            $installedHash = (Get-FileHash -LiteralPath (Join-Path $installation $binary) -Algorithm SHA256).Hash
            if ($sourceHash -ne $installedHash) { throw "Installed binary differs from release output: $binary" }
        }
        foreach ($notice in @('LICENSE','third-party/README.md')) {
            if (-not (Test-Path -LiteralPath (Join-Path $installation $notice))) { throw "Installed notice is missing: $notice" }
        }
        # The default installation and metadata folder coincide. Exercise that
        # layout too, while keeping every test file inside this isolated folder.
        & $cli --data-dir $installation roots add (Join-Path $workspace 'tests/fixtures') --quiet
        if ($LASTEXITCODE -ne 0) { throw 'Co-located metadata initialization failed' }
        & $cli --data-dir $data roots add (Join-Path $workspace 'tests/fixtures') --quiet
        if ($LASTEXITCODE -ne 0) { throw 'Installed CLI root management failed' }
        & $cli --data-dir $data scan --quiet
        if ($LASTEXITCODE -ne 0) { throw 'Installed CLI fixture scan failed' }
        $before = & $cli --data-dir $data status --json | ConvertFrom-Json
        if ($before.roots.Count -ne 1 -or $before.scan.projects -lt 6) { throw 'Installed CLI did not persist the expected fixture scan' }
        $process = Start-Process -FilePath $installerPath -ArgumentList @('/S',"/D=$installation") -PassThru -WindowStyle Hidden
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "Reinstall/upgrade failed: $($process.ExitCode)" }
        $after = & $cli --data-dir $data status --json | ConvertFrom-Json
        if ($LASTEXITCODE -ne 0 -or $after.scan.id -ne $before.scan.id -or $after.roots[0] -ne $before.roots[0]) { throw 'Metadata did not survive reinstall' }
        "Install and same-version reinstall passed; $($after.scan.projects) fixture projects remain in isolated metadata."
    } finally {
        if ($installed) {
            $uninstaller = Join-Path $installation 'uninstall.exe'
            if (Test-Path -LiteralPath $uninstaller) {
                $process = Start-Process -FilePath $uninstaller -ArgumentList '/S' -PassThru -WindowStyle Hidden
                $process.WaitForExit()
                if ($process.ExitCode -ne 0) { throw "Uninstall failed: $($process.ExitCode)" }
            }
        }
    }
    $uninstallWait = [System.Diagnostics.Stopwatch]::StartNew()
    while ((Test-Path -LiteralPath (Join-Path $installation 'benchlight-desktop.exe')) -and $uninstallWait.Elapsed.TotalSeconds -lt 30) { Start-Sleep -Milliseconds 100 }
    if (Test-Path -LiteralPath (Join-Path $installation 'benchlight-desktop.exe')) { throw 'Desktop executable remains after uninstall' }
    if (-not (Test-Path -LiteralPath (Join-Path $data 'benchlight.db'))) { throw 'Uninstall unexpectedly removed isolated metadata' }
    if (-not (Test-Path -LiteralPath (Join-Path $installation 'benchlight.db'))) { throw 'Uninstall removed co-located metadata' }
    $retained = & (Join-Path $workspace 'target/release/benchlight.exe') --data-dir $installation roots list --json | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $retained.Count -ne 1) { throw 'Co-located metadata became unreadable after uninstall' }
    'Uninstall passed; isolated user metadata was preserved.'
} finally { Pop-Location }
