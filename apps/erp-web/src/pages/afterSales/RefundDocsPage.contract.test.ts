import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './RefundDocsPage.tsx'), 'utf8');
const mainTableSection =
  source.match(/<UiModule03MainTableSection<RefundDoc>[\s\S]*?\/>/)?.[0] ?? '';

describe('RefundDocsPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('UiModule03MainTableSection<RefundDoc>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('dataTestId="refund-docs-main-grid"');
    expect(source).toContain('columnSettingsKey="refund-docs.table.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('主表标题、说明和主操作显式接入统一工具栏插槽', () => {
    expect(source).toContain('const toolbarLeft = (');
    expect(source).toContain('const toolbarRight = (');
    expect(mainTableSection).toContain('toolbarLeft={toolbarLeft}');
    expect(mainTableSection).toContain('toolbarRight={toolbarRight}');
  });

  it('筛选表单由主表 header 承载且不再包裹额外裸 Card', () => {
    expect(source).toContain('const filterForm = (');
    expect(mainTableSection).toContain('header={filterForm}');
    expect(source).not.toContain('const header = (');
    expect(source).not.toMatch(/<Card(?:\s|>)/);
    expect(source).not.toMatch(/<\/Card>/);
  });
});
