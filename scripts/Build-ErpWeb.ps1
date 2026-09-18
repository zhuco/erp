param([switch]$Release)
$ErrorActionPreference = 'Stop'
$erpRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$bindgen = Join-Path $erpRoot 'qa/acceptance/tools/wasm-bindgen-0.2.128-x86_64-pc-windows-msvc/wasm-bindgen.exe'
if (!(Test-Path -LiteralPath $bindgen)) {
    $command = Get-Command wasm-bindgen -ErrorAction SilentlyContinue
    if (!$command) { throw 'wasm-bindgen 0.2.128 is required; see qa/acceptance/WEB.md' }
    $bindgen = $command.Source
}
if ((& $bindgen --version) -ne 'wasm-bindgen 0.2.128') { throw 'wasm-bindgen must match Cargo.lock (0.2.128)' }
& (Join-Path $PSScriptRoot 'Invoke-ErpBuild.ps1') -Action build -Platform web -Scope grid -Release:$Release
$profile = if ($Release) { 'release' } else { 'debug' }
& $bindgen (Join-Path $erpRoot "build/cargo/wasm32-unknown-unknown/$profile/erp_ui.wasm") --target web --out-dir (Join-Path $erpRoot 'apps/erp-ui/pkg') --out-name erp_ui --no-typescript
if ($LASTEXITCODE -ne 0) { throw 'WASM packaging failed' }
