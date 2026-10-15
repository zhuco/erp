async page => {
 page.materialProof=[];
 page.on('request',r=>{if(r.url().includes('/manufacturing/'))page.materialProof.push({url:r.url(),method:r.method(),body:r.postData(),at:Date.now()});});
 page.materialDropped=false;
 await page.route('**/manufacturing/work-orders/*/material-documents?rejection_envelope=true',async route=>{
  if(route.request().method()==='POST'&&!page.materialDropped){
   const response=await route.fetch(),body=await response.json();
   if(response.status()!==200||body.snapshot?.status!=='DRAFT')throw Error('unexpected actual save result');
   page.materialDropped=true;page.materialOriginal=body.snapshot;await route.abort('failed');
  }else await route.continue();
 });
 await page.mouse.dblclick(600,478);await page.keyboard.press('Control+A');await page.keyboard.insertText('0.25');await page.keyboard.press('Enter');
 await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
}
