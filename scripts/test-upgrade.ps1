param(
    [Parameter(Mandatory)][string]$OldInstaller,
    [Parameter(Mandatory)][string]$NewInstaller,
    [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]{1,40}$')][string]$RunId
)
$ErrorActionPreference = 'Stop'
$workspace = (Resolve-Path -LiteralPath (Split-Path $PSScriptRoot -Parent)).Path
$old = (Resolve-Path -LiteralPath $OldInstaller).Path
$new = (Resolve-Path -LiteralPath $NewInstaller).Path
$expectedOld = 'eedabb9ac4acb5134a59895f133811adc70d1e6e14a5d156e29014ebb946e588'
if ((Get-FileHash -LiteralPath $old -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedOld) {
    throw 'Published 0.1.0 installer hash differs from the published SHA256SUMS file'
}
$run = [System.IO.Path]::GetFullPath((Join-Path $workspace "target/upgrade-qualification/$RunId"))
if (-not $run.StartsWith($workspace + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Upgrade fixture escaped workspace' }
if (Test-Path -LiteralPath $run) { throw 'Upgrade fixture already exists; choose another RunId' }
$existing = Get-ChildItem -LiteralPath HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall |
    Get-ItemProperty | Where-Object DisplayName -eq Benchlight
if ($existing) { throw 'Benchlight is already installed for this user; refusing to replace it' }
$install = Join-Path $run 'Install With Spaces'
$data = Join-Path $run 'metadata'
$project = Join-Path $run 'fixture project'
$artifact = Join-Path $project 'node_modules'
$excluded = Join-Path $project 'excluded'
$installed = $false
function Invoke-Installer([string]$path) {
    $process = Start-Process -FilePath $path -ArgumentList @('/S', "/D=$install") -PassThru -WindowStyle Hidden
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw "Installer failed with exit code $($process.ExitCode): $path" }
}
function Invoke-Cli([string]$binary, [string[]]$arguments) {
    $result = & $binary --data-dir $data @arguments
    if ($LASTEXITCODE -ne 0) { throw "Installed CLI failed: $($arguments -join ' ')" }
    return ($result | Out-String)
}
try {
    New-Item -ItemType Directory -Path $artifact, $excluded -Force | Out-Null
    '{"dependencies":{"fixture":"1"}}' | Set-Content -LiteralPath (Join-Path $project 'package.json')
    'lock one' | Set-Content -LiteralPath (Join-Path $project 'package-lock.json')
    'disposable fixture' | Set-Content -LiteralPath (Join-Path $artifact 'fixture')
    'excluded fixture' | Set-Content -LiteralPath (Join-Path $excluded 'keep')
    Invoke-Installer $old
    $installed = $true
    $cli = Join-Path $install 'benchlight.exe'
    if (-not (Test-Path -LiteralPath (Join-Path $install 'benchlight-desktop.exe'))) { throw 'Old desktop is missing' }
    $oldStatus = Invoke-Cli $cli @('status', '--json') | ConvertFrom-Json
    if ($oldStatus.version -ne '0.1.0') { throw 'Published installer did not provide the expected old CLI' }
    Invoke-Cli $cli @('roots', 'add', $project, '--quiet') | Out-Null
    Invoke-Cli $cli @('exclusions', $excluded, '--quiet') | Out-Null
    $configured = Invoke-Cli $cli @('status', '--json') | ConvertFrom-Json
    $expectedRoot = $configured.roots[0]
    $oldScan = Invoke-Cli $cli @('scan', '--json') | ConvertFrom-Json
    if ($oldScan.projects -lt 1 -or $oldScan.state -ne 'complete') { throw 'Published CLI fixture scan failed' }
    Invoke-Cli $cli @('mark-good', $project, '--quiet') | Out-Null
    Invoke-Cli $cli @('mark-good', $project, '--quiet') | Out-Null
    $plan = Invoke-Cli $cli @('cleanup', 'plan', $artifact) | ConvertFrom-Json
    if ($plan.id -lt 1 -or $plan.state -ne 'preview') { throw 'Published CLI did not save a preview plan' }
    $database = Join-Path $data 'benchlight.db'
    & python -c "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute('INSERT INTO settings(key,value) VALUES (?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value',('theme','dark')); c.commit()" $database
    if ($LASTEXITCODE -ne 0) { throw 'Could not seed isolated appearance setting' }
    $oldStatus = Invoke-Cli $cli @('status', '--json') | ConvertFrom-Json
    if ($oldStatus.theme -ne 'dark') { throw 'Old database did not retain appearance setting' }
    $before = & python -c "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); print(*c.execute('SELECT (SELECT user_version FROM pragma_user_version),(SELECT count(*) FROM working_snapshots),(SELECT count(*) FROM cleanup_plans)').fetchone())" $database
    if ($LASTEXITCODE -ne 0) { throw 'Could not inspect old isolated database' }
    Invoke-Installer $new
    $upgraded = Invoke-Cli $cli @('status', '--json') | ConvertFrom-Json
    if ($upgraded.version -ne '0.1.1' -or $upgraded.theme -ne 'dark' -or
        $upgraded.roots.Count -ne 1 -or $upgraded.roots[0] -ne $expectedRoot -or
        $upgraded.exclusions.Count -ne 1 -or $upgraded.exclusions[0] -ne $excluded) {
        throw 'Upgrade changed version/settings/roots unexpectedly'
    }
    $saved = Invoke-Cli $cli @('working-state', $project) | ConvertFrom-Json
    if (-not $saved -or $saved.project -ne $expectedRoot) { throw 'Working baseline was not preserved' }
    $savedPlan = Invoke-Cli $cli @('cleanup', 'show', [string]$plan.id) | ConvertFrom-Json
    if ($savedPlan.id -ne $plan.id -or $savedPlan.items[0].candidate.path -ne $plan.items[0].candidate.path) { throw 'Cleanup history was not preserved' }
    $after = & python -c "import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); print(*c.execute('SELECT (SELECT user_version FROM pragma_user_version),(SELECT count(*) FROM working_snapshots),(SELECT count(*) FROM cleanup_plans)').fetchone())" $database
    if ($LASTEXITCODE -ne 0 -or $after -ne $before) { throw "Database history changed during upgrade: $before -> $after" }
    'lock two' | Set-Content -LiteralPath (Join-Path $project 'package-lock.json')
    $comparison = Invoke-Cli $cli @('changes', $project) | ConvertFrom-Json
    if (-not ($comparison.changes | Where-Object { $_.field -eq 'Lockfile / package-lock.json' -and $_.state -eq 'Changed' })) {
        throw 'Upgraded CLI did not compare against the saved baseline'
    }
    $newScan = Invoke-Cli $cli @('scan', '--json') | ConvertFrom-Json
    if ($newScan.projects -lt 1 -or $newScan.state -ne 'complete') { throw 'Upgraded CLI fixture rescan failed' }
    if (-not (Test-Path -LiteralPath (Join-Path $artifact 'fixture'))) { throw 'Upgrade touched fixture project files' }
    $uninstaller = Join-Path $install 'uninstall.exe'
    $process = Start-Process -FilePath $uninstaller -ArgumentList '/S' -PassThru -WindowStyle Hidden
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw 'Upgrade test uninstall failed' }
    $installed = $false
    if (-not (Test-Path -LiteralPath $database)) { throw 'Uninstall removed isolated metadata' }
    Invoke-Installer $new
    $installed = $true
    $reinstalled = Invoke-Cli $cli @('status', '--json') | ConvertFrom-Json
    if ($reinstalled.version -ne '0.1.1' -or $reinstalled.theme -ne 'dark' -or $reinstalled.roots.Count -ne 1) {
        throw 'Reinstall did not retain isolated metadata'
    }
    "Upgrade passed: published 0.1.0 -> local 0.1.1; schema/history $before; scan, comparison, uninstall and reinstall retained isolated data."
} finally {
    if ($installed) {
        $uninstaller = Join-Path $install 'uninstall.exe'
        if (Test-Path -LiteralPath $uninstaller) {
            $process = Start-Process -FilePath $uninstaller -ArgumentList '/S' -PassThru -WindowStyle Hidden
            $process.WaitForExit()
            if ($process.ExitCode -ne 0) { Write-Warning "Fixture uninstaller returned $($process.ExitCode)" }
        }
    }
}
