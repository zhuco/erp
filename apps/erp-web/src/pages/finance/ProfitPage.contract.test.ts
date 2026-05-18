import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ProfitPage.tsx'), 'utf8');

describe('ProfitPage 工业主表契约', () => {
  it('将主表标题与主操作迁入统一主表工具栏', () => {
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
    expect(source).toContain('利润分析');
    expect(source).toContain('刷新');
  });

  it('筛选表单由主表 header 承载且不再使用裸 Card', () => {
    expect(source).toContain('header={header}');
    expect(source).toContain('const header = (');
    expect(source).toContain('<Form form={form} layout="inline" onFinish={handleSearch}>');
    expect(source).not.toContain('Card');
    expect(source).not.toContain('<Card');
  });

  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和列设置入口', () => {
    expect(source).toContain('dataTestId="finance-profit-grid"');
    expect(source).toContain('columnSettingsKey="finance.profit.table.columns"');
    expect(source).toContain('showColumnSettings');
  });
});
