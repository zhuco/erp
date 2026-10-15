#Requires -Version 7.0
[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
Set-Location -LiteralPath ([IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..')))
$taskPrefix='qa/acceptance/free-erp-receipt-authority-original'
$taskPython='C:/Users/37768/AppData/Local/Programs/Python/Python312/python.exe'
$taskState="$taskPrefix-status.json"
$taskEvents="$taskPrefix-events.jsonl"
if(Test-Path -LiteralPath $taskState){throw 'Original authority evidence exists'}
$taskExit=$null
function Save-Stage([string]$Stage,[string]$State,[string]$ErrorText=''){
    $taskJson=[ordered]@{utc=[DateTime]::UtcNow.ToString('o');pid=$PID;stage=$Stage;state=$State;functional_exit=$taskExit;error=$ErrorText}|ConvertTo-Json -Compress
    [IO.File]::WriteAllText($taskState,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
    [IO.File]::AppendAllText($taskEvents,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
try{
    Save-Stage 'identity' 'RUNNING'
    $taskIdentity=& 'G:/PostgreSQL/17/bin/psql.exe' -X -q -t -A -h 127.0.0.1 -p 15438 -U erp_grid_test -d erp_shared_contract_test -v ON_ERROR_STOP=1 -c 'select current_database(),current_user,inet_server_addr(),inet_server_port();'
    if($LASTEXITCODE -ne 0 -or ($taskIdentity -join '').Trim() -ne 'erp_shared_contract_test|erp_grid_test|127.0.0.1|15438'){throw 'Protected QA identity failed'}
    $env:ERP_GRID_TEST_DATABASE_URL='postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
    Save-Stage 'before' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-receipt-values-capture.py snapshot authority-original-before
    if($LASTEXITCODE -ne 0){throw 'Original before capture failed'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py receipt-authority-original-before
    if($LASTEXITCODE -ne 0){throw 'Original before source failed'}
    Save-Stage 'original-counterexamples' 'RUNNING'
    try{
        & ./scripts/Invoke-ErpBuild.ps1 -Action test -Scope api -Platform native -IntegrationTest free_erp_acceptance -IntegrationFilter receipt_authority *> "$taskPrefix-test.log"
        $taskExit=$LASTEXITCODE
    }catch{$taskExit=1;$_|Out-String|Add-Content -LiteralPath "$taskPrefix-test.log" -Encoding utf8}
    Save-Stage 'after' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py receipt-authority-original-after
    if($LASTEXITCODE -ne 0){throw 'Original after source failed'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-receipt-values-capture.py snapshot authority-original-after
    if($LASTEXITCODE -ne 0){throw 'Original after capture failed'}
    Save-Stage 'finished' 'COMPLETED'
}catch{Save-Stage 'failed' 'FAILED' $_.Exception.Message;exit 1}
