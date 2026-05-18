import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './CostingPage.tsx'), 'utf8');
const mainTableSection =
  source.match(/<UiModule03MainTableSection<InventoryCostingRun>[\s\S]*?\/>/)?.[0] ?? '';
const toolbarLeftSource = source.slice(
  source.indexOf('const toolbarLeft = ('),
  source.indexOf('const toolbarRight = (')
);
const toolbarRightSource = source.slice(
  source.indexOf('const toolbarRight = ('),
  source.indexOf('const filterForm = (')
);

describe('CostingPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和列设置入口', () => {
    expect(source).toContain('dataTestId="finance-costing-main-grid"');
    expect(source).toContain('columnSettingsKey="finance.costing.table.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('主表标题说明和主操作接入统一工具栏插槽', () => {
    expect(source).toContain('const toolbarLeft = (');
    expect(source).toContain('const toolbarRight = (');
    expect(toolbarLeftSource).toContain('成本核算与库存成本');
    expect(toolbarLeftSource).toContain('核算运行、期间库存成本与差异结果跟踪');
    expect(toolbarRightSource).toContain('刷新');
    expect(toolbarRightSource).toContain('导出');
    expect(toolbarRightSource).toContain('新建核算');
    expect(mainTableSection).toContain('toolbarLeft={toolbarLeft}');
    expect(mainTableSection).toContain('toolbarRight={toolbarRight}');
    expect(mainTableSection).not.toContain('header={header}');
  });

  it('筛选表单由主表 header 承载且不再包裹额外裸 Card', () => {
    expect(source).toContain('const filterForm = (');
    expect(mainTableSection).toContain('header={filterForm}');
    expect(source).not.toContain('const header = (');
    expect(source).not.toMatch(/<Card(?:\s|>)/);
    expect(source).not.toMatch(/<\/Card>/);
  });
});
