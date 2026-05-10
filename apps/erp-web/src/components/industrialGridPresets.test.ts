import { describe, expect, it } from 'vitest';

import {
  INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET,
  resolveErpGridPreset,
} from './industrialGridPresets';

describe('industrialGridPresets', () => {
  it('resolves the order-review compact preset with industrial grid constraints', () => {
    expect(resolveErpGridPreset('industrial-review-compact')).toEqual(
      INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET,
    );
  });

  it('keeps the industrial compact preset aligned with the order-review main-grid density', () => {
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.rowHeight).toBe(22);
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.headerHeight).toBe(30);
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.floatingFilter).toBe(true);
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.structuralColumnStyle).toBe('order-review');
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.selectionCellMode).toBe('custom');
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.selectionHeaderMode).toBe('allInvert');
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.selectionColumnWidth).toBe(24);
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.rowIndexWidth).toBe(40);
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.showRowIndex).toBe(true);
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.animateRows).toBe(false);
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.className).toContain(
      'erp-data-grid--industrial-compact',
    );
    expect(INDUSTRIAL_REVIEW_GRID_COMPACT_PRESET.className).toContain(
      'order-review-main-grid-shell',
    );
  });

  it('falls back to null for unknown presets', () => {
    expect(resolveErpGridPreset('unknown-preset')).toBeNull();
  });
});
