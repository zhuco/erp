#Requires -Version 7.0
[CmdletBinding()]
param()
$ErrorActionPreference='Stop'
$taskRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Set-Location -LiteralPath $taskRoot
$taskBase=Join-Path $taskRoot 'qa/acceptance'
$taskPrefix='free-erp-receipt-values-quality'
$taskStatus=Join-Path $taskBase "$taskPrefix-status.json"
$taskEvents=Join-Path $taskBase "$taskPrefix-events.jsonl"
if (Test-Path -LiteralPath $taskStatus) {throw 'Quality evidence exists'}
$taskResults=[ordered]@{}
function Save-Quality([string]$Step,[string]$State,[string]$Failure=''){
    $taskJson=[ordered]@{utc=[DateTime]::UtcNow.ToString('o');pid=$PID;stage=$Step;state=$State;results=$taskResults;failure=$Failure}|ConvertTo-Json -Depth 4 -Compress
    [IO.File]::WriteAllText($taskStatus,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
    [IO.File]::AppendAllText($taskEvents,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
try{
    foreach($taskStep in @('format','repo-clippy','api-clippy')){
        Save-Quality $taskStep 'RUNNING'
        $taskLog=Join-Path $taskBase "$taskPrefix-$taskStep.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Quality log exists'}
        $taskExit=0
        try{
            if($taskStep -eq 'format'){
                & ./scripts/Invoke-ErpBuild.ps1 -Action fmt -Scope backend -Platform native -FormatFiles @(
                    'crates/erp-repo/src/shared_contract/receipt_values.rs','crates/erp-repo/src/shared_contract/receipt_offsets.rs',
                    'crates/erp-api/src/procurement_handlers.rs','crates/erp-api/tests/free_erp_acceptance/receipt_values.rs',
                    'crates/erp-api/tests/free_erp_acceptance/receipt_value_edges.rs','crates/erp-api/tests/free_erp_acceptance/trade_facts.rs') *> $taskLog
            }else{
                $taskScope=if($taskStep -eq 'repo-clippy'){'repo'}else{'api'}
                & ./scripts/Invoke-ErpBuild.ps1 -Action clippy -Scope $taskScope -Platform native *> $taskLog
            }
            $taskExit=$LASTEXITCODE
        }catch{$taskExit=1;$_|Out-String|Add-Content -LiteralPath $taskLog -Encoding utf8}
        $taskResults[$taskStep]=$taskExit
    }
    Save-Quality 'finished' 'COMPLETED'
}catch{Save-Quality 'failed' 'FAILED' $_.Exception.Message;exit 1}
