import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './WmsReplenishmentPage.tsx'), 'utf8');

describe('WmsReplenishmentPage 工业主表契约', () => {
  it('统一使用 UiModule03 工业主表壳', () => {
    expect(source).toContain('UiModule03MainTableSection<WmsReplenishmentTask>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('dataTestId="wms-replenishment-main-grid"');
    expect(source).toContain('columnSettingsKey="wms.replenishment.table.columns"');
  });
});
