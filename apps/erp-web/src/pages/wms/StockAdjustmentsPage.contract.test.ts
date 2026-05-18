import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, 'StockAdjustmentsPage.tsx'), 'utf-8');

describe('StockAdjustmentsPage 工业工作台契约', () => {
  it('显式接入工业主表预设、选择列和列设置契约', () => {
    expect(source).toContain('UiModule03MainTableSection<StockAdjustSummary>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('columnSettingsKey="wms.stock-adjustments.table.columns"');
    expect(source).toContain('dataTestId="stock-adjustments-main-grid"');
  });

  it('主表标题、说明和主操作统一进入工具栏插槽', () => {
    expect(source).toContain('const toolbarLeft = (');
    expect(source).toContain('const toolbarRight = (');
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
    expect(source).toContain('统一跟踪库存报损、报溢调整与应用状态');
    expect(source).toContain('data-testid="stock-adjustments-create-button"');
  });

  it('详情抽屉不再使用裸 Card 和裸 Table', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
    expect(source).toContain('<LoadingState');
    expect(source).toContain('UiModule03MainTableSection<StockAdjustItem>');
    expect(source).toContain('dataTestId="stock-adjustments-detail-items-grid"');
    expect(source).toContain('columnSettingsKey="wms.stock-adjustments.detail-items.columns"');
  });

  it('保留主操作按钮的稳定测试标识', () => {
    expect(source).toContain('data-testid="stock-adjustments-create-button"');
    expect(source).toContain('data-testid="stock-adjustments-detail-button"');
  });
});
