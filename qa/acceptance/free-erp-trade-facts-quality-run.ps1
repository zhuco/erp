# Scope-local formatting and strict quality checks through the one build entry.
[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$taskRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Set-Location -LiteralPath $taskRoot
$taskBase=Join-Path $taskRoot 'qa/acceptance'
$taskStatus=Join-Path $taskBase 'free-erp-trade-facts-quality-status.json'
if (Test-Path -LiteralPath $taskStatus) {throw 'Refusing duplicate trade quality run'}
$taskResults=[ordered]@{}
function Write-TradeQuality([string]$State,[string]$Stage) {
    $taskRecord=[ordered]@{utc=[DateTime]::UtcNow.ToString('o');pid=$PID;state=$State;stage=$Stage;results=$taskResults}
    [IO.File]::WriteAllText($taskStatus,($taskRecord|ConvertTo-Json -Depth 5)+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
try {
    Write-TradeQuality 'RUNNING' 'format'
    $taskLog=Join-Path $taskBase 'free-erp-trade-facts-format-final.log'
    if (Test-Path -LiteralPath $taskLog) {throw 'Refusing to replace format evidence'}
    try {
        & ./scripts/Invoke-ErpBuild.ps1 -Action fmt -Scope backend -Platform native -FormatFiles @(
            'crates/erp-domain/src/trade.rs','crates/erp-repo/src/shared_contract/receipt_offsets.rs',
            'crates/erp-api/src/onboarding.rs','crates/erp-api/tests/free_erp_acceptance/trade_facts.rs',
            'crates/erp-api/tests/free_erp_acceptance/login_sessions.rs'
        ) *> $taskLog
        $taskResults.format=0
        Add-Content -LiteralPath $taskLog -Value 'ERP five changed trade leaves format exit_code=0' -Encoding utf8
    } catch {$taskResults.format=1;$_|Out-String|Add-Content -LiteralPath $taskLog -Encoding utf8}
    foreach ($taskScope in @('domain','repo','api')) {
        Write-TradeQuality 'RUNNING' "clippy-$taskScope"
        $taskLog=Join-Path $taskBase "free-erp-trade-facts-current-$taskScope-clippy.log"
        if (Test-Path -LiteralPath $taskLog) {throw 'Refusing to replace strict quality evidence'}
        $taskExit=0
        try {& ./scripts/Invoke-ErpBuild.ps1 -Action clippy -Scope $taskScope -Platform native *> $taskLog;$taskExit=$LASTEXITCODE}
        catch {$taskExit=1;$_|Out-String|Add-Content -LiteralPath $taskLog -Encoding utf8}
        Add-Content -LiteralPath $taskLog -Value "ERP strict $taskScope Clippy exit_code=$taskExit" -Encoding utf8
        $taskResults["clippy_$taskScope"]=$taskExit
    }
    Write-TradeQuality 'COMPLETED' 'finished'
} catch {$taskResults.failure=$_.Exception.Message;Write-TradeQuality 'FAILED' 'failed';exit 1}
