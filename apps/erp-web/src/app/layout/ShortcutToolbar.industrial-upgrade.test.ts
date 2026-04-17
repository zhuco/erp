import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('ShortcutToolbar industrial upgrade', () => {
  it('uses non-deprecated modal lifecycle props', () => {
    const source = readFileSync(resolve(__dirname, 'ShortcutToolbar.tsx'), 'utf8');

    expect(source).toContain('destroyOnHidden');
    expect(source).not.toContain('destroyOnClose');
  });
});

