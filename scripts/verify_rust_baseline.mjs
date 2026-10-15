import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import YAML from 'yaml';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const read = name => fs.readFileSync(path.join(root, name), 'utf8');
function requireContract(condition, message) {
  if (!condition) throw new Error(message);
}
for (const file of [
  '.editorconfig', '.gitattributes', '.gitignore', 'rustfmt.toml', 'clippy.toml',
  'rust-toolchain.toml', 'apps/erp-ui/Cargo.toml', 'crates/erp-grid-core/Cargo.toml',
  'crates/erp-ui-egui/Cargo.toml', 'scripts/Invoke-ErpBuild.ps1',
  'scripts/verify_no_ag_grid.mjs', 'scripts/verify_dependency_licenses.mjs', 'qa/acceptance/browser-smoke.mjs',
]) requireContract(fs.existsSync(path.join(root, file)), `Missing baseline file: ${file}`);

const pkg = JSON.parse(read('package.json'));
for (const name of ['dev', 'build', 'build:web', 'lint', 'typecheck', 'test:ui', 'test',
  'accept:web', 'accept:backend', 'accept:all', 'test:backend:release-gate',
  'test:e2e:release-gate', 'release:gate', 'repo:verify:grid-policy', 'repo:verify:openapi']) {
  requireContract(typeof pkg.scripts?.[name] === 'string', `Missing root command: ${name}`);
}
requireContract(!Object.values(pkg.scripts).some(command => /erp-web|\bcargo\s+(build|check|test|clippy|fmt|run)/.test(command)),
  'Root commands must use the Rust UI and controlled build entry');
requireContract(!fs.existsSync(path.join(root, 'apps/erp-web/package.json')),
  'The retired React app still has an executable package manifest');
requireContract(pkg.scripts['release:gate'].includes('test:backend:release-gate') &&
  pkg.scripts['release:gate'].includes('test:e2e:release-gate'), 'Release entry must retain backend and browser gates');

const workflow = YAML.parse(read('.github/workflows/ci.yml'));
const grid = workflow.jobs?.['rust-grid-native-wasm'];
requireContract(grid, 'Missing Rust native/WASM CI job');
const commands = grid.steps.filter(step => step.run).map(step => step.run);
for (const [action, platform] of [['fmt','native'], ['test','native'], ['clippy','native'], ['clippy','web']]) {
  requireContract(commands.some(command => command.includes('Invoke-ErpBuild.ps1') &&
    command.includes(`-Action ${action}`) && command.includes(`-Platform ${platform}`)),
  `Missing controlled CI ${platform} ${action}`);
}
requireContract(!Object.values(workflow.jobs).some(job => (job.steps ?? []).some(step => /erp-web/.test(step.run ?? ''))),
  'CI still launches the retired UI');
requireContract(workflow.jobs['backend-release-gate'], 'Backend release tests must not be removed with the UI');

const allowedDuplicates = new Set([
  '0060_stock_count_diff_reason.sql', '0060_wms_pack_checks.sql',
  '0073_marketing_management.sql', '0073_procurement_management.sql',
  '0074_after_sales_service_ops.sql', '0074_logistics_fulfillment.sql', '0074_product_master_data.sql',
  '0079_ai1_platform_integration.sql', '0079_global_status_fields.sql', '0079_oms_order_statuses.sql',
  '0095_multi_merchant_org_baseline.sql', '0095_payment_reconcile_query_indexes.sql',
]);
const versions = new Map();
for (const file of fs.readdirSync(path.join(root, 'migrations')).filter(file => file.endsWith('.sql'))) {
  const prefix = /^(\d+)_/.exec(file)?.[1];
  requireContract(prefix, `Migration has no numeric prefix: ${file}`);
  const group = versions.get(prefix) ?? [];
  group.push(file);
  versions.set(prefix, group);
}
for (const [version, group] of versions) requireContract(group.length === 1 || group.every(file => allowedDuplicates.has(file)),
  `New duplicate migration version ${version}: ${group.join(', ')}`);
for (const [file, command] of [['scripts/accept_web.sh','pnpm accept:web'],
  ['scripts/accept_backend.sh','pnpm accept:backend'], ['scripts/accept_all.sh','pnpm accept:all']]) {
  requireContract(read(file).includes(command), `${file} must delegate to ${command}`);
}
requireContract(read('Makefile').includes('test:ui'), 'Makefile must delegate UI testing to the root command');
console.log('Rust UI repository baseline passed. Entrypoint checks do not prove product release acceptance.');
