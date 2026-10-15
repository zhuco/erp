param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('check', 'test', 'clippy', 'build', 'fmt')]
    [string]$Action,
    [ValidateSet('native', 'web')][string]$Platform = 'native',
    [ValidateSet('grid', 'renderer', 'backend', 'domain', 'api', 'repo', 'worker', 'adapters')][string]$Scope = 'grid',
    [string]$TestFilter,
    [ValidateSet(
        'order_match_ship_wave_persistence_guard',
        'wave_ship_integration',
        'platform_sync_and_mapping_flow_integration',
        'frontend_route_diagnostics_smoke',
        'oms_orders',
        'shipment_confirm',
        'platform_oauth_webhook_integration',
        'order_exception_sla_integration',
        'wms_labor_slotting_flow',
        'inventory_atp_fefo_integration',
        'planning_replenishment_flow',
        'after_sales_exchange_profit_flow',
        'system_governance_workflow_integration',
        'stats_clickhouse_sso_smoke',
        'shared_contract',
        'free_erp_acceptance',
        'free_erp_bom_performance'
    )]
    [string]$IntegrationTest,
    [ValidatePattern('^[A-Za-z0-9_:]+$')][string]$IntegrationFilter,
    [switch]$IncludeIgnored,
    [switch]$ShowTestOutput,
    [switch]$JsonDiagnostics,
    [switch]$WriteFormat,
    [string[]]$FormatFiles,
    [switch]$Release,
    [ValidateRange(1, 2)][int]$BuildJobs = 1,
    [switch]$RunPerformance
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ErpBuildStorage.ps1')
if ($Scope -in @('backend', 'domain', 'api', 'repo', 'worker', 'adapters') -and $Platform -ne 'native') { throw 'Backend packages are native only' }
if ($TestFilter -and $Action -ne 'test') { throw 'TestFilter is only valid for test' }
if ($WriteFormat -and $Action -ne 'fmt') { throw 'WriteFormat is only valid for fmt' }
if ($FormatFiles -and $Action -ne 'fmt') { throw 'FormatFiles is only valid for fmt' }
if ($IncludeIgnored -and (-not $TestFilter -or $Action -ne 'test')) { throw 'IncludeIgnored requires a named test filter' }
if ($ShowTestOutput -and (-not $TestFilter -or $Action -ne 'test' -or $IntegrationTest)) { throw 'ShowTestOutput requires a named library TestFilter' }
if ($JsonDiagnostics -and $Action -ne 'clippy') { throw 'JsonDiagnostics is only valid for clippy' }
if ($IntegrationTest -and ($Action -ne 'test' -or $Scope -ne 'api')) { throw 'IntegrationTest requires Action test and Scope api' }
if ($IntegrationTest -and ($TestFilter -or $IncludeIgnored)) { throw 'IntegrationTest cannot be combined with TestFilter or IncludeIgnored' }
if ($IntegrationTest -and $Release -and $IntegrationTest -ne 'free_erp_bom_performance') { throw 'IntegrationTest Release is restricted to the registered performance target' }
if ($RunPerformance -and ($Action -ne 'test' -or $Scope -ne 'api' -or $IntegrationTest -ne 'free_erp_bom_performance' -or -not $Release)) {
    throw 'RunPerformance requires the registered API performance target and Release'
}
if ($IntegrationFilter -and -not $IntegrationTest) { throw 'IntegrationFilter requires a registered IntegrationTest' }
$packages = if ($Scope -eq 'backend') { @('erp-grid-core', 'erp-domain', 'erp-repo', 'erp-api') } elseif ($Scope -eq 'domain') { @('erp-domain') } elseif ($Scope -eq 'renderer') { @('erp-ui-egui') } elseif ($Scope -eq 'api') { @('erp-api') } elseif ($Scope -eq 'repo') { @('erp-repo') } elseif ($Scope -eq 'worker') { @('erp-worker') } elseif ($Scope -eq 'adapters') { @('erp-adapter-pdd', 'erp-adapter-jd', 'erp-adapter-taobao', 'erp-adapter-douyin') } else { @('erp-grid-core', 'erp-ui-egui', 'erp-ui') }
$packageArgs = @()
foreach ($package in $packages) { $packageArgs += @('-p', $package) }
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$cache = [IO.Path]::GetFullPath((Join-Path $repo 'build/cargo'))
$formatPaths = @()
if ($FormatFiles) {
    $packageRoots = @($packages | ForEach-Object {
        $relativeRoot = if ($_ -eq 'erp-ui') { 'apps/erp-ui' } elseif ($_ -like 'erp-adapter-*') { "crates/adapter-$($_.Substring('erp-adapter-'.Length))" } else { "crates/$_" }
        [IO.Path]::GetFullPath((Join-Path $repo $relativeRoot))
    })
    foreach ($file in $FormatFiles) {
        $candidate = [IO.Path]::GetFullPath($(if ([IO.Path]::IsPathRooted($file)) { $file } else { Join-Path $repo $file }))
        if (-not ($packageRoots | Where-Object {
            $candidate.StartsWith($_ + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)
        })) { throw "Format file is outside selected packages: $file" }
        if ([IO.Path]::GetExtension($candidate) -ne '.rs' -or -not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            throw "Format file must be an existing Rust source: $file"
        }
        $ancestor = Get-Item -LiteralPath $candidate -Force
        while ($null -ne $ancestor -and $ancestor.FullName.StartsWith($repo, [StringComparison]::OrdinalIgnoreCase)) {
            if ($ancestor.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Refusing format reparse point: $($ancestor.FullName)" }
            $ancestor = if ($ancestor.PSIsContainer) { $ancestor.Parent } else { $ancestor.Directory }
        }
        # A direct rustfmt invocation follows external modules. Only leaf modules
        # are admitted here so a local change cannot format unrelated files.
        $source = Get-Content -LiteralPath $candidate -Raw -Encoding UTF8
        if ([IO.Path]::GetFileName($candidate) -in @('lib.rs', 'main.rs', 'mod.rs') -or
            $source -match '(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+[A-Za-z_]\w*\s*;') {
            throw "FormatFiles requires a leaf module; use package fmt for module roots: $file"
        }
        $formatPaths += $candidate
    }
    $formatPaths = @($formatPaths | Select-Object -Unique)
}
foreach ($path in @($repo, (Join-Path $repo 'build'), $cache)) {
    if ((Test-Path -LiteralPath $path) -and ((Get-Item -LiteralPath $path).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing reparse point: $path"
    }
}
if (-not $cache.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Cache outside workspace' }
# One fixed cache/build at a time. Registered failed-working maintenance holds this lock; no process is terminated.
$previousBuildEnvironment = $null
$lock = $null
$deadline = [DateTime]::UtcNow.AddMinutes(10)
while ($null -eq $lock) {
    try { $lock = [IO.File]::Open((Join-Path $repo 'build.lock'), 'OpenOrCreate', 'ReadWrite', 'None') }
    catch [IO.IOException] {
        if ([DateTime]::UtcNow -gt $deadline) { throw 'Build lock wait exceeded 10 minutes' }
        Write-Host 'Waiting for ERP build lock...'
        Start-Sleep -Seconds 5
    }
}
try {
    $storage = Initialize-ErpBuildStorage $repo $cache
    $previousBuildEnvironment = Set-ErpBuildEnvironment $storage
    $cargoExe = $storage.Cargo
    $rustfmtExe = $storage.Rustfmt
    Write-Host "ERP build storage: target=$($storage.Target); cargo-home=$($storage.CargoHome); temp=$($storage.Temp)"
    Push-Location $repo
    try {
        $argsList = @($Action)
        if ($FormatFiles) {
            $formatArgs = @('--edition', '2024', '--style-edition', '2024')
            if (-not $WriteFormat) { $formatArgs += '--check' }
            & $rustfmtExe @formatArgs @formatPaths
            if ($LASTEXITCODE -ne 0) { throw "Rustfmt failed with exit code $LASTEXITCODE" }
        } elseif ($Action -eq 'fmt') {
            $argsList += $packageArgs
            if (-not $WriteFormat) { $argsList += @('--', '--check') }
        } else {
            $argsList += @('--target-dir', $cache, '--jobs', $BuildJobs) + $packageArgs
            if ($JsonDiagnostics) { $argsList += '--message-format=json' }
            if ($Release) { $argsList += '--release' }
            if ($TestFilter) { $argsList += @('--lib', $TestFilter) }
            if ($IntegrationTest) {
                $argsList += @('--test', $IntegrationTest)
                if ($IntegrationFilter) { $argsList += $IntegrationFilter }
                $argsList += @('--', '--nocapture', '--test-threads=1')
                if ($RunPerformance) { $argsList += '--ignored' }
            }
            if ($Platform -eq 'web') {
                if ($Action -eq 'test') { throw 'WASM tests require browser runner; use native tests and web check' }
                $argsList += @('--target', 'wasm32-unknown-unknown', '--lib')
            }
            if ($Platform -eq 'native' -and $Action -eq 'clippy') { $argsList += '--all-targets' }
            if ($Action -eq 'clippy') { $argsList += @('--', '-D', 'warnings') }
            if ($IncludeIgnored -or $ShowTestOutput) {
                $argsList += '--'
                if ($IncludeIgnored) { $argsList += '--include-ignored' }
                if ($ShowTestOutput) { $argsList += '--nocapture' }
            }
        }
        if (-not $FormatFiles) {
            & $cargoExe @argsList
            if ($LASTEXITCODE -ne 0) { throw "Cargo $Action failed with exit code $LASTEXITCODE" }
        }
    } finally { Pop-Location }
} finally {
    if ($null -ne $previousBuildEnvironment) { Restore-ErpBuildEnvironment $previousBuildEnvironment }
    $lock.Dispose()
}

