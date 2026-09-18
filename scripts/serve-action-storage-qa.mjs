// Only exposes the storage adapter and its isolated browser QA fixture.
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
const root = new URL('../', import.meta.url);
const routes = new Map([
  ['/', ['qa/action-store-browser.html', 'text/html; charset=utf-8']],
  ['/indexed_db.js', ['apps/erp-ui/src/action_store/indexed_db.js', 'text/javascript; charset=utf-8']],
]);
createServer(async (request, response) => {
  const route = routes.get(request.url);
  if (request.method !== 'GET' || !route) { response.writeHead(404); response.end(); return; }
  try {
    const bytes = await readFile(new URL(route[0], root));
    response.writeHead(200, { 'content-type': route[1], 'cache-control': 'no-store' });
    response.end(bytes);
  } catch { response.writeHead(500); response.end('QA fixture unavailable'); }
}).listen(15439, '127.0.0.1', () => process.stdout.write('Action storage QA: http://127.0.0.1:15439/\n'));
