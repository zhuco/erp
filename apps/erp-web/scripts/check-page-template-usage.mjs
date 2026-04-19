/* eslint-env node */

import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, '..');
const pagesRoot = path.join(projectRoot, 'src', 'pages');
const baselineFile = path.join(__dirname, 'page-template-baseline.json');

const args = new Set(process.argv.slice(2));
const shouldWriteBaseline = args.has('--write-baseline');

const toPosixPath = (value) => value.split(path.sep).join('/');
const srcRoot = path.join(projectRoot, 'src');

const hasTemplateReference = (source) =>
  source.includes("from '@/components/workbench/templates'") ||
  source.includes('from "@/components/workbench/templates"') ||
  source.includes("from '@/components/workbench/OrderReviewWorkbenchListPage'") ||
  source.includes('from "@/components/workbench/OrderReviewWorkbenchListPage"') ||
  source.includes('UiModule03MainTableSection');

const strictTemplateRules = [
  {
    file: 'src/pages/channels/ChannelsPage.tsx',
    requiredTokens: [
      'OrderReviewWorkbenchTemplate',
      'OrderWorkbenchMainGridEntry',
      'variant="review"',
      'UiModule03ContextMenu',
      'toolbarPrimaryActions',
    ],
    forbiddenTokens: [
      'ServerTable',
      'ColumnsType<',
      'LegacyWorkbenchListPage',
      'variant="query"',
    ],
  },
  {
    file: 'src/pages/orders/OrderReviewPage.tsx',
    requiredTokens: [
      'OrderReviewWorkbenchTemplate',
      'OrderWorkbenchMainGridEntry',
      'variant="review"',
      'UiModule03ContextMenu',
    ],
    forbiddenTokens: ['ServerTable', 'LegacyWorkbenchListPage'],
  },
  {
    file: 'src/pages/orders/OrderQueryPage.tsx',
    requiredTokens: [
      'OrderReviewWorkbenchTemplate',
      'OrderWorkbenchMainGridEntry',
      'variant="query"',
      'OrderReviewWorkbenchToolbarTemplate',
    ],
    forbiddenTokens: ['ServerTable', 'LegacyWorkbenchListPage'],
  },
  {
    file: 'src/pages/orders/OrderExceptionsPage.tsx',
    requiredTokens: [
      'OrderReviewWorkbenchTemplate',
      'OrderWorkbenchMainGridEntry',
      'variant="recheck"',
      'UiModule03ContextMenu',
    ],
    forbiddenTokens: ['ServerTable', 'LegacyWorkbenchListPage'],
  },
];

const forbiddenLegacyTemplateTokens = [
  'WorkbenchMainTableTemplate',
  'IndustrialListWorkbenchTemplate',
  'LegacyWorkbenchListPage',
];

const collectPageFiles = async (dir) => {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const results = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      const nested = await collectPageFiles(fullPath);
      results.push(...nested);
      continue;
    }

    if (entry.isFile() && entry.name.endsWith('Page.tsx')) {
      results.push(fullPath);
    }
  }

  return results;
};

const collectSourceFiles = async (dir) => {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const results = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      const nested = await collectSourceFiles(fullPath);
      results.push(...nested);
      continue;
    }

    if (entry.isFile() && (entry.name.endsWith('.ts') || entry.name.endsWith('.tsx'))) {
      results.push(fullPath);
    }
  }

  return results;
};

const readBaseline = async () => {
  try {
    const raw = await fs.readFile(baselineFile, 'utf8');
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      throw new Error('baseline must be an array');
    }
    return parsed.filter((item) => typeof item === 'string').sort();
  } catch (error) {
    if (error && typeof error === 'object' && 'code' in error && error.code === 'ENOENT') {
      return [];
    }
    throw error;
  }
};

const writeBaseline = async (items) => {
  const payload = `${JSON.stringify(items, null, 2)}\n`;
  await fs.writeFile(baselineFile, payload, 'utf8');
};

const pageFiles = await collectPageFiles(pagesRoot);
const allMissing = [];

for (const file of pageFiles) {
  const source = await fs.readFile(file, 'utf8');
  if (!hasTemplateReference(source)) {
    allMissing.push(toPosixPath(path.relative(projectRoot, file)));
  }
}

allMissing.sort();

if (shouldWriteBaseline) {
  await writeBaseline(allMissing);
  console.log(
    `[page-template] baseline updated: ${allMissing.length} files -> ${toPosixPath(path.relative(projectRoot, baselineFile))}`,
  );
  process.exit(0);
}

const baseline = await readBaseline();
const baselineSet = new Set(baseline);
const missingWithoutBaseline = allMissing.filter((item) => !baselineSet.has(item));
const staleBaseline = baseline.filter((item) => !allMissing.includes(item));

const strictViolations = [];
for (const rule of strictTemplateRules) {
  const fullPath = path.join(projectRoot, rule.file);
  let source = '';
  try {
    source = await fs.readFile(fullPath, 'utf8');
  } catch (error) {
    strictViolations.push({
      file: toPosixPath(rule.file),
      missingTokens: [],
      forbiddenHits: [],
      readError: error instanceof Error ? error.message : String(error),
    });
    continue;
  }

  const missingTokens = rule.requiredTokens.filter((token) => !source.includes(token));
  const forbiddenHits = rule.forbiddenTokens.filter((token) => source.includes(token));
  if (missingTokens.length || forbiddenHits.length) {
    strictViolations.push({
      file: toPosixPath(rule.file),
      missingTokens,
      forbiddenHits,
    });
  }
}

const legacyTemplateViolations = [];
const sourceFiles = await collectSourceFiles(srcRoot);
for (const file of sourceFiles) {
  const source = await fs.readFile(file, 'utf8');
  const hits = forbiddenLegacyTemplateTokens.filter((token) => source.includes(token));
  if (!hits.length) {
    continue;
  }
  legacyTemplateViolations.push({
    file: toPosixPath(path.relative(projectRoot, file)),
    hits,
  });
}

if (
  missingWithoutBaseline.length ||
  staleBaseline.length ||
  strictViolations.length ||
  legacyTemplateViolations.length
) {
  if (missingWithoutBaseline.length) {
    console.error('[page-template] files missing template reference (not in baseline):');
    missingWithoutBaseline.forEach((item) => console.error(`  - ${item}`));
  }

  if (staleBaseline.length) {
    console.error('[page-template] baseline contains stale entries (now compliant or removed):');
    staleBaseline.forEach((item) => console.error(`  - ${item}`));
    console.error(
      `[page-template] run: node ./scripts/check-page-template-usage.mjs --write-baseline`,
    );
  }

  if (strictViolations.length) {
    console.error('[page-template] strict template rule violations:');
    strictViolations.forEach((violation) => {
      console.error(`  - ${violation.file}`);
      if (violation.readError) {
        console.error(`      read error: ${violation.readError}`);
        return;
      }
      if (violation.missingTokens.length) {
        console.error(`      missing: ${violation.missingTokens.join(', ')}`);
      }
      if (violation.forbiddenHits.length) {
        console.error(`      forbidden: ${violation.forbiddenHits.join(', ')}`);
      }
    });
  }

  if (legacyTemplateViolations.length) {
    console.error('[page-template] legacy template tokens detected (strong-constraint violation):');
    legacyTemplateViolations.forEach((violation) => {
      console.error(`  - ${violation.file}`);
      console.error(`      tokens: ${violation.hits.join(', ')}`);
    });
  }

  process.exit(1);
}

console.log(
  `[page-template] ok. checked ${pageFiles.length} pages, baseline exceptions: ${baseline.length}`,
);
