// Never delete the shared cache through a legacy command.
console.error('Automatic cleanup is disabled. Reuse scripts/Invoke-ErpBuild.ps1; cache cleanup requires explicit approval and path validation.');
process.exitCode = 1;
