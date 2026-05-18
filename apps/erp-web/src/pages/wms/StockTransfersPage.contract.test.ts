import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const source = readFileSync(resolve(__dirname, './StockTransfersPage.tsx'), 'utf8');

describe('StockTransfersPage 工业主表契约', () => {
  it('主调拨列表接入工业表格契约', () => {
    expect(source).toContain('UiModule03MainTableSection<StockTransferSummary>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings={true}');
    expect(source).toContain('columnSettingsKey="wms.stock-transfers.table.columns"');
    expect(source).toContain('dataTestId="stock-transfers-main-grid"');
  });

  it('主操作统一收敛到主表工具栏插槽', () => {
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
    expect(source).toMatch(/const toolbarLeft = \(/);
    expect(source).toMatch(/const toolbarRight = \(/);
  });

  it('明细表格使用统一底部表格和工业数据网格', () => {
    expect(source).toContain('UiModule04BottomTable');
    expect(source).toContain('ErpDataGrid');
    expect(source).toContain('dataTestId="stock-transfer-detail-items-grid"');
  });

  it('禁止页面遗留裸 Card 和裸 Table', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });

  it('保留新建调拨单按钮的稳定测试标识', () => {
    expect(source).toContain('data-testid="stock-transfers-create-button"');
  });
});
