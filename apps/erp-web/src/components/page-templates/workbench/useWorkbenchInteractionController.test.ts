import { describe, expect, it, vi } from 'vitest';

import {
  createWorkbenchContextMenuState,
  isEditableWorkbenchTarget,
  matchWorkbenchShortcut,
} from './useWorkbenchInteractionController';

describe('useWorkbenchInteractionController helpers', () => {
  it('detects editable targets so shared shortcuts can bail out safely', () => {
    const inputTarget = {
      tagName: 'INPUT',
      isContentEditable: false,
      closest: vi.fn(() => null),
    };
    const divTarget = {
      tagName: 'DIV',
      isContentEditable: false,
      closest: vi.fn(() => null),
    };

    expect(isEditableWorkbenchTarget(inputTarget)).toBe(true);
    expect(isEditableWorkbenchTarget(divTarget)).toBe(false);
  });

  it('matches keyboard shortcut descriptors exactly', () => {
    const event = {
      key: 'a',
      altKey: true,
      ctrlKey: false,
      metaKey: false,
      shiftKey: true,
    };

    expect(
      matchWorkbenchShortcut(
        {
          id: 'approve',
          key: 'a',
          altKey: true,
          shiftKey: true,
        },
        event,
      ),
    ).toBe(true);
    expect(
      matchWorkbenchShortcut(
        {
          id: 'approve',
          key: 'a',
          altKey: true,
          shiftKey: false,
        },
        event,
      ),
    ).toBe(false);
  });

  it('creates a closed context-menu state by default', () => {
    expect(createWorkbenchContextMenuState()).toEqual({
      open: false,
      position: null,
      row: null,
    });
  });
});
