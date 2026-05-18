import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './InventoryCostsPage.tsx'), 'utf8');

describe('InventoryCostsPage 工业工作台契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('UiModule03MainTableSection<InventoryCostRecord>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和列设置入口', () => {
    expect(source).toContain('dataTestId="inventory-costs-main-grid"');
    expect(source).toContain('columnSettingsKey="inventory.costs.table.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('主表标题、说明和主操作统一进入工具栏插槽', () => {
    expect(source).toContain('const toolbarLeft = (');
    expect(source).toContain('const toolbarRight = (');
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
    expect(source).toContain('库存成本');
    expect(source).toContain('返回库存');
  });

  it('筛选区迁入主表 header 且不再使用裸 Card', () => {
    expect(source).toContain('const tableHeader = (');
    expect(source).toContain('header={tableHeader}');
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });
});
