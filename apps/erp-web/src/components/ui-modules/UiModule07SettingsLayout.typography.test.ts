import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('UiModule07SettingsLayout typography', () => {
  it('uses the shared 18px page title and 15px section title scales', () => {
    const cssSource = readFileSync(resolve(__dirname, 'UiModule07SettingsLayout.css'), 'utf8');

    expect(cssSource).toContain('font-size: var(--erp-font-size-title-display);');
    expect(cssSource).toContain('font-size: var(--erp-font-size-title);');
    expect(cssSource).toContain('font-size: var(--erp-font-size-body);');
  });
});
