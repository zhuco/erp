import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './ConsignmentPage.tsx'), 'utf8');
const mainTableProps = source.match(/<UiModule03MainTableSection<ConsignmentSettlement>[\s\S]*?\/>/u)?.[0] ?? '';
const antdImportBlock = source.match(/import \{[\s\S]*?\} from 'antd';/u)?.[0] ?? '';

describe('ConsignmentPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和列设置入口', () => {
    expect(source).toContain('dataTestId="finance-consignment-main-grid"');
    expect(source).toContain('columnSettingsKey="finance.consignment.table.columns"');
    expect(source).toContain('showColumnSettings');
  });

  it('将主表标题和主操作迁入统一主表工具栏', () => {
    expect(mainTableProps).toContain('toolbarLeft={toolbarLeft}');
    expect(mainTableProps).toContain('toolbarRight={toolbarRight}');
    expect(source).toContain('const toolbarLeft = (');
    expect(source).toContain('代销结算管理');
    expect(source).toContain('代销结算单的创建、审核与结算状态跟踪');
    expect(source).toContain('const toolbarRight = (');
    expect(source).toContain('刷新');
    expect(source).toContain('导出');
    expect(source).toContain('新建结算单');
  });

  it('筛选表单由主表 actionBar 承载且不再使用裸 Card 包裹', () => {
    expect(mainTableProps).toContain('actionBar={filterActionBar}');
    expect(mainTableProps).not.toContain('header={header}');
    expect(source).toContain('const filterActionBar = (');
    expect(antdImportBlock).not.toMatch(/\bCard\b/u);
    expect(source).not.toContain('<Card>');
    expect(source).not.toContain('</Card>');
  });
});
