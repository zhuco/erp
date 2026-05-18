import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ShopOrderQueryPage.tsx'), 'utf8');

describe('ShopOrderQueryPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和关键操作标识', () => {
    expect(source).toContain('dataTestId="shop-order-query-main-grid"');
    expect(source).toContain('columnSettingsKey="sales.shop-order-query.main-grid"');
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('data-testid="shop-order-query-search"');
    expect(source).toContain('data-testid="shop-order-query-export-button"');
    expect(source).toContain('data-testid="shop-order-query-help-button"');
    expect(source).toContain('data-testid="shop-order-query-left-apply"');
    expect(source).toContain('data-testid="shop-order-query-left-reset"');
  });

  it('主表操作区统一承载在 UiModule03MainTableSection', () => {
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
  });

  it('不再使用裸 Card 或裸 Table 承载工作台主结构', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });
});
