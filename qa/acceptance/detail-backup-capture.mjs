import fs from 'node:fs';
import path from 'node:path';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
const require=createRequire(import.meta.url);const cli=path.join(path.dirname(require.resolve('@playwright/cli/package.json')),'playwright-cli.js');
const out=execFileSync(process.execPath,[cli,'-s=erp-detail-backup','run-code','async page => {return page.finalBackupProof;}'],{cwd:process.cwd(),encoding:'utf8',windowsHide:true,timeout:30000});
if(/^### Error/m.test(out))throw new Error(out);const line=out.split(/\r?\n/).find(line=>line.startsWith('{'));if(!line)throw new Error('No browser proof');
fs.writeFileSync('qa/acceptance/free-erp-detail-backup-oct10-browser.json',JSON.stringify(JSON.parse(line),null,2)+'\n',{flag:'wx'});console.log('Actual conflict readback proof captured');
