import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ApDocsPage.tsx'), 'utf8');

describe('ApDocsPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和关键操作标识', () => {
    expect(source).toContain('dataTestId="finance-ap-docs-main-grid"');
    expect(source).toContain('columnSettingsKey="finance-ap-docs.table.columns"');
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('data-testid="finance-ap-docs-detail-button"');
  });

  it('将主表标题和主操作迁入统一主表工具栏槽位', () => {
    expect(source).toContain('toolbarLeft={mainTableToolbarLeft}');
    expect(source).toContain('toolbarRight={mainTableToolbarRight}');
    expect(source).toContain('actionBar={filtersActionBar}');
    expect(source).not.toContain('<Card');
  });
});
