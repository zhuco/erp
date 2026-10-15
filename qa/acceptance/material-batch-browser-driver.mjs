import fs from 'node:fs';
import path from 'node:path';
import {createRequire} from 'node:module';
import {execFileSync} from 'node:child_process';
const root=process.cwd(),require=createRequire(import.meta.url);
const cli=path.join(path.dirname(require.resolve('@playwright/cli/package.json')),'playwright-cli.js');
const session='erp-material-ui';
const fixture=JSON.parse(fs.readFileSync('qa/acceptance/free-erp-material-batch-web-oct10-fixture.json','utf8'));
function command(...args){const output=execFileSync(process.execPath,[cli,`-s=${session}`,...args],{cwd:root,encoding:'utf8',timeout:45000,windowsHide:true,maxBuffer:4*1024*1024});if(/^### Error/m.test(output))throw Error(output);return output;}
function run(code){return command('run-code',`async page => { const frame = () => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))); ${code} }`);}
const mode=process.argv[2];
if(mode==='login'){
 const fields=[[165,86,fixture.api],[460,86,fixture.tenant_id],[180,114,fixture.email],[470,114,'ERP-owned-QA-material-browser-2026!']];
 // Coordinates come from the inspected 1600x1000 canvas screenshot.
 console.log(run(`for(const [x,y,value] of ${JSON.stringify(fields)}){await page.mouse.click(x,y);await page.keyboard.press('Control+A');await page.keyboard.insertText(value);await frame();} await page.mouse.click(670,114);await frame();`));
}
if(mode==='screenshot')console.log(command('screenshot','--filename=output/playwright/material-documents/batch-logged-in.png'));
