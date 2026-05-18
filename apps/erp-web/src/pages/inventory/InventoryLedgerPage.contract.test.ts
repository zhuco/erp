import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './InventoryLedgerPage.tsx'), 'utf8');

describe('InventoryLedgerPage 工业主表契约', () => {
  it('统一使用 UiModule03 工业主表壳', () => {
    expect(source).toContain('UiModule03MainTableSection<InventoryLedger>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('dataTestId="inventory-ledger-main-grid"');
    expect(source).toContain('columnSettingsKey="inventory.ledger.table.columns"');
  });

  it('使用统一工具栏与筛选 header', () => {
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
    expect(source).toContain('header={tableHeader}');
  });

  it('禁止页面级裸 Card/Table', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });
});
