import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ExpenseAllocationPage.tsx'), 'utf8');

describe('ExpenseAllocationPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和列设置入口', () => {
    expect(source).toContain('dataTestId="finance-expenses-main-grid"');
    expect(source).toContain('columnSettingsKey="finance.expenses.table.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('将主表标题与主操作迁入 UiModule03MainTableSection 工具栏', () => {
    expect(source).toContain('toolbarLeft={tableToolbarLeft}');
    expect(source).toContain('toolbarRight={tableToolbarRight}');
    expect(source).not.toContain('header={header}');
  });

  it('筛选表单由 actionBar 承载且不再使用裸 Card 包裹', () => {
    expect(source).toContain('actionBar={tableActionBar}');
    expect(source).not.toContain('Card,');
    expect(source).not.toContain('<Card>');
    expect(source).not.toContain('</Card>');
  });
});
