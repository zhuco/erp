import { describe, expect, it } from 'vitest';

import { buildWorkbenchLayoutState, resolveWorkbenchPanelVisible } from './useWorkbenchLayoutController';

describe('useWorkbenchLayoutController helpers', () => {
  it('shows provided panels by default when visibility is not overridden', () => {
    expect(resolveWorkbenchPanelVisible('left-panel')).toBe(true);
    expect(resolveWorkbenchPanelVisible('right-panel')).toBe(true);
    expect(resolveWorkbenchPanelVisible('bottom-panel')).toBe(true);
  });

  it('allows each page to explicitly hide any workbench panel', () => {
    expect(resolveWorkbenchPanelVisible('left-panel', false)).toBe(false);
    expect(resolveWorkbenchPanelVisible('right-panel', false)).toBe(false);
    expect(resolveWorkbenchPanelVisible('bottom-panel', false)).toBe(false);
  });

  it('builds a full three-panel layout state for unified ERP workbench pages', () => {
    const state = buildWorkbenchLayoutState({
      leftPanel: 'left-panel',
      leftPanelVisible: true,
      rightPanel: 'right-panel',
      rightPanelVisible: true,
      bottomPanel: 'bottom-panel',
      bottomPanelVisible: true,
    });

    expect(state.left).toEqual({ panel: 'left-panel', visible: true });
    expect(state.right).toEqual({ panel: 'right-panel', visible: true });
    expect(state.bottom).toEqual({ panel: 'bottom-panel', visible: true });
  });
});
