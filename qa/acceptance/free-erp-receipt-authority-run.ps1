#Requires -Version 7.0
[CmdletBinding()]
param([ValidateSet('first','full')][string]$Stage='first')
$ErrorActionPreference='Stop'
Set-Location -LiteralPath ([IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..')))
$taskPrefix="qa/acceptance/free-erp-receipt-authority-$Stage"
$taskPython='C:/Users/37768/AppData/Local/Programs/Python/Python312/python.exe'
$taskState="$taskPrefix-status.json"
$taskEvents="$taskPrefix-events.jsonl"
if(Test-Path -LiteralPath $taskState){throw 'Authority evidence exists'}
$taskExit=$null
$taskVerifyExit=$null
function Save-Stage([string]$Step,[string]$State,[string]$ErrorText=''){
    $taskJson=[ordered]@{utc=[DateTime]::UtcNow.ToString('o');pid=$PID;stage=$Step;state=$State;functional_exit=$taskExit;independent_exit=$taskVerifyExit;error=$ErrorText}|ConvertTo-Json -Compress
    [IO.File]::WriteAllText($taskState,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
    [IO.File]::AppendAllText($taskEvents,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
try{
    Save-Stage 'identity' 'RUNNING'
    $taskIdentity=& 'G:/PostgreSQL/17/bin/psql.exe' -X -q -t -A -h 127.0.0.1 -p 15438 -U erp_grid_test -d erp_shared_contract_test -v ON_ERROR_STOP=1 -c 'select current_database(),current_user,inet_server_addr(),inet_server_port();'
    if($LASTEXITCODE -ne 0 -or ($taskIdentity -join '').Trim() -ne 'erp_shared_contract_test|erp_grid_test|127.0.0.1|15438'){throw 'Protected QA identity failed'}
    $env:ERP_GRID_TEST_DATABASE_URL='postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
    Save-Stage 'before' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-receipt-authority-capture.py snapshot "$Stage-before"
    if($LASTEXITCODE -ne 0){throw 'Authority before capture failed'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py "receipt-authority-$Stage-before"
    if($LASTEXITCODE -ne 0){throw 'Authority before source failed'}
    Save-Stage 'registered-tests' 'RUNNING'
    $taskArgs=@{Action='test';Scope='api';Platform='native';IntegrationTest='free_erp_acceptance'}
    if($Stage -eq 'first'){$taskArgs.IntegrationFilter='receipt_authority'}
    try{& ./scripts/Invoke-ErpBuild.ps1 @taskArgs *> "$taskPrefix-test.log";$taskExit=$LASTEXITCODE}
    catch{$taskExit=1;$_|Out-String|Add-Content -LiteralPath "$taskPrefix-test.log" -Encoding utf8}
    Save-Stage 'after' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py "receipt-authority-$Stage-after"
    if($LASTEXITCODE -ne 0){throw 'Authority after source failed'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-receipt-authority-capture.py snapshot "$Stage-after"
    if($LASTEXITCODE -ne 0){throw 'Authority after capture failed'}
    Save-Stage 'independent-readback' 'RUNNING'
    $taskVerify=@($Stage,'--before',"qa/acceptance/free-erp-receipt-authority-$Stage-before.jsonl",'--after',"qa/acceptance/free-erp-receipt-authority-$Stage-after.jsonl",
      '--log',"$taskPrefix-test.log",'--source-before',"qa/acceptance/free-erp-inventory-consumers-receipt-authority-$Stage-before-source.json",
      '--source-after',"qa/acceptance/free-erp-inventory-consumers-receipt-authority-$Stage-after-source.json")
    if($Stage -eq 'full'){$taskVerify+='--full'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-receipt-authority-verify.py @taskVerify
    $taskVerifyExit=$LASTEXITCODE
    Save-Stage 'finished' 'COMPLETED'
    if($taskExit -ne 0 -or $taskVerifyExit -ne 0){exit 1}
}catch{Save-Stage 'failed' 'FAILED' $_.Exception.Message;exit 1}
