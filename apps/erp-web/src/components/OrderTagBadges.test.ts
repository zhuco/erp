import { isValidElement } from 'react';
import { describe, expect, it } from 'vitest';

import { OrderTagBadges } from './OrderTagBadges';

describe('OrderTagBadges', () => {
  it('supports a single-line truncation mode for workbench table cells', () => {
    const element = OrderTagBadges({
      tags: ['加急', '冻结'],
      emptyText: null,
      singleLine: true,
    });

    expect(isValidElement(element)).toBe(true);
    expect(element?.type).toBe('div');
    expect(element?.props.className).toContain('order-tag-badges');
    expect(element?.props.className).toContain('order-tag-badges--single-line');
  });
});
