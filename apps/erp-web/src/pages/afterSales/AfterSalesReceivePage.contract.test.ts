import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './AfterSalesReceivePage.tsx'), 'utf8');

describe('AfterSalesReceivePage 工业主表契约', () => {
  it('退货明细主表接入统一工作台主表契约', () => {
    expect(source).toContain('UiModule03MainTableSection<AfterSalesItem>');
    expect(source).toContain('dataTestId="after-sales-receive-main-grid"');
    expect(source).toContain('columnSettingsKey="after-sales.receive.main-grid"');
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
    expect(source).toContain('showColumnSettings');
  });

  it('通过统一 header 和 toolbar 插槽承载收货操作', () => {
    expect(source).toContain('header={header}');
    expect(source).toContain('toolbarLeft={toolbarLeft}');
    expect(source).toContain('toolbarRight={toolbarRight}');
    expect(source).toContain('data-testid="after-sales-receive-warehouse"');
    expect(source).toContain('data-testid="after-sales-receive-submit"');
  });

  it('不再使用裸 Card/Table 承载退货明细主表', () => {
    expect(source).not.toContain('<Card title="退货明细">');
    expect(source).not.toContain('<Table rowKey="sku_id"');
  });
});
