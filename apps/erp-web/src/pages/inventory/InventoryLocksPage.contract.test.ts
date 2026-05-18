import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './InventoryLocksPage.tsx'), 'utf8');
const mainTableStart = source.indexOf('<UiModule03MainTableSection<InventoryLockLog>');
const mainTableEnd = source.indexOf('<Drawer', mainTableStart);
const mainTableSource = source.slice(mainTableStart, mainTableEnd);
const mainTableBeforeSource = source.slice(0, mainTableStart);
const mainTableToolbarLeftSource = mainTableSource.slice(
  mainTableSource.indexOf('toolbarLeft={'),
  mainTableSource.indexOf('toolbarRight={')
);
const mainTableToolbarRightSource = mainTableSource.slice(
  mainTableSource.indexOf('toolbarRight={'),
  mainTableSource.indexOf('actionBar={')
);
const mainTableActionBarSource = mainTableSource.slice(
  mainTableSource.indexOf('actionBar={'),
  mainTableSource.indexOf('gridPreset="industrial-review-compact"')
);
const mainTableHeaderSource = mainTableSource.includes('header={')
  ? mainTableSource.slice(
      mainTableSource.indexOf('header={'),
      mainTableSource.indexOf('gridPreset="industrial-review-compact"')
    )
  : '';

describe('InventoryLocksPage 工业主表契约', () => {
  it('接入工业主表统一契约', () => {
    expect(source).toContain('UiModule03MainTableSection<InventoryLockLog>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings={true}');
    expect(source).toContain('columnSettingsKey="inventory.locks.table.columns"');
    expect(source).toContain('dataTestId="inventory-locks-main-grid"');
  });

  it('保留筛选和工具栏稳定测试标识', () => {
    expect(source).toContain('data-testid="inventory-locks-filter-form"');
    expect(source).toContain('data-testid="inventory-locks-search-button"');
    expect(source).toContain('data-testid="inventory-locks-reset-button"');
    expect(source).toContain('data-testid="inventory-locks-recalc-button"');
    expect(source).toContain('data-testid="inventory-locks-export-button"');
  });

  it('主表显式使用统一工具栏插槽承载标题和操作', () => {
    expect(mainTableStart).toBeGreaterThanOrEqual(0);
    expect(mainTableSource).toContain('toolbarLeft={');
    expect(mainTableSource).toContain('toolbarRight={');
    expect(mainTableToolbarLeftSource).toContain('库存锁定');
    expect(mainTableToolbarLeftSource).toContain('锁定日志、占用来源与释放审计统一诊断。');
    expect(mainTableToolbarRightSource).toContain('data-testid="inventory-locks-recalc-button"');
    expect(mainTableToolbarRightSource).toContain('data-testid="inventory-locks-export-button"');
  });

  it('主表筛选和锁定模型说明迁入 actionBar 而非额外头部 Card', () => {
    expect(mainTableSource).toContain('actionBar={');
    expect(mainTableActionBarSource).toContain('data-testid="inventory-locks-filter-form"');
    expect(mainTableActionBarSource).toContain('lockModelNotes.map');
    expect(mainTableHeaderSource).not.toContain('data-testid="inventory-locks-filter-form"');
    expect(mainTableHeaderSource).not.toContain('锁定来源');
  });

  it('锁定日志主表前不再由额外裸 Card 承载标题或工具栏', () => {
    expect(mainTableBeforeSource).not.toContain('<Card>');
    expect(mainTableBeforeSource).not.toContain('<Card size="small" title="锁定模型">');
    expect(mainTableBeforeSource).not.toContain('<Row justify="space-between" align="middle">');
  });

  it('页面不再保留裸 Card 或裸 Table 结构', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });
});
