import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './AfterSalesSlaPage.tsx'), 'utf8');
const mainTableSection = source.match(
  /<UiModule03MainTableSection<AfterSalesSlaRule>[\s\S]*?\/>/u
)?.[0];

describe('AfterSalesSlaPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('UiModule03MainTableSection<AfterSalesSlaRule>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('dataTestId="after-sales-sla-main-grid"');
    expect(source).toContain('columnSettingsKey="after-sales.sla.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('主表标题说明与主操作显式迁入统一工具栏插槽', () => {
    expect(mainTableSection).toContain('toolbarLeft={toolbarLeft}');
    expect(mainTableSection).toContain('toolbarRight={toolbarRight}');
    expect(source).toMatch(/const toolbarLeft = \([\s\S]*after-sales-sla-title[\s\S]*售后 SLA 配置[\s\S]*SLA/u);
    expect(source).toMatch(/const toolbarRight = \([\s\S]*刷新[\s\S]*after-sales-sla-create[\s\S]*新增规则/u);
  });

  it('筛选表单由主表 header 承载且不再使用额外裸 Card 包裹', () => {
    expect(mainTableSection).toContain('header={header}');
    expect(source).toContain('<Form form={form} layout="vertical" onFinish={handleSearch}>');
    expect(source).not.toContain('Card,');
    expect(source).not.toContain('<Card>');
  });
});
