import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ApManagementPage.tsx'), 'utf8');

const extractMainTableSection = (dataTestId: string) => {
  const dataTestIdIndex = source.indexOf(dataTestId);
  const start = source.lastIndexOf('<UiModule03MainTableSection', dataTestIdIndex);
  const end = source.indexOf('/>', dataTestIdIndex);

  expect(dataTestIdIndex).toBeGreaterThanOrEqual(0);
  expect(start).toBeGreaterThanOrEqual(0);
  expect(end).toBeGreaterThan(start);

  return source.slice(start, end);
};

describe('ApManagementPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置键和列设置入口', () => {
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('dataTestId="finance-ap-docs-main-grid"');
    expect(source).toContain('columnSettingsKey="finance.ap.docs.table.columns"');
    expect(source).toContain('dataTestId="finance-ap-plans-main-grid"');
    expect(source).toContain('columnSettingsKey="finance.ap.plans.table.columns"');
  });

  it('两个主表显式接入统一表格工具栏插槽', () => {
    const docsTable = extractMainTableSection('dataTestId="finance-ap-docs-main-grid"');
    const plansTable = extractMainTableSection('dataTestId="finance-ap-plans-main-grid"');

    expect(docsTable).toContain('toolbarLeft={docsToolbarLeft}');
    expect(docsTable).toContain('toolbarRight={docsToolbarRight}');
    expect(docsTable).toContain('header={docsHeader}');
    expect(plansTable).toContain('toolbarLeft={plansToolbarLeft}');
    expect(plansTable).toContain('toolbarRight={plansToolbarRight}');
    expect(plansTable).toContain('header={plansHeader}');
  });

  it('筛选区不再使用只承载筛选和操作的裸 Card', () => {
    expect(source).not.toContain('<Card');
    expect(source).not.toMatch(/^\s*Card,?$/m);
  });
});
