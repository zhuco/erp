import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const source = readFileSync(resolve(__dirname, './StockCountsPage.tsx'), 'utf8');

describe('StockCountsPage 工业主表契约', () => {
  it('主盘点任务列表接入工业表格契约', () => {
    expect(source).toContain('UiModule03MainTableSection<StockCountSummary>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings={true}');
    expect(source).toContain('columnSettingsKey="wms.stock-counts.table.columns"');
    expect(source).toContain('dataTestId="stock-counts-main-grid"');
  });

  it('主操作统一收口到主表工具栏插槽', () => {
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
  });

  it('不再使用裸 Card 或裸 Table 组织页面区块', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });

  it('保留新建盘点任务按钮的稳定测试标识', () => {
    expect(source).toContain('data-testid="stock-counts-create-button"');
  });
});