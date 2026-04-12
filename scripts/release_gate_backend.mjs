import { spawnSync } from 'node:child_process';

const testTargets = [
  'platform_oauth_webhook_integration',
  'order_exception_sla_integration',
  'wms_labor_slotting_flow',
  'inventory_atp_fefo_integration',
  'planning_replenishment_flow',
  'after_sales_exchange_profit_flow',
  'system_governance_workflow_integration',
  'stats_clickhouse_sso_smoke',
];

function run(command, args) {
  const result = spawnSync(command, args, {
    stdio: 'inherit',
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

const cargoArgs = ['test', '-p', 'erp-api'];
for (const target of testTargets) {
  cargoArgs.push('--test', target);
}
cargoArgs.push('--', '--nocapture');

run('cargo', cargoArgs);
