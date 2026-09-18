$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

Write-Host "==> 启动 Rust UI 本地夹具" -ForegroundColor Cyan
& (Join-Path $PSScriptRoot "dev.ps1")
if ($LASTEXITCODE -ne 0) {
  throw "Rust UI 本地夹具退出码：$LASTEXITCODE"
}
