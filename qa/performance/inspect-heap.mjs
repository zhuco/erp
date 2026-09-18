// Read-only retaining-path inspection of a Chrome heap snapshot. Paths exclude
// weak edges; shallow size is not retained size or proof of a leak.
import { readFileSync } from 'node:fs';
const file = process.argv[2];
if (!file) throw new Error('Pass a .heapsnapshot path');
const heap = JSON.parse(readFileSync(file, 'utf8'));
const { node_fields: nf, edge_fields: ef, node_types: nt, edge_types: et } = heap.snapshot.meta;
const width = nf.length;
const edgeWidth = ef.length;
const count = heap.nodes.length / width;
const field = (node, key) => heap.nodes[node * width + nf.indexOf(key)];
const name = node => heap.strings[field(node, 'name')];
const type = node => nt[0][field(node, 'type')];
const offsets = new Uint32Array(count + 1);
for (let i = 0; i < count; i++) offsets[i + 1] = offsets[i] + field(i, 'edge_count') * edgeWidth;
const parent = new Int32Array(count).fill(-1);
const parentEdge = new Int32Array(count).fill(-1);
const queue = [0];
parent[0] = 0;
for (let head = 0; head < queue.length; head++) {
  const source = queue[head];
  for (let i = offsets[source]; i < offsets[source + 1]; i += edgeWidth) {
    const edgeType = et[0][heap.edges[i + ef.indexOf('type')]];
    if (edgeType === 'weak') continue;
    const target = heap.edges[i + ef.indexOf('to_node')] / width;
    if (parent[target] >= 0) continue;
    parent[target] = source;
    parentEdge[target] = i;
    queue.push(target);
  }
}
function path(node) {
  const parts = [];
  for (let depth = 0; node !== 0 && depth < 24; depth++) {
    const edge = parentEdge[node];
    if (edge < 0) { parts.push('(not strongly reachable)'); break; }
    const edgeType = et[0][heap.edges[edge + ef.indexOf('type')]];
    const value = heap.edges[edge + ef.indexOf('name_or_index')];
    const label = ['element', 'hidden'].includes(edgeType) ? value : heap.strings[value];
    parts.push(`${label} -> ${type(node)}:${name(node)}`);
    node = parent[node];
  }
  return parts.reverse();
}
const targets = [];
for (let node = 0; node < count; node++) {
  if (type(node) === 'object' && name(node) === 'Array') {
    targets.push({ id: field(node, 'id'), shallowBytes: field(node, 'self_size'), path: path(node) });
  }
}
let comparison;
if (process.argv[3]) {
  const before = JSON.parse(readFileSync(process.argv[3], 'utf8'));
  const bf = before.snapshot.meta.node_fields;
  const oldIds = new Set();
  const totals = new Map();
  const adjust = (key, bytes, direction) => {
    const entry = totals.get(key) ?? { count: 0, shallowBytes: 0 };
    entry.count += direction;
    entry.shallowBytes += bytes * direction;
    totals.set(key, entry);
  };
  for (let i = 0; i < before.nodes.length; i += bf.length) {
    oldIds.add(before.nodes[i + bf.indexOf('id')]);
    const key = before.snapshot.meta.node_types[0][before.nodes[i]] + ':' + before.strings[before.nodes[i + bf.indexOf('name')]];
    adjust(key, before.nodes[i + bf.indexOf('self_size')], -1);
  }
  const added = [];
  for (let node = 0; node < count; node++) {
    adjust(`${type(node)}:${name(node)}`, field(node, 'self_size'), 1);
    if (!oldIds.has(field(node, 'id'))) added.push(node);
  }
  added.sort((a, b) => field(b, 'self_size') - field(a, 'self_size'));
  comparison = {
    baseline: process.argv[3],
    warning: 'Object IDs are comparable only within one live isolate; sizes are shallow.',
    net: [...totals].filter(([, value]) => value.count || value.shallowBytes)
      .sort((a, b) => b[1].shallowBytes - a[1].shallowBytes).slice(0, 30),
    largestAdded: added.slice(0, 15).map(node => ({ id: field(node, 'id'), type: type(node),
      name: name(node), shallowBytes: field(node, 'self_size'), path: path(node) })),
  };
}
console.log(JSON.stringify({ file, nodes: count, arrays: targets, comparison }, null, 2));
