import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './WmsPickTasksPage.tsx'), 'utf8');

describe('WmsPickTasksPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('UiModule03MainTableSection<WmsPickTask>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('dataTestId="wms-pick-tasks-main-grid"');
    expect(source).toContain('columnSettingsKey="wms.pick-tasks.table.columns"');
    expect(source).toContain('showColumnSettings');
  });
});
