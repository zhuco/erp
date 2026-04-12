$ErrorActionPreference = 'Stop'

$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$spec = Join-Path $root 'openapi/erp_api_v1.yaml'
$output = Join-Path $root 'apps/erp-web/src/api/generated/schema.ts'

pnpm.cmd --filter erp-web exec openapi-typescript $spec -o $output
