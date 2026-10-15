# Uses the same registered build entry, cache and lock as the acceptance run.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Set-Location -LiteralPath $taskRoot
$taskBase = Join-Path $taskRoot 'qa/acceptance'
$taskStatusPath = Join-Path $taskBase 'free-erp-session-pool-oct7-quality-status.json'
if (Test-Path -LiteralPath $taskStatusPath) { throw 'Refusing to replace quality evidence' }
$taskResults = [ordered]@{}
function Write-QualityStatus([string]$State,[string]$Stage) {
    $taskRecord = [ordered]@{utc=[DateTime]::UtcNow.ToString('o');pid=$PID;state=$State;stage=$Stage;results=$taskResults}
    [IO.File]::WriteAllText($taskStatusPath,($taskRecord|ConvertTo-Json -Depth 5)+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
try {
    Write-QualityStatus 'RUNNING' 'format'
    $taskLog = Join-Path $taskBase 'free-erp-session-pool-oct7-format-final.log'
    if (Test-Path -LiteralPath $taskLog) { throw 'Refusing to replace final format evidence' }
    try {
        & ./scripts/Invoke-ErpBuild.ps1 -Action fmt -Scope backend -Platform native -FormatFiles @(
            'crates/erp-repo/src/access_plan.rs','crates/erp-api/src/auth_handlers.rs',
            'crates/erp-api/src/export_job_views.rs','crates/erp-api/tests/free_erp_acceptance/account_login.rs',
            'crates/erp-api/tests/free_erp_acceptance/session_pool.rs','crates/erp-api/tests/free_erp_acceptance/login_sessions.rs'
        ) *> $taskLog
        $taskResults.format = 0
        Add-Content -LiteralPath $taskLog -Value 'ERP six current leaves format check exit_code=0' -Encoding utf8
    } catch { $taskResults.format=1; $_|Out-String|Add-Content -LiteralPath $taskLog -Encoding utf8 }
    foreach ($taskScope in @('repo','api')) {
        Write-QualityStatus 'RUNNING' "clippy-$taskScope"
        $taskLog = Join-Path $taskBase "free-erp-session-pool-oct7-$taskScope-clippy.log"
        if (Test-Path -LiteralPath $taskLog) { throw 'Refusing to replace strict Clippy evidence' }
        $taskExit = 0
        try {
            & ./scripts/Invoke-ErpBuild.ps1 -Action clippy -Scope $taskScope -Platform native *> $taskLog
            $taskExit = $LASTEXITCODE
        } catch { $taskExit=1; $_|Out-String|Add-Content -LiteralPath $taskLog -Encoding utf8 }
        Add-Content -LiteralPath $taskLog -Value "ERP strict $taskScope Clippy exit_code=$taskExit" -Encoding utf8
        $taskResults["clippy_$taskScope"] = $taskExit
    }
    Write-QualityStatus 'COMPLETED' 'finished'
} catch {
    $taskResults.failure = $_.Exception.Message
    Write-QualityStatus 'FAILED' 'failed'
    exit 1
}
