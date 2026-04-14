import { spawnSync } from 'node:child_process';

const testTargets = [
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

const skipPatterns = [
  /skipping[^\r\n]*DATABASE_URL/i,
  /skipping[^\r\n]*pool timed out/i,
];

function fail(message) {
  console.error(`[release-gate] ${message}`);
  process.exit(1);
}

function ensureDatabaseConfigured() {
  const databaseUrl = process.env.DATABASE_URL?.trim() ?? '';
  const testDatabaseUrl = process.env.TEST_DATABASE_URL?.trim() ?? '';

  if (!databaseUrl && !testDatabaseUrl) {
    fail(
      'DATABASE_URL/TEST_DATABASE_URL 未配置。发布门禁禁止在无数据库配置下执行（NO-GO）。',
    );
  }

  // 部分集成测试仅读取 DATABASE_URL；若只配置 TEST_DATABASE_URL，统一镜像到 DATABASE_URL。
  if (!databaseUrl && testDatabaseUrl) {
    process.env.DATABASE_URL = testDatabaseUrl;
  }
}

function run(command, args, target) {
  const result = spawnSync(command, args, {
    encoding: 'utf8',
    stdio: 'pipe',
  });

  if (result.stdout) {
    process.stdout.write(result.stdout);
  }
  if (result.stderr) {
    process.stderr.write(result.stderr);
  }

  if (result.error) {
    fail(`执行 ${target} 失败: ${result.error.message}`);
  }

  const combinedOutput = `${result.stdout ?? ''}\n${result.stderr ?? ''}`;
  const skipLine = combinedOutput
    .split(/\r?\n/)
    .find((line) => skipPatterns.some((pattern) => pattern.test(line)));

  if (skipLine) {
    fail(`检测到关键测试被跳过（${target}）：${skipLine.trim()}`);
  }

  if (result.status !== 0) {
    fail(`测试失败（${target}），退出码=${result.status ?? 1}`);
  }
}

ensureDatabaseConfigured();

for (const target of testTargets) {
  const cargoArgs = ['test', '-p', 'erp-api', '--test', target, '--', '--nocapture'];
  console.log(`[release-gate] running cargo ${cargoArgs.join(' ')}`);
  run('cargo', cargoArgs, target);
}
