import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const source = readFileSync(resolve(__dirname, './StocktakesPage.tsx'), 'utf8');
const mainTableStart = source.indexOf('<UiModule03MainTableSection<StockCountSummary>');
const mainTableEnd = source.indexOf('<Modal', mainTableStart);
const mainTableSource =
  mainTableStart >= 0 && mainTableEnd > mainTableStart
    ? source.slice(mainTableStart, mainTableEnd)
    : '';
const mainTableActionBarStart = source.indexOf('const mainTableActionBar = (');
const mainTableActionBarEnd = source.indexOf('  return (', mainTableActionBarStart);
const mainTableActionBarSource =
  mainTableActionBarStart >= 0 && mainTableActionBarEnd > mainTableActionBarStart
    ? source.slice(mainTableActionBarStart, mainTableActionBarEnd)
    : '';

describe('StocktakesPage 工业主表契约', () => {
  it('主盘点列表接入工业表格契约', () => {
    expect(source).toContain('UiModule03MainTableSection<StockCountSummary>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings={true}');
    expect(source).toContain('columnSettingsKey="stocktakes.table.columns"');
    expect(source).toContain('dataTestId="stocktakes-main-grid"');
  });

  it('主盘点列表通过统一插槽承载标题和工具栏', () => {
    expect(mainTableStart).toBeGreaterThanOrEqual(0);
    expect(mainTableEnd).toBeGreaterThan(mainTableStart);
    expect(mainTableSource).toContain('toolbarLeft={mainTableToolbarLeft}');
    expect(mainTableSource).toContain('toolbarRight={mainTableToolbarRight}');
    expect(mainTableSource).toContain('header={mainTableHeader}');
    expect(mainTableSource).toContain('actionBar={mainTableActionBar}');
    expect(source).not.toContain('<Card title="盘点单列表">');
    expect(source).not.toContain('<Card title="筛选">');
  });

  it('主表筛选表单不再由额外裸 Card 承载', () => {
    expect(mainTableActionBarStart).toBeGreaterThanOrEqual(0);
    expect(mainTableActionBarEnd).toBeGreaterThan(mainTableActionBarStart);
    expect(mainTableActionBarSource).toContain('data-testid="stocktakes-filter-form"');
    expect(mainTableActionBarSource).toContain('data-testid="stocktakes-search-button"');
    expect(mainTableActionBarSource).toContain('data-testid="stocktakes-reset-button"');
    expect(mainTableActionBarSource).not.toContain('<Card');
  });

  it('页面不再遗留裸 Card 和裸 Table', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });

  it('保留高频操作的稳定测试标识', () => {
    expect(source).toContain('data-testid="stocktakes-create-button"');
    expect(source).toContain('data-testid="stocktakes-import-button"');
    expect(source).toContain('data-testid="stocktakes-export-button"');
    expect(source).toContain('data-testid="stocktakes-filter-form"');
    expect(source).toContain('data-testid="stocktakes-search-button"');
    expect(source).toContain('data-testid="stocktakes-reset-button"');
  });
});
