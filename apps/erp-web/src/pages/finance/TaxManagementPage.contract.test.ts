import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './TaxManagementPage.tsx'), 'utf8');

describe('TaxManagementPage 工业主表契约', () => {
  const mainTableSectionCount = (source.match(/<UiModule03MainTableSection\b/g) ?? []).length;

  it('保留两个主表模块入口', () => {
    expect(mainTableSectionCount).toBe(2);
  });

  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('dataTestId="finance-tax-main-grid"');
    expect(source).toContain('columnSettingsKey="finance.tax.table.columns"');
    expect(source).toContain('dataTestId="finance-invoice-adjustments-main-grid"');
    expect(source).toContain('columnSettingsKey="finance.invoice.adjustments.table.columns"');
  });

  it('通过统一主表工具栏与筛选槽位承载主操作', () => {
    expect(source).toContain('toolbarLeft=');
    expect(source).toContain('toolbarRight=');
    expect(source).toMatch(/(?:header|actionBar)=/);
  });

  it('禁止回退到裸 Card 或裸 Table 容器', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });
});
