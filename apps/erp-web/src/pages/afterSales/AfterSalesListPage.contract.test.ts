import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './AfterSalesListPage.tsx'), 'utf8');
const mainTableStart = source.indexOf('<UiModule03MainTableSection<AfterSalesSummary>');
const mainTableEnd = source.indexOf('\n      />', mainTableStart);
const mainTableSection =
  mainTableStart === -1 || mainTableEnd === -1
    ? ''
    : source.slice(mainTableStart, mainTableEnd + '\n      />'.length);

describe('AfterSalesListPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('UiModule03MainTableSection<AfterSalesSummary>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('dataTestId="after-sales-list-main-grid"');
    expect(source).toContain('columnSettingsKey="after-sales.table.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('主表显式使用统一工具栏左右区域承载主操作', () => {
    expect(mainTableSection).toContain('toolbarLeft=');
    expect(mainTableSection).toContain('toolbarRight=');
  });

  it('主表标题与筛选不再由额外裸 Card 承载', () => {
    expect(source).not.toContain('Card,');
    expect(source).not.toContain('<Card>');
    expect(source).not.toContain('</Card>');
  });
});
