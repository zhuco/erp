# User-authorized maintenance runs only while Invoke-ErpBuild holds build.lock.
function Invoke-ErpRegisteredCacheMaintenance([string]$Repo, [string]$Target, [string[]]$StoragePaths, [long]$Budget) {
    $registryPath = Join-Path $Repo 'qa/acceptance/erp-cache-cleanup-registry.json'
    if (-not (Test-Path -LiteralPath $registryPath)) { return }
    $registry = Get-Content -LiteralPath $registryPath -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($registry.version -ne 1 -or -not $registry.automatic_failed_working_only) { throw 'Invalid ERP cache maintenance registry' }
    $fixedTarget = [IO.Path]::GetFullPath($Target)
    if (-not $fixedTarget.Equals([IO.Path]::GetFullPath($registry.fixed_target), [StringComparison]::OrdinalIgnoreCase)) { throw 'Cleanup registry target differs from fixed build target' }
    $incremental = Join-Path $fixedTarget 'debug/incremental'
    # A budget breach may register only compiler-reported failed working sessions.
    # Completed increments, deps, binaries and registry downloads are never candidates.
    if ((Get-ErpStorageBytes $StoragePaths) -gt $Budget) {
        foreach ($log in Get-ChildItem -LiteralPath (Join-Path $Repo 'qa/acceptance') -Filter '*.log' -File) {
            foreach ($line in Get-Content -LiteralPath $log.FullName -TotalCount 256 -Encoding UTF8) {
                if ($line -notmatch '^note: did not finalize incremental compilation session directory `([^`]+)`') { continue }
                $candidate = $Matches[1]
                if ($candidate.StartsWith('\\?\')) { $candidate = $candidate.Substring(4) }
                $candidate = [IO.Path]::GetFullPath($candidate)
                if (-not $candidate.StartsWith($incremental + '\', [StringComparison]::OrdinalIgnoreCase)) { continue }
                $relative = [IO.Path]::GetRelativePath($incremental, $candidate)
                if ($relative -notmatch '^[a-zA-Z0-9_-]+-[a-zA-Z0-9]+[\\/]s-[a-zA-Z0-9]+-[a-zA-Z0-9]+-working$') { continue }
                Assert-ErpStoragePath $candidate $incremental | Out-Null
                if ($registry.items.path -contains $candidate) { continue }
                if (-not (Test-Path -LiteralPath $candidate -PathType Container)) { continue }
                $registry.items = @($registry.items) + [pscustomobject]@{path=$candidate; state='PENDING'; evidence=$log.FullName; expected_bytes=(Get-ErpStorageBytes @($candidate)); registered_utc=[DateTime]::UtcNow.ToString('o')}
            }
        }
    }
    $registry | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $registryPath -Encoding UTF8
    foreach ($entry in @($registry.items)) {
        if ($entry.state -ne 'PENDING') { continue }
        $safe = Assert-ErpStoragePath $entry.path $incremental
        $relative = [IO.Path]::GetRelativePath($incremental, $safe)
        if ($relative -notmatch '^[a-zA-Z0-9_-]+-[a-zA-Z0-9]+[\\/]s-[a-zA-Z0-9]+-[a-zA-Z0-9]+-working$') { throw "Unapproved cache shape: $safe" }
        if (-not (Test-Path -LiteralPath $safe)) { $entry.state='ABSENT'; continue }
        $evidence = Assert-ErpStoragePath $entry.evidence (Join-Path $Repo 'qa/acceptance')
        $proof = Get-Content -LiteralPath $evidence -TotalCount 256 -Encoding UTF8
        if (-not ($proof | Where-Object { $_.Contains($safe) -and $_.Contains('did not finalize incremental compilation session directory') })) { throw "No failed-compiler evidence for $safe" }
        $bytes = Get-ErpStorageBytes @($safe) # Checks every descendant for reparse points.
        if ($bytes -ne [long]$entry.expected_bytes) { throw "Registered cache changed: $safe" }
        if (Get-Process cargo,rustc,rustfmt -ErrorAction SilentlyContinue) {
            Write-Host 'ERP cache cleanup deferred while Rust tools are active'
            continue
        }
        # Exclusive open checks occupancy without terminating or modifying a process.
        $files = @(Get-ChildItem -LiteralPath $safe -File -Force -Recurse)
        foreach ($file in $files) {
            $handle = [IO.File]::Open($file.FullName,'Open','Read','None')
            $handle.Dispose()
        }
        # Only a canonical, registered, failed working session can reach deletion.
        Remove-Item -LiteralPath $safe -Recurse -Force -ErrorAction Stop
        $entry.state='DELETED'
        $entry | Add-Member -NotePropertyName deleted_bytes -NotePropertyValue $bytes -Force
        $entry | Add-Member -NotePropertyName deleted_utc -NotePropertyValue ([DateTime]::UtcNow.ToString('o')) -Force
        $registry | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $registryPath -Encoding UTF8
        Write-Host "ERP failed working cache removed: $safe ($bytes bytes)"
    }
    $registry | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $registryPath -Encoding UTF8
}
