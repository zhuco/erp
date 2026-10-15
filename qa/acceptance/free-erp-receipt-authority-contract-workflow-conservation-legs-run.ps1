#Requires -Version 7.0
[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
Set-Location -LiteralPath ([IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..')))
$taskPrefix='qa/acceptance/free-erp-receipt-authority-contract-workflow-conservation-legs'
$taskStatus="$taskPrefix-status.json"
$taskEvents="$taskPrefix-events.jsonl"
$taskResults=[ordered]@{}
$taskPython='C:/Users/37768/AppData/Local/Programs/Python/Python312/python.exe'
if(Test-Path -LiteralPath $taskStatus){throw 'Contract evidence exists'}
function Save-Contract([string]$Step,[string]$State,[string]$Failure=''){
    $taskJson=[ordered]@{utc=[DateTime]::UtcNow.ToString('o');pid=$PID;stage=$Step;state=$State;results=$taskResults;failure=$Failure}|ConvertTo-Json -Depth 4 -Compress
    [IO.File]::WriteAllText($taskStatus,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
    [IO.File]::AppendAllText($taskEvents,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
try{
    $taskIdentity=& 'G:/PostgreSQL/17/bin/psql.exe' -X -q -t -A -h 127.0.0.1 -p 15438 -U erp_grid_test -d erp_shared_contract_test -v ON_ERROR_STOP=1 -c 'select current_database(),current_user,inet_server_addr(),inet_server_port();'
    if($LASTEXITCODE -ne 0 -or ($taskIdentity -join '').Trim() -ne 'erp_shared_contract_test|erp_grid_test|127.0.0.1|15438'){throw 'Protected contract QA identity failed'}
    $env:ERP_GRID_TEST_DATABASE_URL='postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
    Save-Contract 'before-capture' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-receipt-authority-capture.py snapshot contract-workflow-conservation-legs-before
    if($LASTEXITCODE -ne 0){throw 'Contract before capture failed'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py receipt-authority-contract-workflow-conservation-legs-before
    if($LASTEXITCODE -ne 0){throw 'Contract before source failed'}
    foreach($taskStep in @('direct-purchase')){

        Save-Contract $taskStep 'RUNNING'
        $taskLog="$taskPrefix-$taskStep.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Contract log exists'}
        $taskExit=0
        try{
            if($taskStep -eq 'direct-purchase'){
                & ./scripts/Invoke-ErpBuild.ps1 -Action test -Scope api -Platform native -IntegrationTest shared_contract -IntegrationFilter receipt_retry_and_mrp_snapshot_read_back *> $taskLog
            }elseif($taskStep -eq 'format'){
                & ./scripts/Invoke-ErpBuild.ps1 -Action fmt -Scope backend -Platform native -FormatFiles @(
                  'crates/erp-repo/src/shared_contract/receipt_authority.rs','crates/erp-repo/src/shared_contract/receipt_values.rs',
                  'crates/erp-api/src/auth_context.rs','crates/erp-api/src/auth_layers.rs','crates/erp-api/src/procurement_handlers.rs',
                  'crates/erp-api/tests/free_erp_acceptance/receipt_authority.rs','crates/erp-api/tests/shared_contract_receipt_actor.rs') *> $taskLog
            }else{
                $taskScope=if($taskStep -eq 'repo-clippy'){'repo'}else{'api'}
                & ./scripts/Invoke-ErpBuild.ps1 -Action clippy -Scope $taskScope -Platform native *> $taskLog
            }
            $taskExit=$LASTEXITCODE
        }catch{$taskExit=1;$_|Out-String|Add-Content -LiteralPath $taskLog -Encoding utf8}
        $taskResults[$taskStep]=$taskExit
    }
    Save-Contract 'after-capture' 'RUNNING'
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py receipt-authority-contract-workflow-conservation-legs-after
    if($LASTEXITCODE -ne 0){throw 'Contract after source failed'}
    & $taskPython -X utf8 -W error qa/acceptance/free-erp-receipt-authority-capture.py snapshot contract-workflow-conservation-legs-after
    if($LASTEXITCODE -ne 0){throw 'Contract after capture failed'}
    Save-Contract 'finished' 'COMPLETED'
}catch{Save-Contract 'failed' 'FAILED' $_.Exception.Message;exit 1}
