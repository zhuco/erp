import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './AfterSalesExchangesPage.tsx'), 'utf8');

const getMainTableOpeningTag = () => {
  const tableStart = source.indexOf('<UiModule03MainTableSection<AfterSalesSummary>');
  const tableEnd = source.indexOf('      />', tableStart);
  if (tableStart < 0 || tableEnd < 0) {
    throw new Error('未找到售后换货主表 UiModule03MainTableSection 调用');
  }
  return source.slice(tableStart, tableEnd);
};

const getHeaderBlock = () => {
  const headerStart = source.indexOf('const header = (');
  const returnStart = source.indexOf('  return (', headerStart);
  if (headerStart < 0 || returnStart < 0) {
    throw new Error('未找到售后换货主表 header 区块');
  }
  return source.slice(headerStart, returnStart);
};

describe('AfterSalesExchangesPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('UiModule03MainTableSection<AfterSalesSummary>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('dataTestId="after-sales-exchanges-main-grid"');
    expect(source).toContain('columnSettingsKey="after-sales.exchanges.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('显式使用统一工具栏插槽承载售后换货主表动作', () => {
    const mainTable = getMainTableOpeningTag();

    expect(mainTable).toMatch(/toolbar(?:Left|Right)=\{/);
    expect(mainTable).toContain('after-sales-exchanges-title');
    expect(mainTable).toContain('after-sales-exchanges-refresh-button');
  });

  it('售后换货主表标题和工具栏不再通过额外裸 Card 承载', () => {
    const headerBlock = getHeaderBlock();
    const mainTable = getMainTableOpeningTag();

    expect(headerBlock).not.toContain('<Card');
    expect(mainTable).not.toContain('<Card');
  });
});
