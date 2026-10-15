# Durable acceptance only; the registered build entry owns cache/lock/admission.
[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$taskRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Set-Location -LiteralPath $taskRoot
$taskBase=Join-Path $taskRoot 'qa/acceptance'
$taskPrefix='free-erp-trade-facts-full'
$taskLog=Join-Path $taskBase "$taskPrefix-test.log"
$taskStatus=Join-Path $taskBase "$taskPrefix-status.json"
$taskEvents=Join-Path $taskBase "$taskPrefix-events.jsonl"
$taskPython='C:/Users/37768/AppData/Local/Programs/Python/Python312/python.exe'
if ((Test-Path -LiteralPath $taskLog) -or (Test-Path -LiteralPath $taskStatus)) {throw 'Refusing duplicate full trade acceptance'}
$taskFunctionalExit=$null
$taskFactsExit=$null
function Write-TradeStage([string]$Stage,[string]$State,[string]$Failure='') {
    $taskRecord=[ordered]@{utc=[DateTime]::UtcNow.ToString('o');pid=$PID;stage=$Stage;state=$State;functional_exit=$taskFunctionalExit;independent_exit=$taskFactsExit;failure=$Failure}
    $taskJson=$taskRecord|ConvertTo-Json -Compress
    [IO.File]::WriteAllText($taskStatus,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
    [IO.File]::AppendAllText($taskEvents,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
try {
    Write-TradeStage 'qa-identity' 'RUNNING'
    $taskIdentity=& 'G:/PostgreSQL/17/bin/psql.exe' -X -q -t -A -h 127.0.0.1 -p 15438 -U erp_grid_test -d erp_shared_contract_test -v ON_ERROR_STOP=1 -c 'select current_database(),current_user,inet_server_addr(),inet_server_port();'
    if ($LASTEXITCODE -ne 0 -or ($taskIdentity -join '').Trim() -ne 'erp_shared_contract_test|erp_grid_test|127.0.0.1|15438') {throw 'Protected trade QA identity unavailable'}
    $env:ERP_GRID_TEST_DATABASE_URL='postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
    Remove-Item Env:ERP_REAL_LOGIN_LEGACY_FIXTURE -ErrorAction SilentlyContinue
    Write-TradeStage 'before-snapshot' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-trade-facts-capture.py snapshot full-before
    if ($LASTEXITCODE -ne 0) {throw 'Full before snapshot failed'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py trade-facts-full-before
    if ($LASTEXITCODE -ne 0) {throw 'Full before source failed'}
    Write-TradeStage 'registered-full-regression' 'RUNNING'
    try {
        & ./scripts/Invoke-ErpBuild.ps1 -Action test -Scope api -Platform native -IntegrationTest free_erp_acceptance *> $taskLog
        $taskFunctionalExit=$LASTEXITCODE
    } catch {$taskFunctionalExit=1;$_|Out-String|Add-Content -LiteralPath $taskLog -Encoding utf8}
    Write-TradeStage 'after-snapshot' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py trade-facts-full-after
    if ($LASTEXITCODE -ne 0) {throw 'Full after source failed'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-trade-facts-capture.py snapshot full-after
    if ($LASTEXITCODE -ne 0) {throw 'Full after snapshot failed'}
    $taskLogText=[IO.File]::ReadAllText($taskLog)
    $taskMatches=[regex]::Matches($taskLogText,'test result: ok\. \d+ passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;')
    if ($taskMatches.Count -ne 1) {throw 'No unique complete passing registered result; retain actual failure or interruption'}
    Write-TradeStage 'independent-readback' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-trade-facts-verify.py full --before qa/acceptance/free-erp-trade-facts-full-before.jsonl --after qa/acceptance/free-erp-trade-facts-full-after.jsonl --log qa/acceptance/free-erp-trade-facts-full-test.log --source-before qa/acceptance/free-erp-inventory-consumers-trade-facts-full-before-source.json --source-after qa/acceptance/free-erp-inventory-consumers-trade-facts-full-after-source.json --full
    $taskFactsExit=$LASTEXITCODE
    Write-TradeStage 'finished' 'COMPLETED'
    if ($taskFunctionalExit -ne 0 -or $taskFactsExit -ne 0) {exit 1}
} catch {Write-TradeStage 'failed' 'FAILED' $_.Exception.Message;exit 1}
