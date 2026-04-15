import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const TEST_TARGETS = [
  'order_match_ship_wave_persistence_guard',
  'wave_ship_integration',
  'platform_sync_and_mapping_flow_integration',
  'frontend_route_diagnostics_smoke',
  'oms_orders',
  'shipment_confirm',
  'platform_oauth_webhook_integration',
  'order_exception_sla_integration',
  'wms_labor_slotting_flow',
  'inventory_atp_fefo_integration',
  'planning_replenishment_flow',
  'after_sales_exchange_profit_flow',
  'system_governance_workflow_integration',
  'stats_clickhouse_sso_smoke',
];

const SKIP_PATTERNS = [
  /skipping[^\r\n]*DATABASE_URL/i,
  /skipping[^\r\n]*pool timed out/i,
];

const currentFilePath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(currentFilePath);
const repoRoot = path.resolve(scriptDir, '..');

function fail(message) {
  console.error(`[release-gate] ${message}`);
  process.exit(1);
}

function parseEnvFile(filePath) {
  if (!fs.existsSync(filePath)) {
    return {};
  }

  const env = {};
  const content = fs.readFileSync(filePath, 'utf8');
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) {
      continue;
    }
    const separatorIndex = line.indexOf('=');
    if (separatorIndex <= 0) {
      continue;
    }
    const key = line.slice(0, separatorIndex).trim();
    const value = line.slice(separatorIndex + 1).trim();
    if (!key) {
      continue;
    }
    env[key] = value;
  }
  return env;
}

function hydrateDatabaseEnvFromDotEnv() {
  const dotenvPath = path.join(repoRoot, '.env');
  const envFromFile = parseEnvFile(dotenvPath);

  if (!process.env.DATABASE_URL && envFromFile.DATABASE_URL) {
    process.env.DATABASE_URL = envFromFile.DATABASE_URL;
  }
  if (!process.env.TEST_DATABASE_URL && envFromFile.TEST_DATABASE_URL) {
    process.env.TEST_DATABASE_URL = envFromFile.TEST_DATABASE_URL;
  }
}

function ensureDatabaseConfigured() {
  const databaseUrl = process.env.DATABASE_URL?.trim() ?? '';
  const testDatabaseUrl = process.env.TEST_DATABASE_URL?.trim() ?? '';

  if (!databaseUrl && !testDatabaseUrl) {
    fail('DATABASE_URL/TEST_DATABASE_URL 未配置，发布门禁禁止在无数据库环境下执行。');
  }

  if (!databaseUrl && testDatabaseUrl) {
    process.env.DATABASE_URL = testDatabaseUrl;
  }
}

function parseDatabaseEndpoint(databaseUrl) {
  let parsed;
  try {
    parsed = new URL(databaseUrl);
  } catch {
    fail(`DATABASE_URL 非法：${databaseUrl}`);
  }

  const host = parsed.hostname || 'localhost';
  const port = Number(parsed.port || '5432');
  if (!Number.isInteger(port) || port <= 0) {
    fail(`DATABASE_URL 端口非法：${databaseUrl}`);
  }

  return { host, port };
}

function waitForDatabaseEndpoint({ host, port, timeoutMs = 4000 }) {
  return new Promise((resolve, reject) => {
    const socket = new net.Socket();
    let settled = false;

    const finish = (callback, value) => {
      if (settled) {
        return;
      }
      settled = true;
      socket.destroy();
      callback(value);
    };

    socket.setTimeout(timeoutMs);
    socket.once('connect', () => finish(resolve, true));
    socket.once('timeout', () => finish(reject, new Error(`连接 ${host}:${port} 超时`)));
    socket.once('error', (error) => finish(reject, error));
    socket.connect(port, host);
  });
}

function formatNetworkError(error) {
  if (!error) {
    return '未知错误';
  }

  if (error instanceof AggregateError && Array.isArray(error.errors) && error.errors.length > 0) {
    const messages = error.errors
      .map((item) => item?.message?.trim() || item?.code || String(item))
      .filter(Boolean);
    if (messages.length > 0) {
      return messages.join(' | ');
    }
  }

  const message = error.message?.trim();
  if (message) {
    return message;
  }

  if (error.code) {
    return String(error.code);
  }

  return String(error);
}

function hasSkipLine(output) {
  return output
    .split(/\r?\n/)
    .find((line) => SKIP_PATTERNS.some((pattern) => pattern.test(line)));
}

function runTarget(target) {
  const args = ['test', '-p', 'erp-api', '--test', target, '--', '--nocapture', '--test-threads=1'];
  console.log(`[release-gate] running cargo ${args.join(' ')}`);

  const result = spawnSync('cargo', args, {
    cwd: repoRoot,
    encoding: 'utf8',
    stdio: 'pipe',
    env: {
      ...process.env,
      RUST_TEST_THREADS: process.env.RUST_TEST_THREADS || '1',
    },
  });

  if (result.stdout) {
    process.stdout.write(result.stdout);
  }
  if (result.stderr) {
    process.stderr.write(result.stderr);
  }

  if (result.error) {
    fail(`执行 ${target} 失败：${result.error.message}`);
  }

  const combined = `${result.stdout ?? ''}\n${result.stderr ?? ''}`;
  const skipLine = hasSkipLine(combined);
  if (skipLine) {
    fail(`检测到关键测试被跳过（${target}）：${skipLine.trim()}`);
  }

  if (result.status !== 0) {
    fail(`测试失败（${target}），退出码=${result.status ?? 1}`);
  }
}

async function main() {
  hydrateDatabaseEnvFromDotEnv();
  ensureDatabaseConfigured();

  const endpoint = parseDatabaseEndpoint(process.env.DATABASE_URL);
  try {
    await waitForDatabaseEndpoint(endpoint);
  } catch (error) {
    fail(
      `数据库不可达 ${endpoint.host}:${endpoint.port}，请先启动数据库。原因：${formatNetworkError(error)}`,
    );
  }

  for (const target of TEST_TARGETS) {
    runTarget(target);
  }

  console.log('[release-gate] backend release gate passed');
}

await main();
