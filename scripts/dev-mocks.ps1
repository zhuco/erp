$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$pidFile = Join-Path $PSScriptRoot ".dev-processes.json"

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
    Write-Err "$name not found. $hint"
    exit 1
  }
}

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
        Write-Warn "Port $port is in use, stopping PID: $processId"
        Stop-ProcessTree $processId
      }
    } catch {
      Write-Warn "Failed to query port ${port}. Error: $($_.Exception.Message)"
    }
  }
}

function Start-ServiceProcess([string]$name, [string]$command) {
  $escapedRoot = $root.Path.Replace("'", "''")
  $fullCommand = "Set-Location '$escapedRoot'; $command"
  Write-Step "Start ${name}: $command"
  $proc = Start-Process -FilePath "powershell" -ArgumentList @("-NoExit", "-Command", $fullCommand) -PassThru
  Write-Host "   PID: $($proc.Id)"
  return [pscustomobject]@{
    name = $name
    pid = $proc.Id
    command = $command
  }
}

try {
  Write-Step "Check dependencies"
  Assert-Command "pnpm.cmd" "Run: corepack enable"

  $env:VITE_USE_MOCKS = "true"
  $env:VITE_API_BASE_URL = "http://localhost:8080"

  if (Test-Path $pidFile) {
    Write-Warn "Existing process file found, cleaning up."
    try {
      $items = Get-Content $pidFile -Raw | ConvertFrom-Json
      if ($items -and -not ($items -is [System.Array])) {
        $items = @($items)
      }
      foreach ($item in $items) {
        $processId = [int]$item.pid
        $proc = Get-Process -Id $processId -ErrorAction SilentlyContinue
        if ($proc) {
          Write-Host "Stopping $($item.name) (PID: $processId)"
          Stop-ProcessTree $processId
        }
      }
      Remove-Item $pidFile -Force -ErrorAction SilentlyContinue
    } catch {
      Write-Warn "Failed to read process file. Close old terminals manually."
    }
  }

  Write-Step "Free port 5173"
  Stop-PortProcesses @(5173)

  $processes = @()
  $processes += Start-ServiceProcess "erp-web" "pnpm.cmd --filter erp-web dev"
  $processes | ConvertTo-Json | Set-Content $pidFile

  Write-Host ""
  Write-Step "Ready"
  Write-Host "Web UI: http://localhost:5173"
  Write-Host "Stop: .\\scripts\\stop.ps1"
} catch {
  Write-Err "Failed: $($_.Exception.Message)"
  Write-Host "Check pnpm / Node availability."
  exit 1
}
