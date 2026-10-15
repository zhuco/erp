import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(process.argv[2] ?? path.resolve(import.meta.dirname, '..'));
const allowed = new Set(['MIT', 'Apache-2.0', 'BSD-3-Clause', 'ISC', 'MPL-2.0', 'Unicode-3.0', 'OFL-1.1', 'Proprietary']);
const banned = /@ag-grid-|ag-grid/i;
const owned = new Set(['erp-ui', 'erp-grid-core', 'erp-ui-egui', 'erp-domain', 'erp-repo', 'erp-api', 'erp-worker', 'erp-types', 'erp-adapter', '@ag-grid-community']);
const exemptions = new Map();

function fail(message) {
  console.error(message);
  process.exitCode = 1;
}

function readText(file) {
  return fs.readFileSync(file, 'utf8').replace(/^\uFEFF/, '');
}

function loadExemptions() {
  const file = path.join(root, 'docs', 'license-exemptions.json');
  if (!fs.existsSync(file)) return;
  const today = new Date().toISOString().slice(0, 10);
  for (const row of JSON.parse(readText(file))) {
    if (!row.name || !row.scope || !row.expires || row.expires < today) {
      fail(`License exemption is missing a name, scope, or unexpired date: ${JSON.stringify(row)}`);
      continue;
    }
    exemptions.set(row.name, row);
  }
}

function cargoRows() {
  const blocks = readText(path.join(root, 'Cargo.lock')).split(/\r?\n\[\[package\]\]\r?\n/).slice(1);
  return blocks.flatMap(block => {
    const name = /^name = "([^"]+)"/m.exec(block)?.[1] ?? '';
    const version = /^version = "([^"]+)"/m.exec(block)?.[1] ?? '';
    if (!name) return [];
    const manifest = path.join(root, 'crates', name, 'Cargo.toml');
    const appManifest = path.join(root, 'apps', name, 'Cargo.toml');
    const chosen = fs.existsSync(manifest) ? manifest : fs.existsSync(appManifest) ? appManifest : '';
    const workspaceLicense = fs.existsSync(path.join(root, 'Cargo.toml'))
      ? /^license\s*=\s*"([^"]+)"/m.exec(readText(path.join(root, 'Cargo.toml')))?.[1] ?? '' : '';
    const declared = chosen ? /^license(?:\.workspace)?\s*=\s*"([^"]+)"/m.exec(readText(chosen))?.[1] ?? '' : '';
    const usesWorkspace = chosen ? /^license\.workspace\s*=\s*true/m.test(readText(chosen)) : false;
    const license = declared || (usesWorkspace ? workspaceLicense : '');
    return [{ source: 'Cargo.lock', name, version, license, owned: owned.has(name) }];
  });
}

function packageRows() {
  const file = path.join(root, 'package.json');
  if (!fs.existsSync(file)) return [];
  const manifest = JSON.parse(readText(file));
  const declared = manifest.license && allowed.has(manifest.license) ? manifest.license : '';
  return Object.entries({ ...manifest.dependencies, ...manifest.devDependencies }).map(([name, version]) => ({
    source: 'package.json', name, version: String(version), license: declared, owned: true,
  }));
}

function fontRows() {
  const directory = path.join(root, 'apps', 'erp-ui', 'assets', 'fonts');
  if (!fs.existsSync(directory)) return [];
  const names = fs.readdirSync(directory);
  const licenseFile = names.find(name => name.startsWith('LICENSE'));
  const license = licenseFile && readText(path.join(directory, licenseFile)).includes('SIL OPEN FONT LICENSE') ? 'OFL-1.1' : '';
  return names.filter(name => name.endsWith('.otf') || name.endsWith('.ttf')).map(name => ({
    source: 'font', name, version: 'file', license, owned: true,
  }));
}

loadExemptions();
const rows = [...cargoRows(), ...packageRows(), ...fontRows()];
const report = ['source\tname\tversion\tlicense\tresult'];
for (const row of rows) {
  if (!row.name || !row.version) {
    fail(`Dependency row is missing a name or version: ${row.source}`);
    continue;
  }
  if (banned.test(row.name) && !exemptions.has(row.name)) {
    fail(`Banned dependency has no current exemption: ${row.source} ${row.name}`);
    report.push(`${row.source}\t${row.name}\t${row.version}\t${row.license}\tBANNED`);
    continue;
  }
  if (row.owned && !allowed.has(row.license)) {
    fail(`Owned dependency license is missing or not allowed: ${row.source} ${row.name} ${row.license || 'MISSING'}`);
    report.push(`${row.source}\t${row.name}\t${row.version}\t${row.license || 'MISSING'}\tMISSING`);
    continue;
  }
  const result = row.owned ? 'OK' : 'UNDECLARED';
  report.push(`${row.source}\t${row.name}\t${row.version}\t${row.license}\t${result}`);
}
const destination = path.join(root, 'output', 'license-report.tsv');
fs.mkdirSync(path.dirname(destination), { recursive: true });
fs.writeFileSync(destination, `${report.join('\n')}\n`);
const ownedCount = rows.filter(row => row.owned).length;
if (process.exitCode) console.error(`Dependency license gate failed. Report: ${destination}`);
else console.log(`Dependency license gate passed (${ownedCount} owned rows, ${rows.length} scanned). Report: ${destination}`);
