async page => {
 const last=page.materialProof.filter(r=>r.method==='POST'&&r.url.includes('/material-documents?')).at(-1);
 const body=JSON.parse(last.body);
 if(body.expected_version!==2||!page.materialAuth)throw Error('actual v3 save capture required');
 const request=await page.evaluate(()=>crypto.randomUUID());
 const fresh={...body,request_id:request,expected_version:3,lines:body.lines.map(l=>({...l,qty:'0.600000'}))};
 const response=await page.request.post(last.url,{headers:{Authorization:page.materialAuth,'Idempotency-Key':request},data:fresh});
 const result=await response.json();
 if(response.status()!==200||result.snapshot?.version_no!==4)throw Error('concurrent actual save failed');
 page.materialConcurrent={request: fresh,response:result};
 return {version:result.snapshot.version_no,quantity:result.snapshot.lines[0].qty_micros};
}
