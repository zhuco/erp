async page => {
 return await page.evaluate(async () => {
  const source=await (await fetch('/pkg/erp_ui.js')).text();
  const spec=source.match(/["'](\.\/snippets\/[^"']+\/src\/workbench\/detail_db\.js)["']/)?.[1];
  if(!spec) throw new Error('Missing adapter');
  const adapter=await import('/pkg/'+spec.slice(2));
  const read=async key=>{const [revision,bytes]=await adapter.loadFormCheckpoint(key);return {revision,checkpoint:bytes?JSON.parse(new TextDecoder().decode(bytes)):null};};
  return {primary:await read('local-order-module'),backup:await read(JSON.stringify(['detail-local-backup-v1','local-order-module']))};
 });
}