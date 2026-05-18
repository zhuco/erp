import { mkdir, rm, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, '..');
const scriptPath = path.join(projectRoot, 'scripts', 'assess-ui-uniformity.mjs');
const fixturePath = path.join(projectRoot, 'src', 'pages', 'tmp', 'DataTestIdFixturePage.tsx');

const runAssessment = () => {
  const result = spawnSync(process.execPath, [scriptPath, '--json'], {
    cwd: projectRoot,
    encoding: 'utf8',
  });
  assert.equal(result.status, 0, result.stderr);
  return JSON.parse(result.stdout.replace(/^﻿/, ''));
};

test('assess-ui-uniformity treats component dataTestId as stable selector', async () => {
  const before = runAssessment();
  await mkdir(path.dirname(fixturePath), { recursive: true });
  await writeFile(
    fixturePath,
    `import { UiModule03MainTableSection } from '@/components/ui-modules';

` +
      `export const DataTestIdFixturePage = () => (
` +
      `  <UiModule03MainTableSection
` +
      `    dataTestId="fixture-main-grid"
` +
      `    gridPreset="industrial-review-compact"
` +
      `    selectionCellMode="custom"
` +
      `    selectionHeaderMode="allInvert"
` +
      `    columnSettingsKey="fixture.columns"
` +
      `    showColumnSettings
` +
      `    rowKey="id"
` +
      `    columns={[]}
` +
      `    data={[]}
` +
      `    onRetry={() => undefined}
` +
      `    total={0}
` +
      `    pagination={{ current: 1, pageSize: 20 }}
` +
      `    onPaginationChange={() => undefined}
` +
      `  />
` +
      `);
`,
    'utf8',
  );

  try {
    const after = runAssessment();
    assert.ok(after.summary.dataTestIdPages >= before.summary.dataTestIdPages + 1);
  } finally {
    await rm(fixturePath, { force: true });
  }
});
