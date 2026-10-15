# Durable orchestration only. Every Rust action uses the registered build entry.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$taskRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$taskBase = Join-Path $taskRoot 'qa/acceptance'
$taskPrefix = 'free-erp-session-pool-oct7-detached'
$taskStatus = Join-Path $taskBase "$taskPrefix-status.json"
$taskEvents = Join-Path $taskBase "$taskPrefix-events.jsonl"
$taskLog = Join-Path $taskBase "$taskPrefix-test.log"
$taskPython = 'C:/Users/37768/AppData/Local/Programs/Python/Python312/python.exe'
if ((Test-Path -LiteralPath $taskStatus) -or (Test-Path -LiteralPath $taskLog)) {
    throw 'Refusing to replace durable regression evidence'
}
Set-Location -LiteralPath $taskRoot
$taskFunctionalExit = $null
$taskFactsExit = $null
function Write-TaskStage([string]$Stage, [string]$State, [string]$Failure = '') {
    $taskRecord = [ordered]@{
        utc = [DateTime]::UtcNow.ToString('o')
        pid = $PID
        stage = $Stage
        state = $State
        functional_exit = $taskFunctionalExit
        independent_exit = $taskFactsExit
        failure = $Failure
    }
    $taskJson = $taskRecord | ConvertTo-Json -Compress
    [IO.File]::WriteAllText($taskStatus, $taskJson + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))
    [IO.File]::AppendAllText($taskEvents, $taskJson + [Environment]::NewLine, [Text.UTF8Encoding]::new($false))
}
try {
    Write-TaskStage 'qa-identity' 'RUNNING'
    $taskIdentity = & 'G:/PostgreSQL/17/bin/psql.exe' -X -q -t -A -h 127.0.0.1 -p 15438 -U erp_grid_test -d erp_shared_contract_test -v ON_ERROR_STOP=1 -c 'select current_database(),current_user,inet_server_addr(),inet_server_port();'
    if ($LASTEXITCODE -ne 0 -or ($taskIdentity -join '').Trim() -ne 'erp_shared_contract_test|erp_grid_test|127.0.0.1|15438') {
        throw 'Protected QA database unavailable or identity differs'
    }
    $env:ERP_GRID_TEST_DATABASE_URL = 'postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
    Remove-Item Env:ERP_REAL_LOGIN_LEGACY_FIXTURE -ErrorAction SilentlyContinue
    Write-TaskStage 'before-snapshot' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-session-pool-capture.py oct7-detached-before
    if ($LASTEXITCODE -ne 0) { throw 'Before snapshot failed' }
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py session-pool-oct7-detached-before
    if ($LASTEXITCODE -ne 0) { throw 'Before source manifest failed' }
    Write-TaskStage 'registered-full-regression' 'RUNNING'
    try {
        & ./scripts/Invoke-ErpBuild.ps1 -Action test -Scope api -Platform native -IntegrationTest free_erp_acceptance *> $taskLog
        $taskFunctionalExit = $LASTEXITCODE
    } catch {
        $taskFunctionalExit = 1
        $_ | Out-String | Add-Content -LiteralPath $taskLog -Encoding utf8
    }
    Write-TaskStage 'after-snapshot' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py session-pool-oct7-detached-after
    if ($LASTEXITCODE -ne 0) { throw 'After source manifest failed' }
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-session-pool-capture.py oct7-detached-after
    if ($LASTEXITCODE -ne 0) { throw 'After snapshot failed' }
    $taskLogText = [IO.File]::ReadAllText($taskLog)
    $taskSummary = [regex]::Matches($taskLogText, 'test result: (?:ok|FAILED)\. \d+ passed; (\d+) failed; 0 ignored; 0 measured; 0 filtered out;')
    if ($taskSummary.Count -ne 1) { throw 'No unique complete registered test result; preserve interrupted/compile-failure evidence' }
    $taskFailed = [int]$taskSummary[0].Groups[1].Value
    Write-TaskStage 'independent-readback' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-session-pool-verify.py oct7-detached --mode repaired --before qa/acceptance/free-erp-session-pool-oct7-detached-before.jsonl --after qa/acceptance/free-erp-session-pool-oct7-detached-after.jsonl --log qa/acceptance/free-erp-session-pool-oct7-detached-test.log --source-before qa/acceptance/free-erp-inventory-consumers-session-pool-oct7-detached-before-source.json --source-after qa/acceptance/free-erp-inventory-consumers-session-pool-oct7-detached-after-source.json --full --expected-regression-failures $taskFailed
    $taskFactsExit = $LASTEXITCODE
    Write-TaskStage 'finished' 'COMPLETED'
    if ($taskFunctionalExit -ne 0 -or $taskFactsExit -ne 0) { exit 1 }
    exit 0
} catch {
    Write-TaskStage 'failed' 'FAILED' $_.Exception.Message
    exit 1
}
