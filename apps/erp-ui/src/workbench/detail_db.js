// Non-expiring, scope-isolated form checkpoints. Policy/receipt proof stays in Rust.
const DB = 'erp-detail-forms-v1';
const STORE = 'checkpoints';
const MAX = 8 * 1024 * 1024;
function open() {return new Promise((resolve,reject)=>{
  const r=indexedDB.open(DB,1);let abandoned=false;
  r.onupgradeneeded=()=>r.result.createObjectStore(STORE);
  r.onblocked=()=>{abandoned=true;reject(new Error('Form database blocked'));};
  r.onerror=()=>reject(new Error('Form database unavailable'));
  r.onsuccess=()=>{if(abandoned){r.result.close();return;}r.result.onversionchange=()=>r.result.close();resolve(r.result);};
});}
function valid(v){return v&&Number.isSafeInteger(v.revision)&&v.revision>0&&v.bytes instanceof Uint8Array&&v.bytes.byteLength>0&&v.bytes.byteLength<=MAX;}
export async function loadFormCheckpoint(scope){const db=await open();return new Promise((resolve,reject)=>{
 const tx=db.transaction(STORE,'readonly');let value;const r=tx.objectStore(STORE).get(scope);r.onsuccess=()=>{value=r.result;};
 tx.oncomplete=()=>{db.close();if(value===undefined)resolve([0,null]);else if(valid(value))resolve([value.revision,value.bytes]);else reject(new Error('Invalid form envelope'));};
 tx.onabort=tx.onerror=()=>{db.close();reject(new Error('Form read failed'));};
});}
export async function commitFormCheckpoint(scope,expected,bytes){const copy=new Uint8Array(bytes);
 if(!Number.isSafeInteger(expected)||expected<0||expected>=Number.MAX_SAFE_INTEGER||copy.byteLength===0||copy.byteLength>MAX)throw new Error('Form revision/capacity');
 const db=await open();return new Promise((resolve,reject)=>{
 let tx;try{tx=db.transaction(STORE,'readwrite',{durability:'strict'});if(tx.durability!=='strict'){tx.abort();db.close();reject(new Error('Strict form durability unavailable'));return;}}catch{db.close();reject(new Error('Strict form transaction unavailable'));return;}
 const store=tx.objectStore(STORE);const r=store.get(scope);
 r.onsuccess=()=>{const old=r.result;if((old!==undefined&&!valid(old))||(old?.revision||0)!==expected){tx.abort();return;}
 const put=()=>store.put({revision:expected+1,bytes:copy},scope);
 if(old===undefined){const c=store.count();c.onsuccess=()=>c.result>=32?tx.abort():put();}else put();};
 tx.oncomplete=()=>{db.close();resolve(expected+1);};tx.onabort=tx.onerror=()=>{db.close();reject(new Error('Form write failed/conflicted'));};
 });}
