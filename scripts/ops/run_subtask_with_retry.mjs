#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { isTransientFailure, runWithRetry } from './lib/subtask_retry.mjs';

const args = process.argv.slice(2);

const usage = () => {
  console.log(
    'Usage: node scripts/ops/run_subtask_with_retry.mjs [--name NAME] [--max-retries N] [--delay-seconds N] [--cwd PATH] -- <command> [args...]',
  );
  process.exit(1);
};

function parseArgs(argv) {
  const parsed = {
    name: 'subtask',
    maxRetries: 2,
    delaySeconds: 8,
    cwd: process.cwd(),
    command: null,
    commandArgs: [],
  };

  const separator = argv.indexOf('--');
  const flagPart = separator >= 0 ? argv.slice(0, separator) : argv;
  const commandPart = separator >= 0 ? argv.slice(separator + 1) : [];

  for (let i = 0; i < flagPart.length; i += 1) {
    const flag = flagPart[i];
    const value = flagPart[i + 1];
    if (flag === '--name') {
      if (!value || value.startsWith('--')) usage();
      parsed.name = value;
      i += 1;
      continue;
    }
    if (flag === '--max-retries') {
      if (!value || value.startsWith('--')) usage();
      parsed.maxRetries = Number.parseInt(value, 10);
      i += 1;
      continue;
    }
    if (flag === '--delay-seconds') {
      if (!value || value.startsWith('--')) usage();
      parsed.delaySeconds = Number.parseInt(value, 10);
      i += 1;
      continue;
    }
    if (flag === '--cwd') {
      if (!value || value.startsWith('--')) usage();
      parsed.cwd = value;
      i += 1;
      continue;
    }
    usage();
  }

  if (!Number.isInteger(parsed.maxRetries) || parsed.maxRetries < 0) {
    throw new Error('--max-retries must be a non-negative integer');
  }
  if (!Number.isInteger(parsed.delaySeconds) || parsed.delaySeconds < 0) {
    throw new Error('--delay-seconds must be a non-negative integer');
  }
  if (!commandPart.length) {
    throw new Error('missing command after --');
  }

  parsed.command = commandPart[0];
  parsed.commandArgs = commandPart.slice(1);
  return parsed;
}

function runCommandOnce(command, commandArgs, cwd) {
  return new Promise((resolve) => {
    const child = spawn(command, commandArgs, {
      cwd,
      shell: false,
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
      const message = error?.message ? `${error.message}\n` : '';
      stderr += message;
      if (message) {
        process.stderr.write(message);
      }
      resolve({
        code: 1,
        stdout,
        stderr,
      });
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

async function main() {
  try {
    const parsed = parseArgs(args);
    const delayMs = parsed.delaySeconds * 1000;
    const commandPreview = [parsed.command, ...parsed.commandArgs].join(' ');

    console.log(
      `[subtask-retry] start name=${parsed.name} maxRetries=${parsed.maxRetries} delaySeconds=${parsed.delaySeconds} command="${commandPreview}"`,
    );

    const outcome = await runWithRetry({
      maxRetries: parsed.maxRetries,
      delayMs,
      shouldRetry: isTransientFailure,
      execute: () => runCommandOnce(parsed.command, parsed.commandArgs, parsed.cwd),
      onAttempt: ({ attempt, maxAttempts, ok, transient, willRetry }) => {
        const state = ok ? 'ok' : transient ? 'transient-fail' : 'fatal-fail';
        console.log(
          `[subtask-retry] attempt ${attempt}/${maxAttempts} state=${state} willRetry=${willRetry}`,
        );
      },
    });

    if (outcome.ok) {
      console.log(
        `[subtask-retry] success name=${parsed.name} attempts=${outcome.attempts} retried=${outcome.retried}`,
      );
      process.exit(0);
      return;
    }

    const lastCode = Number.isInteger(outcome.lastResult?.code) ? outcome.lastResult.code : 1;
    console.error(
      `[subtask-retry] failed name=${parsed.name} attempts=${outcome.attempts} transient=${outcome.transient} exitCode=${lastCode}`,
    );
    process.exit(lastCode);
  } catch (error) {
    console.error(`[subtask-retry] ${error.message}`);
    usage();
  }
}

main();
