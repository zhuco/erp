param([Parameter(Mandatory = $true)][ValidateRange(1, 2147483647)][int]$MeasurementProcessId)
$ErrorActionPreference = 'Stop'
$erpRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$exe = Join-Path $erpRoot 'build/cargo/release/erp-grid-lab.exe'
if (!(Test-Path -LiteralPath $exe)) { throw 'Build release through Invoke-ErpBuild.ps1 before measuring' }
$folder = Join-Path $erpRoot 'qa/performance'
[IO.Directory]::CreateDirectory($folder) | Out-Null
$lock = [IO.File]::Open((Join-Path $folder 'native.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
$process = $null
try {
    $started = [DateTime]::UtcNow
    $process = Get-Process -Id $MeasurementProcessId
    $command = (Get-CimInstance Win32_Process -Filter "ProcessId = $MeasurementProcessId").CommandLine
    if ($process.Path -ne $exe -or $command -notmatch '--grid-perf') { throw 'Attachment requires the exact release grid performance process' }
    $memory = [Collections.Generic.List[object]]::new()
    while (!$process.HasExited) {
        $process.Refresh()
        if ($process.HasExited) { break }
        $memory.Add(@{ elapsed_ms = ([DateTime]::UtcNow - $started).TotalMilliseconds; working_set_bytes = $process.WorkingSet64; peak_working_set_bytes = $process.PeakWorkingSet64 })
        if (([DateTime]::UtcNow - $started).TotalSeconds -gt 90) {
            throw 'Grid performance run exceeded 90 seconds; no process was terminated'
        }
        Start-Sleep -Milliseconds 200
    }
    $process.WaitForExit()
    $resultPath = Join-Path $folder 'native-latest.json'
    if (!(Test-Path -LiteralPath $resultPath) -or (Get-Item -LiteralPath $resultPath).LastWriteTimeUtc -lt $started) { throw 'No fresh native performance report was produced' }
    $report = Get-Content -LiteralPath $resultPath -Raw | ConvertFrom-Json
    if ($report.build -ne 'release' -or $report.samples.Count -lt 30) { throw 'Report is not a release measurement with sufficient samples' }
    $metadata = @{
        measured_at_utc = $started.ToString('o')
        attached_after_launch = ($MeasurementProcessId -gt 0)
        exe_sha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
        cpu = @(Get-CimInstance Win32_Processor | Select-Object Name, NumberOfCores, NumberOfLogicalProcessors)
        os = Get-CimInstance Win32_OperatingSystem | Select-Object Caption, Version, BuildNumber, TotalVisibleMemorySize
        gpu = @(Get-CimInstance Win32_VideoController | Select-Object Name, DriverVersion)
        cargo_lock_sha256 = (Get-FileHash -LiteralPath (Join-Path $erpRoot 'Cargo.lock') -Algorithm SHA256).Hash
        toolchain = Get-Content -LiteralPath (Join-Path $erpRoot 'rust-toolchain.toml') -Raw
        memory_samples = $memory
    }
    $metadata | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $folder 'native-metadata.json') -Encoding utf8
    $report | Select-Object scenario, build, frame_interval_p95_ms, interval_budget_met, max_cell_text_calls, measured_seconds | ConvertTo-Json
    & node (Join-Path $folder 'validate-report.mjs') $resultPath
    if ($LASTEXITCODE -ne 0) { throw 'PERF-02 report validation failed; preserve raw samples' }
} finally { $lock.Dispose(); if ($null -ne $process) { $process.Dispose() } }
