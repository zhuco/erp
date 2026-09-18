import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const source = await readFile(new URL("../apps/erp-ui/src/http/download.js", import.meta.url), "utf8");
const { erpDownload } = await import(`data:text/javascript;base64,${Buffer.from(source).toString("base64")}`);
const url = "https://fixture.invalid/erp/exports/jobs/example/file";
let written = 0, closed = 0, aborted = 0, fetched = 0, pickerCalls = 0;
globalThis.showSaveFilePicker = () => {
  pickerCalls++;
  return Promise.resolve({ createWritable: async () => ({
    write: async bytes => { written += bytes.byteLength; },
    close: async () => { closed++; },
    abort: async () => { aborted++; },
  }) });
};
globalThis.fetch = async (_url, options) => {
  fetched++;
  assert.equal(pickerCalls, 1);
  assert.equal(options.redirect, "error");
  assert.equal(options.headers.Authorization, "Bearer fixture-token");
  let chunks = 0;
  return new Response(new ReadableStream({ pull(controller) {
    if (chunks++ < 9) controller.enqueue(new Uint8Array(1024 * 1024));
    else controller.close();
  } }), { headers: { "Content-Length": String(9 * 1024 * 1024) } });
};
let progress = 0n;
await erpDownload(url, "fixture-token", "fixture.csv", new AbortController(), value => { progress = value; });
assert.equal(written, 9 * 1024 * 1024);
assert.equal(progress, BigInt(written));
assert.equal(closed, 1);
assert.equal(aborted, 0);
assert.equal(fetched, 1);

// An incomplete body must abort the file, never close/publish it.
globalThis.fetch = async () => new Response(new Uint8Array(2), { headers: { "Content-Length": "3" } });
await assert.rejects(erpDownload(url, "", "fixture.csv", new AbortController(), () => {}), /SIZE_MISMATCH/);
assert.equal(closed, 1);
assert.equal(aborted, 1);

// Auth errors retain their status for the Rust session boundary.
globalThis.fetch = async () => new Response(null, { status: 401 });
await assert.rejects(erpDownload(url, "", "fixture.csv", new AbortController(), () => {}), /HTTP:401/);

// Unsupported browsers reject declared oversize fallback before reading data.
delete globalThis.showSaveFilePicker;
let reads = 0;
globalThis.fetch = async () => ({ ok: true, headers: new Headers({ "Content-Length": String(64 * 1024 * 1024 + 1) }),
  body: { getReader() { return { read: async () => { reads++; return { done: true }; }, cancel: async () => {} }; } } });
await assert.rejects(erpDownload(url, "", "fixture.csv", new AbortController(), () => {}), /TOO_LARGE/);
assert.equal(reads, 0);
console.log("download web bridge: streaming >8MiB, partial abort, auth status, bounded fallback passed");


// Cancellation while the last file write is pending must not publish the file.
const cancelled = new AbortController();
let cancelClosed = 0, cancelAborted = 0;
globalThis.showSaveFilePicker = () => Promise.resolve({ createWritable: async () => ({
  write: async () => cancelled.abort(),
  close: async () => { cancelClosed++; },
  abort: async () => { cancelAborted++; },
}) });
globalThis.fetch = async () => new Response(new Uint8Array(2), { headers: { "Content-Length": "2" } });
await assert.rejects(erpDownload(url, "", "fixture.csv", cancelled, () => {}), /CANCELLED/);
assert.equal(cancelClosed, 0);
assert.equal(cancelAborted, 1);
console.log("download cancellation before publication passed");
