export async function erpDownload(url, token, filename, controller, progress) {
  let handle = null;
  let writable = null;
  let timer = null;
  let timedOut = false;
  let reader = null;
  const signal = controller.signal;
  const picker = typeof globalThis.showSaveFilePicker === "function";
  try {
    // The picker call itself runs before this async function reaches its first await.
    if (picker) handle = await globalThis.showSaveFilePicker({ suggestedName: filename });
    timer = setTimeout(() => { timedOut = true; controller.abort(); }, 300000);
    if (signal.aborted) throw "CANCELLED";
    const headers = token ? { Authorization: `Bearer ${token}` } : {};
    const response = await fetch(url, { method: "GET", headers, redirect: "error", signal });
    if (!response.ok) throw new Error(`HTTP:${response.status}`);
    const rawLength = response.headers.get("Content-Length");
    const declared = rawLength === null ? null : Number(rawLength);
    if (declared !== null && Number.isFinite(declared) && declared > 1073741824) throw new Error("TOO_LARGE");
    if (!picker && declared !== null && Number.isFinite(declared) && declared > 67108864) throw new Error("TOO_LARGE");
    reader = response.body?.getReader();
    let total = 0;
    const chunks = [];
    if (picker) writable = await handle.createWritable();
    while (reader) {
      const { done, value } = await reader.read();
      if (signal.aborted) throw "CANCELLED";
      if (done) break;
      total += value.byteLength;
      if (total > 1073741824 || (!picker && total > 67108864)) throw new Error("TOO_LARGE");
      if (!signal.aborted) progress(BigInt(total));
      if (signal.aborted) throw "CANCELLED";
      if (picker) await writable.write(value); else chunks.push(value);
    }
    if (declared !== null && Number.isFinite(declared) && declared !== total) throw new Error("SIZE_MISMATCH");
    if (signal.aborted) throw "CANCELLED";
    if (picker) await writable.close();
    else {
      const blob = new Blob(chunks);
      const objectUrl = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = objectUrl;
      anchor.download = filename;
      anchor.click();
      setTimeout(() => URL.revokeObjectURL(objectUrl), 0);
    }
    clearTimeout(timer);
    return filename;
  } catch (error) {
    if (timer !== null) clearTimeout(timer);
    if (reader) { try { await reader.cancel(); } catch (_) {} }
    if (writable) { try { await writable.abort(); } catch (_) {} }
    if (timedOut) throw "TRANSPORT";
    if (signal.aborted || error === "CANCELLED" || error?.name === "AbortError") throw "CANCELLED";
    const message = typeof error === "string" ? error : error?.message;
    if (typeof message === "string" && /^(HTTP:\d+|TOO_LARGE|SIZE_MISMATCH)$/.test(message)) throw message;
    throw "TRANSPORT";
  }
}

