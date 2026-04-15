#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { normalizePath } from './task_card.mjs';

const LIST_FLAGS = {
  '--pattern': 'patterns',
};

const SINGLE_FLAGS = {
  '--mode': 'mode',
  '--task': 'task',
  '--owner': 'owner',
  '--policy-file': 'policyFile',
  '--lock-registry': 'lockRegistry',
};

const USAGE = `Usage:
  node scripts/ops/register_shared_lock.mjs --mode status [--policy-file PATH] [--lock-registry PATH]
  node scripts/ops/register_shared_lock.mjs --mode acquire --task TASK_CARD --owner OWNER --pattern GLOB [--pattern GLOB]
  node scripts/ops/register_shared_lock.mjs --mode release --task TASK_CARD [--owner OWNER] [--pattern GLOB]
`;

function fail(message) {
  console.error(`FAIL: ${message}`);
  process.exit(1);
}

function parseArgs() {
  const args = process.argv.slice(2);
  if (!args.length || args.includes('--help')) {
    console.log(USAGE);
    process.exit(args.includes('--help') ? 0 : 1);
  }

  const parsed = {
    patterns: [],
  };

  for (let i = 0; i < args.length; i += 1) {
    const flag = args[i];
    if (LIST_FLAGS[flag]) {
      const next = args[i + 1];
      if (!next || next.startsWith('--')) {
        fail(`missing value for ${flag}`);
      }
      parsed[LIST_FLAGS[flag]].push(next);
      i += 1;
      continue;
    }

    if (SINGLE_FLAGS[flag]) {
      const next = args[i + 1];
      if (!next || next.startsWith('--')) {
        fail(`missing value for ${flag}`);
      }
      parsed[SINGLE_FLAGS[flag]] = next;
      i += 1;
      continue;
    }

    fail(`unknown argument ${flag}`);
  }

  if (!parsed.mode) {
    fail('missing required argument --mode');
  }

  return parsed;
}

function loadJson(filePath, fallback) {
  if (!fs.existsSync(filePath)) {
    return fallback;
  }

  const text = fs.readFileSync(filePath, 'utf8').trim();
  if (!text) {
    return fallback;
  }

  return JSON.parse(text);
}

function saveJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function main() {
  const parsed = parseArgs();
  const repoRoot = process.cwd();
  const policyPath = path.resolve(parsed.policyFile || path.join(repoRoot, 'docs', 'ops', 'config', 'PARALLEL_SHARED_LOCKS_V2.json'));
  const registryPath = path.resolve(parsed.lockRegistry || path.join(repoRoot, 'docs', 'ops', 'state', 'PARALLEL_SHARED_LOCK_REGISTRY.json'));

  const policy = loadJson(policyPath, { locked_patterns: [] });
  const lockedPatterns = Array.isArray(policy.locked_patterns) ? policy.locked_patterns : [];
  const registry = loadJson(registryPath, { version: 'v2', locks: [] });
  registry.version = registry.version || 'v2';
  registry.locks = Array.isArray(registry.locks) ? registry.locks : [];

  const mode = parsed.mode;
  if (!['status', 'acquire', 'release'].includes(mode)) {
    fail(`unsupported mode: ${mode}`);
  }

  if (mode === 'status') {
    const active = registry.locks;
    if (!active.length) {
      console.log('OK: no active shared locks');
      return;
    }
    console.log('Active shared locks:');
    for (const lock of active) {
      console.log(`- ${lock.pattern} | owner=${lock.owner} | task=${lock.task}`);
    }
    return;
  }

  const task = parsed.task ? normalizePath(path.resolve(parsed.task)) : '';
  if (!task) {
    fail('--task is required for acquire/release');
  }

  if (!fs.existsSync(task)) {
    fail(`task card not found: ${task}`);
  }

  const owner = String(parsed.owner || '').trim();
  const patterns = parsed.patterns.length ? parsed.patterns : lockedPatterns;

  if (mode === 'acquire') {
    if (!owner) {
      fail('--owner is required for acquire');
    }
    if (!patterns.length) {
      fail('no --pattern provided and policy has no locked patterns');
    }

    for (const pattern of patterns) {
      if (!lockedPatterns.includes(pattern)) {
        fail(`pattern not in shared lock policy: ${pattern}`);
      }
      const conflict = registry.locks.find((lock) => lock.pattern === pattern && lock.task !== task);
      if (conflict) {
        fail(`pattern already locked by ${conflict.task} (${conflict.owner}): ${pattern}`);
      }
    }

    for (const pattern of patterns) {
      const existingIndex = registry.locks.findIndex((lock) => lock.pattern === pattern && lock.task === task);
      const record = {
        pattern,
        task,
        owner,
        acquired_at: new Date().toISOString(),
      };
      if (existingIndex >= 0) {
        registry.locks[existingIndex] = record;
      } else {
        registry.locks.push(record);
      }
    }

    registry.updated_at = new Date().toISOString();
    saveJson(registryPath, registry);
    console.log(`OK: acquired ${patterns.length} shared lock(s) for ${task}`);
    return;
  }

  if (mode === 'release') {
    const before = registry.locks.length;
    registry.locks = registry.locks.filter((lock) => {
      if (lock.task !== task) {
        return true;
      }
      if (owner && lock.owner !== owner) {
        return true;
      }
      if (parsed.patterns.length && !parsed.patterns.includes(lock.pattern)) {
        return true;
      }
      return false;
    });

    const released = before - registry.locks.length;
    registry.updated_at = new Date().toISOString();
    saveJson(registryPath, registry);
    console.log(`OK: released ${released} shared lock(s) for ${task}`);
  }
}

main();
