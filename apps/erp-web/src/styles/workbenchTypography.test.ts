import { describe, expect, it } from 'vitest';

import { resolveWorkbenchTypographyScale } from './typography';

describe('workbench typography scale', () => {
  it('provides small, medium, and large typography scales for integrated workbench pages only', () => {
    expect(resolveWorkbenchTypographyScale('small')).toEqual({
      body: 11.8,
      bodyStrong: 11.8,
      title: 14,
      titleDisplay: 17,
    });

    expect(resolveWorkbenchTypographyScale('medium')).toEqual({
      body: 12.8,
      bodyStrong: 12.8,
      title: 15,
      titleDisplay: 18,
    });

    expect(resolveWorkbenchTypographyScale('large')).toEqual({
      body: 13.8,
      bodyStrong: 13.8,
      title: 16,
      titleDisplay: 19,
    });
  });
});
