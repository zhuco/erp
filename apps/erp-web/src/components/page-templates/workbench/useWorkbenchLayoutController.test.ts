import { describe, expect, it } from 'vitest';

import {
  buildWorkbenchLayoutState,
  resolveWorkbenchPanelVisible,
} from './useWorkbenchLayoutController';

describe('useWorkbenchLayoutController helpers', () => {
  it('marks panels visible by default when content exists', () => {
    expect(resolveWorkbenchPanelVisible('content')).toBe(true);
    expect(resolveWorkbenchPanelVisible('content', false)).toBe(false);
    expect(resolveWorkbenchPanelVisible(undefined)).toBe(false);
  });

  it('builds a unified panel layout state snapshot', () => {
    expect(
      buildWorkbenchLayoutState({
        leftPanel: 'left',
        rightPanel: 'right',
        bottomPanel: 'bottom',
        leftPanelVisible: true,
        rightPanelVisible: false,
      }),
    ).toEqual({
      left: { panel: 'left', visible: true },
      right: { panel: 'right', visible: false },
      bottom: { panel: 'bottom', visible: true },
    });
  });
});
