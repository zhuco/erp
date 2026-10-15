#Requires -Version 7.0
param([ValidatePattern('^[a-z0-9-]+$')][string]$Stage='focused',[switch]$Full)
$ErrorActionPreference='Stop'
Set-Location -LiteralPath ([IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..')))
$taskPython='C:/Users/37768/AppData/Local/Programs/Python/Python312/python.exe'
$taskPrefix="qa/acceptance/free-erp-rework-cost-oct8-$Stage"
if(Test-Path -LiteralPath "$taskPrefix-status.json"){throw 'Evidence exists'}
$taskExit=$null;$taskVerify=$null
function Save-Run([string]$Step,[string]$State,[string]$Failure=''){
 $taskJson=[ordered]@{utc=[DateTime]::UtcNow.ToString('o');pid=$PID;stage=$Step;state=$State;test_exit=$taskExit;verify_exit=$taskVerify;failure=$Failure}|ConvertTo-Json -Compress
 [IO.File]::WriteAllText("$taskPrefix-status.json",$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
 [IO.File]::AppendAllText("$taskPrefix-events.jsonl",$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
try{
 Save-Run 'before' 'RUNNING'
 $env:ERP_GRID_TEST_DATABASE_URL='postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
 & $taskPython -X utf8 -W error qa/acceptance/free-erp-rework-cost-oct8.py capture "$Stage-before"
 if($LASTEXITCODE -ne 0){throw 'Before snapshot failed'}
 & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py "rework-cost-oct8-$Stage-before"
 if($LASTEXITCODE -ne 0){throw 'Before source failed'}
 Save-Run 'test' 'RUNNING'
 $taskArgs=@{Action='test';Scope='api';Platform='native';IntegrationTest='free_erp_acceptance'}
 if(-not $Full){$taskArgs.IntegrationFilter='source_dispositions'}
 try{& ./scripts/Invoke-ErpBuild.ps1 @taskArgs *> "$taskPrefix-test.log";$taskExit=$LASTEXITCODE}catch{$taskExit=1;$_|Out-String|Add-Content -LiteralPath "$taskPrefix-test.log"}
 Save-Run 'after' 'RUNNING'
 & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py "rework-cost-oct8-$Stage-after"
 if($LASTEXITCODE -ne 0){throw 'After source failed'}
 & $taskPython -X utf8 -W error qa/acceptance/free-erp-rework-cost-oct8.py capture "$Stage-after"
 if($LASTEXITCODE -ne 0){throw 'After snapshot failed'}
 Save-Run 'independent' 'RUNNING'
 & $taskPython -X utf8 -W error qa/acceptance/free-erp-rework-cost-oct8.py verify $Stage
 $taskVerify=$LASTEXITCODE
 Save-Run 'finished' $(if($taskExit -eq 0 -and $taskVerify -eq 0){'PASS'}else{'PARTIAL'})
 if($taskExit -ne 0 -or $taskVerify -ne 0){exit 1}
}catch{Save-Run 'failed' 'FAILED' $_.Exception.Message;exit 1}
