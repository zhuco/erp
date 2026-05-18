import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ProductsPage.tsx'), 'utf8');

describe('ProductsPage 工业主表契约', () => {
  it('主表操作统一收敛到模块工具栏', () => {
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
    expect(source).toContain('actionBar={actionBar}');
    expect(source).not.toContain('toolbarCenter={toolbarCenter}');
  });

  it('不再使用裸 Card 或裸 Table 承载页面模块', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });

  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和关键操作标识', () => {
    expect(source).toContain('dataTestId="products-data-grid"');
    expect(source).toContain('columnSettingsKey="products.table.columns"');
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('data-testid="products-search"');
    expect(source).toContain('data-testid="products-search-reset"');
    expect(source).toContain('data-testid="products-create-button"');
    expect(source).toContain('data-testid="products-copy-button"');
    expect(source).toContain('data-testid="products-edit-button"');
    expect(source).toContain('data-testid="products-delete-button"');
  });
});
