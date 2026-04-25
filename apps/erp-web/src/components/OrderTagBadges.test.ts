import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
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

  it('uses compact square badges for the industrial workbench mark column', () => {
    const cssSource = readFileSync(resolve(__dirname, 'OrderTagBadges.css'), 'utf8');

    expect(cssSource).toContain('gap: 2px;');
    expect(cssSource).toContain('min-height: 16px;');
    expect(cssSource).toContain('padding: 0 4px;');
    expect(cssSource).toContain('border-radius: 2px;');
    expect(cssSource).not.toContain('border-radius: 999px;');
  });
});
