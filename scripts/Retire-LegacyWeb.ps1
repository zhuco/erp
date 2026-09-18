param([switch]$RemoveVerifiedSource)
$ErrorActionPreference = 'Stop'
$erpRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$legacyRoot = [IO.Path]::GetFullPath((Join-Path $erpRoot 'apps/erp-web'))
$backupRoot = [IO.Path]::GetFullPath((Join-Path $erpRoot 'qa/legacy-web-retirement'))
if (!(Test-Path -LiteralPath $legacyRoot)) { throw 'Legacy source directory is missing' }
foreach ($entryPath in @($erpRoot, (Join-Path $erpRoot 'apps'), $legacyRoot, (Join-Path $erpRoot 'qa'), $backupRoot)) {
    if ((Test-Path -LiteralPath $entryPath) -and ((Get-Item -LiteralPath $entryPath).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing reparse point: $entryPath"
    }
}
$archive = Join-Path $backupRoot 'source.zip'
$manifest = Join-Path $backupRoot 'manifest.json'
Add-Type -AssemblyName System.IO.Compression.FileSystem
if (!(Test-Path -LiteralPath $manifest)) {
    if (Test-Path -LiteralPath $archive) { throw 'Archive exists without a completed manifest; preserve and investigate it' }
    New-Item -ItemType Directory -Path $backupRoot -Force | Out-Null
    $queue = [Collections.Generic.Queue[string]]::new()
    $queue.Enqueue($legacyRoot)
    $files = [Collections.Generic.List[object]]::new()
    while ($queue.Count) {
        foreach ($item in Get-ChildItem -LiteralPath $queue.Dequeue() -Force) {
            # Installed dependencies are not source and are not authorized cache cleanup.
            if ($item.Name -eq 'node_modules' -and $item.PSIsContainer) { continue }
            if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "Refusing reparse point: $($item.FullName)" }
            if ($item.PSIsContainer) { $queue.Enqueue($item.FullName); continue }
            $relative = [IO.Path]::GetRelativePath($legacyRoot, $item.FullName).Replace('\', '/')
            $files.Add(@{ path=$relative; sha256=(Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash; bytes=$item.Length })
        }
    }
    $zip = [IO.Compression.ZipFile]::Open($archive, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($file in $files) {
            [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, (Join-Path $legacyRoot $file.path), $file.path, [IO.Compression.CompressionLevel]::Optimal) | Out-Null
        }
    } finally { $zip.Dispose() }
    @{ source=$legacyRoot; files=@($files.ToArray()); archiveSha256=(Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash } |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $manifest -Encoding utf8
}
$record = Get-Content -LiteralPath $manifest -Raw | ConvertFrom-Json
if ($record.source -ne $legacyRoot -or (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $record.archiveSha256) { throw 'Backup identity/checksum mismatch' }
$zip = [IO.Compression.ZipFile]::OpenRead($archive)
try {
    if ($zip.Entries.Count -ne $record.files.Count) { throw 'Backup file count mismatch' }
    foreach ($file in $record.files) {
        $source = [IO.Path]::GetFullPath((Join-Path $legacyRoot $file.path))
        if (!$source.StartsWith($legacyRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or $file.path -match '(^|/)node_modules(/|$)') { throw 'Manifest path escapes approved source' }
        $parent = [IO.Path]::GetDirectoryName($source)
        while ($parent.Length -ge $legacyRoot.Length) {
            if ((Test-Path -LiteralPath $parent) -and ((Get-Item -LiteralPath $parent).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw "Refusing source ancestor reparse point: $parent" }
            if ($parent -eq $legacyRoot) { break }
            $parent = [IO.Path]::GetDirectoryName($parent)
        }
        $entry = $zip.GetEntry($file.path)
        if (!$entry) { throw "Backup entry missing: $($file.path)" }
        $stream = $entry.Open()
        try { $digest=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($stream)) } finally { $stream.Dispose() }
        if ($digest -ne $file.sha256) { throw "Backup entry checksum mismatch: $($file.path)" }
        if (Test-Path -LiteralPath $source) {
            if (((Get-Item -LiteralPath $source).Attributes -band [IO.FileAttributes]::ReparsePoint) -or (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ne $file.sha256) { throw "Source changed since backup: $($file.path)" }
        }
    }
} finally { $zip.Dispose() }
if ($RemoveVerifiedSource) {
    foreach ($file in $record.files) {
        $source = [IO.Path]::GetFullPath((Join-Path $legacyRoot $file.path))
        if (Test-Path -LiteralPath $source) { Remove-Item -LiteralPath $source -Force }
    }
    Write-Host "Retired $($record.files.Count) verified source files. Installed caches and unlisted files were preserved."
} else {
    Write-Host "Verified backup of $($record.files.Count) source files. No source removed."
}
