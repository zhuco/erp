# Fixed ERP storage with user-authorized registered maintenance; no toolchain migration or machine-wide env changes.
function Assert-ErpStoragePath([string]$Path, [string]$Root) {
    $resolved = [IO.Path]::GetFullPath($Path)
    $boundary = [IO.Path]::GetFullPath($Root).TrimEnd('\', '/')
    if (-not $resolved.StartsWith($boundary + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
        throw "ERP build storage is outside its fixed root: $resolved"
    }
    $ancestor = $resolved
    while ($ancestor -and $ancestor.StartsWith($boundary, [StringComparison]::OrdinalIgnoreCase)) {
        if ((Test-Path -LiteralPath $ancestor) -and ((Get-Item -LiteralPath $ancestor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw "ERP storage contains a reparse point: $ancestor"
        }
        $ancestor = [IO.Path]::GetDirectoryName($ancestor)
    }
    return $resolved
}

function Get-ErpStorageBytes([string[]]$Paths) {
    $total = 0L
    foreach ($path in $Paths) {
        if (-not (Test-Path -LiteralPath $path)) { continue }
        if ((Get-Item -LiteralPath $path -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "ERP cache root is a reparse point: $path" }
        $queue = [Collections.Generic.Queue[string]]::new()
        $queue.Enqueue($path)
        while ($queue.Count -gt 0) {
            foreach ($item in Get-ChildItem -LiteralPath $queue.Dequeue() -Force) {
                if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "ERP cache contains a reparse point: $($item.FullName)" }
                if ($item.PSIsContainer) { $queue.Enqueue($item.FullName) } else { $total += $item.Length }
            }
        }
    }
    return $total
}

function Initialize-ErpBuildStorage([string]$Repo, [string]$Target) {
    $root = [IO.Path]::GetFullPath((Join-Path $Repo 'build'))
    $targetPath = Assert-ErpStoragePath $Target $root
    $homePath = Assert-ErpStoragePath (Join-Path $root 'cargo-home') $root
    $tempPath = Assert-ErpStoragePath (Join-Path $root 'tmp') $root
    $volume = [IO.DriveInfo]::new([IO.Path]::GetPathRoot($root))
    if ($volume.AvailableFreeSpace -lt 20GB) { throw 'ERP build storage volume requires at least 20 GiB free' }
    $paths = @($targetPath, $homePath, $tempPath)
    . (Join-Path $Repo 'scripts/ErpCacheMaintenance.ps1')
    Invoke-ErpRegisteredCacheMaintenance $Repo $targetPath $paths 200GB
    if ((Get-ErpStorageBytes $paths) -gt 200GB) { throw 'ERP combined build storage exceeds 200 GiB admission budget; registered failed-working cleanup could not restore admission' }

    # Resolve only an already installed, pinned toolchain. Actual build invokes it directly.
    $rustc = & rustup which --toolchain 1.98.1 rustc
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $rustc -PathType Leaf)) { throw 'Installed Rust 1.98.1 is required; this entry does not install or migrate toolchains' }
    $bin = Split-Path -Parent $rustc
    $extension = if ([OperatingSystem]::IsWindows()) { '.exe' } else { '' }
    $cargo = Join-Path $bin "cargo$extension"
    $rustfmt = Join-Path $bin "rustfmt$extension"
    $rustdoc = Join-Path $bin "rustdoc$extension"
    foreach ($program in @($cargo, $rustfmt, $rustdoc)) {
        if (-not (Test-Path -LiteralPath $program -PathType Leaf)) { throw "Pinned toolchain executable unavailable: $program" }
    }

    foreach ($path in $paths) { [IO.Directory]::CreateDirectory($path) | Out-Null }
    $marker = Join-Path $homePath '.erp-registry-seed-v1'
    if (-not (Test-Path -LiteralPath $marker)) {
        $oldHome = [Environment]::GetEnvironmentVariable('CARGO_HOME', 'Process')
        if ([String]::IsNullOrWhiteSpace($oldHome)) {
            $oldHome = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)) '.cargo'
        }
        if (-not [IO.Path]::GetFullPath($oldHome).Equals($homePath, [StringComparison]::OrdinalIgnoreCase)) {
            # Reuse only downloaded packages/index metadata. Never copy credentials or config.
            $sources = @('registry/cache', 'registry/index') | ForEach-Object { Join-Path $oldHome $_ }
            $seedBytes = Get-ErpStorageBytes $sources
            if ((Get-ErpStorageBytes $paths) + $seedBytes -gt 200GB) { throw 'Registry seed would exceed ERP 200 GiB storage budget' }
            if ($volume.AvailableFreeSpace - $seedBytes -lt 20GB) { throw 'Registry seed would leave less than 20 GiB on ERP storage volume' }
            foreach ($relative in @('registry/cache', 'registry/index')) {
                $source = Join-Path $oldHome $relative
                if (-not (Test-Path -LiteralPath $source)) { continue }
                if ((Get-Item -LiteralPath $source -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Registry seed root is a reparse point: $source" }
                $queue = [Collections.Generic.Queue[string]]::new()
                $queue.Enqueue($source)
                while ($queue.Count -gt 0) {
                    $directory = $queue.Dequeue()
                    $suffix = [IO.Path]::GetRelativePath($oldHome, $directory)
                    $destination = Assert-ErpStoragePath (Join-Path $homePath $suffix) $root
                    [IO.Directory]::CreateDirectory($destination) | Out-Null
                    foreach ($item in Get-ChildItem -LiteralPath $directory -Force) {
                        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Registry seed contains a reparse point: $($item.FullName)" }
                        if ($item.PSIsContainer) { $queue.Enqueue($item.FullName) } else {
                            $file = Join-Path $destination $item.Name
                            if (-not (Test-Path -LiteralPath $file)) { $item.CopyTo($file, $false) | Out-Null }
                        }
                    }
                }
            }
        }
        [IO.File]::WriteAllText($marker, 'registry cache/index only; source unchanged', [Text.UTF8Encoding]::new($false))
    }
    return [pscustomobject]@{ Target = $targetPath; CargoHome = $homePath; Temp = $tempPath; Bin = $bin; Cargo = $cargo; Rustc = $rustc; Rustfmt = $rustfmt; Rustdoc = $rustdoc }
}

function Set-ErpBuildEnvironment($Storage) {
    $values = @{ CARGO_HOME = $Storage.CargoHome; TEMP = $Storage.Temp; TMP = $Storage.Temp; TMPDIR = $Storage.Temp; RUSTC = $Storage.Rustc; RUSTDOC = $Storage.Rustdoc; PATH = $Storage.Bin + [IO.Path]::PathSeparator + [Environment]::GetEnvironmentVariable('PATH', 'Process') }
    $old = @{}
    foreach ($name in $values.Keys) {
        $old[$name] = [Environment]::GetEnvironmentVariable($name, 'Process')
        [Environment]::SetEnvironmentVariable($name, $values[$name], 'Process')
    }
    return $old
}

function Restore-ErpBuildEnvironment([hashtable]$Values) {
    foreach ($name in $Values.Keys) {
        if ($null -eq $Values[$name]) {
            [Environment]::SetEnvironmentVariable($name, [NullString]::Value, 'Process')
        } else {
            [Environment]::SetEnvironmentVariable($name, $Values[$name], 'Process')
        }
    }
}
