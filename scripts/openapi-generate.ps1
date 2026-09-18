$ErrorActionPreference = 'Stop'

$root = Resolve-Path (Join-Path $PSScriptRoot '..')
pnpm.cmd -C $root run openapi:bundle
if ($LASTEXITCODE -ne 0) {
  throw "OpenAPI bundle failed with exit code $LASTEXITCODE"
}
