import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './PurchaseReceiptsPage.tsx'), 'utf8');

const getMainTableOpeningTag = () => {
  const tableStart = source.indexOf('<UiModule03MainTableSection<PurchaseReceiptSummary>');
  const drawerStart = source.indexOf('      <Drawer', tableStart);
  if (tableStart < 0 || drawerStart < 0) {
    throw new Error('未找到采购收货主表 UiModule03MainTableSection 调用');
  }
  return source.slice(tableStart, drawerStart);
};

const getHeaderBlock = () => {
  const headerStart = source.indexOf('const header = (');
  const returnStart = source.indexOf('  return (', headerStart);
  if (headerStart < 0 || returnStart < 0) {
    throw new Error('未找到采购收货主表 header 区块');
  }
  return source.slice(headerStart, returnStart);
};

describe('PurchaseReceiptsPage 工业主表契约', () => {
  it('显式接入工业主表预设、选择列和列设置契约', () => {
    expect(source).toContain('UiModule03MainTableSection<PurchaseReceiptSummary>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('columnSettingsKey="purchase-receipts.table.columns"');
    expect(source).toContain('dataTestId="purchase-receipts-main-grid"');
  });

  it('显式使用统一工具栏插槽承载采购收货主表动作', () => {
    const mainTable = getMainTableOpeningTag();

    expect(mainTable).toMatch(/toolbar(?:Left|Right)=\{/);
    expect(mainTable).toContain('purchase-receipts-refresh-button');
  });

  it('采购收货主表标题和工具栏不再通过额外裸 Card 承载', () => {
    const headerBlock = getHeaderBlock();
    const mainTable = getMainTableOpeningTag();

    expect(headerBlock).not.toContain('<Card');
    expect(mainTable).not.toContain('<Card');
  });

  it('保留查询与刷新的稳定测试标识', () => {
    expect(source).toContain('data-testid="purchase-receipts-refresh-button"');
    expect(source).toContain('data-testid="purchase-receipts-search-button"');
    expect(source).toContain('data-testid="purchase-receipts-reset-button"');
  });

  it('本次触碰的采购收货 UI 文案保持中文', () => {
    const forbiddenUiFragments = [
      "title: 'Receipt ID'",
      "title: 'Supplier'",
      "title: 'Warehouse'",
      "title: 'Status'",
      "title: 'Confirmed At'",
      "title: 'Created At'",
      '>\n            Detail\n          </Button>',
      '>\n              Search\n            </Button>',
      '>\n              Reset\n            </Button>',
      'Purchase Receipts',
      '>\n            Refresh\n          </Button>',
      'Failed to load purchase receipts',
      'No receipts',
      'Purchase Receipt Detail',
      'No detail',
      'label="Receipt ID"',
      'label="Supplier"',
      'label="Warehouse"',
      'label="Status"',
      'label="Confirmed At"',
      'label="Created At"',
      'label="Created Range"'
    ];

    for (const fragment of forbiddenUiFragments) {
      expect(source).not.toContain(fragment);
    }
    expect(source).not.toContain('?'.repeat(4));
  });

  it('明细区域使用统一 ErpDataGrid 且禁止页面级裸 Table', () => {
    expect(source).toContain("ErpDataGrid<PurchaseReceiptDetail['items'][number]>");
    expect(source).toContain('purchase-receipts-detail-items-grid');
    expect(source).not.toMatch(/<Table\b/);
  });
});
