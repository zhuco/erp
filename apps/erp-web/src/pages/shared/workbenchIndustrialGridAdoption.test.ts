import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('workbench industrial grid adoption', () => {
  it('keeps channel directory on the industrial compact main-grid preset', () => {
    const source = readFileSync(resolve(__dirname, '../channels/ChannelDirectoryPage.tsx'), 'utf8');

    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('gridFilterModel={gridFilterModel}');
    expect(source).toContain('onGridFilterModelChange={(model) => {');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).not.toContain('headerFilters={headerFilters}');
  });

  it('keeps couriers on the industrial compact main-grid preset', () => {
    const source = readFileSync(resolve(__dirname, '../couriers/CouriersPage.tsx'), 'utf8');

    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('gridFilterModel={gridFilterModel}');
    expect(source).toContain('onGridFilterModelChange={(model) => {');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('keeps warehouse directory on the industrial compact main-grid preset', () => {
    const source = readFileSync(resolve(__dirname, '../warehouses/WarehouseDirectoryPage.tsx'), 'utf8');

    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('gridFilterModel={gridFilterModel}');
    expect(source).toContain('onGridFilterModelChange={(model) => {');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('keeps suppliers on the industrial compact main-grid preset', () => {
    const source = readFileSync(resolve(__dirname, '../suppliers/SuppliersPage.tsx'), 'utf8');

    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('gridFilterModel={gridFilterModel}');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('keeps logistics fee directory on the industrial compact main-grid preset', () => {
    const source = readFileSync(resolve(__dirname, '../logistics/LogisticsFeeDirectoryPage.tsx'), 'utf8');

    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('keeps tags on the industrial compact main-grid preset', () => {
    const source = readFileSync(resolve(__dirname, '../tags/TagDirectoryWorkbench.tsx'), 'utf8');

    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('keeps customer categories on the industrial compact main-grid preset', () => {
    const source = readFileSync(resolve(__dirname, '../customers/CustomerCategoriesPage.tsx'), 'utf8');

    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('gridFilterModel={gridFilterModel}');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('keeps member levels on the industrial compact main-grid preset', () => {
    const source = readFileSync(resolve(__dirname, '../customers/MemberLevelsPage.tsx'), 'utf8');

    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('gridFilterModel={gridFilterModel}');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('keeps customer tags on the industrial compact main-grid preset', () => {
    const source = readFileSync(resolve(__dirname, '../customers/CustomerTagsPage.tsx'), 'utf8');

    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('gridFilterModel={gridFilterModel}');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });
});
