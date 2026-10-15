#Requires -Version 7.0
[CmdletBinding()]
param(
    [ValidatePattern('^[a-z0-9-]+$')][string]$Stage='reported-scrap',
    [ValidateSet('shared_contract','free_erp_acceptance')][string]$IntegrationTarget='shared_contract',
    [ValidatePattern('^[A-Za-z0-9_:]+$')][string]$IntegrationFilter='receipt_retry_and_mrp_snapshot_read_back',
    [switch]$Full
)
$ErrorActionPreference='Stop'
Set-Location -LiteralPath ([IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..')))
$taskPrefix="qa/acceptance/free-erp-workflow-oct8-$Stage"
$taskPython='C:/Users/37768/AppData/Local/Programs/Python/Python312/python.exe'
$taskExit=$null
$taskVerifyExit=$null
if(Test-Path -LiteralPath "$taskPrefix-status.json"){throw 'Workflow evidence exists'}
function Save-Workflow([string]$Step,[string]$State,[string]$Failure=''){
    $taskJson=[ordered]@{utc=[DateTime]::UtcNow.ToString('o');pid=$PID;stage=$Step;state=$State;functional_exit=$taskExit;independent_exit=$taskVerifyExit;failure=$Failure}|ConvertTo-Json -Compress
    [IO.File]::WriteAllText("$taskPrefix-status.json",$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
    [IO.File]::AppendAllText("$taskPrefix-events.jsonl",$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
try{
    Save-Workflow 'identity' 'RUNNING'
    $taskIdentity=& 'G:/PostgreSQL/17/bin/psql.exe' -X -q -t -A -h 127.0.0.1 -p 15438 -U erp_grid_test -d erp_shared_contract_test -v ON_ERROR_STOP=1 -c 'select current_database(),current_user,inet_server_addr(),inet_server_port();'
    if($LASTEXITCODE -ne 0 -or ($taskIdentity -join '').Trim() -ne 'erp_shared_contract_test|erp_grid_test|127.0.0.1|15438'){throw 'Protected QA identity failed'}
    $env:ERP_GRID_TEST_DATABASE_URL='postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
    Save-Workflow 'before' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-workflow-oct8.py capture "$Stage-before"
    if($LASTEXITCODE -ne 0){throw 'Before snapshot failed'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py "workflow-oct8-$Stage-before"
    if($LASTEXITCODE -ne 0){throw 'Before source failed'}
    Save-Workflow 'registered-test' 'RUNNING'
    try{
        $taskArguments=@{Action='test';Scope='api';Platform='native';IntegrationTest=$IntegrationTarget}
        if(-not $Full){$taskArguments.IntegrationFilter=$IntegrationFilter}
        & ./scripts/Invoke-ErpBuild.ps1 @taskArguments *> "$taskPrefix-test.log"
        $taskExit=$LASTEXITCODE
    }catch{$taskExit=1;$_|Out-String|Add-Content -LiteralPath "$taskPrefix-test.log" -Encoding utf8}
    Save-Workflow 'after' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py "workflow-oct8-$Stage-after"
    if($LASTEXITCODE -ne 0){throw 'After source failed'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-workflow-oct8.py capture "$Stage-after"
    if($LASTEXITCODE -ne 0){throw 'After snapshot failed'}
    Save-Workflow 'independent' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-workflow-oct8.py verify $Stage
    $taskVerifyExit=$LASTEXITCODE
    $taskState=if($taskExit -eq 0 -and $taskVerifyExit -eq 0){'PASS'}else{'PARTIAL'}
    Save-Workflow 'finished' $taskState
    if($taskExit -ne 0 -or $taskVerifyExit -ne 0){exit 1}
}catch{Save-Workflow 'failed' 'FAILED' $_.Exception.Message;exit 1}
