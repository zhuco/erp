import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './StocktakeTasksPage.tsx'), 'utf8');
const mainTableStart = source.indexOf('<UiModule03MainTableSection<StocktakeTaskSummary>');
const firstModalStart = source.indexOf('<Modal', mainTableStart);
const mainTableEnd = firstModalStart === -1 ? source.length : firstModalStart;
const mainTableSource = source.slice(mainTableStart, mainTableEnd);
const detailTableStart = source.indexOf('<UiModule03MainTableSection<StocktakeTaskItem>');
const drawerEnd = source.indexOf('</Drawer>', detailTableStart);
const drawerTableSource = source.slice(detailTableStart, drawerEnd);

describe('StocktakeTasksPage 工业主表契约', () => {
  it('接入工业主表统一契约', () => {
    expect(source).toContain('UiModule03MainTableSection<StocktakeTaskSummary>');
    expect(mainTableStart).toBeGreaterThanOrEqual(0);
    expect(mainTableEnd).toBeGreaterThan(mainTableStart);
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings={true}');
    expect(source).toContain('columnSettingsKey="inventory.stocktake.tasks.table.columns"');
    expect(source).toContain('dataTestId="stocktake-tasks-main-grid"');
  });

  it('保留筛选和工具栏稳定测试标识', () => {
    expect(source).toContain('data-testid="stocktake-tasks-filter-form"');
    expect(source).toContain('data-testid="stocktake-tasks-search-button"');
    expect(source).toContain('data-testid="stocktake-tasks-reset-button"');
    expect(source).toContain('data-testid="stocktake-tasks-generate-button"');
    expect(source).toContain('data-testid="stocktake-tasks-print-button"');
  });

  it('主表使用统一工具栏插槽承载标题和主要动作', () => {
    expect(mainTableSource).toContain('toolbarLeft=');
    expect(mainTableSource).toContain('toolbarRight=');
    expect(mainTableSource).toContain('盘点任务');
    expect(mainTableSource).toContain('data-testid="stocktake-tasks-generate-button"');
    expect(mainTableSource).toContain('data-testid="stocktake-tasks-print-button"');
  });

  it('任务主表不再通过额外裸 Card 承载标题或工具栏', () => {
    const beforeMainTableSource = source.slice(0, mainTableStart);

    expect(beforeMainTableSource).not.toContain('<Typography.Title');
    expect(beforeMainTableSource).not.toContain('<Card>');
  });

  it('详情抽屉明细也使用统一工业表格并移除裸 Table/Card', () => {
    expect(detailTableStart).toBeGreaterThanOrEqual(0);
    expect(drawerTableSource).toContain('dataTestId="stocktake-tasks-detail-grid"');
    expect(drawerTableSource).toContain('gridPreset="industrial-review-compact"');
    expect(drawerTableSource).not.toContain('<Table');
    expect(source).not.toContain('<Card');
  });
});
