async page => { page.once('dialog',async d=>{if(d.type()==='beforeunload')await d.accept();else await d.dismiss();});await page.reload(); }
