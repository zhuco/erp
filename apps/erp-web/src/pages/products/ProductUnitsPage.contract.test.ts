import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ProductUnitsPage.tsx'), 'utf8');

describe('ProductUnitsPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('UiModule03MainTableSection<UnitConversion>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('dataTestId="product-units-table"');
    expect(source).toContain('columnSettingsKey="product.units.table.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('使用统一主表工具栏', () => {
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
  });
});
