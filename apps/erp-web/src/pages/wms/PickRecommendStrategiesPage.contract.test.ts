import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const source = readFileSync(resolve(__dirname, './PickRecommendStrategiesPage.tsx'), 'utf8');

describe('PickRecommendStrategiesPage 双表工业主表契约', () => {
  it('两个页签表格均接入工业紧凑表格与统一选择契约', () => {
    expect(source).toContain('gridPreset="industrial-review-compact"');
    expect(source.match(/gridPreset="industrial-review-compact"/g)?.length).toBeGreaterThanOrEqual(2);
    expect(source.match(/selectionCellMode="custom"/g)?.length).toBeGreaterThanOrEqual(2);
    expect(source.match(/selectionHeaderMode="allInvert"/g)?.length).toBeGreaterThanOrEqual(2);
  });

  it('保留两个表格的稳定标识、列设置键和列设置入口', () => {
    expect(source.match(/showColumnSettings/g)?.length).toBeGreaterThanOrEqual(2);
    expect(source).toContain('UiModule03MainTableSection<PickRecommendStrategy>');
    expect(source).toContain('dataTestId="wms-pick-recommend-strategies-main-grid"');
    expect(source).toContain('columnSettingsKey="wms.pick-recommend-strategies.table.columns"');
    expect(source).toContain('UiModule03MainTableSection<PickRecommendHit>');
    expect(source).toContain('dataTestId="wms-pick-recommend-hits-main-grid"');
    expect(source).toContain('columnSettingsKey="wms.pick-recommend-hits.table.columns"');
  });
});
