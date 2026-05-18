import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './StockMovesPage.tsx'), 'utf8');

describe('StockMovesPage 工业主表契约', () => {
  it('显式接入工业主表预设、选择列和列设置契约', () => {
    expect(source).toContain('UiModule03MainTableSection<StockMoveSummary>');
    expect(source).toContain('dataTestId="stock-moves-main-grid"');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('columnSettingsKey="wms.stock-moves.table.columns"');
  });

  it('统一主操作区到 toolbarLeft 和 toolbarRight', () => {
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
  });

  it('不再保留裸 Card 和裸 JSX Table', () => {
    expect(source).not.toMatch(/<Card\b/);
    expect(source).not.toMatch(/<Table\b/);
  });

  it('保留主操作按钮的稳定测试标识', () => {
    expect(source).toContain('data-testid="stock-moves-create-button"');
    expect(source).toContain('data-testid="stock-moves-detail-button"');
  });
});
