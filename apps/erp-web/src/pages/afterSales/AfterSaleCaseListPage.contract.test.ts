import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './AfterSaleCaseListPage.tsx'), 'utf8');

const mainTableSection =
  source.match(/<UiModule03MainTableSection<AfterSaleCaseSummary>[\s\S]*?\/>/)?.[0] ?? '';

describe('AfterSaleCaseListPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('UiModule03MainTableSection<AfterSaleCaseSummary>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('dataTestId="after-sale-case-list-main-grid"');
    expect(source).toContain('columnSettingsKey="after-sales.table.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('主表显式使用统一工具栏插槽承载入口', () => {
    expect(mainTableSection).toMatch(/toolbar(?:Left|Right)=/);
  });

  it('主表标题和筛选不再通过额外裸 Card 承载', () => {
    expect(source).not.toMatch(/import\s*\{[^}]*\bCard\b[^}]*\}\s*from\s*'antd'/);
    expect(source).not.toContain('<Card');
  });
});
