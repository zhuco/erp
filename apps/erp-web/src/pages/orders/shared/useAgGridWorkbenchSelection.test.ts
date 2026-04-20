import { afterAll, beforeAll, describe, expect, it, vi } from 'vitest';

import {
  suppressAgGridSpaceKeyboardEvent,
  suppressAgGridSpaceKeyboardEventOnKeyUp,
} from './useAgGridWorkbenchSelection';

describe('useAgGridWorkbenchSelection keyboard guards', () => {
  const originalHTMLElement = globalThis.HTMLElement;

  class FakeHTMLElement {
    tagName: string;
    isContentEditable = false;

    constructor(tagName = 'DIV') {
      this.tagName = tagName;
    }

    closest() {
      return null;
    }
  }

  beforeAll(() => {
    globalThis.HTMLElement = FakeHTMLElement as unknown as typeof HTMLElement;
  });

  afterAll(() => {
    globalThis.HTMLElement = originalHTMLElement;
  });

  it('fully consumes non-editable space key events to prevent grid and page scrolling', () => {
    const target = new FakeHTMLElement('DIV');
    const event = {
      code: 'Space',
      key: ' ',
      preventDefault: vi.fn(),
      stopImmediatePropagation: vi.fn(),
      stopPropagation: vi.fn(),
      target,
    };

    expect(suppressAgGridSpaceKeyboardEvent(event as never)).toBe(true);
    expect(event.preventDefault).toHaveBeenCalledTimes(1);
    expect(event.stopPropagation).toHaveBeenCalledTimes(1);
    expect(event.stopImmediatePropagation).toHaveBeenCalledTimes(1);
  });

  it('does not consume editable input space key events', () => {
    const target = new FakeHTMLElement('INPUT');
    const event = {
      code: 'Space',
      key: ' ',
      preventDefault: vi.fn(),
      stopImmediatePropagation: vi.fn(),
      stopPropagation: vi.fn(),
      target,
    };

    expect(suppressAgGridSpaceKeyboardEvent(event as never)).toBe(false);
    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(event.stopPropagation).not.toHaveBeenCalled();
    expect(event.stopImmediatePropagation).not.toHaveBeenCalled();
  });

  it('also consumes non-editable space keyup events to block trailing viewport scroll', () => {
    const target = new FakeHTMLElement('DIV');
    const event = {
      code: 'Space',
      key: ' ',
      preventDefault: vi.fn(),
      stopImmediatePropagation: vi.fn(),
      stopPropagation: vi.fn(),
      target,
    };

    expect(suppressAgGridSpaceKeyboardEventOnKeyUp(event as never)).toBe(true);
    expect(event.preventDefault).toHaveBeenCalledTimes(1);
    expect(event.stopPropagation).toHaveBeenCalledTimes(1);
    expect(event.stopImmediatePropagation).toHaveBeenCalledTimes(1);
  });
});
