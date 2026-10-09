$ErrorActionPreference = 'Stop'
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    cargo build -p benchlight-core --example benchmark --release
    if ($LASTEXITCODE -ne 0) { throw 'Benchmark build failed' }
    $benchmarkOutput = Join-Path $env:TEMP 'benchlight-benchmark.txt'
    $benchmarkProcess = Start-Process -FilePath (Join-Path (Get-Location) 'target/release/examples/benchmark.exe') -RedirectStandardOutput $benchmarkOutput -PassThru -WindowStyle Hidden
    # Retain the native process handle so ExitCode stays available after it exits.
    $null = $benchmarkProcess.Handle
    $sampledPeak = 0L
    while (-not $benchmarkProcess.HasExited) {
        $sample = Get-Process -Id $benchmarkProcess.Id -ErrorAction SilentlyContinue
        if ($sample) { $sampledPeak = [Math]::Max($sampledPeak, $sample.WorkingSet64) }
        $benchmarkProcess.Refresh()
        Start-Sleep -Milliseconds 200
    }
    $benchmarkProcess.WaitForExit()
    if ($benchmarkProcess.ExitCode -ne 0) { throw "Benchmark failed: $($benchmarkProcess.ExitCode)" }
    Get-Content -LiteralPath $benchmarkOutput
    "sampled_process_peak_working_set_bytes=$sampledPeak sample_interval_ms=200"
} finally { Pop-Location }
