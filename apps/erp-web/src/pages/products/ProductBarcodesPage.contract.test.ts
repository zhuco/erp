import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ProductBarcodesPage.tsx'), 'utf8');

describe('ProductBarcodesPage 工业主表契约', () => {
  const mainTableStart = source.indexOf('<UiModule03MainTableSection<BarcodeRecord>');
  const drawerStart = source.indexOf('<Drawer');
  const mainTableProps = source.slice(mainTableStart, drawerStart);

  if (mainTableStart < 0 || drawerStart < 0) {
    throw new Error('未找到商品条码 UiModule03MainTableSection 主表');
  }

  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('主表显式接入统一工具栏插槽', () => {
    expect(mainTableProps).toMatch(/toolbar(?:Left|Right)=/);
    expect(mainTableProps).toContain('toolbarLeft=');
    expect(mainTableProps).toContain('toolbarRight=');
    expect(mainTableProps).toContain('data-testid="product-barcodes-title"');
    expect(mainTableProps).toContain('data-testid="product-barcodes-import-button"');
    expect(mainTableProps).toContain('data-testid="product-barcodes-create-button"');
  });

  it('商品条码主表不再由额外裸 Card 承载标题或工具栏', () => {
    const beforeMainTable = source.slice(0, source.indexOf('<UiModule03MainTableSection<BarcodeRecord>'));
    expect(beforeMainTable).not.toContain('data-testid="product-barcodes-title"');
    expect(beforeMainTable).not.toContain('data-testid="product-barcodes-import-button"');
    expect(beforeMainTable).not.toContain('data-testid="product-barcodes-create-button"');
    expect(beforeMainTable).not.toMatch(/<Card>\s*<Form[\s\S]*?data-testid="product-barcodes-filter-form"/);
  });

  it('不再使用裸 Card 或裸 JSX Table 承载页面区块', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
    expect(source).toContain('<ErpDataGrid<PackagingSample>');
  });

  it('保留主表标识、列设置和关键操作标识', () => {
    expect(source).toContain('dataTestId="product-barcodes-table"');
    expect(source).toContain('columnSettingsKey="product.barcodes.table.columns"');
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('data-testid="product-barcodes-title"');
    expect(source).toContain('data-testid="product-barcodes-filter-form"');
    expect(source).toContain('data-testid="product-barcodes-create-button"');
    expect(source).toContain('data-testid="product-barcodes-search-button"');
    expect(source).toContain('data-testid="product-barcodes-reset-button"');
  });
});
