import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './SalesOrderImportPage.tsx'), 'utf8');

describe('SalesOrderImportPage 工业主表契约', () => {
  it('moves the import order list to the unified UiModule03 main table shell', () => {
    expect(source).toContain('UiModule03MainTableSection<ImportOrderRow>');
    expect(source).not.toContain('<Table<ImportOrderRow>');
  });

  it('does not render naked Ant Card or Table in the page source', () => {
    expect(source).not.toContain('<Card');
    expect(source).not.toContain('<Table');
  });

  it('declares stable grid identity, column settings and industrial table behavior', () => {
    expect(source).toContain('dataTestId="sales-order-import-main-grid"');
    expect(source).toContain('columnSettingsKey="sales.order-import.main-grid"');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings');
  });

  it('keeps actions in unified toolbar slots and preserves alert/detail tabs', () => {
    expect(source).toContain('toolbarLeft={');
    expect(source).toContain('toolbarRight={');
    expect(source).toContain('actionBar={');
    expect(source).toContain('<Alert');
    expect(source).toContain('<Tabs');
    expect(source).toContain('bottomPanel={');
    expect(source).toContain('UiModule04BottomTable');
    expect(source).toContain('UiModule03MainTableSection<ImportOrderItemRow>');
    expect(source).toContain('dataTestId="sales-order-import-item-grid"');
    expect(source).toContain('columnSettingsKey="sales.order-import.item-grid"');
  });
});
