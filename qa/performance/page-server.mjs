// Loopback-only synthetic million-row server for grid protocol measurements.
// This is not the ERP API/database or authorization acceptance.
import { createServer } from 'node:http';

const total = 1_000_000;
const records = [];
let active = 0;
let sequence = 0;
let sentBytes = 0;
let completed = 0;
const server = createServer(async (request, response) => {
  const url = new URL(request.url, 'http://127.0.0.1:15444');
  response.setHeader('Access-Control-Allow-Origin', 'http://127.0.0.1:15440');
  response.setHeader('Access-Control-Allow-Headers', 'Content-Type,X-Fixture-Delay-Ms');
  response.setHeader('Access-Control-Allow-Methods', 'POST,GET,OPTIONS');
  response.setHeader('Cache-Control', 'no-store');
  const send = (status, value) => {
    const body = JSON.stringify(value);
    response.writeHead(status, { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) });
    response.end(body);
    return Buffer.byteLength(body);
  };
  if (request.method === 'OPTIONS') { response.writeHead(204).end(); return; }
  if (request.url === '/metrics' && request.method === 'GET') {
    send(200, { active, requests: sequence, sentBytes, records }); return;
  }
  if (url.pathname !== '/page' || request.method !== 'POST') { send(404, { error: 'Unknown fixture route' }); return; }
  if (active >= 2) { send(429, { error: 'Fixture allows two concurrent requests' }); return; }
  active++;
  const id = ++sequence;
  let released = false;
  let timer;
  const record = { id, status: 'reading', offset: null, rows: 0, bytes: 0 };
  records.push(record);
  if (records.length > 1000) records.shift();
  const release = () => {
    if (released) return;
    released = true;
    active--;
    if (timer) clearTimeout(timer);
    if (record.status === 'reading' || record.status === 'waiting') record.status = 'cancelled';
  };
  response.on('close', release);
  response.on('finish', release);
  try {
    const chunks = [];
    let length = 0;
    for await (const chunk of request) {
      length += chunk.length;
      if (length > 16384) {
        record.status = 'rejected'; send(413, { error: 'Request exceeds 16 KiB' }); return;
      }
      chunks.push(chunk);
    }
    const query = JSON.parse(Buffer.concat(chunks).toString('utf8'));
    const delay = Number(request.headers['x-fixture-delay-ms'] ?? url.searchParams.get('delay_ms') ?? 0);
    if (!Number.isSafeInteger(query.offset) || query.offset < 0 || query.offset > total
      || !Number.isInteger(query.limit) || query.limit < 1 || query.limit > 100
      || query.schema !== 1 || !Number.isInteger(delay) || delay < 0 || delay > 2000
      || Object.keys(query).some(key => !['schema', 'offset', 'limit', 'sorts', 'filters', 'expression', 'search'].includes(key))
      || (query.search != null && query.search !== '') || query.expression != null
      || (query.filters != null && (typeof query.filters !== 'object' || Object.keys(query.filters).length !== 0))
      || (query.sorts != null && (!Array.isArray(query.sorts) || query.sorts.length > 1
        || query.sorts.some(sort => sort.column !== 0 || typeof sort.descending !== 'boolean')))) {
      record.status = 'rejected'; send(400, { error: 'Unsupported or invalid fixture query' }); return;
    }
    const descending = query.sorts?.[0]?.descending ?? false;
    record.offset = query.offset;
    record.status = 'waiting';
    // Generate only the requested page after its delay. No million-row array.
    timer = setTimeout(() => {
      if (released) return;
      const data = Array.from({ length: Math.min(query.limit, total - query.offset) }, (_, index) => {
        const ordinal = query.offset + index;
        const row = descending ? total - ordinal : ordinal + 1;
        return { id: row, version: '1', code: `ROW-${String(row).padStart(7, '0')}`, value: row };
      });
      record.rows = data.length;
      record.status = 'sent';
      record.sentOrder = ++completed;
      record.bytes = send(200, { total, offset: query.offset, data });
      sentBytes += record.bytes;
    }, delay);
  } catch {
    record.status = 'rejected';
    if (!response.destroyed && !response.headersSent) send(400, { error: 'Invalid fixture JSON' });
  }
});
server.headersTimeout = 5000;
server.requestTimeout = 5000;
server.on('error', error => { console.error(error.message); process.exitCode = 1; });
server.listen(15444, '127.0.0.1', () => console.log('Synthetic page fixture: http://127.0.0.1:15444; PID ' + process.pid));
