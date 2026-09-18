// Browser storage adapter only. UI and action policy remain in Rust.
const DATABASE = 'erp-grid-actions-v1';
const STORE = 'checkpoints';
const DRAFTS = 'drafts';
const DRAFT_META = 'draft-meta';
const DRAFT_EPOCH = 'global-epoch';
const MAX_BYTES = 16 * 1024 * 1024;
const MAX_DRAFT_BYTES = 1024 * 1024;
const MAX_SCOPES = 32;
const MAX_DRAFTS = 32;
const DRAFT_TTL_SECONDS = 7 * 24 * 60 * 60;

function database() {
  return new Promise((resolve, reject) => {
    let abandoned = false;
    const request = indexedDB.open(DATABASE, 3);
    request.onupgradeneeded = () => {
      if (!request.result.objectStoreNames.contains(STORE)) request.result.createObjectStore(STORE);
      if (!request.result.objectStoreNames.contains(DRAFTS)) request.result.createObjectStore(DRAFTS);
      if (!request.result.objectStoreNames.contains(DRAFT_META)) {
        request.result.createObjectStore(DRAFT_META).put({ epoch: 0 }, DRAFT_EPOCH);
      }
    };
    request.onblocked = () => { abandoned = true; reject(new Error('Recovery database blocked')); };
    request.onerror = () => reject(new Error('Recovery database unavailable'));
    request.onsuccess = () => {
      if (abandoned) { request.result.close(); return; }
      request.result.onversionchange = () => request.result.close();
      resolve(request.result);
    };
  });
}
function valid(record) {
  return record && Number.isSafeInteger(record.revision) && record.revision > 0 &&
    record.bytes instanceof Uint8Array && record.bytes.byteLength <= MAX_BYTES;
}
export async function loadCheckpoint(key) {
  const db = await database();
  return new Promise((resolve, reject) => {
    let value;
    const tx = db.transaction(STORE, 'readonly');
    const request = tx.objectStore(STORE).get(key);
    request.onsuccess = () => { value = request.result; };
    tx.oncomplete = () => {
      db.close();
      if (value === undefined) resolve([0, null]);
      else if (valid(value)) resolve([value.revision, value.bytes]);
      else reject(new Error('Invalid recovery record'));
    };
    tx.onabort = tx.onerror = () => { db.close(); reject(new Error('Recovery read failed')); };
  });
}
export function commitCheckpoint(key, expected, bytes) {
  // Copy the wasm memory view before awaiting; Rust may release it immediately.
  const copy = new Uint8Array(bytes);
  if (!Number.isSafeInteger(expected) || expected < 0 || expected >= Number.MAX_SAFE_INTEGER || copy.byteLength > MAX_BYTES) {
    return Promise.reject(new Error('Recovery revision or size limit'));
  }
  return database().then(db => new Promise((resolve, reject) => {
    let tx;
    try {
      tx = db.transaction(STORE, 'readwrite', { durability: 'strict' });
      if (tx.durability !== 'strict') { tx.abort(); db.close(); reject(new Error('Strict durability unsupported')); return; }
    } catch {
      db.close(); reject(new Error('Strict recovery transaction unavailable')); return;
    }
    const store = tx.objectStore(STORE);
    const request = store.get(key);
    const fail = () => tx.abort();
    request.onsuccess = () => {
      const old = request.result;
      if ((old !== undefined && !valid(old)) || (old?.revision ?? 0) !== expected) { fail(); return; }
      const put = () => {
        try { store.put({ revision: expected + 1, bytes: copy }, key); }
        catch { fail(); }
      };
      if (old === undefined) {
        const count = store.count();
        count.onsuccess = () => { if (count.result >= MAX_SCOPES) fail(); else put(); };
      } else put();
    };
    // Request success does not mean transaction commit or durable persistence.
    tx.oncomplete = () => { db.close(); resolve(expected + 1); };
    tx.onabort = tx.onerror = () => { db.close(); reject(new Error('Recovery write failed or changed in another window')); };
  }));
}

function validDraft(record) {
  return record && Number.isSafeInteger(record.expiresAt) && record.expiresAt > 0 &&
    record.bytes instanceof Uint8Array && record.bytes.byteLength > 0 &&
    record.bytes.byteLength <= MAX_DRAFT_BYTES;
}

function validEpoch(record) {
  return record && Number.isSafeInteger(record.epoch) &&
    record.epoch >= 0 && record.epoch <= Number.MAX_SAFE_INTEGER;
}

function draftExpiry(bytes, now) {
  try {
    const value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    const expiresAt = value?.expires_at;
    if (Number.isSafeInteger(expiresAt) && expiresAt > now && expiresAt <= now + DRAFT_TTL_SECONDS) {
      return expiresAt;
    }
  } catch {}
  return null;
}

export async function loadDraft(key) {
  const db = await database();
  return new Promise((resolve, reject) => {
    const now = Math.floor(Date.now() / 1000);
    let value;
    let epoch;
    const tx = db.transaction([DRAFTS, DRAFT_META], 'readwrite', { durability: 'strict' });
    const store = tx.objectStore(DRAFTS);
    const meta = tx.objectStore(DRAFT_META);
    const epochRequest = meta.get(DRAFT_EPOCH);
    const request = store.get(key);
    epochRequest.onsuccess = () => {
      const record = epochRequest.result;
      if (!validEpoch(record)) { tx.abort(); return; }
      epoch = record.epoch;
    };
    epochRequest.onerror = () => tx.abort();
    request.onsuccess = () => {
      value = request.result;
      if (value !== undefined && !validDraft(value)) {
        // Version 2 clear records were permanent tombstones. Remove them during
        // normal access so they no longer consume an active draft slot.
        if (value?.bytes === null) {
          store.delete(key);
          value = undefined;
          const advance = () => {
            if (!Number.isSafeInteger(epoch) || epoch >= Number.MAX_SAFE_INTEGER) { tx.abort(); return; }
            epoch += 1;
            meta.put({ epoch }, DRAFT_EPOCH);
          };
          if (epochRequest.readyState === 'done') advance();
          else epochRequest.addEventListener('success', advance, { once: true });
        }
        else tx.abort();
      } else if (value?.expiresAt <= now) {
        store.delete(key);
        value = undefined;
        const advance = () => {
          if (!Number.isSafeInteger(epoch) || epoch >= Number.MAX_SAFE_INTEGER) { tx.abort(); return; }
          epoch += 1;
          meta.put({ epoch }, DRAFT_EPOCH);
        };
        if (epochRequest.readyState === 'done') advance();
        else epochRequest.addEventListener('success', advance, { once: true });
      }
    };
    request.onerror = () => tx.abort();
    tx.oncomplete = () => {
      db.close();
      resolve([epoch, value?.bytes ?? null]);
    };
    tx.onabort = tx.onerror = () => { db.close(); reject(new Error('Draft read failed')); };
  });
}

export function commitDraft(key, expected, bytes, clear, now) {
  const copy = new Uint8Array(bytes);
  const expiresAt = clear ? now + DRAFT_TTL_SECONDS : draftExpiry(copy, now);
  if (!Number.isSafeInteger(expected) || expected < 0 || expected >= Number.MAX_SAFE_INTEGER ||
      !Number.isSafeInteger(now) || now <= 0 || !Number.isSafeInteger(expiresAt) ||
      typeof clear !== 'boolean' || (!clear && (copy.byteLength === 0 || copy.byteLength > MAX_DRAFT_BYTES))) {
    return Promise.reject(new Error('Draft revision, timestamp or size limit'));
  }
  return database().then(db => new Promise((resolve, reject) => {
    let tx;
    try {
      tx = db.transaction([DRAFTS, DRAFT_META], 'readwrite', { durability: 'strict' });
      if (tx.durability !== 'strict') { tx.abort(); db.close(); reject(new Error('Strict durability unsupported')); return; }
    } catch {
      db.close(); reject(new Error('Strict draft transaction unavailable')); return;
    }
    const store = tx.objectStore(DRAFTS);
    const meta = tx.objectStore(DRAFT_META);
    const epochRequest = meta.get(DRAFT_EPOCH);
    let epoch;
    epochRequest.onsuccess = () => {
      const record = epochRequest.result;
      if (!validEpoch(record) || record.epoch !== expected) { tx.abort(); return; }
      epoch = record.epoch;
    };
    epochRequest.onerror = () => tx.abort();
    const sweep = store.openCursor();
    sweep.onsuccess = () => {
      const cursor = sweep.result;
      if (cursor) {
        const value = cursor.value;
        if (value?.bytes === null || (validDraft(value) && value.expiresAt <= now)) cursor.delete();
        else if (!validDraft(value)) { tx.abort(); return; }
        cursor.continue();
        return;
      }
      const request = store.get(key);
      request.onsuccess = () => {
        const old = request.result;
        const put = () => {
          try {
            if (clear) store.delete(key);
            else store.put({ expiresAt, bytes: copy }, key);
            meta.put({ epoch: epoch + 1 }, DRAFT_EPOCH);
          }
          catch { tx.abort(); }
        };
        const afterEpoch = () => {
          if (!Number.isSafeInteger(epoch) || epoch !== expected) { tx.abort(); return; }
          if (clear || old !== undefined) { put(); return; }
          const count = store.count();
          count.onsuccess = () => { if (count.result >= MAX_DRAFTS) tx.abort(); else put(); };
          count.onerror = () => tx.abort();
        };
        if (epochRequest.readyState === 'done') afterEpoch();
        else epochRequest.addEventListener('success', afterEpoch, { once: true });
      };
      request.onerror = () => tx.abort();
    };
    sweep.onerror = () => tx.abort();
    tx.oncomplete = () => { db.close(); resolve(expected + 1); };
    tx.onabort = tx.onerror = () => { db.close(); reject(new Error('Draft write failed or changed in another window')); };
  }));
}
