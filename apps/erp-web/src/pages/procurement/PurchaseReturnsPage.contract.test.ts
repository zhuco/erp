import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './PurchaseReturnsPage.tsx'), 'utf8');

const getMainTableOpeningTag = () => {
  const tableStart = source.indexOf('<UiModule03MainTableSection<PurchaseReturnSummary>');
  const modalStart = source.indexOf('      <Modal', tableStart);
  if (tableStart < 0 || modalStart < 0) {
    throw new Error('未找到采购退货主表 UiModule03MainTableSection 调用');
  }
  return source.slice(tableStart, modalStart);
};

const getHeaderBlock = () => {
  const headerStart = source.indexOf('const header = (');
  const returnStart = source.indexOf('  return (', headerStart);
  if (headerStart < 0 || returnStart < 0) {
    throw new Error('未找到采购退货主表 header 区块');
  }
  return source.slice(headerStart, returnStart);
};

describe('PurchaseReturnsPage 工业主表契约', () => {
  it('显式接入工业主表预设、选择列和列设置契约', () => {
    expect(source).toContain('UiModule03MainTableSection<PurchaseReturnSummary>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('columnSettingsKey="purchase-returns.table.columns"');
    expect(source).toContain('dataTestId="purchase-returns-main-grid"');
  });

  it('显式使用统一工具栏插槽承载采购退货主表动作', () => {
    const mainTable = getMainTableOpeningTag();

    expect(mainTable).toMatch(/toolbar(?:Left|Right)=\{/);
    expect(mainTable).toContain('purchase-returns-create-button');
    expect(mainTable).toContain('purchase-returns-refresh-button');
  });

  it('采购退货主表标题和工具栏不再通过额外裸 Card 承载', () => {
    const headerBlock = getHeaderBlock();
    const mainTable = getMainTableOpeningTag();

    expect(headerBlock).not.toContain('<Card');
    expect(mainTable).not.toContain('<Card');
  });

  it('保留新建、查询与刷新的稳定测试标识', () => {
    expect(source).toContain('data-testid="purchase-returns-create-button"');
    expect(source).toContain('data-testid="purchase-returns-refresh-button"');
    expect(source).toContain('data-testid="purchase-returns-search-button"');
    expect(source).toContain('data-testid="purchase-returns-reset-button"');
  });

  it('明细区域使用统一 ErpDataGrid 且禁止页面级裸 Table', () => {
    expect(source).toContain("ErpDataGrid<PurchaseReturnDetail['items'][number]>");
    expect(source).toContain('purchase-returns-detail-items-grid');
    expect(source).not.toMatch(/<Table\b/);
  });
});
