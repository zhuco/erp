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
const fixturePath = path.join(projectRoot, 'src', 'pages', 'tmp', 'ToolbarSlotsFixturePage.tsx');

const runAssessment = () => {
  const result = spawnSync(process.execPath, [scriptPath, '--json'], {
    cwd: projectRoot,
    encoding: 'utf8',
  });
  assert.equal(result.status, 0, result.stderr);
  return JSON.parse(result.stdout.replace(/^﻿/, ''));
};

test('assess-ui-uniformity treats UiModule03 toolbar slots as unified WorkbenchToolbar usage', async () => {
  const before = runAssessment();
  await mkdir(path.dirname(fixturePath), { recursive: true });
  await writeFile(
    fixturePath,
    `import { Button } from 'antd';
` +
      `import { UiModule03MainTableSection } from '@/components/ui-modules';

` +
      `export const ToolbarSlotsFixturePage = () => (
` +
      `  <UiModule03MainTableSection
` +
      `    toolbarLeft={<Button>查询</Button>}
` +
      `    dataTestId="toolbar-slots-main-grid"
` +
      `    gridPreset="industrial-review-compact"
` +
      `    selectionCellMode="custom"
` +
      `    selectionHeaderMode="allInvert"
` +
      `    columnSettingsKey="toolbar.slots.columns"
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
    assert.ok(after.summary.workbenchToolbarPages >= before.summary.workbenchToolbarPages + 1);
  } finally {
    await rm(fixturePath, { force: true });
  }
});
