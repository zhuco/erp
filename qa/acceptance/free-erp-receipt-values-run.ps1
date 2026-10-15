# Immutable logical stages; the registered entry retains its fixed cache and lock.
#Requires -Version 7.0
[CmdletBinding()]
param([ValidateSet('edges','edges-pwsh','full')][string]$Stage='edges-pwsh')
$ErrorActionPreference='Stop'
$taskRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Set-Location -LiteralPath $taskRoot
$taskBase=Join-Path $taskRoot 'qa/acceptance'
$taskPrefix="free-erp-receipt-values-$Stage"
$taskLog=Join-Path $taskBase "$taskPrefix-test.log"
$taskStatus=Join-Path $taskBase "$taskPrefix-status.json"
$taskEvents=Join-Path $taskBase "$taskPrefix-events.jsonl"
$taskPython='C:/Users/37768/AppData/Local/Programs/Python/Python312/python.exe'
if ((Test-Path -LiteralPath $taskLog) -or (Test-Path -LiteralPath $taskStatus)) {throw 'Refusing duplicate receipt value acceptance'}
$taskFunctionalExit=$null
$taskFactsExit=$null
function Write-ValueStage([string]$Step,[string]$State,[string]$Failure='') {
  $taskRecord=[ordered]@{utc=[DateTime]::UtcNow.ToString('o');pid=$PID;powershell=$PSVersionTable.PSVersion.ToString();stage=$Step;state=$State;functional_exit=$taskFunctionalExit;independent_exit=$taskFactsExit;failure=$Failure}
  $taskJson=$taskRecord|ConvertTo-Json -Compress
  [IO.File]::WriteAllText($taskStatus,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
  [IO.File]::AppendAllText($taskEvents,$taskJson+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
try {
  Write-ValueStage 'qa-identity' 'RUNNING'
  $taskIdentity=& 'G:/PostgreSQL/17/bin/psql.exe' -X -q -t -A -h 127.0.0.1 -p 15438 -U erp_grid_test -d erp_shared_contract_test -v ON_ERROR_STOP=1 -c 'select current_database(),current_user,inet_server_addr(),inet_server_port();'
  if ($LASTEXITCODE -ne 0 -or ($taskIdentity -join '').Trim() -ne 'erp_shared_contract_test|erp_grid_test|127.0.0.1|15438') {throw 'Protected receipt value QA identity unavailable'}
  $env:ERP_GRID_TEST_DATABASE_URL='postgresql://erp_grid_test@127.0.0.1:15438/erp_shared_contract_test'
  Remove-Item Env:ERP_REAL_LOGIN_LEGACY_FIXTURE -ErrorAction SilentlyContinue
  Write-ValueStage 'before-snapshot' 'RUNNING'
  & $taskPython -X utf8 -W error qa/acceptance/free-erp-receipt-values-capture.py snapshot "$Stage-before"
  if ($LASTEXITCODE -ne 0) {throw 'Receipt value before snapshot failed'}
  & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py "receipt-values-$Stage-before"
  if ($LASTEXITCODE -ne 0) {throw 'Receipt value before source failed'}
  Write-ValueStage 'registered-regression' 'RUNNING'
  $taskArgs=@{Action='test';Scope='api';Platform='native';IntegrationTest='free_erp_acceptance'}
  if ($Stage.StartsWith('edges')) {$taskArgs.IntegrationFilter='receipt_value'}
  try {& ./scripts/Invoke-ErpBuild.ps1 @taskArgs *> $taskLog;$taskFunctionalExit=$LASTEXITCODE}
  catch {$taskFunctionalExit=1;$_|Out-String|Add-Content -LiteralPath $taskLog -Encoding utf8}
  Write-ValueStage 'after-snapshot' 'RUNNING'
  & $taskPython -X utf8 -W error qa/acceptance/free-erp-inventory-consumers-source.py "receipt-values-$Stage-after"
  if ($LASTEXITCODE -ne 0) {throw 'Receipt value after source failed'}
  & $taskPython -X utf8 -W error qa/acceptance/free-erp-receipt-values-capture.py snapshot "$Stage-after"
  if ($LASTEXITCODE -ne 0) {throw 'Receipt value after snapshot failed'}
  Write-ValueStage 'independent-readback' 'RUNNING'
  $taskVerify=@($Stage,'--before',"qa/acceptance/free-erp-receipt-values-$Stage-before.jsonl",'--after',"qa/acceptance/free-erp-receipt-values-$Stage-after.jsonl",
    '--log',"qa/acceptance/$taskPrefix-test.log",'--source-before',"qa/acceptance/free-erp-inventory-consumers-receipt-values-$Stage-before-source.json",
    '--source-after',"qa/acceptance/free-erp-inventory-consumers-receipt-values-$Stage-after-source.json")
  if ($Stage -eq 'full') {$taskVerify+='--full'}
  & $taskPython -X utf8 -W error qa/acceptance/free-erp-receipt-values-verify.py @taskVerify
  $taskFactsExit=$LASTEXITCODE
  Write-ValueStage 'finished' 'COMPLETED'
  if ($taskFunctionalExit -ne 0 -or $taskFactsExit -ne 0) {exit 1}
} catch {Write-ValueStage 'failed' 'FAILED' $_.Exception.Message;exit 1}
