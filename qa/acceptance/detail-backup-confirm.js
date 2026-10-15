async page => {
 const readback=await page.evaluate(async () => {
  const source=await (await fetch('/pkg/erp_ui.js')).text();
  const spec=source.match(/["'](\.\/snippets\/[^"']+\/src\/workbench\/detail_db\.js)["']/)?.[1];
  if(!spec) throw new Error('Missing adapter');
  const adapter=await import('/pkg/'+spec.slice(2));
  const read=async key=>{const [revision,bytes]=await adapter.loadFormCheckpoint(key);return {revision,checkpoint:bytes?JSON.parse(new TextDecoder().decode(bytes)):null};};
  return {primary:await read('local-order-module'),backup:await read(JSON.stringify(['detail-local-backup-v1','local-order-module']))};
 });
 if(JSON.stringify(readback.primary)!==JSON.stringify(page.finalBackupProof.primaryBefore))throw new Error('Primary changed');
 if(readback.backup.revision!==1||readback.backup.checkpoint.changes['2']!=='当前副本客户')throw new Error('Backup not durable');
 if(readback.backup.checkpoint.pending.operation_id===readback.primary.checkpoint.pending.operation_id)throw new Error('Identities merged');
 page.finalBackupProof.afterSave=readback;
 await page.waitForTimeout(200);await page.screenshot({path:'G:/erp/output/playwright/detail-backup/confirmed.png'});
 return readback;
}