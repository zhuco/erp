import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './LogisticsUploadPage.tsx'), 'utf8');

describe('LogisticsUploadPage 工业主表契约', () => {
  it('显式接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source).toContain('selectionCellMode="custom"');
    expect(source).toContain('selectionHeaderMode="allInvert"');
  });

  it('保留主表标识、列设置和关键操作标识', () => {
    expect(source).toContain('dataTestId="logistics-upload-main-grid"');
    expect(source).toContain('columnSettingsKey="sales.logistics-upload.main-grid"');
    expect(source).toContain('showColumnSettings');
    expect(source).toContain('UiModule03MainTableSection');
  });

  it('明细区域使用统一 ErpDataGrid 且禁止页面级裸 Table', () => {
    expect(source).toContain('ErpDataGrid<LogisticsUploadLog>');
    expect(source).toContain('logistics-upload-logs-grid');
    expect(source).toContain('logistics-upload-goods-grid');
    expect(source).not.toMatch(/<Table\b/);
  });
});
