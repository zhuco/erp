$ErrorActionPreference = "Continue"
Set-StrictMode -Version Latest

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$pidFile = Join-Path $PSScriptRoot ".dev-processes.json"
$composeFile = Join-Path $root "docker-compose.local.yml"

function Get-ChildProcessIds([int]$parentId) {
  $children = Get-CimInstance Win32_Process -Filter "ParentProcessId = $parentId" -ErrorAction SilentlyContinue
  if (-not $children) {
    return @()
  }
  $ids = @()
  foreach ($child in $children) {
    $ids += $child.ProcessId
    $ids += Get-ChildProcessIds $child.ProcessId
  }
  return $ids
}

function Stop-ProcessTree([int]$processId) {
  $childIds = Get-ChildProcessIds $processId
  foreach ($childId in $childIds | Select-Object -Unique) {
    Stop-Process -Id $childId -Force -ErrorAction SilentlyContinue
  }
  Stop-Process -Id $processId -Force -ErrorAction SilentlyContinue
}

function Stop-PortProcesses([int[]]$ports) {
  foreach ($port in $ports) {
    try {
      $processIds = Get-NetTCPConnection -LocalPort $port -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty OwningProcess -Unique
      foreach ($processId in $processIds) {
        if (-not $processId -or $processId -le 0) {
          continue
        }
        Write-Host "清理占用端口 $port 的进程 PID: $processId"
        Stop-ProcessTree $processId
      }
    } catch {
      Write-Warn "查询端口 $port 失败：$($_.Exception.Message)"
    }
  }
}

function Write-Step([string]$message) {
  Write-Host "==> $message" -ForegroundColor Cyan
}

function Write-Warn([string]$message) {
  Write-Host "!! $message" -ForegroundColor Yellow
}

function Write-Err([string]$message) {
  Write-Host "!! $message" -ForegroundColor Red
}

Write-Step "停止本地进程"
if (Test-Path $pidFile) {
  try {
    $items = Get-Content $pidFile -Raw | ConvertFrom-Json
    if ($items -and -not ($items -is [System.Array])) {
      $items = @($items)
    }
    foreach ($item in $items) {
      $processId = [int]$item.pid
      $proc = Get-Process -Id $processId -ErrorAction SilentlyContinue
      if ($proc) {
        Write-Host "终止 $($item.name) (PID: $processId)"
        Stop-ProcessTree $processId
      }
    }
    Remove-Item $pidFile -Force
  } catch {
    Write-Warn "读取进程记录失败，请手动关闭终端窗口。"
  }
} else {
  Write-Warn "未找到进程记录文件。"
}

Write-Step "清理常用端口占用"
Stop-PortProcesses @(5173, 8080, 8081, 9090)

Write-Step "停止 Docker 服务"
if (Test-Path $composeFile) {
  try {
    docker compose -f $composeFile down
  } catch {
    Write-Warn "docker compose down 执行失败，请确认 Docker Desktop 是否运行。"
  }
} else {
  Write-Warn "未找到 docker-compose.local.yml"
}

Write-Step "完成"

