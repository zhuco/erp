import { describe, expect, it } from 'vitest';

import {
  WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT,
  WORKBENCH_DEFAULT_BOTTOM_PANEL_HEIGHT,
  WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH,
  WORKBENCH_RIGHT_PANEL_MAX_WIDTH,
  WORKBENCH_RIGHT_PANEL_MIN_WIDTH,
  clampWorkbenchPanelSize,
} from './workbenchPanelSizing';

describe('workbenchPanelSizing', () => {
  it('统一模板默认值落在允许范围内', () => {
    expect(WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH).toBeGreaterThanOrEqual(WORKBENCH_RIGHT_PANEL_MIN_WIDTH);
    expect(WORKBENCH_DEFAULT_RIGHT_PANEL_WIDTH).toBeLessThanOrEqual(WORKBENCH_RIGHT_PANEL_MAX_WIDTH);
    expect(WORKBENCH_DEFAULT_BOTTOM_PANEL_HEIGHT).toBeGreaterThanOrEqual(WORKBENCH_BOTTOM_PANEL_MIN_HEIGHT);
  });

  it('会把尺寸约束在给定范围内', () => {
    expect(clampWorkbenchPanelSize(120, 200, 400)).toBe(200);
    expect(clampWorkbenchPanelSize(260, 200, 400)).toBe(260);
    expect(clampWorkbenchPanelSize(520, 200, 400)).toBe(400);
  });
});
