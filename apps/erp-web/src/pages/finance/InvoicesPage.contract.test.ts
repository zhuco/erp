import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './InvoicesPage.tsx'), 'utf8');

describe('InvoicesPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('UiModule03MainTableSection<Invoice>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('dataTestId="finance-invoices-main-grid"');
    expect(source).toContain('columnSettingsKey="finance.invoices.table.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('使用统一工具栏或筛选 header', () => {
    expect(source).toContain('header={header}');
    expect(source).toContain('toolbarLeft');
  });

  it('禁止页面级裸 Card/Table', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });
});
