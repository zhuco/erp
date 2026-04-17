import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import YAML from 'yaml';

const currentFilePath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(currentFilePath);
const repoRoot = path.resolve(scriptDir, '..');

const defaultOutputPath = path.join(
  repoRoot,
  'docs',
  'reports',
  '_generated',
  'industrial-progress-latest.json',
);

function fail(message) {
  console.error(`[industrial-progress] ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  let outPath = defaultOutputPath;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--out') {
      const next = argv[i + 1];
      if (!next || next.startsWith('--')) {
        fail('参数 --out 需要提供输出路径');
      }
      outPath = path.resolve(repoRoot, next);
      i += 1;
      continue;
    }
  }

  return { outPath };
}

function ensureFileExists(filePath, label) {
  if (!fs.existsSync(filePath)) {
    fail(`缺少 ${label}: ${path.relative(repoRoot, filePath)}`);
  }
}

function listFilesRecursively(rootDir, matcher) {
  if (!fs.existsSync(rootDir)) {
    return [];
  }

  const result = [];
  const entries = fs.readdirSync(rootDir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(rootDir, entry.name);
    if (entry.isDirectory()) {
      result.push(...listFilesRecursively(fullPath, matcher));
      continue;
    }
    if (entry.isFile() && matcher(fullPath)) {
      result.push(fullPath);
    }
  }
  return result;
}

function countWebPagesTsx() {
  const pagesDir = path.join(repoRoot, 'apps', 'erp-web', 'src', 'pages');
  return listFilesRecursively(pagesDir, (filePath) => filePath.endsWith('.tsx')).length;
}

function countWebApiModulesTs() {
  const apiDir = path.join(repoRoot, 'apps', 'erp-web', 'src', 'api');
  return listFilesRecursively(apiDir, (filePath) => (
    filePath.endsWith('.ts')
    && !filePath.endsWith('.test.ts')
    && !filePath.endsWith('.d.ts')
  )).length;
}

function countErpApiTestsRs() {
  const testsDir = path.join(repoRoot, 'crates', 'erp-api', 'tests');
  return listFilesRecursively(testsDir, (filePath) => filePath.endsWith('.rs')).length;
}

function countOpenApiV1Paths() {
  const openApiPath = path.join(repoRoot, 'openapi', 'erp_api_v1.yaml');
  ensureFileExists(openApiPath, 'OpenAPI 文件');
  const content = fs.readFileSync(openApiPath, 'utf8');
  const doc = YAML.parse(content);
  const paths = doc?.paths;
  if (!paths || typeof paths !== 'object') {
    return 0;
  }
  return Object.keys(paths).filter((pathKey) => pathKey.startsWith('/v1/')).length;
}

function buildGateStatuses() {
  const packageJsonPath = path.join(repoRoot, 'package.json');
  ensureFileExists(packageJsonPath, 'package.json');
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
  const scripts = packageJson.scripts ?? {};

  const backendGateScriptPath = path.join(repoRoot, 'scripts', 'release_gate_backend.mjs');
  const backendGateScriptExists = fs.existsSync(backendGateScriptPath);

  function gateFromScript(scriptKey, fallbackLabel) {
    const command = scripts[scriptKey];
    return {
      status: command ? 'ready' : 'missing',
      script_key: scriptKey,
      command: command ?? '',
      label: fallbackLabel,
    };
  }

  const repoVerify = gateFromScript('repo:verify', 'repo verify baseline');
  const acceptWeb = gateFromScript('accept:web', 'web acceptance');
  const backendPrecheck = gateFromScript(
    'test:backend:release-gate',
    'backend release gate precheck',
  );

  if (backendPrecheck.status === 'ready' && !backendGateScriptExists) {
    backendPrecheck.status = 'partial';
  }
  backendPrecheck.script_file = path.relative(repoRoot, backendGateScriptPath).replace(/\\/g, '/');
  backendPrecheck.script_file_exists = backendGateScriptExists;

  return {
    repo_verify: repoVerify,
    accept_web: acceptWeb,
    backend_release_gate_precheck: backendPrecheck,
  };
}

function countPlaceholderReferences() {
  const routerPath = path.join(repoRoot, 'apps', 'erp-web', 'src', 'app', 'router.tsx');
  ensureFileExists(routerPath, 'router.tsx');
  const content = fs.readFileSync(routerPath, 'utf8');

  const importLines = content.match(/^import\s+.+$/gim) ?? [];
  const importCount = importLines.filter((line) => /placeholder/i.test(line)).length;
  const totalMentions = (content.match(/placeholder/gi) ?? []).length;
  const usageCount = Math.max(totalMentions - importCount, 0);

  return {
    import_count: importCount,
    usage_count: usageCount,
    total_mentions: totalMentions,
    source: 'apps/erp-web/src/app/router.tsx',
  };
}

function ensureParentDir(filePath) {
  const dir = path.dirname(filePath);
  fs.mkdirSync(dir, { recursive: true });
}

function main() {
  const { outPath } = parseArgs(process.argv.slice(2));

  const report = {
    generated_at: new Date().toISOString(),
    repo_root: repoRoot.replace(/\\/g, '/'),
    metrics: {
      web_pages_tsx: countWebPagesTsx(),
      web_api_modules_ts: countWebApiModulesTs(),
      erp_api_tests_rs: countErpApiTestsRs(),
      openapi_v1_paths: countOpenApiV1Paths(),
    },
    gates: buildGateStatuses(),
    placeholder_route_references: countPlaceholderReferences(),
  };

  ensureParentDir(outPath);
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');

  const relativeOutPath = path.relative(repoRoot, outPath).replace(/\\/g, '/');
  console.log(`[industrial-progress] wrote report: ${relativeOutPath}`);
  console.log(`[industrial-progress] metrics: ${JSON.stringify(report.metrics)}`);
}

main();
