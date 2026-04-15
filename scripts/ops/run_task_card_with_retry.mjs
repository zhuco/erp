#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { parseTaskCard } from './task_card.mjs';
import { runWithRetry } from './lib/subtask_retry.mjs';

const args = process.argv.slice(2);

const usageText = [
  'Usage: node scripts/ops/run_task_card_with_retry.mjs --task <path> [options]',
  'Options:',
  '  --task <path>          任务卡 markdown 文件（必须包含 front matter）',
  '  --mode <working|staged|all>  任务边界校验模式，默认 working',
  '  --cwd <path>           命令执行目录，默认当前目录',
  '  --max-retries <n>      覆盖任务卡重试次数',
  '  --delay-seconds <n>    覆盖任务卡重试间隔秒数',
  '  --skip-boundary        跳过边界校验',
];

function printUsage(exitCode = 1) {
  console.log(usageText.join('\n'));
  process.exit(exitCode);
}

function ensureArray(value) {
  if (value === undefined || value === null) return [];
  return Array.isArray(value) ? value : [value];
}

function parseRetryPolicy(rawPolicy) {
  const fallback = {
    maxRetries: 2,
    delaySeconds: 8,
  };
  if (!rawPolicy) return fallback;

  const text = String(rawPolicy).trim();
  const match = text.match(/transient-auto-retry\s*\(\s*max\s*=\s*(\d+)\s*,\s*delay\s*=\s*(\d+)s\s*\)/i);
  if (!match) return fallback;

  return {
    maxRetries: Number.parseInt(match[1], 10),
    delaySeconds: Number.parseInt(match[2], 10),
  };
}

function parseArgs(argv) {
  const parsed = {
    mode: 'working',
    cwd: process.cwd(),
    skipBoundary: false,
  };

  for (let i = 0; i < argv.length; i += 1) {
    const flag = argv[i];
    if (flag === '--help') {
      printUsage(0);
    }
    if (flag === '--skip-boundary') {
      parsed.skipBoundary = true;
      continue;
    }
    const next = argv[i + 1];
    if (!next || next.startsWith('--')) {
      console.error(`[task-runner] missing value for ${flag}`);
      printUsage(1);
    }
    if (flag === '--task') {
      parsed.task = next;
      i += 1;
      continue;
    }
    if (flag === '--mode') {
      parsed.mode = next;
      i += 1;
      continue;
    }
    if (flag === '--cwd') {
      parsed.cwd = next;
      i += 1;
      continue;
    }
    if (flag === '--max-retries') {
      parsed.maxRetries = Number.parseInt(next, 10);
      i += 1;
      continue;
    }
    if (flag === '--delay-seconds') {
      parsed.delaySeconds = Number.parseInt(next, 10);
      i += 1;
      continue;
    }
    console.error(`[task-runner] unknown argument ${flag}`);
    printUsage(1);
  }

  if (!parsed.task) {
    console.error('[task-runner] --task is required');
    printUsage(1);
  }
  if (!['working', 'staged', 'all'].includes(parsed.mode)) {
    throw new Error(`unsupported --mode: ${parsed.mode}`);
  }
  if (parsed.maxRetries !== undefined && (!Number.isInteger(parsed.maxRetries) || parsed.maxRetries < 0)) {
    throw new Error('--max-retries must be a non-negative integer');
  }
  if (parsed.delaySeconds !== undefined && (!Number.isInteger(parsed.delaySeconds) || parsed.delaySeconds < 0)) {
    throw new Error('--delay-seconds must be a non-negative integer');
  }

  return parsed;
}

function quoteForShell(value) {
  return `"${String(value).replace(/"/g, '\\"')}"`;
}

function runCommandLine(commandLine, cwd) {
  return new Promise((resolve) => {
    const child = spawn(commandLine, {
      cwd,
      shell: true,
      windowsHide: true,
    });

    let stdout = '';
    let stderr = '';

    child.stdout.on('data', (chunk) => {
      const text = String(chunk);
      stdout += text;
      process.stdout.write(text);
    });

    child.stderr.on('data', (chunk) => {
      const text = String(chunk);
      stderr += text;
      process.stderr.write(text);
    });

    child.on('error', (error) => {
      const text = `${error?.message ?? 'unknown process error'}\n`;
      stderr += text;
      process.stderr.write(text);
      resolve({ code: 1, stdout, stderr });
    });

    child.on('close', (code) => {
      resolve({
        code: Number.isInteger(code) ? code : 1,
        stdout,
        stderr,
      });
    });
  });
}

async function executeWithPolicy({ label, commandLine, cwd, maxRetries, delaySeconds, fallbackStrategy }) {
  const delayMs = delaySeconds * 1000;
  console.log(`[task-runner] execute ${label}: ${commandLine}`);

  const outcome = await runWithRetry({
    maxRetries,
    delayMs,
    execute: async () => runCommandLine(commandLine, cwd),
    onAttempt: ({ attempt, maxAttempts, ok, transient, willRetry }) => {
      const state = ok ? 'ok' : transient ? 'transient-fail' : 'fatal-fail';
      console.log(`[task-runner] ${label} attempt ${attempt}/${maxAttempts} state=${state} willRetry=${willRetry}`);
    },
  });

  if (outcome.ok) {
    return {
      ok: true,
      fallbackUsed: false,
      attempts: outcome.attempts,
    };
  }

  if (outcome.transient && String(fallbackStrategy || '').includes('switch-to-local-execution')) {
    console.log(`[task-runner] ${label} fallback start: switch-to-local-execution`);
    const fallbackResult = await runCommandLine(commandLine, cwd);
    if (fallbackResult.code === 0) {
      console.log(`[task-runner] ${label} fallback success`);
      return {
        ok: true,
        fallbackUsed: true,
        attempts: outcome.attempts + 1,
      };
    }
    console.error(`[task-runner] ${label} fallback failed code=${fallbackResult.code}`);
    return {
      ok: false,
      fallbackUsed: true,
      attempts: outcome.attempts + 1,
    };
  }

  return {
    ok: false,
    fallbackUsed: false,
    attempts: outcome.attempts,
  };
}

async function main() {
  try {
    const parsed = parseArgs(args);
    const taskPath = path.resolve(parsed.task);
    const cwd = path.resolve(parsed.cwd);

    if (!fs.existsSync(taskPath)) {
      throw new Error(`task file not found: ${taskPath}`);
    }

    const cardText = fs.readFileSync(taskPath, 'utf8');
    const task = parseTaskCard(cardText);

    const policyFromCard = parseRetryPolicy(task.retryPolicy);
    const maxRetries = parsed.maxRetries ?? policyFromCard.maxRetries;
    const delaySeconds = parsed.delaySeconds ?? policyFromCard.delaySeconds;
    const fallbackStrategy = task.fallbackStrategy || 'switch-to-local-execution';

    console.log(
      `[task-runner] start task=${task.name || path.basename(taskPath)} maxRetries=${maxRetries} delaySeconds=${delaySeconds} fallback=${fallbackStrategy}`,
    );

    const checks = [];
    if (!parsed.skipBoundary) {
      const boundaryCommand = [
        'node scripts/ops/validate_task_boundary.mjs',
        `--task ${quoteForShell(taskPath)}`,
        `--mode ${parsed.mode}`,
      ].join(' ');
      checks.push({
        label: 'boundary-check',
        commandLine: boundaryCommand,
      });
    }

    const testCommands = ensureArray(task.testCommands).filter((item) => String(item).trim().length > 0);
    testCommands.forEach((command, index) => {
      checks.push({
        label: `test-${index + 1}`,
        commandLine: String(command),
      });
    });

    if (!checks.length) {
      console.log('[task-runner] no checks configured, finished');
      process.exit(0);
      return;
    }

    for (const check of checks) {
      const result = await executeWithPolicy({
        ...check,
        cwd,
        maxRetries,
        delaySeconds,
        fallbackStrategy,
      });
      if (!result.ok) {
        console.error(`[task-runner] failed on ${check.label}`);
        process.exit(1);
        return;
      }
    }

    console.log('[task-runner] all checks passed');
    process.exit(0);
  } catch (error) {
    console.error(`[task-runner] ${error.message}`);
    process.exit(1);
  }
}

main();
