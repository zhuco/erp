async page => {
 await page.mouse.click(1170,181);
 await page.keyboard.press('Control+a');await page.keyboard.insertText('当前副本客户');
 await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
 page.finalBackupProof.primaryBefore=await page.evaluate(async()=>{
 const source=await(await fetch('/pkg/erp_ui.js')).text();
 const spec=source.match(/["'](\.\/snippets\/[^"']+\/src\/workbench\/detail_db\.js)["']/)[1];
 const adapter=await import('/pkg/'+spec.slice(2));
 const [revision,bytes]=await adapter.loadFormCheckpoint('local-order-module');
 if(revision!==0||bytes!==null)throw new Error('Refuse occupied primary');
 const backup=await adapter.loadFormCheckpoint(JSON.stringify(['detail-local-backup-v1','local-order-module']));
 if(backup[0]!==0||backup[1]!==null)throw new Error('Refuse occupied backup');
 const request={operation_id:crypto.randomUUID(),scope:'local-order-module',record_id:'1',observed_version:'0',before:{'2':'示例客户 001'},changes:{'2':'存储未决客户'}};
 const checkpoint={schema:1,scope:request.scope,record:'1',version:'0',before:request.before,changes:request.changes,pending:request,conflict:false};
 await adapter.commitFormCheckpoint(request.scope,0,new TextEncoder().encode(JSON.stringify(checkpoint)));
 const [afterRevision,afterBytes]=await adapter.loadFormCheckpoint(request.scope);
 return {revision:afterRevision,checkpoint:JSON.parse(new TextDecoder().decode(afterBytes))};
 });
 await page.mouse.click(1030,397);
 await page.waitForTimeout(300);
 await page.screenshot({path:'G:/erp/output/playwright/detail-backup/conflict.png'});
 return page.finalBackupProof.primaryBefore;
}