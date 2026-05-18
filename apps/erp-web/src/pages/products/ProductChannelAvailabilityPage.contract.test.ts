import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ProductChannelAvailabilityPage.tsx'), 'utf8');

const getAvailabilityTableSection = () => {
  const tableStart = source.indexOf('<UiModule03MainTableSection<AvailabilityRecord>');
  expect(tableStart).toBeGreaterThanOrEqual(0);

  const tableEnd = source.indexOf('/>', tableStart);
  expect(tableEnd).toBeGreaterThan(tableStart);

  return source.slice(tableStart, tableEnd);
};

describe('ProductChannelAvailabilityPage 工业主表契约', () => {
  it('保留主表统一组件与关键契约', () => {
    const tableSection = getAvailabilityTableSection();

    expect(tableSection).toContain('dataTestId="product-channel-table"');
    expect(tableSection).toContain('columnSettingsKey="product.channel.availability.table.columns"');
    expect(tableSection).toContain('gridPreset="industrial-review-compact"');
    expect(tableSection).toContain('selectionCellMode="custom"');
    expect(tableSection).toContain('selectionHeaderMode="allInvert"');
  });

  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和关键操作标识', () => {
    expect(source).toContain('dataTestId="product-channel-table"');
    expect(source).toContain('columnSettingsKey="product.channel.availability.table.columns"');
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('data-testid="product-channel-title"');
    expect(source).toContain('dataTestId="product-channel-rules-table"');
    expect(source).toContain('data-testid="product-channel-filter-form"');
    expect(source).toContain('data-testid="product-channel-search-button"');
    expect(source).toContain('data-testid="product-channel-reset-button"');
  });

  it('可售清单主表显式接入统一工具栏插槽', () => {
    const tableSection = getAvailabilityTableSection();

    expect(tableSection).toContain('toolbarLeft={mainTableToolbarLeft}');
    expect(tableSection).toContain('toolbarRight={mainTableToolbarRight}');
    expect(tableSection).toContain('actionBar={mainTableActionBar}');
  });

  it('页面不再使用裸 Card 或裸 JSX Table 承载统一区块', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });
});
