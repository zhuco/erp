import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ArManagementPage.tsx'), 'utf8');

describe('ArManagementPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和列设置入口', () => {
    expect(source).toContain('dataTestId="finance-ar-main-grid"');
    expect(source).toContain('columnSettingsKey="finance.ar.table.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('主表标题、说明和主操作迁入统一主表工具栏', () => {
    expect(source).toContain('toolbarLeft=');
    expect(source).toContain('toolbarRight=');
    expect(source).toContain('应收管理与核销');
    expect(source).toContain('跟踪客户应收、到期与核销状态');
    expect(source).not.toContain('<Card>');
    expect(source).not.toContain('<Typography.Title');
  });
});
