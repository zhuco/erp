async page => { page.on('request',r=>{if(r.url().includes('/manufacturing/')&&r.method()==='POST')page.materialAuth=r.headers()['authorization'];}); }
