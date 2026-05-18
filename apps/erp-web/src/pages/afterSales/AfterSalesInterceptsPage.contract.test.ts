import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './AfterSalesInterceptsPage.tsx'), 'utf8');
const mainTableSectionStart = source.indexOf('<UiModule03MainTableSection<AfterSalesIntercept>');
const mainTableSectionEnd = source.indexOf('/>', mainTableSectionStart);
const mainTableSectionSource = source.slice(mainTableSectionStart, mainTableSectionEnd);

describe('AfterSalesInterceptsPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('UiModule03MainTableSection<AfterSalesIntercept>');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('dataTestId="after-sales-intercepts-main-grid"');
    expect(source).toContain('columnSettingsKey="after-sales.intercepts.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('通过统一主表 toolbar 插槽承载标题和主操作', () => {
    expect(source).toContain('const toolbarLeft = (');
    expect(source).toContain('const toolbarRight = (');
    expect(mainTableSectionSource).toContain('toolbarLeft={toolbarLeft}');
    expect(mainTableSectionSource).toContain('toolbarRight={toolbarRight}');
    expect(mainTableSectionSource).toContain('header={header}');
    expect(source).toContain('data-testid="after-sales-intercepts-title"');
    expect(source).toContain('data-testid="after-sales-intercepts-create"');
  });

  it('筛选表单保留在 header 且不再使用裸 Card 承载主表区', () => {
    expect(source).toContain('const header = (');
    expect(source).toContain('<Form form={form} layout="vertical" onFinish={handleSearch}>');
    expect(source).not.toContain('<Card>');
    expect(source).not.toContain('</Card>');
    expect(source).not.toContain('Button, Card,');
  });
});
