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

const recommendedTemplateTokens = [
  'WorkbenchPageTemplate',
  'SettingsPageTemplate',
  'UiModule03MainTableSection',
  'PlaceholderPage',
];

const resolveWrappedFile = async (basePath) => {
  const candidates = [
    basePath,
    `${basePath}.tsx`,
    `${basePath}.ts`,
    path.join(basePath, 'index.tsx'),
    path.join(basePath, 'index.ts'),
  ];

  for (const candidate of candidates) {
    try {
      await fs.access(candidate);
      return candidate;
    } catch {
      continue;
    }
  }

  return null;
};

const hasTemplateReference = async (source, file, visited = new Set()) => {
  if (recommendedTemplateTokens.some((token) => source.includes(token))) {
    return true;
  }

  const normalizedFile = toPosixPath(file);
  if (visited.has(normalizedFile)) {
    return false;
  }
  visited.add(normalizedFile);

  const wrapperImportPattern = /import\s+\{\s*([^}]+)\s*\}\s+from\s+['"](\.\/[^'"]+)['"];?/g;
  const wrapperMatches = Array.from(source.matchAll(wrapperImportPattern));

  for (const match of wrapperMatches) {
    const importedNames = match[1]
      .split(',')
      .map((item) => item.trim())
      .filter(Boolean)
      .map((item) => item.replace(/^type\s+/, ''))
      .map((item) => item.split(/\s+as\s+/i).pop()?.trim() ?? '')
      .filter(Boolean);

    const referencedName = importedNames.find((name) => new RegExp(`<${name}\\b`).test(source));
    if (!referencedName) {
      continue;
    }

    const wrappedFile = await resolveWrappedFile(path.resolve(path.dirname(file), match[2]));
    if (!wrappedFile) {
      continue;
    }

    try {
      const wrappedSource = await fs.readFile(wrappedFile, 'utf8');
      if (await hasTemplateReference(wrappedSource, wrappedFile, visited)) {
        return true;
      }
    } catch {
      continue;
    }
  }

  return false;
};

const strictTemplateRules = [
  {
    file: 'src/pages/channels/ChannelDirectoryPage.tsx',
    requiredTokens: ['WorkbenchPageTemplate', 'WorkbenchToolbar', 'UiModule03ContextMenu'],
    forbiddenTokens: [
      'OrderReviewWorkbenchTemplate',
      'OrderReviewWorkbenchToolbarTemplate',
      'ServerTable',
      'ColumnsType<',
      'LegacyWorkbenchListPage',
    ],
  },
  {
    file: 'src/pages/orders/OrderReviewPage.tsx',
    requiredTokens: [
      'WorkbenchPageTemplate',
      'WorkbenchToolbar',
      'OrderWorkbenchMainGridEntry',
      'variant="review"',
      'UiModule03ContextMenu',
    ],
    forbiddenTokens: ['OrderReviewWorkbenchTemplate', 'OrderReviewWorkbenchToolbarTemplate', 'ServerTable', 'LegacyWorkbenchListPage'],
  },
  {
    file: 'src/pages/orders/OrderQueryPage.tsx',
    requiredTokens: [
      'WorkbenchPageTemplate',
      'WorkbenchToolbar',
      'OrderWorkbenchMainGridEntry',
      'variant="query"',
    ],
    forbiddenTokens: ['OrderReviewWorkbenchTemplate', 'OrderReviewWorkbenchToolbarTemplate', 'ServerTable', 'LegacyWorkbenchListPage'],
  },
  {
    file: 'src/pages/orders/OrderExceptionsPage.tsx',
    requiredTokens: [
      'WorkbenchPageTemplate',
      'WorkbenchToolbar',
      'OrderWorkbenchMainGridEntry',
      'variant="recheck"',
      'UiModule03ContextMenu',
    ],
    forbiddenTokens: ['OrderReviewWorkbenchTemplate', 'OrderReviewWorkbenchToolbarTemplate', 'ServerTable', 'LegacyWorkbenchListPage'],
  },
  {
    file: 'src/pages/sales/SalesSettingsPage.tsx',
    requiredTokens: ['SettingsPageTemplate', 'SettingsPageSection'],
    forbiddenTokens: ['UiModule07SettingsLayout', 'UiModule07SettingsSection'],
  },
  {
    file: 'src/pages/system/SystemSettingsPage.tsx',
    requiredTokens: ['SettingsPageTemplate', 'SettingsPageSection'],
    forbiddenTokens: ['UiModule07SettingsLayout', 'UiModule07SettingsSection'],
  },
];

const forbiddenLegacyTemplateTokens = [
  'WorkbenchMainTableTemplate',
  'IndustrialListWorkbenchTemplate',
  'LegacyWorkbenchListPage',
];

const forbiddenPageTemplateImportTokens = [
  "from '@/components/workbench/templates",
  'from "@/components/workbench/templates',
  "from '@/components/ui-modules/UiModule07SettingsLayout",
  'from "@/components/ui-modules/UiModule07SettingsLayout',
  'UiModule07SettingsLayout',
  'UiModule07SettingsSection',
  'OrderReviewWorkbenchTemplate',
  'OrderReviewWorkbenchToolbarTemplate',
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
  if (!(await hasTemplateReference(source, file))) {
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
  const relativeFile = toPosixPath(path.relative(projectRoot, file));
  const hits = forbiddenLegacyTemplateTokens.filter((token) => source.includes(token));
  const importHits = relativeFile.startsWith('src/pages/')
    ? forbiddenPageTemplateImportTokens.filter((token) => source.includes(token))
    : [];
  const allHits = [...hits, ...importHits];
  if (!allHits.length) {
    continue;
  }
  legacyTemplateViolations.push({
    file: relativeFile,
    hits: allHits,
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
