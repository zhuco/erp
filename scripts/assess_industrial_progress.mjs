import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '..');
const defaultOutputPath = path.join(repoRoot, 'tmp', 'industrial-progress', 'industrial-progress-latest.json');

function fail(message) {
  console.error(`[industrial-progress] ${message}`);
  process.exit(1);
}

function parseArgs(argv) {
  let outPath = defaultOutputPath;
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] !== '--out') continue;
    const value = argv[index + 1];
    if (!value || value.startsWith('--')) fail('参数 --out 需要提供输出路径');
    outPath = path.resolve(repoRoot, value);
    index += 1;
  }
  return { outPath };
}

function relative(filePath) {
  return path.relative(repoRoot, filePath).replace(/\\/g, '/');
}

function declaredGate(scripts, scriptKey, platforms) {
  return {
    status: 'unknown',
    reason: '本报告未运行该门禁，不能根据脚本或文件存在推断通过',
    script_key: scriptKey,
    command: scripts[scriptKey] ?? null,
    platforms,
  };
}

function main() {
  const { outPath } = parseArgs(process.argv.slice(2));
  const packagePath = path.join(repoRoot, 'package.json');
  const roadmapPath = path.join(repoRoot, 'docs', 'ROADMAP.md');
  const nativeHost = path.join(repoRoot, 'apps', 'erp-ui', 'src', 'main.rs');
  const sharedUi = path.join(repoRoot, 'apps', 'erp-ui', 'src', 'lib.rs');
  const renderer = path.join(repoRoot, 'crates', 'erp-ui-egui', 'src', 'lib.rs');

  for (const [filePath, label] of [
    [packagePath, 'package.json'],
    [roadmapPath, '路线图'],
    [nativeHost, 'Rust native host'],
    [sharedUi, 'Rust shared UI'],
    [renderer, 'egui renderer'],
  ]) {
    if (!fs.existsSync(filePath)) fail(`缺少 ${label}: ${relative(filePath)}`);
  }

  const scripts = JSON.parse(fs.readFileSync(packagePath, 'utf8')).scripts ?? {};
  const report = {
    generated_at: new Date().toISOString(),
    roadmap: relative(roadmapPath),
    ui_architecture: {
      native_host: relative(nativeHost),
      shared_ui: relative(sharedUi),
      renderer: relative(renderer),
      legacy_web_metrics: 'retired; excluded from Rust progress',
    },
    gates: {
      native_typecheck: declaredGate(scripts, 'typecheck', ['native']),
      native_clippy: declaredGate(scripts, 'lint', ['native']),
      native_tests: declaredGate(scripts, 'test:ui', ['native']),
      wasm_acceptance: declaredGate(scripts, 'accept:web', ['wasm32', 'browser-package']),
      backend_acceptance: declaredGate(scripts, 'accept:backend', ['native']),
      repository_policy: declaredGate(scripts, 'repo:verify', ['repository']),
    },
  };

  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(`[industrial-progress] wrote report: ${relative(outPath)}`);
  console.log('[industrial-progress] gate results: unknown (checks were not run)');
}

main();
