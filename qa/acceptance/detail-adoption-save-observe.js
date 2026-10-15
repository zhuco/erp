async page => {
 await page.mouse.click(1030,397);
 const pending=await page.evaluate(async()=>{
  const source=await(await fetch('/pkg/erp_ui.js')).text();
  const spec=source.match(/["'](\.\/snippets\/[^"']+\/src\/workbench\/detail_db\.js)["']/)[1];
  const adapter=await import('/pkg/'+spec.slice(2));
  for(let i=0;i<200;i++) {
   const [revision,bytes]=await adapter.loadFormCheckpoint('local-stock-module');
   if(revision===1&&bytes)return {revision,checkpoint:JSON.parse(new TextDecoder().decode(bytes))};
   if(revision>1)throw new Error('Pending capture missed; do not restart or overwrite');
   await new Promise(r=>setTimeout(r,10));
  }
  throw new Error('No durable new primary request');
 });
 const request=pending.checkpoint.pending;
 if(!request||request.scope!=='local-stock-module'||request.record_id!=='1'||request.observed_version!=='0'||request.before['2']!=='示例货品 001'||request.changes['2']!=='复用副本第二份')throw new Error('New request not based on current binding');
 const originals=page.finalAdoptionProof.before['local-order-module'];
 if([originals.primary.checkpoint.pending.operation_id,originals.backup.checkpoint.pending.operation_id].includes(request.operation_id))throw new Error('Old identity reused');
 page.finalAdoptionProof.newPrimaryPending=pending;
 await page.waitForTimeout(1200);
 await page.screenshot({path:'G:/erp/output/playwright/detail-adoption/saved-main.png'});
 return pending;
}