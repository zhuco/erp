param(
    [Parameter(Mandatory = $true)]
    [ValidateSet('check', 'test', 'clippy', 'build', 'fmt')]
    [string]$Action,
    [ValidateSet('native', 'web')][string]$Platform = 'native',
    [ValidateSet('grid', 'renderer', 'backend', 'api', 'repo', 'worker')][string]$Scope = 'grid',
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
        'stats_clickhouse_sso_smoke'
    )]
    [string]$IntegrationTest,
    [ValidatePattern('^[A-Za-z0-9_:]+$')][string]$IntegrationFilter,
    [switch]$IncludeIgnored,
    [switch]$WriteFormat,
    [switch]$Release
)
$ErrorActionPreference = 'Stop'
if ($Scope -in @('backend', 'api', 'repo', 'worker') -and $Platform -ne 'native') { throw 'Backend packages are native only' }
if ($TestFilter -and $Action -ne 'test') { throw 'TestFilter is only valid for test' }
if ($WriteFormat -and $Action -ne 'fmt') { throw 'WriteFormat is only valid for fmt' }
if ($IncludeIgnored -and (-not $TestFilter -or $Action -ne 'test')) { throw 'IncludeIgnored requires a named test filter' }
if ($IntegrationTest -and ($Action -ne 'test' -or $Scope -ne 'api')) { throw 'IntegrationTest requires Action test and Scope api' }
if ($IntegrationTest -and ($TestFilter -or $IncludeIgnored)) { throw 'IntegrationTest cannot be combined with TestFilter or IncludeIgnored' }
if ($IntegrationTest -and $Release) { throw 'IntegrationTest cannot be combined with Release' }
if ($IntegrationFilter -and -not $IntegrationTest) { throw 'IntegrationFilter requires a registered IntegrationTest' }
$packages = if ($Scope -eq 'backend') { @('erp-grid-core', 'erp-repo', 'erp-api') } elseif ($Scope -eq 'renderer') { @('erp-ui-egui') } elseif ($Scope -eq 'api') { @('erp-api') } elseif ($Scope -eq 'repo') { @('erp-repo') } elseif ($Scope -eq 'worker') { @('erp-worker') } else { @('erp-grid-core', 'erp-ui-egui', 'erp-ui') }
$packageArgs = @()
foreach ($package in $packages) { $packageArgs += @('-p', $package) }
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$cache = [IO.Path]::GetFullPath((Join-Path $repo 'build/cargo'))
foreach ($path in @($repo, (Join-Path $repo 'build'), $cache)) {
    if ((Test-Path -LiteralPath $path) -and ((Get-Item -LiteralPath $path).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing reparse point: $path"
    }
}
if (-not $cache.StartsWith($repo + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Cache outside workspace' }
# One fixed cache and one build at a time. Never delete cache or another process.
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
    $drive = [IO.DriveInfo]::new([IO.Path]::GetPathRoot($cache))
    if ($drive.AvailableFreeSpace -lt 20GB) { throw 'ERP build requires at least 20 GiB free on cache volume' }
    $userProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
    if ([String]::IsNullOrWhiteSpace($userProfile)) { throw 'Cannot resolve user profile for toolchain volume admission' }
    $toolchainDrive = [IO.DriveInfo]::new([IO.Path]::GetPathRoot([IO.Path]::GetFullPath($userProfile)))
    if ($toolchainDrive.AvailableFreeSpace -lt 10GB) { throw 'Toolchain volume requires at least 10 GiB free' }
    $bytes = 0L
    if (Test-Path -LiteralPath $cache) {
        $dirs = [Collections.Generic.Queue[string]]::new()
        $dirs.Enqueue($cache)
        while ($dirs.Count -gt 0) {
            foreach ($item in Get-ChildItem -LiteralPath $dirs.Dequeue() -Force) {
                if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Cache contains reparse point: $($item.FullName)" }
                if ($item.PSIsContainer) { $dirs.Enqueue($item.FullName) } else { $bytes += $item.Length }
            }
        }
    }
    if ($bytes -gt 80GB) { throw 'ERP cache exceeds 80 GiB admission budget; no automatic cleanup' }
    Push-Location $repo
    try {
        $argsList = @('+1.98.1', $Action)
        if ($Action -eq 'fmt') {
            $argsList += $packageArgs
            if (-not $WriteFormat) { $argsList += @('--', '--check') }
        } else {
            $argsList += @('--target-dir', $cache) + $packageArgs
            if ($TestFilter) { $argsList += @('--lib', $TestFilter) }
            if ($IntegrationTest) {
                $argsList += @('--test', $IntegrationTest)
                if ($IntegrationFilter) { $argsList += $IntegrationFilter }
                $argsList += @('--', '--nocapture', '--test-threads=1')
            }
            if ($Platform -eq 'web') {
                if ($Action -eq 'test') { throw 'WASM tests require browser runner; use native tests and web check' }
                $argsList += @('--target', 'wasm32-unknown-unknown', '--lib')
            }
            if ($Release) { $argsList += '--release' }
            if ($Platform -eq 'native' -and $Action -eq 'clippy') { $argsList += '--all-targets' }
            if ($Action -eq 'clippy') { $argsList += @('--', '-D', 'warnings') }
            if ($IncludeIgnored) { $argsList += @('--', '--include-ignored') }
        }
        & cargo @argsList
        if ($LASTEXITCODE -ne 0) { throw "Cargo $Action failed with exit code $LASTEXITCODE" }
    } finally { Pop-Location }
} finally { $lock.Dispose() }

