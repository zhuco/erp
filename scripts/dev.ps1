$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$pidFile = Join-Path $PSScriptRoot ".dev-processes.json"
$composeFile = Join-Path $root "docker-compose.local.yml"

function Write-Step([string]$message) {
  Write-Host "==> $message" -ForegroundColor Cyan
}

function Write-Warn([string]$message) {
  Write-Host "!! $message" -ForegroundColor Yellow
}

function Write-Err([string]$message) {
  Write-Host "!! $message" -ForegroundColor Red
}

function Assert-Command([string]$name, [string]$hint) {
  if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
    Write-Err "$name 未安装或不在 PATH。$hint"
    exit 1
  }
}

function Import-DotEnv([string]$path) {
  if (-not (Test-Path $path)) {
    return
  }
  Get-Content $path | ForEach-Object {
    $line = $_.Trim()
    if (-not $line -or $line.StartsWith("#")) {
      return
    }
    $pair = $line -split "=", 2
    if ($pair.Length -ne 2) {
      return
    }
    $name = $pair[0].Trim()
    $value = $pair[1].Trim().Trim('"')
    if ($name) {
      Set-Item -Path "env:$name" -Value $value
    }
  }
}

function Start-ServiceProcess([string]$name, [string]$command) {
  $escapedRoot = $root.Path.Replace("'", "''")
  $fullCommand = "Set-Location '$escapedRoot'; $command"
  Write-Step "启动 $name：$command"
  $proc = Start-Process -FilePath "powershell" -ArgumentList @("-NoExit", "-Command", $fullCommand) -PassThru
  Write-Host "   PID: $($proc.Id)"
  return [pscustomobject]@{
    name = $name
    pid = $proc.Id
    command = $command
  }
}

function Wait-ForPostgres([int]$retries = 30, [int]$delaySeconds = 2) {
  Write-Step "等待 Postgres 就绪"
  for ($attempt = 1; $attempt -le $retries; $attempt++) {
    docker exec erp-postgres pg_isready -U erp -d erp *> $null
    if ($LASTEXITCODE -eq 0) {
      return $true
    }
    Start-Sleep -Seconds $delaySeconds
  }
  return $false
}

try {
  Write-Step "检查依赖"
  Assert-Command "docker" "请安装 Docker Desktop 并确保已启动。"
  Assert-Command "cargo" "请安装 Rust stable。"
  Assert-Command "pnpm.cmd" "请运行：corepack enable"

  Write-Step "准备环境变量"
  $envPath = Join-Path $root ".env"
  if (-not (Test-Path $envPath)) {
    $examplePath = Join-Path $root "env/.env.example"
    if (Test-Path $examplePath) {
      Copy-Item $examplePath $envPath
      Write-Warn "已从 env/.env.example 生成 .env，请根据需要修改。"
    }
  }
  Import-DotEnv $envPath
  if (-not $env:DATABASE_URL) {
    $env:DATABASE_URL = "postgres://erp:erp@localhost:5432/erp"
  }
  if (-not $env:HTTP_PORT_API) {
    $env:HTTP_PORT_API = "8080"
  }
  if (-not $env:HTTP_PORT_CHANNEL) {
    $env:HTTP_PORT_CHANNEL = "8081"
  }
  if (-not $env:REDIS_URL) {
    $env:REDIS_URL = "redis://localhost:6379"
  }
  if (-not $env:NATS_URL) {
    $env:NATS_URL = "nats://localhost:4222"
  }
  $env:VITE_USE_MOCKS = "false"
  if (-not $env:VITE_API_BASE_URL) {
    $env:VITE_API_BASE_URL = "http://localhost:8080"
  }

  Write-Step "启动本地依赖（Docker）"
  if (-not (Test-Path $composeFile)) {
    Write-Err "未找到 docker-compose.local.yml"
    exit 1
  }
  docker compose -f $composeFile up -d

  Write-Step "执行数据库迁移（若已安装 sqlx）"
  if (Get-Command "sqlx" -ErrorAction SilentlyContinue) {
    if (Wait-ForPostgres) {
      sqlx migrate run
    } else {
      Write-Warn "Postgres 未就绪，跳过迁移步骤。"
    }
  } else {
    Write-Warn "未检测到 sqlx-cli，可执行：cargo install sqlx-cli --no-default-features --features rustls,postgres"
  }

  if (Test-Path $pidFile) {
    Write-Warn "检测到旧的进程记录，建议先运行 scripts/stop.ps1。"
  }

  $processes = @()
  $processes += Start-ServiceProcess "erp-api" "cargo run -p erp-api"
  $processes += Start-ServiceProcess "erp-worker" "cargo run -p erp-worker"
  $processes += Start-ServiceProcess "erp-channel" "cargo run -p erp-channel"
  $processes += Start-ServiceProcess "erp-web" "pnpm.cmd --filter erp-web dev"

  $processes | ConvertTo-Json | Set-Content $pidFile

  Write-Host ""
  Write-Step "启动完成"
  Write-Host "Web UI: http://localhost:5173"
  Write-Host "API health: http://localhost:8080/healthz"
  Write-Host "Channel health: http://localhost:8081/healthz"
  Write-Host "停止服务：.\scripts\stop.ps1"
} catch {
  Write-Err "启动失败：$($_.Exception.Message)"
  Write-Host "下一步建议：检查 Docker 是否运行、.env 配置、端口占用情况。"
  Write-Host "如需清理，请运行：.\scripts\stop.ps1"
  exit 1
}

