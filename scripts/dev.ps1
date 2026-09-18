$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$buildEntry = Join-Path $PSScriptRoot "Invoke-ErpBuild.ps1"
$executable = Join-Path $root "build/cargo/debug/erp-grid-lab.exe"

if (-not (Test-Path -LiteralPath $buildEntry -PathType Leaf)) {
  throw "未找到统一构建入口：$buildEntry"
}

Write-Host "==> 构建 Rust 原生 UI" -ForegroundColor Cyan
& $buildEntry -Action build -Platform native -Scope grid
if ($LASTEXITCODE -ne 0) {
  throw "Rust UI 构建失败，退出码：$LASTEXITCODE"
}

if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
  throw "构建完成但未找到实验台：$executable"
}

Write-Host "==> 启动易格 Rust UI" -ForegroundColor Cyan
& $executable
if ($LASTEXITCODE -ne 0) {
  throw "Rust UI 退出码：$LASTEXITCODE"
}
