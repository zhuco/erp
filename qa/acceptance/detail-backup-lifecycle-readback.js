async page => {
 const records=await page.evaluate(async()=>{
  const source=await(await fetch('/pkg/erp_ui.js')).text();
  const spec=source.match(/["'](\.\/snippets\/[^"']+\/src\/workbench\/detail_db\.js)["']/)?.[1];
  if(!spec)throw new Error('Missing form adapter');
  const adapter=await import('/pkg/'+spec.slice(2));
  const read=async key=>{const [revision,bytes]=await adapter.loadFormCheckpoint(key);return {revision,checkpoint:bytes?JSON.parse(new TextDecoder().decode(bytes)):null};};
  const out={};for(const scope of ['local-order-module','local-stock-module'])out[scope]={primary:await read(scope),backup:await read(JSON.stringify(['detail-local-backup-v1',scope]))};
  return out;
 });
 return records;
}