import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './InventoryAlertsPage.tsx'), 'utf8');
const mainTableSectionMatch = source.match(
  /<UiModule03MainTableSection<InventoryAlertRecord>[\s\S]*?\n\s*\/>/
);
const mainTableSectionSource = mainTableSectionMatch?.[0] ?? '';

describe('InventoryAlertsPage 工业主表契约', () => {
  it('接入工业主表统一契约', () => {
    expect(mainTableSectionSource).toContain('UiModule03MainTableSection<InventoryAlertRecord>');
    expect(mainTableSectionSource).toContain('gridPreset="industrial-review-compact"');
    expect(mainTableSectionSource).toContain('selectionCellMode="custom"');
    expect(mainTableSectionSource).toContain('selectionHeaderMode="allInvert"');
    expect(mainTableSectionSource).toContain('showColumnSettings={true}');
    expect(mainTableSectionSource).toContain('columnSettingsKey="inventory.alerts.table.columns"');
    expect(mainTableSectionSource).toContain('dataTestId="inventory-alerts-main-grid"');
  });

  it('主表使用统一工具栏插槽并移除标题工具栏裸 Card 包裹', () => {
    expect(mainTableSectionSource).toContain('toolbarLeft={');
    expect(mainTableSectionSource).toContain('toolbarRight={');
    expect(mainTableSectionSource).toContain('header={');
    expect(mainTableSectionSource).toContain('actionBar={');
    expect(source).not.toMatch(
      /<Card\s+size="small"\s+className="inventory-alerts-table-card">[\s\S]*?<UiModule03MainTableSection<InventoryAlertRecord>/
    );
    expect(source).not.toMatch(
      /<Card\s+size="small">\s*<Form[\s\S]*?data-testid="inventory-alerts-filter-form"[\s\S]*?<\/Form>\s*<\/Card>/
    );
  });

  it('页面不再出现裸 Card 和裸 JSX Table', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<\/Card>/);
    expect(source).not.toMatch(/<Table\b/);
  });

  it('保留筛选和工具栏稳定测试标识', () => {
    expect(source).toContain('data-testid="inventory-alerts-filter-form"');
    expect(source).toContain('data-testid="inventory-alerts-search-button"');
    expect(source).toContain('data-testid="inventory-alerts-reset-button"');
    expect(source).toContain('data-testid="inventory-alerts-export-button"');
    expect(source).toContain('data-testid="inventory-alerts-refresh-button"');
  });
});

